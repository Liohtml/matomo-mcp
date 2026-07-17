use anyhow::{Context, Result};
use clap::Parser;
use rmcp::{transport::stdio, ServiceExt};
use tracing::{info, warn};
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

    // Lazy configuration: the MCP server always starts and answers
    // introspection; without a URL, tool calls return setup guidance.
    let client = match &config.base_url {
        Some(_) => Some(MatomoClient::new(&config)?),
        None => {
            warn!("MATOMO_URL is not set — tool calls will return configuration guidance");
            None
        }
    };

    if args.check {
        let client = client.context("--check requires a Matomo URL (--url or MATOMO_URL)")?;
        return check(&client).await;
    }

    let url_display = config
        .base_url
        .as_ref()
        .map(|u| u.to_string())
        .unwrap_or_else(|| "(not configured)".to_string());

    let registry = Registry::new(config.default_site_id);
    info!(
        tools = registry.tool_count(),
        url = %url_display,
        "starting matomo-mcp on stdio"
    );

    let service = MatomoServer::new(client, registry, url_display, config.max_response_chars);

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
