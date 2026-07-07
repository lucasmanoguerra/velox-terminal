//! Shared application state between UI panels and GPU renderer.
//!
//! `AppState` is the single source of truth for UI-visible state.
//! It is mutated by:
//! - `PanelManager::show()` → updates `chart_panel_rect`
//! - `input::route_to_chart()` → updates `chart_interaction`, `cursor_pos`
//! - Render code → reads `chart_interaction.view` + `chart_panel_rect` to compute uniforms
//!
//! All mutations happen on the main thread (winit event loop), so no locks are needed.

use velox_chart::interaction::{ChartInteraction, ChartView};
use velox_chart::overlay::OverlayManager;
use velox_core::Candle;

/// Shared application state, mutated sequentially on the main thread.
pub struct AppState {
    /// Current candles displayed on the chart.
    pub candles: Vec<Candle>,

    /// Zoom/pan state.
    pub chart_interaction: ChartInteraction,

    /// Indicator overlays.
    pub overlays: OverlayManager,

    /// Area of the chart panel in egui logical pixels.
    /// Set by `PanelManager::show()` every frame.
    pub chart_panel_rect: egui::Rect,

    /// Cursor position in physical (DPI-scaled) pixels.
    pub cursor_pos_physical: (f64, f64),

    /// Whether a redraw has been requested.
    pub needs_redraw: bool,

    /// Monotonic frame counter.
    pub frame_count: u64,
}

impl AppState {
    /// Create a new `AppState` with initial candles.
    pub fn new(candles: Vec<Candle>) -> Self {
        let view = ChartView::from_candles(&candles);
        Self {
            candles,
            chart_interaction: ChartInteraction::new(view),
            overlays: OverlayManager::new(),
            chart_panel_rect: egui::Rect::ZERO,
            cursor_pos_physical: (0.0, 0.0),
            needs_redraw: true,
            frame_count: 0,
        }
    }
}
