//! Order manager — handles order lifecycle.

use std::collections::HashMap;
use velox_core::{Order, OrderId, OrderState, NewOrder, Fill, Side};
use crate::error::OmsError;
use crate::state_machine::validate_transition;
use chrono::Utc;

/// The central order manager.
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

        let new_filled = order.filled_quantity + fill.quantity;

        // Check for overfill
        if new_filled > order.quantity {
            return Err(OmsError::Rejected("Fill exceeds order quantity".into()));
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
        self.transition_order(order_id, OrderState::PendingCancel)?;
        self.transition_order(order_id, OrderState::Canceled)
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
    use velox_core::{OrderType, TimeInForce, Side};

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

    #[test]
    fn test_submit_and_fill() {
        let mut om = OrderManager::new();
        let order_id = om.submit_order(make_new_order()).unwrap();

        let fill = Fill {
            fill_id: OrderId::new(),
            order_id,
            symbol: "ES".to_string(),
            side: Side::Buy,
            quantity: 1.0,
            price: 4500.0,
            timestamp: Utc::now(),
        };
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
            let fill = Fill {
                fill_id: OrderId::new(),
                order_id,
                symbol: "ES".to_string(),
                side: Side::Buy,
                quantity: 2.0,
                price: 4500.0 + i as f64,
                timestamp: Utc::now(),
            };
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

        let fill1 = Fill {
            fill_id: OrderId::new(),
            order_id,
            symbol: "ES".to_string(),
            side: Side::Buy,
            quantity: 1.0,
            price: 4500.0,
            timestamp: Utc::now(),
        };
        om.apply_fill(fill1).unwrap();

        let fill2 = Fill {
            fill_id: OrderId::new(),
            order_id,
            symbol: "ES".to_string(),
            side: Side::Buy,
            quantity: 1.0,
            price: 4501.0,
            timestamp: Utc::now(),
        };
        assert!(om.apply_fill(fill2).is_err());
    }
}
