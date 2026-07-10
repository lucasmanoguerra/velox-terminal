//! Tests for the PaperTrader engine.

use super::*;
use velox_core::{NewOrder, Order, OrderState, OrderType, Side, TimeInForce};

/// Helper: call execute_open_orders with close=high=low (flat bar).
fn exec(pt: &mut PaperTrader, symbol: &str, price: f64) -> usize {
    pt.execute_open_orders(symbol, price, price, price)
}

#[test]
fn test_submit_and_execute_market_buy() {
    let mut pt = PaperTrader::new(100_000.0);
    let symbol = "BTC/USDT";

    pt.submit_market_order(symbol, Side::Buy, 1.0).unwrap();
    assert_eq!(pt.open_orders().len(), 1);

    let filled = exec(&mut pt, symbol, 50000.0);
    assert_eq!(filled, 1);
    assert_eq!(pt.open_orders().len(), 0);
    assert_eq!(pt.net_position(symbol), 1.0);
}

#[test]
fn test_submit_and_execute_market_sell() {
    let mut pt = PaperTrader::new(100_000.0);
    let symbol = "ETH/USDT";

    pt.submit_market_order(symbol, Side::Sell, 2.0).unwrap();
    let filled = exec(&mut pt, symbol, 3000.0);
    assert_eq!(filled, 1);
    assert_eq!(pt.net_position(symbol), -2.0);
}

#[test]
fn test_multiple_fills_position_averaging() {
    let mut pt = PaperTrader::new(100_000.0);
    let symbol = "SOL/USDT";

    pt.submit_market_order(symbol, Side::Buy, 10.0).unwrap();
    exec(&mut pt, symbol, 20.0);

    pt.submit_market_order(symbol, Side::Buy, 5.0).unwrap();
    exec(&mut pt, symbol, 30.0);

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
    exec(&mut pt, symbol, 50000.0);

    pt.submit_market_order(symbol, Side::Sell, 1.0).unwrap();
    exec(&mut pt, symbol, 51000.0);

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
    exec(&mut pt, symbol, 50000.0);

    pt.update_account();
    assert!((pt.account().equity - 100_000.0).abs() < 0.01);

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

// ── Limit & Stop order tests ──────────────────────────────────────────

#[test]
fn test_limit_buy_fills_when_price_dips() {
    let mut pt = PaperTrader::new(100_000.0);
    let sym = "BTC/USDT";

    let order = NewOrder {
        symbol: sym.to_string(),
        side: Side::Buy,
        order_type: OrderType::Limit,
        quantity: 1.0,
        price: Some(49000.0),
        stop_price: None,
        time_in_force: TimeInForce::Day,
        client_order_id: None,
        take_profit_price: None,
        stop_loss_price: None,
    };
    pt.submit_order(order).unwrap();
    assert_eq!(pt.open_orders().len(), 1);

    let filled = pt.execute_open_orders(sym, 49200.0, 49300.0, 48500.0);
    assert_eq!(filled, 1);
    assert_eq!(pt.net_position(sym), 1.0);
}

#[test]
fn test_limit_buy_not_filled_if_price_stays_above() {
    let mut pt = PaperTrader::new(100_000.0);
    let sym = "BTC/USDT";

    let order = NewOrder {
        symbol: sym.to_string(),
        side: Side::Buy,
        order_type: OrderType::Limit,
        quantity: 1.0,
        price: Some(49000.0),
        stop_price: None,
        time_in_force: TimeInForce::Day,
        client_order_id: None,
        take_profit_price: None,
        stop_loss_price: None,
    };
    pt.submit_order(order).unwrap();

    let filled = pt.execute_open_orders(sym, 49500.0, 49800.0, 49100.0);
    assert_eq!(filled, 0);
}

#[test]
fn test_limit_sell_fills_when_price_rises() {
    let mut pt = PaperTrader::new(100_000.0);
    let sym = "ETH/USDT";

    let order = NewOrder {
        symbol: sym.to_string(),
        side: Side::Sell,
        order_type: OrderType::Limit,
        quantity: 1.0,
        price: Some(3100.0),
        stop_price: None,
        time_in_force: TimeInForce::Day,
        client_order_id: None,
        take_profit_price: None,
        stop_loss_price: None,
    };
    pt.submit_order(order).unwrap();

    let filled = pt.execute_open_orders(sym, 3120.0, 3150.0, 3080.0);
    assert_eq!(filled, 1);
    assert_eq!(pt.net_position(sym), -1.0);
}

#[test]
fn test_stop_market_buy_triggers_on_rise() {
    let mut pt = PaperTrader::new(100_000.0);
    let sym = "BTC/USDT";

    let order = NewOrder {
        symbol: sym.to_string(),
        side: Side::Buy,
        order_type: OrderType::StopMarket,
        quantity: 1.0,
        price: None,
        stop_price: Some(51000.0),
        time_in_force: TimeInForce::Day,
        client_order_id: None,
        take_profit_price: None,
        stop_loss_price: None,
    };
    pt.submit_order(order).unwrap();

    let filled = pt.execute_open_orders(sym, 50800.0, 51200.0, 50000.0);
    assert_eq!(filled, 1);
    assert_eq!(pt.net_position(sym), 1.0);
}

#[test]
fn test_stop_market_sell_triggers_on_drop() {
    let mut pt = PaperTrader::new(100_000.0);
    let sym = "SOL/USDT";

    let order = NewOrder {
        symbol: sym.to_string(),
        side: Side::Sell,
        order_type: OrderType::StopMarket,
        quantity: 2.0,
        price: None,
        stop_price: Some(18.0),
        time_in_force: TimeInForce::Day,
        client_order_id: None,
        take_profit_price: None,
        stop_loss_price: None,
    };
    pt.submit_order(order).unwrap();

    let filled = pt.execute_open_orders(sym, 18.5, 19.0, 17.5);
    assert_eq!(filled, 1);
    assert_eq!(pt.net_position(sym), -2.0);
}

// ── Bracket order tests ──────────────────────────────────────────────

#[test]
fn test_bracket_market_buy_creates_tp_sl() {
    let mut pt = PaperTrader::new(100_000.0);
    let sym = "BTC/USDT";

    let order = NewOrder {
        symbol: sym.to_string(),
        side: Side::Buy,
        order_type: OrderType::Market,
        quantity: 1.0,
        price: None,
        stop_price: None,
        time_in_force: TimeInForce::Day,
        client_order_id: None,
        take_profit_price: Some(51000.0),
        stop_loss_price: Some(49000.0),
    };
    let entry_id = pt.submit_order(order).unwrap();

    let filled = exec(&mut pt, sym, 50000.0);
    assert_eq!(filled, 1, "Entry should fill");
    assert_eq!(pt.net_position(sym), 1.0);

    let orders = pt.closed_orders();
    assert_eq!(orders.len(), 1, "One filled entry");

    let tp_sl_orders: Vec<&Order> = pt
        .orders()
        .into_iter()
        .filter(|o| o.parent_order_id == Some(entry_id))
        .collect();
    assert_eq!(tp_sl_orders.len(), 2, "Entry should have 2 bracket children");

    let tp = tp_sl_orders.iter().find(|o| o.order_type == OrderType::Limit).unwrap();
    let sl = tp_sl_orders.iter().find(|o| o.order_type == OrderType::StopMarket).unwrap();
    assert_eq!(tp.price, Some(51000.0));
    assert_eq!(sl.stop_price, Some(49000.0));
    assert_eq!(tp.quantity, 1.0);
    assert_eq!(sl.quantity, 1.0);
}

#[test]
fn test_bracket_tp_fill_cancels_sl() {
    let mut pt = PaperTrader::new(100_000.0);
    let sym = "BTC/USDT";

    let order = NewOrder {
        symbol: sym.to_string(),
        side: Side::Buy,
        order_type: OrderType::Market,
        quantity: 1.0,
        price: None,
        stop_price: None,
        time_in_force: TimeInForce::Day,
        client_order_id: None,
        take_profit_price: Some(51000.0),
        stop_loss_price: Some(49000.0),
    };
    let entry_id = pt.submit_order(order).unwrap();
    exec(&mut pt, sym, 50000.0);

    let filled = pt.execute_open_orders(sym, 51500.0, 52000.0, 51000.0);
    assert_eq!(filled, 1, "TP should fill");
    assert_eq!(pt.net_position(sym), 0.0, "Position should be flat");

    let sl = pt
        .orders()
        .into_iter()
        .find(|o| o.order_type == OrderType::StopMarket && o.parent_order_id == Some(entry_id))
        .unwrap();
    assert_eq!(sl.state, OrderState::Canceled, "SL should be canceled after TP fills");
}

#[test]
fn test_bracket_sl_fill_cancels_tp() {
    let mut pt = PaperTrader::new(100_000.0);
    let sym = "SOL/USDT";

    let order = NewOrder {
        symbol: sym.to_string(),
        side: Side::Sell,
        order_type: OrderType::Market,
        quantity: 5.0,
        price: None,
        stop_price: None,
        time_in_force: TimeInForce::Day,
        client_order_id: None,
        take_profit_price: Some(15.0),
        stop_loss_price: Some(25.0),
    };
    let entry_id = pt.submit_order(order).unwrap();
    exec(&mut pt, sym, 20.0);

    let filled = pt.execute_open_orders(sym, 14.0, 16.0, 13.0);
    assert_eq!(filled, 1, "TP should fill");
    assert_eq!(pt.net_position(sym), 0.0, "Position should be flat");

    let sl = pt
        .orders()
        .into_iter()
        .find(|o| o.order_type == OrderType::StopMarket && o.parent_order_id == Some(entry_id))
        .unwrap();
    assert_eq!(sl.state, OrderState::Canceled, "SL should be canceled after TP fills");
}

#[test]
fn test_bracket_market_buy_no_tp_sl_if_not_set() {
    let mut pt = PaperTrader::new(100_000.0);
    let sym = "BTC/USDT";

    let entry_id = pt.submit_market_order(sym, Side::Buy, 1.0).unwrap();
    exec(&mut pt, sym, 50000.0);

    let children: Vec<&Order> = pt
        .orders()
        .into_iter()
        .filter(|o| o.parent_order_id == Some(entry_id))
        .collect();
    assert_eq!(children.len(), 0, "No bracket children expected");
}

#[test]
fn test_bracket_creates_children_with_parent_id() {
    let mut pt = PaperTrader::new(100_000.0);
    let sym = "ETH/USDT";

    let order = NewOrder {
        symbol: sym.to_string(),
        side: Side::Buy,
        order_type: OrderType::Market,
        quantity: 2.0,
        price: None,
        stop_price: None,
        time_in_force: TimeInForce::Day,
        client_order_id: None,
        take_profit_price: Some(2500.0),
        stop_loss_price: Some(2300.0),
    };
    let entry_id = pt.submit_order(order).unwrap();
    exec(&mut pt, sym, 2400.0);

    let children: Vec<&Order> = pt
        .orders()
        .into_iter()
        .filter(|o| o.parent_order_id == Some(entry_id))
        .collect();
    assert_eq!(children.len(), 2);
    for child in &children {
        assert_eq!(child.quantity, 2.0, "Child should have same qty as entry");
        assert_eq!(child.side, Side::Sell, "Children should be opposite side");
    }
}

#[test]
fn test_stop_market_not_triggered_below_stop() {
    let mut pt = PaperTrader::new(100_000.0);
    let sym = "BTC/USDT";

    let order = NewOrder {
        symbol: sym.to_string(),
        side: Side::Buy,
        order_type: OrderType::StopMarket,
        quantity: 1.0,
        price: None,
        stop_price: Some(51000.0),
        time_in_force: TimeInForce::Day,
        client_order_id: None,
        take_profit_price: None,
        stop_loss_price: None,
    };
    pt.submit_order(order).unwrap();

    let filled = pt.execute_open_orders(sym, 50500.0, 50900.0, 50000.0);
    assert_eq!(filled, 0);
}
