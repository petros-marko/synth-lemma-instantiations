use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::{self, EnvFilter};

use crate::flux_mcp::FluxMcp;

mod diagnostics;
mod flux_mcp;
mod flux_runner;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting Flux MCP Server");

    let service = FluxMcp::new().serve(stdio()).await.inspect_err(|err| {
        tracing::error!("serving error {:?}", err);
    })?;

    service.waiting().await?;
    Ok(())
}
