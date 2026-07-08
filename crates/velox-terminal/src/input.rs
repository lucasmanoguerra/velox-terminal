//! Input routing — dispatch winit events to either egui or ChartInteraction.
//!
//! # Event routing priority
//!
//! 1. Always pass to egui first (via `egui_winit::State::on_window_event`).
//! 2. If egui did NOT consume the event and the mouse is over the chart,
//!    route to `ChartInteraction` for zoom/pan.
//!
//! This prevents conflicts: scrolling over a side panel does NOT zoom the chart.

use velox_ui::app_state::AppState;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};

/// Route a window event to the chart interaction handler.
///
/// Returns `true` if the event was consumed by the chart.
/// Call this ONLY if `egui_winit::State::on_window_event()` returned `false`.
pub fn route_to_chart(event: &WindowEvent, state: &mut AppState, scale_factor: f64) -> bool {
    let rect = state.chart_panel_rect;
    let (cx, cy) = state.cursor_pos_physical;

    match event {
        // ── Scroll wheel → Zoom ────────────────────────────────
        WindowEvent::MouseWheel { delta, .. } => {
            let y_delta = match delta {
                MouseScrollDelta::LineDelta(_, y) => *y as f64,
                MouseScrollDelta::PixelDelta(pos) => pos.y,
            };

            if is_over_chart(cx, cy, rect, scale_factor) {
                let chart_x = rect.min.x as f64 * scale_factor;
                let chart_w = rect.width() as f64 * scale_factor;
                state
                    .chart_interaction
                    .zoom_scroll(y_delta, cx, chart_x, chart_w);
                state.needs_redraw = true;
                true
            } else {
                false
            }
        }

        // ── Mouse press → Begin pan ────────────────────────────
        WindowEvent::MouseInput {
            state: press_state,
            button: MouseButton::Left,
            ..
        } => {
            match press_state {
                ElementState::Pressed => {
                    if is_over_chart(cx, cy, rect, scale_factor) {
                        state.chart_interaction.begin_pan(cx, cy);
                        state.needs_redraw = true;
                        return true;
                    }
                }
                ElementState::Released => {
                    state.chart_interaction.end_pan();
                    state.needs_redraw = true;
                    return true;
                }
            }
            false
        }

        // ── Right-click → Undo zoom ────────────────────────────
        WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button: MouseButton::Right,
            ..
        } => {
            state.chart_interaction.undo_zoom();
            state.needs_redraw = true;
            true
        }

        // ── Mouse move → Update pan or cursor ──────────────────
        WindowEvent::CursorMoved { position, .. } => {
            state.cursor_pos_physical = (position.x, position.y);

            if state.chart_interaction.is_dragging() {
                let chart_w = rect.width() as f64 * scale_factor;
                let chart_h = rect.height() as f64 * scale_factor;
                state
                    .chart_interaction
                    .update_pan(position.x, position.y, chart_w, chart_h);
                state.needs_redraw = true;
            }

            // Don't consume CursorMoved — egui and other parts also need it
            false
        }

        _ => false,
    }
}

/// Check whether a cursor position is over the chart area.
fn is_over_chart(cx: f64, cy: f64, rect: egui::Rect, scale: f64) -> bool {
    let left = rect.min.x as f64 * scale;
    let right = rect.max.x as f64 * scale;
    let top = rect.min.y as f64 * scale;
    let bottom = rect.max.y as f64 * scale;
    cx >= left && cx <= right && cy >= top && cy <= bottom
}
