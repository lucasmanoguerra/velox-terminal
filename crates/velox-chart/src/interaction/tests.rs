//! Tests for chart interaction: zoom, pan, scrollbar, and view management.

use chrono::{TimeZone, Utc};
use velox_core::Candle;

use crate::interaction::{ChartInteraction, ChartView};

fn make_candle(ts_secs: i64) -> Candle {
    Candle {
        symbol: *b"BTCUSD\0\0",
        open: 50000.0,
        high: 50100.0,
        low: 49900.0,
        close: 50050.0,
        volume: 100.0,
        timestamp: Utc.timestamp_opt(ts_secs, 0).unwrap(),
        timeframe_secs: 60,
        trade_count: Some(10),
        vwap: Some(50050.0),
    }
}

#[test]
fn test_scroll_pos_at_center() {
    let view = ChartView {
        price_min: 49900.0,
        price_max: 50100.0,
        time_start: 200.0,
        time_end: 400.0,
    };
    let interaction = ChartInteraction::new(view);
    let pos = interaction.scroll_pos(0.0, 600.0);
    // View center = 300, data range = 0..600
    assert!((pos - 0.5).abs() < 1e-6, "Expected 0.5, got {pos}");
}

#[test]
fn test_scroll_pos_at_right_edge() {
    let view = ChartView {
        price_min: 49900.0,
        price_max: 50100.0,
        time_start: 500.0,
        time_end: 600.0,
    };
    let interaction = ChartInteraction::new(view);
    let pos = interaction.scroll_pos(0.0, 600.0);
    // View center = 550, data range = 0..600
    assert!((pos - 0.916666).abs() < 1e-3, "Expected ~0.917, got {pos}");
}

#[test]
fn test_set_scroll_pos_far_left() {
    let mut interaction = ChartInteraction::new(ChartView {
        price_min: 49900.0,
        price_max: 50100.0,
        time_start: 200.0,
        time_end: 400.0,
    });
    interaction.set_scroll_pos(0.0, 0.0, 600.0);
    // View range = 200, half = 100. Center clamped to 100.
    assert!((interaction.view.time_start - 0.0).abs() < 1e-6);
    assert!((interaction.view.time_end - 200.0).abs() < 1e-6);
}

#[test]
fn test_set_scroll_pos_far_right() {
    let mut interaction = ChartInteraction::new(ChartView {
        price_min: 49900.0,
        price_max: 50100.0,
        time_start: 200.0,
        time_end: 400.0,
    });
    interaction.set_scroll_pos(1.0, 0.0, 600.0);
    // View range = 200, half = 100. Center clamped to 500.
    assert!((interaction.view.time_start - 400.0).abs() < 1e-6);
    assert!((interaction.view.time_end - 600.0).abs() < 1e-6);
}

#[test]
fn test_is_at_right_edge() {
    let view = ChartView {
        price_min: 49900.0,
        price_max: 50100.0,
        time_start: 500.0,
        time_end: 600.0,
    };
    let interaction = ChartInteraction::new(view);
    assert!(interaction.is_at_right_edge(600.0));
    assert!(!interaction.is_at_right_edge(700.0));
}

#[test]
fn test_data_range_from_candles() {
    let candles = vec![make_candle(100), make_candle(200), make_candle(300)];
    let (start, end) = ChartInteraction::data_range(&candles);
    assert!((start - 100.0).abs() < 1e-6);
    assert!((end - 300.0).abs() < 1e-6);
}

#[test]
fn test_scroll_does_not_panic_empty() {
    let view = ChartView {
        price_min: 0.0,
        price_max: 100.0,
        time_start: 0.0,
        time_end: 1.0,
    };
    let mut interaction = ChartInteraction::new(view);
    let pos = interaction.scroll_pos(0.0, 0.0);
    assert!((pos - 0.0).abs() < 1e-6);
    interaction.set_scroll_pos(0.5, 0.0, 0.0);
    assert!(interaction.is_at_right_edge(0.0));
}

#[test]
fn test_zoom_pan_basic() {
    let view = ChartView {
        price_min: 0.0,
        price_max: 100.0,
        time_start: 0.0,
        time_end: 100.0,
    };
    let mut interaction = ChartInteraction::new(view);

    // Zoom in at center (factor < 1 = zoom in)
    let prev_range = interaction.view.time_range();
    interaction.zoom_at(0.5, 50.0, 0.0, 100.0);
    assert!(interaction.view.time_range() < prev_range);

    // Pan: drag from (0,0) to (-50,0) → dx > 0 → time moves earlier
    interaction.begin_pan(0.0, 0.0);
    interaction.update_pan(-50.0, 0.0, 100.0, 100.0);
    interaction.end_pan();
    // After panning right (drag-to-left), time_start decreases (we see earlier data)
    assert!(interaction.view.time_start < 25.0);

    // Undo zoom restores the pre-zoom view (before zoom was applied)
    interaction.undo_zoom();
    assert!((interaction.view.time_range() - 100.0).abs() < 1e-6);
}

#[test]
fn test_reset_view() {
    let candles = vec![make_candle(100), make_candle(200)];
    let mut interaction = ChartInteraction::new(ChartView {
        price_min: 0.0,
        price_max: 100.0,
        time_start: 0.0,
        time_end: 100.0,
    });
    interaction.reset_view(&candles);
    // View should be auto-fitted to candle range [100..200] with ~2% padding
    // time_start ≈ 100 - (200-100)*0.02 = 98
    assert!(interaction.view.time_start > 50.0);
    assert!(interaction.view.time_end > 150.0);
}
