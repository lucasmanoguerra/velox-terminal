//! Order manager — handles order lifecycle.

use std::collections::HashMap;
use velox_core::{Order, OrderId, OrderState, NewOrder, Fill, Side};
use crate::error::OmsError;
use crate::state_machine::validate_transition;
use chrono::Utc;

/// Maximum allowed order quantity.
const MAX_ORDER_QUANTITY: f64 = 1_000_000.0;

/// The central order manager.
///
/// Owns all orders and fills. All state transitions are validated
/// against the state machine before being applied.
pub struct OrderManager {
    orders: HashMap<OrderId, Order>,
    fills: Vec<Fill>,
}

impl OrderManager {
    pub fn new() -> Self {
        Self {
            orders: HashMap::new(),
            fills: Vec::new(),
        }
    }

    /// Submit a new order. Returns the assigned OrderId.
    pub fn submit_order(&mut self, new_order: NewOrder) -> Result<OrderId, OmsError> {
        // Validate quantity
        if new_order.quantity <= 0.0 {
            return Err(OmsError::Rejected("Order quantity must be positive".into()));
        }
        if new_order.quantity > MAX_ORDER_QUANTITY {
            return Err(OmsError::Rejected(format!(
                "Order quantity {} exceeds maximum {}",
                new_order.quantity, MAX_ORDER_QUANTITY
            )));
        }

        let order_id = OrderId::new();
        let now = Utc::now();

        let order = Order {
            order_id,
            symbol: new_order.symbol,
            side: new_order.side,
            order_type: new_order.order_type,
            quantity: new_order.quantity,
            filled_quantity: 0.0,
            avg_fill_price: None,
            price: new_order.price,
            stop_price: new_order.stop_price,
            time_in_force: new_order.time_in_force,
            state: OrderState::PendingNew,
            created_at: now,
            updated_at: now,
            client_order_id: new_order.client_order_id,
            parent_order_id: None,
        };

        self.orders.insert(order_id, order);
        // Transition to New (in practice, this waits for broker ack)
        self.transition_order(order_id, OrderState::New)?;

        Ok(order_id)
    }

    /// Apply a fill to an order.
    pub fn apply_fill(&mut self, fill: Fill) -> Result<(), OmsError> {
        let order = self.orders.get_mut(&fill.order_id)
            .ok_or_else(|| OmsError::OrderNotFound(fill.order_id.0.to_string()))?;

        // Check if order is in a fillable state
        match order.state {
            OrderState::New | OrderState::PartiallyFilled => {},
            OrderState::Filled => return Err(OmsError::Rejected(
                "Cannot fill an already filled order".into())),
            OrderState::Canceled => return Err(OmsError::Rejected(
                "Cannot fill a canceled order".into())),
            OrderState::Rejected => return Err(OmsError::Rejected(
                "Cannot fill a rejected order".into())),
            _ => return Err(OmsError::Rejected(format!(
                "Cannot fill order in state {:?}", order.state))),
        }

        let new_filled = order.filled_quantity + fill.quantity;

        // Check for overfill
        if new_filled > order.quantity {
            return Err(OmsError::Rejected(format!(
                "Fill of {} would overfill order (filled: {}, total: {})",
                fill.quantity, order.filled_quantity, order.quantity
            )));
        }

        // Update order
        order.filled_quantity = new_filled;
        order.avg_fill_price = Some(
            match order.avg_fill_price {
                Some(avg) => ((avg * (new_filled - fill.quantity)) + (fill.price * fill.quantity)) / new_filled,
                None => fill.price,
            }
        );
        order.updated_at = Utc::now();

        // Transition state
        if new_filled >= order.quantity {
            self.transition_order(fill.order_id, OrderState::Filled)?;
        } else {
            self.transition_order(fill.order_id, OrderState::PartiallyFilled)?;
        }

        self.fills.push(fill);
        Ok(())
    }

    /// Cancel an order.
    pub fn cancel_order(&mut self, order_id: OrderId) -> Result<(), OmsError> {
        let order = self.orders.get(&order_id)
            .ok_or_else(|| OmsError::OrderNotFound(order_id.0.to_string()))?;

        // Check if order is already in a terminal state
        match order.state {
            OrderState::Filled => return Err(OmsError::Rejected(
                "Cannot cancel a filled order".into())),
            OrderState::Canceled => return Err(OmsError::Rejected(
                "Order is already canceled".into())),
            OrderState::Rejected => return Err(OmsError::Rejected(
                "Cannot cancel a rejected order".into())),
            OrderState::Expired => return Err(OmsError::Rejected(
                "Cannot cancel an expired order".into())),
            _ => {},
        }

        self.transition_order(order_id, OrderState::PendingCancel)?;
        self.transition_order(order_id, OrderState::Canceled)
    }

    /// Replace/modify an existing order (e.g., change price or quantity).
    pub fn replace_order(
        &mut self,
        order_id: OrderId,
        new_price: Option<f64>,
        new_stop_price: Option<f64>,
        new_quantity: Option<f64>,
    ) -> Result<(), OmsError> {
        let order = self.orders.get(&order_id)
            .ok_or_else(|| OmsError::OrderNotFound(order_id.0.to_string()))?;

        // Only New or PartiallyFilled orders can be replaced
        match order.state {
            OrderState::New | OrderState::PartiallyFilled => {},
            _ => return Err(OmsError::Rejected(format!(
                "Cannot replace order in state {:?}", order.state))),
        }

        // New quantity must be >= filled quantity
        if let Some(qty) = new_quantity {
            if qty <= 0.0 {
                return Err(OmsError::Rejected("Replacement quantity must be positive".into()));
            }
            if qty < order.filled_quantity {
                return Err(OmsError::Rejected(format!(
                    "Replacement quantity {} is less than filled quantity {}",
                    qty, order.filled_quantity
                )));
            }
        }

        // Transition to PendingReplace
        self.transition_order(order_id, OrderState::PendingReplace)?;

        // Apply modifications
        let order = self.orders.get_mut(&order_id).unwrap();
        if let Some(price) = new_price {
            order.price = Some(price);
        }
        if let Some(stop) = new_stop_price {
            order.stop_price = Some(stop);
        }
        if let Some(qty) = new_quantity {
            order.quantity = qty;
        }
        order.updated_at = Utc::now();

        // Transition back to New (replace accepted)
        self.transition_order(order_id, OrderState::New)
    }

    /// Get the net position for a symbol (buys - sells).
    pub fn net_position(&self, symbol: &str) -> f64 {
        let mut position = 0.0;
        for fill in &self.fills {
            if fill.symbol == symbol {
                match fill.side {
                    Side::Buy => position += fill.quantity,
                    Side::Sell => position -= fill.quantity,
                }
            }
        }
        position
    }

    /// Get all orders filtered by state.
    pub fn orders_by_state(&self, state: OrderState) -> Vec<&Order> {
        self.orders.values()
            .filter(|o| o.state == state)
            .collect()
    }

    /// Get a reference to an order.
    pub fn get_order(&self, order_id: &OrderId) -> Option<&Order> {
        self.orders.get(order_id)
    }

    /// Get all orders.
    pub fn all_orders(&self) -> Vec<&Order> {
        self.orders.values().collect()
    }

    /// Get all fills.
    pub fn all_fills(&self) -> &[Fill] {
        &self.fills
    }

    fn transition_order(&mut self, order_id: OrderId, new_state: OrderState) -> Result<(), OmsError> {
        let order = self.orders.get_mut(&order_id)
            .ok_or_else(|| OmsError::OrderNotFound(order_id.0.to_string()))?;

        validate_transition(order.state, new_state)?;
        order.state = new_state;
        order.updated_at = Utc::now();
        Ok(())
    }
}

impl Default for OrderManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use velox_core::{OrderType, TimeInForce};

    fn make_new_order() -> NewOrder {
        NewOrder {
            symbol: "ES".to_string(),
            side: Side::Buy,
            order_type: OrderType::Market,
            quantity: 1.0,
            price: None,
            stop_price: None,
            time_in_force: TimeInForce::Day,
            client_order_id: None,
        }
    }

    fn make_fill(order_id: OrderId, qty: f64, price: f64) -> Fill {
        Fill {
            fill_id: OrderId::new(),
            order_id,
            symbol: "ES".to_string(),
            side: Side::Buy,
            quantity: qty,
            price,
            timestamp: Utc::now(),
        }
    }

    fn make_fill_for_order(order: &Order, qty: f64, price: f64) -> Fill {
        Fill {
            fill_id: OrderId::new(),
            order_id: order.order_id,
            symbol: order.symbol.clone(),
            side: order.side,
            quantity: qty,
            price,
            timestamp: Utc::now(),
        }
    }

    // --- Existing tests ---

    #[test]
    fn test_submit_and_fill() {
        let mut om = OrderManager::new();
        let order_id = om.submit_order(make_new_order()).unwrap();

        let fill = make_fill(order_id, 1.0, 4500.0);
        om.apply_fill(fill).unwrap();

        let order = om.get_order(&order_id).unwrap();
        assert_eq!(order.state, OrderState::Filled);
        assert_eq!(order.filled_quantity, 1.0);
    }

    #[test]
    fn test_partial_fill() {
        let mut om = OrderManager::new();
        let mut new_order = make_new_order();
        new_order.quantity = 10.0;
        let order_id = om.submit_order(new_order).unwrap();

        for i in 0..5 {
            let fill = make_fill(order_id, 2.0, 4500.0 + i as f64);
            om.apply_fill(fill).unwrap();

            let order = om.get_order(&order_id).unwrap();
            if i < 4 {
                assert_eq!(order.state, OrderState::PartiallyFilled);
            } else {
                assert_eq!(order.state, OrderState::Filled);
            }
        }
    }

    #[test]
    fn test_overfill_rejected() {
        let mut om = OrderManager::new();
        let order_id = om.submit_order(make_new_order()).unwrap();

        om.apply_fill(make_fill(order_id, 1.0, 4500.0)).unwrap();
        let result = om.apply_fill(make_fill(order_id, 1.0, 4501.0));
        assert!(result.is_err());
    }

    // --- New edge case tests ---

    #[test]
    fn test_replace_order_price() {
        let mut om = OrderManager::new();
        let mut new_order = make_new_order();
        new_order.order_type = OrderType::Limit;
        new_order.price = Some(4500.0);
        let order_id = om.submit_order(new_order).unwrap();

        om.replace_order(order_id, Some(4510.0), None, None).unwrap();

        let order = om.get_order(&order_id).unwrap();
        assert_eq!(order.price, Some(4510.0));
        assert_eq!(order.state, OrderState::New);
    }

    #[test]
    fn test_replace_order_quantity() {
        let mut om = OrderManager::new();
        let mut new_order = make_new_order();
        new_order.quantity = 10.0;
        let order_id = om.submit_order(new_order).unwrap();

        om.replace_order(order_id, None, None, Some(15.0)).unwrap();

        let order = om.get_order(&order_id).unwrap();
        assert_eq!(order.quantity, 15.0);
    }

    #[test]
    fn test_replace_filled_order_rejected() {
        let mut om = OrderManager::new();
        let order_id = om.submit_order(make_new_order()).unwrap();
        om.apply_fill(make_fill(order_id, 1.0, 4500.0)).unwrap();

        let result = om.replace_order(order_id, Some(4510.0), None, None);
        assert!(result.is_err(), "Should reject replace of filled order");
    }

    #[test]
    fn test_replace_quantity_less_than_filled() {
        let mut om = OrderManager::new();
        let mut new_order = make_new_order();
        new_order.quantity = 10.0;
        let order_id = om.submit_order(new_order).unwrap();

        om.apply_fill(make_fill(order_id, 5.0, 4500.0)).unwrap();

        let result = om.replace_order(order_id, None, None, Some(3.0));
        assert!(result.is_err(), "Should reject reduce below filled qty");
    }

    #[test]
    fn test_cancel_after_fill_fails() {
        let mut om = OrderManager::new();
        let order_id = om.submit_order(make_new_order()).unwrap();
        om.apply_fill(make_fill(order_id, 1.0, 4500.0)).unwrap();

        let result = om.cancel_order(order_id);
        assert!(result.is_err(), "Should reject cancel of filled order");
    }

    #[test]
    fn test_cancel_pending_order() {
        let mut om = OrderManager::new();
        let order_id = om.submit_order(make_new_order()).unwrap();

        // Cancel after submission (it's in New state)
        om.cancel_order(order_id).unwrap();
        let order = om.get_order(&order_id).unwrap();
        assert_eq!(order.state, OrderState::Canceled);
    }

    #[test]
    fn test_double_cancel_fails() {
        let mut om = OrderManager::new();
        let order_id = om.submit_order(make_new_order()).unwrap();
        om.cancel_order(order_id).unwrap();

        let result = om.cancel_order(order_id);
        assert!(result.is_err(), "Double cancel should fail");
    }

    #[test]
    fn test_invalid_quantity_rejected() {
        let mut om = OrderManager::new();
        let mut new_order = make_new_order();

        // Zero quantity
        new_order.quantity = 0.0;
        assert!(om.submit_order(new_order.clone()).is_err());

        // Negative quantity
        new_order.quantity = -1.0;
        assert!(om.submit_order(new_order).is_err());
    }

    #[test]
    fn test_net_position_calculation() {
        let mut om = OrderManager::new();

        // Buy 2 ES
        let mut buy = make_new_order();
        buy.symbol = "ES".to_string();
        buy.side = Side::Buy;
        buy.quantity = 2.0;
        let buy_order = om.submit_order(buy).unwrap();
        let buy_fill = make_fill(buy_order, 2.0, 4500.0);
        om.apply_fill(buy_fill).unwrap();

        // Sell 1 ES
        let mut sell = make_new_order();
        sell.symbol = "ES".to_string();
        sell.side = Side::Sell;
        sell.quantity = 1.0;
        let sell_order = om.submit_order(sell).unwrap();
        // Use make_fill_for_order to get Sell-side fill
        let sell_fill = {
            let order = om.get_order(&sell_order).unwrap();
            make_fill_for_order(order, 1.0, 4510.0)
        };
        om.apply_fill(sell_fill).unwrap();

        assert_eq!(om.net_position("ES"), 1.0);
        assert_eq!(om.net_position("NQ"), 0.0);
    }

    #[test]
    fn test_orders_by_state() {
        let mut om = OrderManager::new();
        let id1 = om.submit_order(make_new_order()).unwrap();
        let id2 = om.submit_order(make_new_order()).unwrap();

        om.cancel_order(id1).unwrap();

        let canceled = om.orders_by_state(OrderState::Canceled);
        assert_eq!(canceled.len(), 1);
        assert_eq!(canceled[0].order_id, id1);

        let active = om.orders_by_state(OrderState::New);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].order_id, id2);
    }

    #[test]
    fn test_fill_on_canceled_rejected() {
        let mut om = OrderManager::new();
        let order_id = om.submit_order(make_new_order()).unwrap();
        om.cancel_order(order_id).unwrap();

        let result = om.apply_fill(make_fill(order_id, 1.0, 4500.0));
        assert!(result.is_err(), "Fill on canceled order should fail");
    }

    // --- Property-based tests ---

    use proptest::prelude::*;

    prop_compose! {
        fn arb_new_order()(symbol in "ES|NQ|CL", side in 0..2usize, qty in 1.0f64..100.0) -> NewOrder {
            NewOrder {
                symbol,
                side: if side == 0 { Side::Buy } else { Side::Sell },
                order_type: OrderType::Market,
                quantity: qty,
                price: None,
                stop_price: None,
                time_in_force: TimeInForce::Day,
                client_order_id: None,
            }
        }
    }

    proptest! {
        /// Property: Applying fills that exactly sum to the order quantity
        /// should result in a Filled order with the correct avg price.
        #[test]
        fn property_fill_exact_quantity(order in arb_new_order()) {
            let mut om = OrderManager::new();
            let qty = order.quantity;
            let order_id = om.submit_order(order).unwrap();

            // Fill in chunks that sum exactly to qty
            let n_chunks = 3.max((qty / 10.0).ceil() as usize);
            let chunk_size = qty / n_chunks as f64;
            let mut total_filled = 0.0;
            let mut price_sum = 0.0;

            for i in 0..n_chunks {
                let fill_qty = if i == n_chunks - 1 {
                    qty - total_filled
                } else {
                    chunk_size
                };
                let price = 4500.0 + i as f64;
                total_filled += fill_qty;
                price_sum += fill_qty * price;

                let fill = Fill {
                    fill_id: OrderId::new(),
                    order_id,
                    symbol: "ES".to_string(),
                    side: Side::Buy,
                    quantity: fill_qty,
                    price,
                    timestamp: Utc::now(),
                };
                om.apply_fill(fill).unwrap();
            }

            let order = om.get_order(&order_id).unwrap();
            prop_assert_eq!(order.state, OrderState::Filled);
            prop_assert!((order.filled_quantity - qty).abs() < 1e-10);

            // Verify avg fill price
            let expected_avg = price_sum / qty;
            if let Some(avg) = order.avg_fill_price {
                prop_assert!((avg - expected_avg).abs() < 0.01,
                    "Avg price mismatch: got {}, expected {}", avg, expected_avg);
            }
        }

        /// Property: An order's filled_quantity never exceeds its original quantity.
        /// Also: once a fill is rejected (overfill or state error), subsequent
        /// fills must also fail.
        fn property_no_overfill(
            order in arb_new_order(),
            fills in prop::collection::vec((1.0f64..20.0f64, 4500.0f64..4600.0f64), 0..10)
        ) {
            let mut om = OrderManager::new();
            let max_qty = order.quantity;
            let order_id = om.submit_order(order).unwrap();

            let mut total_before: f64 = 0.0;
            let mut seen_error = false;

            for (fill_qty, fill_price) in &fills {
                let fq = *fill_qty;
                let fp = *fill_price;

                let fill = Fill {
                    fill_id: OrderId::new(),
                    order_id,
                    symbol: "ES".to_string(),
                    side: Side::Buy,
                    quantity: fq,
                    price: fp,
                    timestamp: Utc::now(),
                };

                let result = om.apply_fill(fill);
                let order = om.get_order(&order_id).unwrap();

                // Invariant: filled_quantity never exceeds original quantity
                prop_assert!(order.filled_quantity <= max_qty,
                    "filled_quantity {} exceeds max qty {}",
                    order.filled_quantity, max_qty);

                if seen_error {
                    // Once an error occurred, all subsequent fills should fail
                    prop_assert!(result.is_err(),
                        "Subsequent fill should fail after first error");
                }

                if result.is_err() {
                    // On error, filled_quantity must not increase
                    prop_assert!(order.filled_quantity <= total_before,
                        "filled_quantity increased despite error: {} > {}",
                        order.filled_quantity, total_before);
                    seen_error = true;
                } else {
                    // On success, track the cumulative filled quantity
                    total_before = order.filled_quantity;
                }
            }
        }
    }
}
