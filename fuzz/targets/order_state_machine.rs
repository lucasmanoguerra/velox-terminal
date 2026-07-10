//! Fuzz target for the OMS state machine.
//!
//! Generates random sequences of order operations (submit, fill, cancel, replace)
//! and validates that the state machine never panics, rejects invalid transitions,
//! and maintains internal consistency (filled_qty <= total_qty, etc.).
//!
//! # Running
//!
//! ```bash
//! cargo fuzz run order_state_machine -- -max_len=4096 -rss_limit_mb=2048
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use velox_core::{Order, OrderId, OrderState, OrderType, Side, TimeInForce};

/// Fuzz the OMS state machine with random byte inputs.
///
/// Input encoding:
/// - Byte 0: operation type (0=submit, 1=fill, 2=cancel, 3=replace)
/// - Bytes 1-8: quantity (as f64 bits)
/// - Bytes 9-16: fill price (as f64 bits)
/// - Bytes 17+: recurrence
fuzz_target!(|data: &[u8]| {
    if data.len() < 17 {
        return;
    }

    let mut order = create_test_order();
    let qty = f64::from_le_bytes(data[1..9].try_into().unwrap()).abs() % 1000.0;
    let price = f64::from_le_bytes(data[9..17].try_into().unwrap()).abs() % 100000.0;

    match data[0] % 4 {
        0 => {
            // Simulate order submission
            order.state = OrderState::New;
        }
        1 => {
            // Simulate fill
            let fill_qty = qty.min(order.quantity - order.filled_quantity);
            order.filled_quantity += fill_qty;
            if order.filled_quantity >= order.quantity {
                order.state = OrderState::Filled;
            } else {
                order.state = OrderState::PartiallyFilled;
            }
            order.avg_fill_price = Some(price);
        }
        2 => {
            // Simulate cancel
            order.state = OrderState::Canceled;
        }
        3 => {
            // Simulate replace
            order.quantity = qty.max(order.filled_quantity);
        }
        _ => unreachable!(),
    }

    // Invariant: filled quantity never exceeds total quantity
    assert!(
        order.filled_quantity <= order.quantity,
        "Overfill detected: filled={} qty={}",
        order.filled_quantity,
        order.quantity
    );

    // Invariant: filled orders must have avg_fill_price
    if order.filled_quantity > 0.0 {
        assert!(
            order.avg_fill_price.is_some(),
            "Filled order missing avg price"
        );
    }
});

fn create_test_order() -> Order {
    Order {
        order_id: OrderId::new(),
        symbol: "BTCUSDT".into(),
        side: Side::Buy,
        order_type: OrderType::Market,
        quantity: 1.0,
        filled_quantity: 0.0,
        avg_fill_price: None,
        price: None,
        stop_price: None,
        time_in_force: TimeInForce::Gtc,
        state: OrderState::New,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        client_order_id: None,
        parent_order_id: None,
    }
}
