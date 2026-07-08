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
        Self
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

                    // ── Price info ────────────────────────────────
                    if let Some(last) = state.candles.last() {
                        let dir = if last.is_bullish() { "▲" } else { "▼" };
                        ui.label(format!(
                            "{dir} {:.2}  O:{:.2} H:{:.2} L:{:.2} V:{:.0}",
                            last.close, last.open, last.high, last.low, last.volume
                        ));
                    } else {
                        ui.label("Waiting for data...");
                    }

                    // ── Connection indicator ──────────────────────
                    if state.feed_connected {
                        ui.label("● Live");
                    } else {
                        ui.label("○ Offline");
                    }

                    ui.separator();

                    // ── Timeframe selector ────────────────────────
                    for &(tf, ref label) in &state.timeframe_labels() {
                        let selected = tf == state.selected_timeframe;
                        if ui.selectable_label(selected, label.as_str()).clicked() {
                            state.set_timeframe(tf);
                        }
                    }

                    // ── Right side ────────────────────────────────
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Reset View").clicked() {
                            state.reset_view();
                            state.needs_redraw = true;
                        }
                        ui.label(format!(
                            "Frame {} | T:{} C:{}",
                            state.frame_count,
                            state.ticks_processed,
                            state.candles_produced,
                        ));
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

                // Symbol selector
                ui.horizontal(|ui| {
                    ui.label("Symbol:");
                    let mut sym = state.symbol.clone();
                    if ui.text_edit_singleline(&mut sym).changed() {
                        state.symbol = sym;
                    }
                });

                ui.separator();

                // Price display
                if let Some(last) = state.candles.last() {
                    ui.horizontal(|ui| {
                        ui.label("Last:");
                        if last.is_bullish() {
                            ui.colored_label(egui::Color32::GREEN, format!("{:.2}", last.close));
                        } else {
                            ui.colored_label(egui::Color32::RED, format!("{:.2}", last.close));
                        }
                    });
                }

                ui.separator();

                // Buy/Sell buttons
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
                "{} · Candles: {} | Zoom stack: {}",
                state.timeframe_label(),
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

                // Live data metrics
                ui.separator();
                ui.heading("Feed");
                if state.feed_connected {
                    ui.label("● Live");
                } else {
                    ui.label("○ Offline");
                }
                ui.label(format!("Ticks: {}", state.ticks_processed));
                ui.label(format!("Candles: {}", state.candles_produced));
            });

        // ── Bottom status bar ────────────────────────────────────
        egui::TopBottomPanel::bottom("status_bar")
            .min_height(22.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if state.feed_connected {
                        ui.label("● Connected");
                    } else {
                        ui.label("○ Disconnected");
                    }
                    ui.separator();
                    ui.label("Paper Trading");
                    ui.separator();
                    ui.label(state.timeframe_label());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("{} · Live Feed", state.symbol));
                    });
                });
            });
    }
}
