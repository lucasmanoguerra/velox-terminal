//! Chart interaction — zoom, pan, crosshair.

use velox_core::Candle;

/// View state representing the visible price/time range.
#[derive(Debug, Clone)]
pub struct ChartView {
    pub price_min: f64,
    pub price_max: f64,
    pub time_start: f64, // unix timestamp
    pub time_end: f64,   // unix timestamp
}

impl ChartView {
    pub fn from_candles(candles: &[Candle]) -> Self {
        if candles.is_empty() {
            return Self {
                price_min: 0.0,
                price_max: 100.0,
                time_start: 0.0,
                time_end: 1.0,
            };
        }

        let mut price_min = f64::MAX;
        let mut price_max = f64::MIN;
        let mut time_start = f64::MAX;
        let mut time_end = f64::MIN;

        for c in candles {
            price_min = price_min.min(c.low);
            price_max = price_max.max(c.high);
            let ts = c.timestamp.timestamp() as f64;
            time_start = time_start.min(ts);
            time_end = time_end.max(ts);
        }

        // Add some padding (5% on each side)
        let price_range = (price_max - price_min).max(1.0);
        let time_range = (time_end - time_start).max(60.0); // at least 1 minute

        Self {
            price_min: price_min - price_range * 0.05,
            price_max: price_max + price_range * 0.05,
            time_start: time_start - time_range * 0.02,
            time_end: time_end + time_range * 0.02,
        }
    }

    /// Price range (max - min).
    pub fn price_range(&self) -> f64 {
        (self.price_max - self.price_min).max(0.001)
    }

    /// Time range (end - start) in seconds.
    pub fn time_range(&self) -> f64 {
        (self.time_end - self.time_start).max(1.0)
    }

    /// Zoom in by a factor (factor < 1 = zoom in, > 1 = zoom out).
    /// `center_x` is the normalized center of zoom (0.0 = left, 1.0 = right).
    pub fn zoom(&mut self, factor: f64, center_x: f64) {
        let price_center = self.price_min + self.price_range() * center_x;
        let time_center = self.time_start + self.time_range() * center_x;

        let new_price_range = self.price_range() * factor;
        let new_time_range = self.time_range() * factor;

        self.price_min = price_center - new_price_range * center_x;
        self.price_max = price_center + new_price_range * (1.0 - center_x);
        self.time_start = time_center - new_time_range * center_x;
        self.time_end = time_center + new_time_range * (1.0 - center_x);
    }

    /// Pan by a delta in normalized coordinates.
    pub fn pan(&mut self, dx: f64, dy: f64) {
        let price_delta = self.price_range() * dy;
        let time_delta = self.time_range() * dx;

        self.price_min += price_delta;
        self.price_max += price_delta;
        self.time_start -= time_delta;
        self.time_end -= time_delta;
    }
}

/// Handles zoom and pan interactions for a chart.
pub struct ChartInteraction {
    /// Current view state.
    pub view: ChartView,
    /// Zoom history for undo.
    zoom_stack: Vec<ChartView>,
    /// Maximum zoom stack depth.
    max_zoom_stack: usize,
    /// Drag state.
    is_dragging: bool,
    drag_start_view: ChartView,
    drag_start_x: f64,
    drag_start_y: f64,
}

impl ChartInteraction {
    pub fn new(view: ChartView) -> Self {
        let drag_start_view = view.clone();
        Self {
            view,
            zoom_stack: Vec::new(),
            max_zoom_stack: 32,
            is_dragging: false,
            drag_start_view,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
        }
    }

    /// Create a new interaction handler with auto-fitted view from candles.
    pub fn from_candles(candles: &[Candle]) -> Self {
        Self::new(ChartView::from_candles(candles))
    }

    // ── Zoom ──────────────────────────────────────────────────────

    /// Zoom in around a point.
    ///
    /// `factor` < 1.0 zooms in, > 1.0 zooms out.
    /// `mouse_x` is the x-coordinate of the mouse in pixels.
    /// `chart_x` is the x-coordinate of the chart area in pixels.
    /// `chart_width` is the width of the chart area in pixels.
    pub fn zoom_at(&mut self, factor: f64, mouse_x: f64, chart_x: f64, chart_width: f64) {
        // Push current view to stack before modifying
        if self.zoom_stack.len() < self.max_zoom_stack {
            self.zoom_stack.push(self.view.clone());
        }

        let center_x = ((mouse_x - chart_x) / chart_width).clamp(0.0, 1.0);
        self.view.zoom(factor, center_x);
    }

    /// Zoom with scroll wheel delta.
    ///
    /// `delta` is the scroll delta (positive = zoom in, negative = zoom out).
    pub fn zoom_scroll(&mut self, delta: f64, mouse_x: f64, chart_x: f64, chart_width: f64) {
        // Each scroll tick zooms by ~10%
        let factor = if delta > 0.0 { 0.9 } else { 1.1 };
        self.zoom_at(factor, mouse_x, chart_x, chart_width);
    }

    // ── Pan ───────────────────────────────────────────────────────

    /// Start a drag (pan) operation.
    pub fn begin_pan(&mut self, mouse_x: f64, mouse_y: f64) {
        self.is_dragging = true;
        self.drag_start_view = self.view.clone();
        self.drag_start_x = mouse_x;
        self.drag_start_y = mouse_y;
    }

    /// Update pan during drag.
    /// `chart_width` and `chart_height` are the chart area dimensions in pixels.
    pub fn update_pan(&mut self, mouse_x: f64, mouse_y: f64, chart_width: f64, chart_height: f64) {
        if !self.is_dragging {
            return;
        }

        let dx = (self.drag_start_x - mouse_x) / chart_width;
        let dy = (self.drag_start_y - mouse_y) / chart_height;
        self.view = self.drag_start_view.clone();
        self.view.pan(dx, dy);
    }

    /// End a drag operation.
    pub fn end_pan(&mut self) {
        self.is_dragging = false;
    }

    // ── View management ───────────────────────────────────────────

    /// Undo last zoom operation.
    pub fn undo_zoom(&mut self) {
        if let Some(previous) = self.zoom_stack.pop() {
            self.view = previous;
        }
    }

    /// Reset view to fit data.
    pub fn reset_view(&mut self, candles: &[Candle]) {
        self.zoom_stack.push(self.view.clone());
        self.view = ChartView::from_candles(candles);
    }

    /// Clear zoom history.
    pub fn clear_zoom_stack(&mut self) {
        self.zoom_stack.clear();
    }

    /// Number of undo levels available.
    pub fn zoom_stack_size(&self) -> usize {
        self.zoom_stack.len()
    }

    /// Whether a pan drag is in progress.
    pub fn is_dragging(&self) -> bool {
        self.is_dragging
    }

    // ── Scrollbar ──────────────────────────────────────────────────

    /// Compute normalized scroll position (0.0 = oldest, 1.0 = newest)
    /// within the full data range.
    ///
    /// `data_start`/`data_end` are the min/max timestamps of all available candles.
    pub fn scroll_pos(&self, data_start: f64, data_end: f64) -> f64 {
        if data_end <= data_start {
            return 0.0;
        }
        let view_center = (self.view.time_start + self.view.time_end) / 2.0;
        let total_range = data_end - data_start;
        let clamped = view_center.clamp(data_start, data_end);
        (clamped - data_start) / total_range
    }

    /// Set the view so its center corresponds to the given scroll position.
    ///
    /// `fraction` is a normalized value (0.0 = oldest, 1.0 = newest).
    /// `data_start`/`data_end` are the full data range timestamps.
    pub fn set_scroll_pos(&mut self, fraction: f64, data_start: f64, data_end: f64) {
        if data_end <= data_start {
            return;
        }
        let total_range = data_end - data_start;
        let view_half = self.view.time_range() / 2.0;
        let max_half = total_range / 2.0;

        // When the view is wider than the data, center everything
        let center = if view_half >= max_half {
            (data_start + data_end) / 2.0
        } else {
            let raw_center = data_start + fraction * total_range;
            // Clamp so the view doesn't extend beyond the data
            raw_center.clamp(data_start + view_half, data_end - view_half)
        };
        self.view.time_start = center - view_half;
        self.view.time_end = center + view_half;
    }

    /// Get the full data timestamp range from candles.
    pub fn data_range(candles: &[Candle]) -> (f64, f64) {
        if candles.is_empty() {
            return (0.0, 1.0);
        }
        let start = candles.first().unwrap().timestamp.timestamp() as f64;
        let end = candles.last().unwrap().timestamp.timestamp() as f64;
        if end <= start {
            (start, start + 60.0)
        } else {
            (start, end)
        }
    }

    /// Whether the view is at the rightmost edge of the data (i.e., showing the newest data).
    /// Used to decide whether to auto-scroll when new candles arrive.
    pub fn is_at_right_edge(&self, data_end: f64) -> bool {
        self.view.time_end >= data_end - 1.0
    }
}

#[cfg(test)]
mod tests;
