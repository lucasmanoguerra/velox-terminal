//! Event types for Binance User Data Stream.
//!
//! Types map to Binance WebSocket event payloads:
//! - `outboundAccountPosition` → `AccountUpdateEvent`
//! - `executionReport` → `OrderUpdateEvent`
//! - `balanceUpdate` → `BalanceUpdateEvent`
//! - `listStatus` → `ListStatusEvent`

/// A single asset balance snapshot from an account update.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetBalance {
    pub asset: String,
    pub free: f64,
    pub locked: f64,
}

/// Account update event (`outboundAccountPosition`).
///
/// Contains the full balance snapshot at a given event time.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountUpdateEvent {
    pub event_time: i64,
    pub balances: Vec<AssetBalance>,
}

/// Order update event (`executionReport`).
///
/// Fired on every order state transition: new, partial fill, fill, cancel, reject, expire.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderUpdateEvent {
    pub event_time: i64,
    pub symbol: String,
    pub client_order_id: String,
    pub order_id: i64,
    pub side: String,
    pub order_type: String,
    pub time_in_force: String,
    pub orig_qty: f64,
    pub cum_filled_qty: f64,
    pub cum_quote_qty: f64,
    pub last_filled_qty: f64,
    pub last_filled_price: f64,
    pub commission: Option<f64>,
    pub commission_asset: Option<String>,
    pub current_status: String,
    pub is_on_book: bool,
    pub is_maker: bool,
}

/// Balance update event (`balanceUpdate`).
///
/// Fired when a single asset's balance changes (e.g. deposit, withdrawal, transfer).
#[derive(Debug, Clone, PartialEq)]
pub struct BalanceUpdateEvent {
    pub event_time: i64,
    pub asset: String,
    pub delta: f64,
}

/// List status event (`listStatus`).
///
/// Fired when an OCO order list changes state.
#[derive(Debug, Clone, PartialEq)]
pub struct ListStatusEvent {
    pub event_time: i64,
    pub symbol: String,
    pub order_list_id: i64,
    pub list_status: String,
    pub list_order_status: String,
}

/// Union of all possible user data stream events.
#[derive(Debug, Clone, PartialEq)]
pub enum UserDataEvent {
    AccountUpdate(AccountUpdateEvent),
    OrderUpdate(OrderUpdateEvent),
    BalanceUpdate(BalanceUpdateEvent),
    ListStatus(ListStatusEvent),
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Ensure all event types have Debug and PartialEq for test assertions.
    #[test]
    fn test_asset_balance_debug() {
        let bal = AssetBalance { asset: "BTC".into(), free: 1.0, locked: 0.5 };
        let debug = format!("{bal:?}");
        assert!(debug.contains("BTC"));
        assert!(debug.contains("1.0"));
    }

    #[test]
    fn test_account_update_event_eq() {
        let a = AccountUpdateEvent { event_time: 100, balances: vec![] };
        let b = AccountUpdateEvent { event_time: 100, balances: vec![] };
        assert_eq!(a, b);
    }

    #[test]
    fn test_user_data_event_variants() {
        let acc = UserDataEvent::AccountUpdate(AccountUpdateEvent {
            event_time: 1, balances: vec![],
        });
        let ord = UserDataEvent::OrderUpdate(OrderUpdateEvent {
            event_time: 2, symbol: "BTCUSDT".into(),
            client_order_id: "c".into(), order_id: 1,
            side: "BUY".into(), order_type: "MARKET".into(),
            time_in_force: "IOC".into(), orig_qty: 0.01,
            cum_filled_qty: 0.01, cum_quote_qty: 450.0,
            last_filled_qty: 0.01, last_filled_price: 45000.0,
            commission: None, commission_asset: None,
            current_status: "FILLED".into(), is_on_book: false, is_maker: false,
        });
        let bal = UserDataEvent::BalanceUpdate(BalanceUpdateEvent {
            event_time: 3, asset: "USDT".into(), delta: 100.0,
        });
        let lst = UserDataEvent::ListStatus(ListStatusEvent {
            event_time: 4, symbol: "BTCUSDT".into(),
            order_list_id: 123, list_status: "EXECUTING".into(),
            list_order_status: "STARTED".into(),
        });

        assert!(matches!(acc, UserDataEvent::AccountUpdate(_)));
        assert!(matches!(ord, UserDataEvent::OrderUpdate(_)));
        assert!(matches!(bal, UserDataEvent::BalanceUpdate(_)));
        assert!(matches!(lst, UserDataEvent::ListStatus(_)));
    }
}
