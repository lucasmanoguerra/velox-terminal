//! # velox-terminal
//!
//! Binary entry point for the velox-terminal trading platform.
//!
//! ## Phases
//!
//! 1. Core data structures + OMS/Risk (current)
//! 2. Broker connectivity (FIX, WebSocket)
//! 3. Storage engine + backtesting
//! 4. GPU rendering + UI
//! 5. Scripting engine
//! 6. Performance optimization
//! 7. Security + compliance hardening

use clap::Parser;

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

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(&cli.log_level)
        .init();

    tracing::info!("velox-terminal starting (paper={})", cli.paper);
    tracing::info!("Phase 1: Core data structures + OMS/Risk ready");

    // Placeholder: UI loop will go here
    tracing::info!("velox-terminal initialized successfully");
    println!("velox-terminal v{}", env!("CARGO_PKG_VERSION"));

    Ok(())
}
