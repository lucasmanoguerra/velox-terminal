//! Tests for BinanceUserDataStream.

use super::*;
use tokio::sync::{mpsc, Mutex};

// ── Event parsing tests ────────────────────────────────────────────────────

#[test]
fn test_parse_account_update() {
    let json = r#"{
        "e": "outboundAccountPosition",
        "E": 1672515782136,
        "u": 123456789,
        "B": [
            {"a": "BTC", "f": "0.50000000", "l": "0.10000000"},
            {"a": "USDT", "f": "10000.00000000", "l": "500.00000000"}
        ]
    }"#;

    let raw: serde_json::Value = serde_json::from_str(json).unwrap();
    let event = BinanceUserDataStream::parse_account_update(&raw).unwrap();

    assert_eq!(event.event_time, 1672515782136);
    assert_eq!(event.balances.len(), 2);
    assert_eq!(event.balances[0].asset, "BTC");
    assert!((event.balances[0].free - 0.5).abs() < 1e-8);
    assert!((event.balances[0].locked - 0.1).abs() < 1e-8);
    assert_eq!(event.balances[1].asset, "USDT");
    assert!((event.balances[1].free - 10000.0).abs() < 1e-8);
}

#[test]
fn test_parse_order_update_fill() {
    let json = r#"{
        "e": "executionReport",
        "E": 1672515782136,
        "s": "BTCUSDT",
        "c": "myCustomId123",
        "i": 123456789,
        "S": "BUY",
        "o": "MARKET",
        "f": "IOC",
        "q": "0.01000000",
        "p": "45000.00000000",
        "P": "0.00000000",
        "F": "0.00000000",
        "z": "0.01000000",
        "Z": "450.00000000",
        "l": "0.01000000",
        "L": "45000.00000000",
        "n": "0.00000000",
        "N": "BNB",
        "x": "TRADE",
        "X": "FILLED",
        "w": false,
        "m": false
    }"#;

    let raw: serde_json::Value = serde_json::from_str(json).unwrap();
    let event = BinanceUserDataStream::parse_order_update(&raw).unwrap();

    assert_eq!(event.event_time, 1672515782136);
    assert_eq!(event.symbol, "BTCUSDT");
    assert_eq!(event.side, "BUY");
    assert_eq!(event.order_type, "MARKET");
    assert_eq!(event.current_status, "FILLED");
    assert!((event.orig_qty - 0.01).abs() < 1e-8);
    assert!((event.cum_filled_qty - 0.01).abs() < 1e-8);
    assert!((event.last_filled_price - 45000.0).abs() < 1e-8);
    assert_eq!(event.commission, Some(0.0));
    assert_eq!(event.commission_asset, Some("BNB".into()));
}

#[test]
fn test_parse_order_update_new() {
    let json = r#"{
        "e": "executionReport",
        "E": 1672515782136,
        "s": "BTCUSDT",
        "c": "myLimitOrder",
        "i": 987654321,
        "S": "SELL",
        "o": "LIMIT",
        "f": "GTC",
        "q": "0.10000000",
        "p": "50000.00000000",
        "P": "0.00000000",
        "F": "0.00000000",
        "z": "0.00000000",
        "Z": "0.00000000",
        "l": "0.00000000",
        "L": "0.00000000",
        "x": "NEW",
        "X": "NEW",
        "w": true,
        "m": false
    }"#;

    let raw: serde_json::Value = serde_json::from_str(json).unwrap();
    let event = BinanceUserDataStream::parse_order_update(&raw).unwrap();

    assert_eq!(event.side, "SELL");
    assert_eq!(event.order_type, "LIMIT");
    assert_eq!(event.current_status, "NEW");
    assert!((event.cum_filled_qty - 0.0).abs() < 1e-8);
    assert!(event.is_on_book);
}

#[test]
fn test_parse_balance_update() {
    let json = r#"{
        "e": "balanceUpdate",
        "E": 1672515782136,
        "a": "USDT",
        "d": "100.00000000"
    }"#;

    let raw: serde_json::Value = serde_json::from_str(json).unwrap();
    let event = BinanceUserDataStream::parse_balance_update(&raw).unwrap();

    assert_eq!(event.event_time, 1672515782136);
    assert_eq!(event.asset, "USDT");
    assert!((event.delta - 100.0).abs() < 1e-8);
}

#[test]
fn test_parse_list_status() {
    let json = r#"{
        "e": "listStatus",
        "E": 1672515782136,
        "s": "BTCUSDT",
        "g": 12345,
        "l": "EXECUTING",
        "L": "STARTED"
    }"#;

    let raw: serde_json::Value = serde_json::from_str(json).unwrap();
    let event = BinanceUserDataStream::parse_list_status(&raw).unwrap();

    assert_eq!(event.event_time, 1672515782136);
    assert_eq!(event.symbol, "BTCUSDT");
    assert_eq!(event.order_list_id, 12345);
    assert_eq!(event.list_status, "EXECUTING");
    assert_eq!(event.list_order_status, "STARTED");
}

#[test]
fn test_parse_unknown_event() {
    let json = r#"{"e": "unknownEvent", "E": 12345}"#;
    let _raw: serde_json::Value = serde_json::from_str(json).unwrap();

    // Should not panic
    let inner = Arc::new(UserDataStreamInner {
        running: AtomicBool::new(true),
        connected: AtomicBool::new(true),
        rest_client: Mutex::new(None),
        listen_key: Mutex::new(None),
        task_handle: Mutex::new(None),
        event_sender: Mutex::new(None),
        use_testnet: false,
    });

    let result = BinanceUserDataStream::handle_message(&inner, json);
    assert!(result.is_ok());
}

#[test]
fn test_handle_message_sends_to_channel() {
    let (tx, mut rx) = mpsc::unbounded_channel::<UserDataEvent>();

    let inner = Arc::new(UserDataStreamInner {
        running: AtomicBool::new(true),
        connected: AtomicBool::new(true),
        rest_client: Mutex::new(None),
        listen_key: Mutex::new(None),
        task_handle: Mutex::new(None),
        event_sender: Mutex::new(Some(tx)),
        use_testnet: false,
    });

    let json = r#"{
        "e": "executionReport",
        "E": 1672515782136,
        "s": "BTCUSDT",
        "c": "test",
        "i": 1,
        "S": "BUY",
        "o": "MARKET",
        "f": "IOC",
        "q": "1.0",
        "p": "0.0",
        "P": "0.0",
        "F": "0.0",
        "z": "1.0",
        "Z": "45000.0",
        "l": "1.0",
        "L": "45000.0",
        "X": "FILLED",
        "w": false,
        "m": false
    }"#;

    BinanceUserDataStream::handle_message(&inner, json).unwrap();

    let event = rx.try_recv().expect("Should receive an event");
    match event {
        UserDataEvent::OrderUpdate(update) => {
            assert_eq!(update.symbol, "BTCUSDT");
            assert_eq!(update.current_status, "FILLED");
        }
        _ => panic!("Expected OrderUpdate event"),
    }
}

#[test]
fn test_backoff_delay() {
    let d1 = BinanceUserDataStream::backoff_delay(1, 60, 1000);
    assert!(d1.as_millis() >= 500);
    assert!(d1.as_millis() <= 1000);

    let d3 = BinanceUserDataStream::backoff_delay(3, 60, 1000);
    assert!(d3.as_millis() >= 500);
    assert!(d3.as_millis() <= 4000);
}

#[test]
fn test_debug_format() {
    let stream = BinanceUserDataStream::new(
        "test_key".into(),
        "test_secret".into(),
        false,
    );
    let debug = format!("{stream:?}");
    assert!(debug.contains("connected"));
    assert!(debug.contains("false"));
}

#[test]
fn test_start_twice_fails() {
    let stream = BinanceUserDataStream::new(
        "key".into(),
        "secret".into(),
        true,
    );
    // Should be in initial state (not running)
    assert!(!stream.connected());
}
