//! # velox-ui
//!
//! UI components built with egui over wgpu.
//!
//! Panels: chart, DOM ladder, order entry, watchlist, positions, account.

pub mod app_state;
pub mod panels;
pub mod theme;

pub use app_state::AppState;
pub use panels::PanelManager;
