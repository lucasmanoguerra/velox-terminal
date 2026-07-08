//! Sandbox — restricts what scripts can do.

/// Placeholder for script sandbox.
pub struct Sandbox;

impl Default for Sandbox {
    fn default() -> Self {
        Self::new()
    }
}

impl Sandbox {
    pub fn new() -> Self {
        Self
    }
}
