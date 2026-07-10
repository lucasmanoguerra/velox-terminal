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

pub mod types;
pub(crate) use types::BracketConfig;

mod execution;
mod position;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use velox_core::{
    AccountInfo, NewOrder, Order, OrderId, OrderState, Side,
};

use crate::OrderManager;

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
            order_type: velox_core::OrderType::Market,
            quantity,
            price: None,
            stop_price: None,
            time_in_force: velox_core::TimeInForce::Day,
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

    // ── Accessors ──────────────────────────────────────────────────

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
}
