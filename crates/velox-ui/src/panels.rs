//! UI panels for the trading terminal.
//!
//! Provides the layout structure: menu bar, chart area, order entry side panel,
//! positions panel, and status bar. The chart area rect is recorded into
//! `AppState` for the GPU renderer to use.

use egui;
use crate::app_state::AppState;

/// Manages the panel layout and UI state.
pub struct PanelManager;

impl Default for PanelManager {
    fn default() -> Self {
        Self
    }
}

impl PanelManager {
    /// Create a new panel manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the egui UI for all panels.
    ///
    /// Must be called once per frame. The `AppState.chart_panel_rect` field
    /// will be updated to reflect the central panel's rectangle (in egui
    /// logical pixels) for the GPU chart renderer.
    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) {
        // ── Top menu bar ──────────────────────────────────────────
        egui::TopBottomPanel::top("menu_bar")
            .min_height(28.0)
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.heading("velox-terminal");
                    ui.separator();
                    if let Some(last) = state.candles.last() {
                        ui.label(format!(
                            "O: {:.2}  H: {:.2}  L: {:.2}  C: {:.2}  Vol: {}",
                            last.open, last.high, last.low, last.close, last.volume
                        ));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Reset View").clicked() {
                            state.chart_interaction.reset_view(&state.candles);
                            state.needs_redraw = true;
                        }
                        ui.label(format!("Frame {}", state.frame_count));
                    });
                });
            });

        // ── Left panel: Order Entry ──────────────────────────────
        egui::SidePanel::left("order_entry")
            .resizable(true)
            .default_width(200.0)
            .min_width(150.0)
            .show(ctx, |ui| {
                ui.heading("Order Entry");
                ui.separator();
                ui.label("Symbol: BTC/USD");
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Buy").clicked() {
                        tracing::info!("Buy clicked");
                    }
                    if ui.button("Sell").clicked() {
                        tracing::info!("Sell clicked");
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Qty:");
                    ui.add(egui::Slider::new(&mut 0.1_f64, 0.0..=10.0).text(""));
                });
                ui.separator();
                if ui.button("Place Order").clicked() {
                    tracing::info!("Order placed");
                }
            });

        // ── Central panel: Chart area ────────────────────────────
        // The chart itself is rendered by the GPU in a separate pass.
        // Here we just record the rect and overlay a text label.
        egui::CentralPanel::default().show(ctx, |ui| {
            state.chart_panel_rect = ui.max_rect();

            // Show crosshair-style info at bottom-left of chart
            let info = format!(
                "Candles: {} | Zoom stack: {}",
                state.candles.len(),
                state.chart_interaction.zoom_stack_size(),
            );
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.label(info);
            });
        });

        // ── Right panel: Positions ───────────────────────────────
        egui::SidePanel::right("positions")
            .resizable(true)
            .default_width(220.0)
            .min_width(150.0)
            .show(ctx, |ui| {
                ui.heading("Positions");
                ui.separator();
                ui.label("No open positions");
                ui.separator();
                ui.heading("Account");
                ui.separator();
                ui.label("Balance: $100,000.00");
                ui.label("Equity:  $100,000.00");
                ui.label("Margin:  $0.00");
            });

        // ── Bottom status bar ────────────────────────────────────
        egui::TopBottomPanel::bottom("status_bar")
            .min_height(22.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Ready");
                    ui.separator();
                    ui.label("Paper Trading");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label("BTC/USD · 1m");
                    });
                });
            });
    }
}
