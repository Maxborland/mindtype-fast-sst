//! MindType Desktop Application
//!
//! A voice-to-text application with System 6 inspired UI.

mod app;
mod messages;
mod theme;
mod ui;

use anyhow::Result;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("mindtype=debug".parse()?))
        .init();

    info!("MindType v{} starting...", env!("CARGO_PKG_VERSION"));

    // Run the app
    app::run()?;

    Ok(())
}
