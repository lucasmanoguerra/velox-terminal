//! Paper trading engine — wraps OrderManager with mock execution and position tracking.
//!
//! # Flow
//!
//! 1. User clicks Buy/Sell → [`submit_order`] → Order created in `New` state.
//! 2. Each frame, [`execute_open_orders`] checks for open orders and fills them
//!    according to their type (market fills at close, limit at limit price, stop at
//!    market when triggered).
//! 3. [`positions`] computes net positions + P&L from the fill history on-the-fly.
//! 4. [`update_account`] refreshes equity / buying power from position P&L.

use std::collections::HashMap;
use velox_core::{
    AccountInfo, Fill, NewOrder, Order, OrderId, OrderState, OrderType, Position, Side,
    TimeInForce,
};

use crate::OrderManager;

/// Bracket configuration stored for an entry order.
#[derive(Debug, Clone)]
struct BracketConfig {
    take_profit_price: f64,
    stop_loss_price: f64,
}

/// A paper trading engine that wraps [`OrderManager`] with auto-fill for all order types.
///
/// Supports bracket orders: when an entry order with `take_profit_price` and
/// `stop_loss_price` is filled, TP (Limit) and SL (StopMarket) child orders
/// are automatically created. When either child fills, the sibling is canceled.
pub struct PaperTrader {
    order_manager: OrderManager,
    account: AccountInfo,
    last_prices: HashMap<String, f64>,
    /// Stores bracket configs keyed by entry order ID.
    /// Removed when children are created.
    bracket_configs: HashMap<OrderId, BracketConfig>,
}

impl PaperTrader {
    /// Create a new paper trader with the given initial cash balance.
    pub fn new(initial_cash: f64) -> Self {
        Self {
            order_manager: OrderManager::new(),
            account: AccountInfo {
                cash: initial_cash,
                buying_power: initial_cash * 2.0,
                equity: initial_cash,
                margin_used: 0.0,
                unrealized_pnl: 0.0,
                realized_pnl: 0.0,
                currency: "USD".to_string(),
            },
            last_prices: HashMap::new(),
            bracket_configs: HashMap::new(),
        }
    }

    /// Record the latest price for a symbol (called each frame).
    pub fn update_price(&mut self, symbol: &str, price: f64) {
        self.last_prices.insert(symbol.to_string(), price);
    }

    /// Submit an order of any type. Returns `Ok(OrderId)` on success.
    ///
    /// If the order has bracket parameters (`take_profit_price` and `stop_loss_price`),
    /// they are stored and TP/SL children are auto-created when the entry fills.
    pub fn submit_order(&mut self, order: NewOrder) -> Result<OrderId, String> {
        if order.quantity <= 0.0 {
            return Err("Quantity must be positive".to_string());
        }

        // Extract bracket prices before consuming `order`
        let bracket = match (order.take_profit_price, order.stop_loss_price) {
            (Some(tp), Some(sl)) if tp > 0.0 && sl > 0.0 => {
                Some(BracketConfig {
                    take_profit_price: tp,
                    stop_loss_price: sl,
                })
            }
            _ => None,
        };

        let entry_id = self
            .order_manager
            .submit_order(order)
            .map_err(|e| format!("{e}"))?;

        // Store bracket config to auto-create children on fill
        if let Some(config) = bracket {
            self.bracket_configs.insert(entry_id, config);
        }

        Ok(entry_id)
    }

    /// Submit a market order (convenience). Returns `Ok(OrderId)` on success.
    pub fn submit_market_order(
        &mut self,
        symbol: &str,
        side: Side,
        quantity: f64,
    ) -> Result<OrderId, String> {
        let new_order = NewOrder {
            symbol: symbol.to_string(),
            side,
            order_type: OrderType::Market,
            quantity,
            price: None,
            stop_price: None,
            time_in_force: TimeInForce::Day,
            client_order_id: None,
            take_profit_price: None,
            stop_loss_price: None,
        };
        self.submit_order(new_order)
    }

    /// Cancel an open order.
    pub fn cancel_order(&mut self, order_id: OrderId) -> Result<(), String> {
        self.order_manager
            .cancel_order(order_id)
            .map_err(|e| format!("{e}"))
    }

    /// Execute open orders for `symbol` using OHLC data from the latest candle.
    ///
    /// - **Market**: fills immediately at `close`.
    /// - **Limit**: fills at `order.price` when price trades through the limit level.
    /// - **StopMarket**: fills at `close` when price trades through the stop level.
    /// - **StopLimit**: fills at `order.price` when price trades through the stop level.
    ///
    /// Also handles bracket lifecycle:
    /// - When an entry order with bracket config fills, TP and SL children are created.
    /// - When a TP or SL child fills, its sibling is canceled.
    ///
    /// Should be called after new candle data arrives.
    /// Returns the number of orders filled.
    pub fn execute_open_orders(
        &mut self,
        symbol: &str,
        close: f64,
        high: f64,
        low: f64,
    ) -> usize {
        self.last_prices.insert(symbol.to_string(), close);

        let orders: Vec<Order> = self
            .order_manager
            .all_orders()
            .into_iter()
            .filter(|o| o.symbol == symbol && o.state == OrderState::New)
            .cloned()
            .collect();

        let mut filled_count = 0;
        let mut filled_ids: Vec<OrderId> = Vec::new();
        for order in &orders {
            let should_fill = Self::should_fill_order(order, high, low);
            if !should_fill {
                continue;
            }

            let fill_price = match order.order_type {
                OrderType::Market | OrderType::StopMarket => close,
                OrderType::Limit | OrderType::StopLimit => {
                    order.price.unwrap_or(close)
                }
            };

            let remaining = order.quantity - order.filled_quantity;
            if remaining <= 0.0 {
                continue;
            }

            let fill = Fill {
                fill_id: OrderId::new(),
                order_id: order.order_id,
                symbol: order.symbol.clone(),
                side: order.side,
                quantity: remaining,
                price: fill_price,
                timestamp: chrono::Utc::now(),
            };

            match self.order_manager.apply_fill(fill) {
                Ok(()) => {
                    filled_count += 1;
                    filled_ids.push(order.order_id);
                }
                Err(e) => {
                    tracing::warn!("Failed to fill order {}: {e}", order.order_id.0);
                }
            }
        }

        if filled_count > 0 {
            self.update_account();
        }

        // ── Bracket lifecycle ────────────────────────────────────────
        for &filled_id in &filled_ids {
            // 1. If this is a bracket entry, create TP and SL children
            if let Some(config) = self.bracket_configs.remove(&filled_id) {
                self.create_bracket_children(filled_id, &config);
            }

            // 2. If this is a child order (TP/SL), cancel its sibling
            let filled_order = self.order_manager.get_order(&filled_id);
            if let Some(parent_id) = filled_order.and_then(|o| o.parent_order_id) {
                self.cancel_sibling_orders(filled_id, parent_id);
            }
        }

        filled_count
    }

    /// Create TP (Limit) and SL (StopMarket) child orders for a filled entry.
    fn create_bracket_children(&mut self, entry_id: OrderId, config: &BracketConfig) {
        let entry = match self.order_manager.get_order(&entry_id) {
            Some(o) => o.clone(),
            None => {
                tracing::warn!("Bracket entry {:.8} not found", entry_id.0);
                return;
            }
        };

        // Determine opposite side for TP and SL
        let opposite_side = match entry.side {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy,
        };

        let filled_qty = entry.filled_quantity;

        if filled_qty <= 0.0 {
            return;
        }

        // TP: Limit order at take_profit_price (opposite side)
        let tp_order = NewOrder {
            symbol: entry.symbol.clone(),
            side: opposite_side,
            order_type: OrderType::Limit,
            quantity: filled_qty,
            price: Some(config.take_profit_price),
            stop_price: None,
            time_in_force: TimeInForce::Day,
            client_order_id: None,
            take_profit_price: None,
            stop_loss_price: None,
        };
        if let Ok(tp_id) = self.order_manager.submit_order_with_parent(tp_order, entry_id) {
            tracing::debug!(
                "Created TP child {:.8} for entry {:.8} at {}",
                tp_id.0,
                entry_id.0,
                config.take_profit_price
            );
        }

        // SL: StopMarket order at stop_loss_price (opposite side)
        let sl_order = NewOrder {
            symbol: entry.symbol.clone(),
            side: opposite_side,
            order_type: OrderType::StopMarket,
            quantity: filled_qty,
            price: None,
            stop_price: Some(config.stop_loss_price),
            time_in_force: TimeInForce::Day,
            client_order_id: None,
            take_profit_price: None,
            stop_loss_price: None,
        };
        if let Ok(sl_id) = self.order_manager.submit_order_with_parent(sl_order, entry_id) {
            tracing::debug!(
                "Created SL child {:.8} for entry {:.8} at {}",
                sl_id.0,
                entry_id.0,
                config.stop_loss_price
            );
        }
    }

    /// Cancel all sibling orders of `filled_child_id` that share `parent_id`.
    fn cancel_sibling_orders(&mut self, filled_child_id: OrderId, parent_id: OrderId) {
        let siblings = self.order_manager.child_order_ids(parent_id);
        for sibling_id in siblings {
            if sibling_id == filled_child_id {
                continue;
            }
            if let Err(e) = self.order_manager.cancel_order(sibling_id) {
                tracing::warn!(
                    "Failed to cancel sibling {:.8} of {:.8}: {e}",
                    sibling_id.0,
                    filled_child_id.0
                );
            } else {
                tracing::debug!(
                    "Canceled sibling {:.8} (filled child {:.8})",
                    sibling_id.0,
                    filled_child_id.0
                );
            }
        }
    }

    /// Determine whether an order should be filled given the current OHLC bar.
    fn should_fill_order(order: &Order, high: f64, low: f64) -> bool {
        match order.order_type {
            OrderType::Market => true,
            OrderType::Limit => {
                // Buy limit fills if price dips to limit or below
                // Sell limit fills if price rises to limit or above
                let lp = order.price.unwrap_or(f64::MAX);
                match order.side {
                    Side::Buy => low <= lp,
                    Side::Sell => high >= lp,
                }
            }
            OrderType::StopMarket => {
                // Buy stop triggers if price rises to stop or above
                // Sell stop triggers if price falls to stop or below
                let sp = order.stop_price.unwrap_or(f64::MAX);
                match order.side {
                    Side::Buy => high >= sp,
                    Side::Sell => low <= sp,
                }
            }
            OrderType::StopLimit => {
                // StopLossLimit: stop is triggered first, then limit fill
                let sp = order.stop_price.unwrap_or(f64::MAX);
                let triggered = match order.side {
                    Side::Buy => high >= sp,
                    Side::Sell => low <= sp,
                };
                if !triggered {
                    return false;
                }
                // Once triggered, check limit price condition
                let lp = order.price.unwrap_or(f64::MAX);
                match order.side {
                    Side::Buy => low <= lp,
                    Side::Sell => high >= lp,
                }
            }
        }
    }

    // ── Accessors ──────────────────────────────────────────────────────

    /// All orders (any state).
    pub fn orders(&self) -> Vec<&Order> {
        self.order_manager.all_orders()
    }

    /// Orders that are still live.
    pub fn open_orders(&self) -> Vec<&Order> {
        self.order_manager
            .all_orders()
            .into_iter()
            .filter(|o| {
                matches!(
                    o.state,
                    OrderState::New
                        | OrderState::PendingNew
                        | OrderState::PartiallyFilled
                        | OrderState::PendingCancel
                        | OrderState::PendingReplace
                )
            })
            .collect()
    }

    /// Orders in a terminal state.
    pub fn closed_orders(&self) -> Vec<&Order> {
        self.order_manager
            .all_orders()
            .into_iter()
            .filter(|o| {
                matches!(
                    o.state,
                    OrderState::Filled
                        | OrderState::Canceled
                        | OrderState::Rejected
                        | OrderState::Expired
                        | OrderState::Stopped
                )
            })
            .collect()
    }

    /// Look up a specific order.
    pub fn get_order(&self, order_id: &OrderId) -> Option<&Order> {
        self.order_manager.get_order(order_id)
    }

    /// Net position (buys − sells) for a symbol.
    pub fn net_position(&self, symbol: &str) -> f64 {
        self.order_manager.net_position(symbol)
    }

    /// Compute positions per symbol from fill history.
    ///
    /// Uses weighted-average cost basis and tracks realized P&L per symbol.
    pub fn positions(&self) -> Vec<Position> {
        // (net_qty, avg_entry_price, realized_pnl)
        let mut pos_map: HashMap<String, (f64, f64, f64)> = HashMap::new();

        for fill in self.order_manager.all_fills() {
            let entry = pos_map.entry(fill.symbol.clone()).or_insert((0.0, 0.0, 0.0));
            let (qty, avg, realized) = *entry;

            match fill.side {
                Side::Buy => {
                    if qty >= 0.0 {
                        // Adding to or starting a long position
                        let new_qty = qty + fill.quantity;
                        let new_avg = if qty > 0.0 {
                            ((avg * qty) + (fill.price * fill.quantity)) / new_qty
                        } else {
                            fill.price
                        };
                        *entry = (new_qty, new_avg, realized);
                    } else {
                        // Reducing a short position
                        let abs_short = -qty;
                        let reduce = fill.quantity.min(abs_short);
                        let new_realized = realized + (reduce * (avg - fill.price));
                        let remaining = fill.quantity - reduce;
                        if remaining > 0.0 {
                            // Flipped to long
                            *entry = (remaining, fill.price, new_realized);
                        } else {
                            *entry = (qty + fill.quantity, avg, new_realized);
                        }
                    }
                }
                Side::Sell => {
                    if qty <= 0.0 {
                        // Adding to or starting a short position
                        let new_qty = qty - fill.quantity;
                        let new_avg = if qty < 0.0 {
                            ((avg * (-qty)) + (fill.price * fill.quantity)) / (-new_qty)
                        } else {
                            fill.price
                        };
                        *entry = (new_qty, new_avg, realized);
                    } else {
                        // Reducing a long position
                        let reduce = fill.quantity.min(qty);
                        let new_realized = realized + (reduce * (fill.price - avg));
                        let remaining = fill.quantity - reduce;
                        if remaining > 0.0 {
                            // Flipped to short
                            *entry = (-remaining, fill.price, new_realized);
                        } else {
                            *entry = (qty - fill.quantity, avg, new_realized);
                        }
                    }
                }
            }
        }

        let current_price = |sym: &str| -> f64 { self.last_prices.get(sym).copied().unwrap_or(0.0) };

        pos_map
            .into_iter()
            .map(|(symbol, (qty, avg_entry, realized_pnl))| {
                let cp = current_price(&symbol);
                let unrealized = if qty > 0.0 {
                    qty * (cp - avg_entry)
                } else if qty < 0.0 {
                    (-qty) * (avg_entry - cp)
                } else {
                    0.0
                };

                Position {
                    symbol,
                    quantity: qty,
                    avg_entry_price: avg_entry,
                    current_price: cp,
                    unrealized_pnl: unrealized,
                    realized_pnl,
                }
            })
            .collect()
    }

    /// Account snapshot.
    pub fn account(&self) -> &AccountInfo {
        &self.account
    }

    /// Recompute equity / buying power / P&L from current positions.
    pub fn update_account(&mut self) {
        let positions = self.positions(); // borrows self immutably

        let total_unrealized: f64 = positions.iter().map(|p| p.unrealized_pnl).sum();
        let total_realized: f64 = positions.iter().map(|p| p.realized_pnl).sum();

        self.account.unrealized_pnl = total_unrealized;
        self.account.realized_pnl = total_realized;
        self.account.equity = (self.account.cash + total_unrealized + total_realized).max(0.0);
        self.account.buying_power = self.account.equity * 2.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: call execute_open_orders with close=high=low (flat bar).
    fn exec(pt: &mut PaperTrader, symbol: &str, price: f64) -> usize {
        pt.execute_open_orders(symbol, price, price, price)
    }

    #[test]
    fn test_submit_and_execute_market_buy() {
        let mut pt = PaperTrader::new(100_000.0);
        let symbol = "BTC/USDT";

        pt.submit_market_order(symbol, Side::Buy, 1.0).unwrap();
        assert_eq!(pt.open_orders().len(), 1);

        // Execute at price 50000
        let filled = exec(&mut pt, symbol, 50000.0);
        assert_eq!(filled, 1);
        assert_eq!(pt.open_orders().len(), 0);
        assert_eq!(pt.net_position(symbol), 1.0);
    }

    #[test]
    fn test_submit_and_execute_market_sell() {
        let mut pt = PaperTrader::new(100_000.0);
        let symbol = "ETH/USDT";

        pt.submit_market_order(symbol, Side::Sell, 2.0).unwrap();
        let filled = exec(&mut pt, symbol, 3000.0);
        assert_eq!(filled, 1);
        assert_eq!(pt.net_position(symbol), -2.0);
    }

    #[test]
    fn test_multiple_fills_position_averaging() {
        let mut pt = PaperTrader::new(100_000.0);
        let symbol = "SOL/USDT";

        // Buy 10 at 20
        pt.submit_market_order(symbol, Side::Buy, 10.0).unwrap();
        exec(&mut pt, symbol, 20.0);

        // Buy 5 more at 30
        pt.submit_market_order(symbol, Side::Buy, 5.0).unwrap();
        exec(&mut pt, symbol, 30.0);

        let positions = pt.positions();
        let sol_pos = positions.iter().find(|p| p.symbol == symbol).unwrap();
        assert_eq!(sol_pos.quantity, 15.0);
        assert!((sol_pos.avg_entry_price - 23.333).abs() < 0.01);
    }

    #[test]
    fn test_partial_reduce_long_realized_pnl() {
        let mut pt = PaperTrader::new(100_000.0);
        let symbol = "BTC/USDT";

        pt.submit_market_order(symbol, Side::Buy, 2.0).unwrap();
        exec(&mut pt, symbol, 50000.0);

        // Sell 1 at 51000 → realize +1000
        pt.submit_market_order(symbol, Side::Sell, 1.0).unwrap();
        exec(&mut pt, symbol, 51000.0);

        let positions = pt.positions();
        let btc_pos = positions.iter().find(|p| p.symbol == symbol).unwrap();
        assert_eq!(btc_pos.quantity, 1.0);
        assert!((btc_pos.realized_pnl - 1000.0).abs() < 0.01);
    }

    #[test]
    fn test_account_equity_updates() {
        let mut pt = PaperTrader::new(100_000.0);
        let symbol = "BTC/USDT";

        pt.submit_market_order(symbol, Side::Buy, 1.0).unwrap();
        exec(&mut pt, symbol, 50000.0);

        pt.update_account();
        assert!((pt.account().equity - 100_000.0).abs() < 0.01); // same price

        // Price drops to 49000 → unrealized P&L = -1000
        pt.update_price(symbol, 49000.0);
        pt.update_account();
        assert!((pt.account().unrealized_pnl - (-1000.0)).abs() < 0.01);
        assert!((pt.account().equity - 99_000.0).abs() < 0.01);
    }

    #[test]
    fn test_cancel_open_order() {
        let mut pt = PaperTrader::new(100_000.0);
        let symbol = "BTC/USDT";

        let order_id = pt.submit_market_order(symbol, Side::Buy, 1.0).unwrap();
        assert_eq!(pt.open_orders().len(), 1);

        pt.cancel_order(order_id).unwrap();
        assert_eq!(pt.open_orders().len(), 0);
    }

    #[test]
    fn test_zero_quantity_rejected() {
        let mut pt = PaperTrader::new(100_000.0);
        assert!(pt.submit_market_order("BTC/USDT", Side::Buy, 0.0).is_err());
        assert!(pt.submit_market_order("BTC/USDT", Side::Buy, -1.0).is_err());
    }

    // ── Limit & Stop order tests ──────────────────────────────────

    #[test]
    fn test_limit_buy_fills_when_price_dips() {
        let mut pt = PaperTrader::new(100_000.0);
        let sym = "BTC/USDT";

        // Place a buy limit at 49000, current market at 50000
        let order = NewOrder {
            symbol: sym.to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            quantity: 1.0,
            price: Some(49000.0),
            stop_price: None,
            time_in_force: TimeInForce::Day,
            client_order_id: None,
            take_profit_price: None,
            stop_loss_price: None,
        };
        pt.submit_order(order).unwrap();
        assert_eq!(pt.open_orders().len(), 1);

        // Candle dips to 48500 (low) but closes at 49200
        let filled = pt.execute_open_orders(sym, 49200.0, 49300.0, 48500.0);
        assert_eq!(filled, 1);
        assert_eq!(pt.net_position(sym), 1.0);
    }

    #[test]
    fn test_limit_buy_not_filled_if_price_stays_above() {
        let mut pt = PaperTrader::new(100_000.0);
        let sym = "BTC/USDT";

        let order = NewOrder {
            symbol: sym.to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            quantity: 1.0,
            price: Some(49000.0),
            stop_price: None,
            time_in_force: TimeInForce::Day,
            client_order_id: None,
            take_profit_price: None,
            stop_loss_price: None,
        };
        pt.submit_order(order).unwrap();

        // Candle never dips below 49100
        let filled = pt.execute_open_orders(sym, 49500.0, 49800.0, 49100.0);
        assert_eq!(filled, 0);
    }

    #[test]
    fn test_limit_sell_fills_when_price_rises() {
        let mut pt = PaperTrader::new(100_000.0);
        let sym = "ETH/USDT";

        let order = NewOrder {
            symbol: sym.to_string(),
            side: Side::Sell,
            order_type: OrderType::Limit,
            quantity: 1.0,
            price: Some(3100.0),
            stop_price: None,
            time_in_force: TimeInForce::Day,
            client_order_id: None,
            take_profit_price: None,
            stop_loss_price: None,
        };
        pt.submit_order(order).unwrap();

        // Candle rises to 3150
        let filled = pt.execute_open_orders(sym, 3120.0, 3150.0, 3080.0);
        assert_eq!(filled, 1);
        assert_eq!(pt.net_position(sym), -1.0);
    }

    #[test]
    fn test_stop_market_buy_triggers_on_rise() {
        let mut pt = PaperTrader::new(100_000.0);
        let sym = "BTC/USDT";

        // Buy stop at 51000, current market at 50000
        let order = NewOrder {
            symbol: sym.to_string(),
            side: Side::Buy,
            order_type: OrderType::StopMarket,
            quantity: 1.0,
            price: None,
            stop_price: Some(51000.0),
            time_in_force: TimeInForce::Day,
            client_order_id: None,
            take_profit_price: None,
            stop_loss_price: None,
        };
        pt.submit_order(order).unwrap();

        // Candle spikes to 51200 high
        let filled = pt.execute_open_orders(sym, 50800.0, 51200.0, 50000.0);
        assert_eq!(filled, 1);
        // Fill price = close (50800) for StopMarket
        assert_eq!(pt.net_position(sym), 1.0);
    }

    #[test]
    fn test_stop_market_sell_triggers_on_drop() {
        let mut pt = PaperTrader::new(100_000.0);
        let sym = "SOL/USDT";

        let order = NewOrder {
            symbol: sym.to_string(),
            side: Side::Sell,
            order_type: OrderType::StopMarket,
            quantity: 2.0,
            price: None,
            stop_price: Some(18.0),
            time_in_force: TimeInForce::Day,
            client_order_id: None,
            take_profit_price: None,
            stop_loss_price: None,
        };
        pt.submit_order(order).unwrap();

        // Candle drops to 17.5 low
        let filled = pt.execute_open_orders(sym, 18.5, 19.0, 17.5);
        assert_eq!(filled, 1);
        assert_eq!(pt.net_position(sym), -2.0);
    }

    // ── Bracket order tests ──────────────────────────────────────

    #[test]
    fn test_bracket_market_buy_creates_tp_sl() {
        let mut pt = PaperTrader::new(100_000.0);
        let sym = "BTC/USDT";

        // Entry with bracket at 50000
        let order = NewOrder {
            symbol: sym.to_string(),
            side: Side::Buy,
            order_type: OrderType::Market,
            quantity: 1.0,
            price: None,
            stop_price: None,
            time_in_force: TimeInForce::Day,
            client_order_id: None,
            take_profit_price: Some(51000.0),
            stop_loss_price: Some(49000.0),
        };
        let entry_id = pt.submit_order(order).unwrap();

        // Fill entry at 50000
        let filled = exec(&mut pt, sym, 50000.0);
        assert_eq!(filled, 1, "Entry should fill");
        assert_eq!(pt.net_position(sym), 1.0);

        // TP and SL children should have been created
        let orders = pt.closed_orders();
        assert_eq!(orders.len(), 1, "One filled entry");

        // The entry's fill should auto-create TP and SL as child orders
        let tp_sl_orders: Vec<&Order> = pt
            .orders()
            .into_iter()
            .filter(|o| o.parent_order_id == Some(entry_id))
            .collect();
        assert_eq!(tp_sl_orders.len(), 2, "Entry should have 2 bracket children");

        // One should be Limit (TP), one StopMarket (SL)
        let tp = tp_sl_orders.iter().find(|o| o.order_type == OrderType::Limit).unwrap();
        let sl = tp_sl_orders.iter().find(|o| o.order_type == OrderType::StopMarket).unwrap();
        assert_eq!(tp.price, Some(51000.0));
        assert_eq!(sl.stop_price, Some(49000.0));
        assert_eq!(tp.quantity, 1.0);
        assert_eq!(sl.quantity, 1.0);
    }

    #[test]
    fn test_bracket_tp_fill_cancels_sl() {
        let mut pt = PaperTrader::new(100_000.0);
        let sym = "BTC/USDT";

        // Entry with bracket, fill at 50000
        let order = NewOrder {
            symbol: sym.to_string(),
            side: Side::Buy,
            order_type: OrderType::Market,
            quantity: 1.0,
            price: None,
            stop_price: None,
            time_in_force: TimeInForce::Day,
            client_order_id: None,
            take_profit_price: Some(51000.0),
            stop_loss_price: Some(49000.0),
        };
        let entry_id = pt.submit_order(order).unwrap();
        exec(&mut pt, sym, 50000.0);

        // Price rises to 51500 — TP (Limit Sell at 51000) should fill
        let filled = pt.execute_open_orders(sym, 51500.0, 52000.0, 51000.0);
        assert_eq!(filled, 1, "TP should fill");
        assert_eq!(pt.net_position(sym), 0.0, "Position should be flat");

        // SL should be canceled
        let sl = pt
            .orders()
            .into_iter()
            .find(|o| o.order_type == OrderType::StopMarket && o.parent_order_id == Some(entry_id))
            .unwrap();
        assert_eq!(sl.state, OrderState::Canceled, "SL should be canceled after TP fills");
    }

    #[test]
    fn test_bracket_sl_fill_cancels_tp() {
        let mut pt = PaperTrader::new(100_000.0);
        let sym = "SOL/USDT";

        // Short entry with bracket
        let order = NewOrder {
            symbol: sym.to_string(),
            side: Side::Sell,
            order_type: OrderType::Market,
            quantity: 5.0,
            price: None,
            stop_price: None,
            time_in_force: TimeInForce::Day,
            client_order_id: None,
            take_profit_price: Some(15.0),
            stop_loss_price: Some(25.0),
        };
        let entry_id = pt.submit_order(order).unwrap();
        exec(&mut pt, sym, 20.0);

        // Price drops to 14 — TP (Limit Buy at 15) should fill
        let filled = pt.execute_open_orders(sym, 14.0, 16.0, 13.0);
        assert_eq!(filled, 1, "TP should fill");
        assert_eq!(pt.net_position(sym), 0.0, "Position should be flat");

        // SL should be canceled
        let sl = pt
            .orders()
            .into_iter()
            .find(|o| o.order_type == OrderType::StopMarket && o.parent_order_id == Some(entry_id))
            .unwrap();
        assert_eq!(sl.state, OrderState::Canceled, "SL should be canceled after TP fills");
    }

    #[test]
    fn test_bracket_market_buy_no_tp_sl_if_not_set() {
        let mut pt = PaperTrader::new(100_000.0);
        let sym = "BTC/USDT";

        // Regular market order (no bracket) — should NOT create children
        let entry_id = pt.submit_market_order(sym, Side::Buy, 1.0).unwrap();
        exec(&mut pt, sym, 50000.0);

        let children: Vec<&Order> = pt
            .orders()
            .into_iter()
            .filter(|o| o.parent_order_id == Some(entry_id))
            .collect();
        assert_eq!(children.len(), 0, "No bracket children expected");
    }

    #[test]
    fn test_bracket_creates_children_with_parent_id() {
        let mut pt = PaperTrader::new(100_000.0);
        let sym = "ETH/USDT";

        let order = NewOrder {
            symbol: sym.to_string(),
            side: Side::Buy,
            order_type: OrderType::Market,
            quantity: 2.0,
            price: None,
            stop_price: None,
            time_in_force: TimeInForce::Day,
            client_order_id: None,
            take_profit_price: Some(2500.0),
            stop_loss_price: Some(2300.0),
        };
        let entry_id = pt.submit_order(order).unwrap();
        exec(&mut pt, sym, 2400.0);

        let children: Vec<&Order> = pt
            .orders()
            .into_iter()
            .filter(|o| o.parent_order_id == Some(entry_id))
            .collect();
        assert_eq!(children.len(), 2);
        for child in &children {
            assert_eq!(child.quantity, 2.0, "Child should have same qty as entry");
            assert_eq!(child.side, Side::Sell, "Children should be opposite side");
        }
    }

    #[test]
    fn test_stop_market_not_triggered_below_stop() {
        let mut pt = PaperTrader::new(100_000.0);
        let sym = "BTC/USDT";

        let order = NewOrder {
            symbol: sym.to_string(),
            side: Side::Buy,
            order_type: OrderType::StopMarket,
            quantity: 1.0,
            price: None,
            stop_price: Some(51000.0),
            time_in_force: TimeInForce::Day,
            client_order_id: None,
            take_profit_price: None,
            stop_loss_price: None,
        };
        pt.submit_order(order).unwrap();

        // High is 50900, below stop
        let filled = pt.execute_open_orders(sym, 50500.0, 50900.0, 50000.0);
        assert_eq!(filled, 0);
    }
}
