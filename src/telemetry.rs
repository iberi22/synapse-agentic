//! Telemetry & Logging Setup
//!
//! Provides structured logging and OpenTelemetry integration.

use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initializes the telemetry system.
///
/// Uses `RUST_LOG` environment variable for filtering (default: info).
pub fn init_telemetry() -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,synapse_agentic=debug"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .pretty();

    // In a real production setup, we would add the OpenTelemetry layer here
    // if the 'telemetry' feature is enabled.

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .try_init()?;

    Ok(())
}

#[cfg(feature = "telemetry")]
pub fn init_opentelemetry(service_name: &str) -> Result<()> {
    // Basic stub for OpenTelemetry initialization
    // Implementation would depend on the specific exporter (OTLP, Jaeger, etc.)
    // For now we just stick to fmt
    init_telemetry()
}
