//! Chart overlays — indicators, drawings, annotations.

use std::any::Any;
use velox_core::Candle;
use velox_indicators::traits::Indicator;

/// Manages indicator overlays on the chart.
///
/// Each overlay is a named indicator with a specific rendering pipeline.
/// Overlays are rendered in order of addition (first added = bottom layer).
pub struct OverlayManager {
    overlays: Vec<Box<dyn AnyOverlay>>,
}

/// Internal trait for type-erased overlay management.
trait AnyOverlay: Send + Sync {
    fn name(&self) -> &str;
    fn update(&mut self, candles: &[Candle]);
    fn values(&self) -> Vec<(f64, f64)>;
    fn color(&self) -> (f32, f32, f32);
    #[expect(dead_code)]
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// A concrete overlay wrapping an indicator and its renderer.
pub struct OverlayInstance<I: Indicator<f64, Output = f64> + Send + Sync + 'static> {
    pub name: String,
    pub indicator: I,
    pub values: Vec<(f64, f64)>, // (timestamp, value) pairs
    pub color: (f32, f32, f32),
}

impl<I: Indicator<f64, Output = f64> + Send + Sync + 'static> OverlayInstance<I> {
    pub fn new(name: &str, indicator: I, color: (f32, f32, f32)) -> Self {
        Self {
            name: name.to_string(),
            indicator,
            values: Vec::new(),
            color,
        }
    }

    /// Process candles and update indicator values.
    pub fn update(&mut self, candles: &[Candle]) {
        self.values.clear();
        for c in candles {
            // Use close price as the indicator input
            let value = self.indicator.update(c.close);
            let ts = c.timestamp.timestamp() as f64;
            self.values.push((ts, value));
        }
    }

    /// Get the last computed value.
    pub fn last_value(&self) -> Option<f64> {
        self.values.last().map(|&(_, v)| v)
    }

    /// Get all computed values as (timestamp, value) pairs.
    pub fn values(&self) -> Vec<(f64, f64)> {
        self.values.clone()
    }

    /// Get the line color.
    pub fn color(&self) -> (f32, f32, f32) {
        self.color
    }
}

impl<I: Indicator<f64, Output = f64> + Send + Sync + 'static> AnyOverlay for OverlayInstance<I> {
    fn name(&self) -> &str {
        &self.name
    }

    fn update(&mut self, candles: &[Candle]) {
        self.update(candles);
    }

    fn values(&self) -> Vec<(f64, f64)> {
        self.values.clone()
    }

    fn color(&self) -> (f32, f32, f32) {
        self.color
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl OverlayManager {
    pub fn new() -> Self {
        Self {
            overlays: Vec::new(),
        }
    }

    /// Add a new indicator overlay.
    pub fn add<I: Indicator<f64, Output = f64> + Send + Sync + 'static>(
        &mut self,
        name: &str,
        indicator: I,
        color: (f32, f32, f32),
    ) {
        self.overlays
            .push(Box::new(OverlayInstance::new(name, indicator, color)));
    }

    /// Remove an overlay by name.
    pub fn remove(&mut self, name: &str) -> bool {
        let idx = self.overlays.iter().position(|o| o.name() == name);
        if let Some(i) = idx {
            self.overlays.remove(i);
            true
        } else {
            false
        }
    }

    /// Update all overlays with new candle data.
    pub fn update_all(&mut self, candles: &[Candle]) {
        for overlay in &mut self.overlays {
            overlay.update(candles);
        }
    }

    /// Get the number of overlays.
    pub fn len(&self) -> usize {
        self.overlays.len()
    }

    /// Returns true if no overlays are registered.
    pub fn is_empty(&self) -> bool {
        self.overlays.is_empty()
    }

    /// Get overlay names.
    pub fn names(&self) -> Vec<&str> {
        self.overlays.iter().map(|o| o.name()).collect()
    }

    /// Check if an overlay with the given name exists.
    pub fn has_overlay(&self, name: &str) -> bool {
        self.overlays.iter().any(|o| o.name() == name)
    }

    /// Collect all overlay data for GPU line rendering.
    ///
    /// Returns Vec of `LineDescriptor`: `(name, [(timestamp, value)], (r, g, b))`.
    /// Empty overlays are filtered out.
    pub fn collect_line_data(&self) -> Vec<crate::renderer::LineDescriptor> {
        self.overlays
            .iter()
            .filter_map(|o| {
                let values = o.values();
                if values.is_empty() {
                    return None;
                }
                Some((o.name().to_string(), values, o.color()))
            })
            .collect()
    }
}

impl Default for OverlayManager {
    fn default() -> Self {
        Self::new()
    }
}
