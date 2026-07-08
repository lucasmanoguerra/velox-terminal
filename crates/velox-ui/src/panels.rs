//! UI panels for the trading terminal.
//!
//! Provides the layout structure: menu bar, chart area, order entry side panel,
//! positions panel, and status bar. The chart area rect is recorded into
//! `AppState` for the GPU renderer to use.

use crate::app_state::AppState;
use egui;
use velox_indicators::{Ema, Rsi, Sma};

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

                    ui.separator();

                    // ── Indicator toggles ──────────────────────
                    ui.label("Indicators:");
                    let sma_name = "SMA(20)";
                    let sma_enabled = state.overlays.has_overlay(sma_name);
                    if ui.selectable_label(sma_enabled, sma_name).clicked() {
                        if sma_enabled {
                            state.overlays.remove(sma_name);
                        } else {
                            state.overlays.add(sma_name, Sma::new(20), (0.0, 1.0, 0.0));
                        }
                        state.needs_redraw = true;
                    }
                    let ema_name = "EMA(20)";
                    let ema_enabled = state.overlays.has_overlay(ema_name);
                    if ui.selectable_label(ema_enabled, ema_name).clicked() {
                        if ema_enabled {
                            state.overlays.remove(ema_name);
                        } else {
                            state.overlays.add(ema_name, Ema::new(20), (1.0, 1.0, 0.0));
                        }
                        state.needs_redraw = true;
                    }
                    let rsi_name = "RSI(14)";
                    let rsi_enabled = state.overlays.has_overlay(rsi_name);
                    if ui.selectable_label(rsi_enabled, rsi_name).clicked() {
                        if rsi_enabled {
                            state.overlays.remove(rsi_name);
                        } else {
                            state.overlays.add(rsi_name, Rsi::new(14), (1.0, 0.5, 0.0));
                        }
                        state.needs_redraw = true;
                    }

                    // ── Right side ────────────────────────────────
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Reset View").clicked() {
                            state.reset_view();
                            state.needs_redraw = true;
                        }
                        ui.label(format!(
                            "Frame {} | T:{} C:{}",
                            state.frame_count, state.ticks_processed, state.candles_produced,
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

                // Quantity slider
                ui.horizontal(|ui| {
                    ui.label("Qty:");
                    ui.add(
                        egui::Slider::new(&mut state.order_entry_qty, 0.0..=10.0)
                            .clamping(egui::SliderClamping::Always),
                    );
                });

                ui.separator();

                // Buy / Sell buttons
                ui.horizontal(|ui| {
                    let buy_btn = egui::Button::new("Buy")
                        .fill(egui::Color32::from_rgb(0, 80, 40))
                        .min_size(egui::vec2(70.0, 28.0));
                    if ui.add(buy_btn).clicked() {
                        state.buy_market();
                    }
                    let sell_btn = egui::Button::new("Sell")
                        .fill(egui::Color32::from_rgb(120, 30, 30))
                        .min_size(egui::vec2(70.0, 28.0));
                    if ui.add(sell_btn).clicked() {
                        state.sell_market();
                    }
                });

                ui.separator();

                // Feedback messages
                if let Some(ref err) = state.order_error {
                    ui.colored_label(egui::Color32::RED, err);
                } else if let Some(ref ok) = state.order_success {
                    ui.colored_label(egui::Color32::GREEN, ok);
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

        // ── Right panel: Positions & Account ──────────────────────
        // Collect order/position data outside closures to avoid borrow issues
        // with the cancel button (which needs &mut state).
        let open_order_summaries: Vec<(velox_core::OrderId, velox_core::Side, f64, String, String)> = state
            .open_orders()
            .into_iter()
            .map(|o| {
                let price_str = o.price.map_or("MKT".to_string(), |p| format!("{:.2}", p));
                (o.order_id, o.side, o.quantity, o.symbol.clone(), price_str)
            })
            .collect();

        let position_summaries: Vec<(f64, String, f64, f64, f64)> = state
            .positions()
            .into_iter()
            .filter(|p| p.quantity != 0.0)
            .map(|p| {
                (
                    p.quantity,
                    p.symbol.clone(),
                    p.avg_entry_price,
                    p.unrealized_pnl,
                    p.realized_pnl,
                )
            })
            .collect();

        let account_cash = state.paper_trader.account().cash;
        let account_equity = state.paper_trader.account().equity;
        let account_bp = state.paper_trader.account().buying_power;
        let account_upnl = state.paper_trader.account().unrealized_pnl;
        let account_rpnl = state.paper_trader.account().realized_pnl;

        egui::SidePanel::right("positions")
            .resizable(true)
            .default_width(260.0)
            .min_width(180.0)
            .show(ctx, |ui| {
                // ── Open Orders ────────────────────────────────────
                ui.heading("Open Orders");
                ui.separator();
                if open_order_summaries.is_empty() {
                    ui.label("No open orders");
                } else {
                    egui::ScrollArea::vertical()
                        .id_salt("open_orders_scroll")
                        .max_height(120.0)
                        .show(ui, |ui| {
                            for (order_id, side, qty, symbol, price_str) in &open_order_summaries {
                                ui.horizontal(|ui| {
                                    let side_color = match side {
                                        velox_core::Side::Buy => egui::Color32::GREEN,
                                        velox_core::Side::Sell => egui::Color32::RED,
                                    };
                                    ui.colored_label(side_color, format!("{:?}", side));
                                    ui.label(format!("{} {} {}", qty, symbol, price_str));
                                    if ui.button("X").clicked() {
                                        state.cancel_order(*order_id);
                                    }
                                });
                            }
                        });
                }

                ui.separator();

                // ── Positions ──────────────────────────────────────
                ui.heading("Positions");
                ui.separator();
                if position_summaries.is_empty() {
                    ui.label("No open positions");
                } else {
                    egui::ScrollArea::vertical()
                        .id_salt("positions_scroll")
                        .max_height(120.0)
                        .show(ui, |ui| {
                            for (qty, symbol, avg_entry, upnl, rpnl) in &position_summaries {
                                ui.horizontal(|ui| {
                                    let side = if *qty > 0.0 { "LONG" } else { "SHORT" };
                                    let side_color = if *qty > 0.0 {
                                        egui::Color32::GREEN
                                    } else {
                                        egui::Color32::RED
                                    };
                                    ui.colored_label(side_color, side);
                                    ui.label(format!(
                                        "{} {} @ {:.2}",
                                        qty.abs(),
                                        symbol,
                                        avg_entry,
                                    ));
                                });
                                // P&L row
                                let total_pnl = upnl + rpnl;
                                let pnl_color = if total_pnl >= 0.0 {
                                    egui::Color32::GREEN
                                } else {
                                    egui::Color32::RED
                                };
                                ui.horizontal(|ui| {
                                    ui.label("  P&L:");
                                    ui.colored_label(
                                        pnl_color,
                                        format!("${:.2}", total_pnl),
                                    );
                                });
                            }
                        });
                }

                ui.separator();

                // ── Account ────────────────────────────────────────
                ui.heading("Account");
                ui.separator();
                ui.label(format!("Cash:    ${:.2}", account_cash));
                ui.label(format!("Equity:  ${:.2}", account_equity));
                ui.label(format!("Buy Pwr: ${:.2}", account_bp));
                let total_pnl = account_upnl + account_rpnl;
                let pnl_color = if total_pnl >= 0.0 {
                    egui::Color32::GREEN
                } else {
                    egui::Color32::RED
                };
                ui.horizontal(|ui| {
                    ui.label("P&L:");
                    ui.colored_label(pnl_color, format!("${:.2}", total_pnl));
                });

                ui.separator();

                // ── Feed info ──────────────────────────────────────
                ui.heading("Feed");
                ui.separator();
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
