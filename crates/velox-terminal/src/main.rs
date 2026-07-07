//! # velox-terminal
//!
//! Binary entry point for the velox-terminal trading platform.
//!
//! Initializes logging, parses CLI args, creates the window + GPU context,
//! and runs the winit event loop.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(deprecated)]

mod app;
mod input;

use clap::Parser;
use winit::event_loop::EventLoop;

#[derive(Parser, Debug)]
#[command(name = "velox-terminal")]
#[command(version, about = "GPU-accelerated trading terminal", long_about = None)]
struct Cli {
    /// Enable paper trading mode
    #[arg(long, default_value_t = true)]
    paper: bool,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // ── Tracing ─────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(&cli.log_level)
        .init();

    tracing::info!("velox-terminal v{} starting", env!("CARGO_PKG_VERSION"));

    // ── Event loop ──────────────────────────────────────────────
    let event_loop = EventLoop::new()?;

    let mut app = app::App::new(&event_loop)?;

    event_loop.run(move |event, elwt| {
        app.handle_event(event, elwt);
    })?;

    Ok(())
}
