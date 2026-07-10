//! Integration tests for BinanceBroker.
//!
//! These tests verify broker lifecycle (connect, disconnect, submit)
//! and edge cases. Format helper tests live in [`super::format`].

use velox_broker::{BrokerClient, BrokerConfig};
use velox_core::{NewOrder, OrderType, Side, TimeInForce};

use crate::binance_broker::BinanceBroker;

#[test]
fn test_debug_format() {
    let broker = BinanceBroker::new();
    let debug = format!("{broker:?}");
    assert!(debug.contains("connected"));
    assert!(debug.contains("false"));
}

#[test]
fn test_is_connected_initially_false() {
    let broker = BinanceBroker::new();
    assert!(!broker.is_connected());
}

#[tokio::test]
async fn test_submit_order_requires_connection() {
    let broker = BinanceBroker::new();
    let order = NewOrder {
        symbol: "BTCUSDT".into(),
        side: Side::Buy,
        order_type: OrderType::Market,
        quantity: 0.01,
        price: None,
        stop_price: None,
        time_in_force: TimeInForce::Ioc,
        client_order_id: None,
        take_profit_price: None,
        stop_loss_price: None,
    };

    let result = broker.submit_order(order).await;
    assert!(result.is_err(), "Should fail without connection");
}

#[tokio::test]
async fn test_connect_and_connection_state() {
    let broker = BinanceBroker::new();
    let config = BrokerConfig {
        api_key: "test_key".into(),
        api_secret: "test_secret".into(),
        base_url: "https://test.binance.com".into(),
        paper_trading: true,
    };

    let handle = broker.connect(config).await.unwrap();
    assert_eq!(handle.broker, "binance");
    assert!(broker.is_connected());

    broker.disconnect(&handle).await.unwrap();
    assert!(!broker.is_connected());
}

#[test]
fn test_default() {
    let broker = BinanceBroker::default();
    assert!(!broker.is_connected());
}
