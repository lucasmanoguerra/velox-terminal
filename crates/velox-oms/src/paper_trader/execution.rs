//! Order execution logic: OHLC-based fill simulation and bracket lifecycle.

use velox_core::{Fill, NewOrder, Order, OrderId, OrderState, OrderType, Side, TimeInForce};

use super::BracketConfig;
use super::PaperTrader;

impl PaperTrader {
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

        // ── Bracket lifecycle ────────────────────────────────────
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
                let lp = order.price.unwrap_or(f64::MAX);
                match order.side {
                    Side::Buy => low <= lp,
                    Side::Sell => high >= lp,
                }
            }
            OrderType::StopMarket => {
                let sp = order.stop_price.unwrap_or(f64::MAX);
                match order.side {
                    Side::Buy => high >= sp,
                    Side::Sell => low <= sp,
                }
            }
            OrderType::StopLimit => {
                let sp = order.stop_price.unwrap_or(f64::MAX);
                let triggered = match order.side {
                    Side::Buy => high >= sp,
                    Side::Sell => low <= sp,
                };
                if !triggered {
                    return false;
                }
                let lp = order.price.unwrap_or(f64::MAX);
                match order.side {
                    Side::Buy => low <= lp,
                    Side::Sell => high >= lp,
                }
            }
        }
    }
}
