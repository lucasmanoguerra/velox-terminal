//! # velox-terminal
//!
//! Binary entry point for the velox-terminal trading platform.
//!
//! Initializes logging, parses CLI args, creates the tokio async runtime
//! (for WebSocket exchange connections), creates the window + GPU context,
//! and runs the winit event loop.
//!
//! # Concurrency Model
//!
//! - **Main thread**: winit event loop + egui UI + GPU rendering
//! - **Tokio tasks**: WebSocket exchange feed, network I/O, market data pipeline
//! - **Ring buffer**: Lock-free SPSC bridge between tokio (producer) and main thread (consumer)

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

    // ── Tokio runtime ───────────────────────────────────────────
    // Required for WebSocket exchange connections and async network I/O.
    // The runtime is entered so that tokio::spawn works from non-async
    // contexts (e.g., BinanceFeed::start()).
    let tokio_rt = tokio::runtime::Runtime::new()?;
    let _guard = tokio_rt.enter();
    tracing::info!("Tokio runtime initialized");

    // ── Event loop ──────────────────────────────────────────────
    let event_loop = EventLoop::new()?;
    let mut app = app::App::new(&event_loop)?;

    event_loop.run(move |event, elwt| {
        app.handle_event(event, elwt);
    })?;

    // Tokio runtime dropped here — ensures all spawned tasks are cancelled.
    tracing::info!("velox-terminal shutting down");
    Ok(())
}
