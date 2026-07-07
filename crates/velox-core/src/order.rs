//! Order types — market, limit, stop, stop-limit, bracket.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique order identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderId(pub Uuid);

impl OrderId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for OrderId {
    fn default() -> Self {
        Self::new()
    }
}

/// Side of the order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

/// Order type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
    StopMarket,
    StopLimit,
}

/// Time in force.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    Day,
    Gtc,
    Ioc,
    Fok,
    Gtd,
}

/// Current state of an order in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderState {
    PendingNew,
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    Expired,
    PendingCancel,
    PendingReplace,
    Stopped,
}

/// A new order request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewOrder {
    pub symbol: String,
    pub side: Side,
    pub order_type: OrderType,
    pub quantity: f64,
    pub price: Option<f64>,
    pub stop_price: Option<f64>,
    pub time_in_force: TimeInForce,
    pub client_order_id: Option<String>,
}

/// An active order in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_id: OrderId,
    pub symbol: String,
    pub side: Side,
    pub order_type: OrderType,
    pub quantity: f64,
    pub filled_quantity: f64,
    pub avg_fill_price: Option<f64>,
    pub price: Option<f64>,
    pub stop_price: Option<f64>,
    pub time_in_force: TimeInForce,
    pub state: OrderState,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub client_order_id: Option<String>,
    pub parent_order_id: Option<OrderId>,
}

/// A fill (partial or full) for an order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub fill_id: OrderId,
    pub order_id: OrderId,
    pub symbol: String,
    pub side: Side,
    pub quantity: f64,
    pub price: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
