//! Event types for the cross-module Event Bus.
//!
//! These are domain-level event types that flow through the Event Bus
//! (tokio::sync::broadcast in the composition root). Hot-path events
//! (ticks, fills) bypass the bus through dedicated lock-free channels;
//! only summary/notification variants reach the bus.

use crate::order::{OrderId, OrderState};
use crate::types::{AccountInfo, Position};
use serde::{Deserialize, Serialize};

/// Connection state for an exchange or broker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Reconnecting { attempt: u32 },
}

/// Severity level for risk alerts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// A user-initiated command from the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserCommand {
    PlaceOrder {
        symbol: String,
        side: crate::order::Side,
        order_type: crate::order::OrderType,
        quantity: f64,
        price: Option<f64>,
        stop_price: Option<f64>,
    },
    CancelOrder {
        order_id: OrderId,
    },
    CloseAllPositions,
    SwitchTimeframe {
        timeframe_secs: i64,
    },
    ToggleFollowMode,
}

/// Central event type for the Event Bus.
///
/// # Hot Path Bridge
///
/// Critical low-latency events (Tick, Fill) bypass the bus entirely
/// through dedicated lock-free channels (RingBuffer, crossbeam).
/// Only lightweight notification variants are published here for
/// non-critical consumers (UI indicators, logging, alerts).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppEvent {
    /// A new candle has been completed by the aggregator.
    CandleClosed {
        symbol: String,
        timeframe_secs: i64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    },

    /// An order's state has changed (summary — full detail via dedicated channel).
    OrderUpdate {
        order_id: OrderId,
        state: OrderState,
        symbol: String,
    },

    /// A fill has occurred (summary notification).
    FillNotification {
        order_id: OrderId,
        fill_id: OrderId,
        symbol: String,
        quantity: f64,
        price: f64,
    },

    /// Connection state changed.
    ConnectionStatus {
        exchange: String,
        status: ConnectionState,
    },

    /// Account or position update.
    AccountUpdate {
        account: AccountInfo,
        positions: Vec<Position>,
    },

    /// Risk validation alert.
    RiskAlert {
        severity: AlertSeverity,
        rule: String,
        message: String,
    },

    /// User command submitted via UI.
    UserCommand(UserCommand),

    /// System-level events.
    System {
        /// "shutdown" | "config_changed" | "error"
        kind: String,
        message: String,
    },
}
