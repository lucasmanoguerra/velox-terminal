//! Paper trading engine — wraps OrderManager with mock execution and position tracking.
//!
//! # Flow
//!
//! 1. User clicks Buy/Sell → [`submit_market_order`] → Order created in `New` state.
//! 2. Each frame, [`execute_open_orders`] checks for `New` market orders and fills them
//!    at the latest candle close price.
//! 3. [`positions`] computes net positions + P&L from the fill history on-the-fly.
//! 4. [`update_account`] refreshes equity / buying power from position P&L.

use std::collections::HashMap;
use velox_core::{
    AccountInfo, Fill, NewOrder, Order, OrderId, OrderState, OrderType, Position, Side,
    TimeInForce,
};

use crate::OrderManager;

/// A paper trading engine that wraps [`OrderManager`] with auto-fill for market orders.
pub struct PaperTrader {
    order_manager: OrderManager,
    account: AccountInfo,
    last_prices: HashMap<String, f64>,
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
        }
    }

    /// Record the latest price for a symbol (called each frame).
    pub fn update_price(&mut self, symbol: &str, price: f64) {
        self.last_prices.insert(symbol.to_string(), price);
    }

    /// Submit a market order. Returns `Ok(OrderId)` on success.
    pub fn submit_market_order(
        &mut self,
        symbol: &str,
        side: Side,
        quantity: f64,
    ) -> Result<OrderId, String> {
        if quantity <= 0.0 {
            return Err("Quantity must be positive".to_string());
        }
        let new_order = NewOrder {
            symbol: symbol.to_string(),
            side,
            order_type: OrderType::Market,
            quantity,
            price: None,
            stop_price: None,
            time_in_force: TimeInForce::Day,
            client_order_id: None,
        };
        self.order_manager
            .submit_order(new_order)
            .map_err(|e| format!("{e}"))
    }

    /// Cancel an open order.
    pub fn cancel_order(&mut self, order_id: OrderId) -> Result<(), String> {
        self.order_manager
            .cancel_order(order_id)
            .map_err(|e| format!("{e}"))
    }

    /// Execute all open market orders for `symbol` at `price`.
    ///
    /// Should be called after new candle data arrives.
    /// Returns the number of orders filled.
    pub fn execute_open_orders(&mut self, symbol: &str, price: f64) -> usize {
        self.last_prices.insert(symbol.to_string(), price);

        let orders: Vec<Order> = self
            .order_manager
            .all_orders()
            .into_iter()
            .filter(|o| {
                o.symbol == symbol
                    && o.state == OrderState::New
                    && o.order_type == OrderType::Market
            })
            .cloned()
            .collect();

        let mut filled_count = 0;
        for order in &orders {
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
                price,
                timestamp: chrono::Utc::now(),
            };

            match self.order_manager.apply_fill(fill) {
                Ok(()) => filled_count += 1,
                Err(e) => {
                    tracing::warn!("Failed to fill order {}: {e}", order.order_id.0);
                }
            }
        }

        if filled_count > 0 {
            self.update_account();
        }

        filled_count
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

    #[test]
    fn test_submit_and_execute_market_buy() {
        let mut pt = PaperTrader::new(100_000.0);
        let symbol = "BTC/USDT";

        pt.submit_market_order(symbol, Side::Buy, 1.0).unwrap();
        assert_eq!(pt.open_orders().len(), 1);

        // Execute at price 50000
        let filled = pt.execute_open_orders(symbol, 50000.0);
        assert_eq!(filled, 1);
        assert_eq!(pt.open_orders().len(), 0);
        assert_eq!(pt.net_position(symbol), 1.0);
    }

    #[test]
    fn test_submit_and_execute_market_sell() {
        let mut pt = PaperTrader::new(100_000.0);
        let symbol = "ETH/USDT";

        pt.submit_market_order(symbol, Side::Sell, 2.0).unwrap();
        let filled = pt.execute_open_orders(symbol, 3000.0);
        assert_eq!(filled, 1);
        assert_eq!(pt.net_position(symbol), -2.0);
    }

    #[test]
    fn test_multiple_fills_position_averaging() {
        let mut pt = PaperTrader::new(100_000.0);
        let symbol = "SOL/USDT";

        // Buy 10 at 20
        pt.submit_market_order(symbol, Side::Buy, 10.0).unwrap();
        pt.execute_open_orders(symbol, 20.0);

        // Buy 5 more at 30
        pt.submit_market_order(symbol, Side::Buy, 5.0).unwrap();
        pt.execute_open_orders(symbol, 30.0);

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
        pt.execute_open_orders(symbol, 50000.0);

        // Sell 1 at 51000 → realize +1000
        pt.submit_market_order(symbol, Side::Sell, 1.0).unwrap();
        pt.execute_open_orders(symbol, 51000.0);

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
        pt.execute_open_orders(symbol, 50000.0);

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
}
