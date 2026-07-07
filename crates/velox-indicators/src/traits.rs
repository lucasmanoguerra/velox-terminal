//! Common traits for all indicators.

/// An indicator that can be updated incrementally with a new price.
pub trait Indicator<T> {
    type Output;

    /// Update the indicator with a new value. Returns the current output.
    fn update(&mut self, value: T) -> Self::Output;

    /// Reset the indicator to its initial state.
    fn reset(&mut self);

    /// Returns true if the indicator has enough data to produce a valid output.
    fn is_ready(&self) -> bool;
}

/// An indicator that can compute its value from a full slice (batch mode).
pub trait BatchIndicator<T> {
    type Output;

    /// Compute the indicator over the entire data slice.
    fn compute(&self, data: &[T]) -> Vec<Self::Output>;
}
