use anyhow::{Context, Result};
use clap::Parser;
use rmcp::{transport::stdio, ServiceExt};
use tracing::info;
use tracing_subscriber::EnvFilter;

use matomo_mcp::client::MatomoClient;
use matomo_mcp::config::{Args, Config};
use matomo_mcp::server::MatomoServer;
use matomo_mcp::tools::Registry;

#[tokio::main]
async fn main() -> Result<()> {
    // Logs go to stderr — stdout is reserved for the MCP stdio transport.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();
    let config = Config::from_args(&args)?;
    let client = MatomoClient::new(&config)?;

    if args.check {
        return check(&client).await;
    }

    let registry = Registry::new(config.default_site_id);
    info!(
        tools = registry.tool_count(),
        url = %config.base_url,
        "starting matomo-mcp on stdio"
    );

    let service = MatomoServer::new(
        client,
        registry,
        config.base_url.to_string(),
        config.max_response_chars,
    );

    let server = service
        .serve(stdio())
        .await
        .context("failed to start MCP server on stdio")?;
    server.waiting().await?;

    info!("matomo-mcp stopped");
    Ok(())
}

/// `--check`: verify URL + token and print what the server can see, then exit.
async fn check(client: &MatomoClient) -> Result<()> {
    let version = client.call("API.getMatomoVersion", &[]).await?;
    let version = version
        .get("value")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    println!("✓ Connected — Matomo version {version}");

    let sites = client
        .call(
            "SitesManager.getSitesWithAtLeastViewAccess",
            &[("filter_limit".to_string(), "100".to_string())],
        )
        .await?;
    match sites.as_array() {
        Some(list) if !list.is_empty() => {
            println!("✓ Token grants access to {} site(s):", list.len());
            for site in list {
                let id = site
                    .get("idsite")
                    .map(|v| v.to_string())
                    .unwrap_or_default();
                let name = site.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                let url = site.get("main_url").and_then(|v| v.as_str()).unwrap_or("");
                println!("    #{} {} ({})", id.trim_matches('"'), name, url);
            }
        }
        _ => println!("⚠ Token is valid but has no site access — check its permissions."),
    }
    Ok(())
}
