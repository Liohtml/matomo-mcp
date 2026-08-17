//! Smoke test for the streamable HTTP transport: boot the server on an
//! ephemeral port and drive an MCP initialize handshake over plain HTTP.

use std::future::IntoFuture;

use matomo_mcp::server::MatomoServer;
use matomo_mcp::tools::Registry;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, tower::StreamableHttpService,
};
use rmcp::transport::StreamableHttpServerConfig;
use serde_json::json;

#[tokio::test]
async fn http_transport_answers_initialize() {
    let server = MatomoServer::new(None, Registry::new(None), "(not configured)".into(), 50_000);
    let service = StreamableHttpService::new(
        move || Ok(server.clone()),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig {
            sse_keep_alive: None,
            ..Default::default()
        },
    );
    let router = axum::Router::new().nest_service("/mcp", service);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(axum::serve(listener, router).into_future());

    let response = reqwest::Client::new()
        .post(format!("http://{addr}/mcp"))
        .header("Accept", "application/json, text/event-stream")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "smoke-test", "version": "0.0.0"}
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert!(
        response.headers().contains_key("mcp-session-id"),
        "initialize response must carry a session id"
    );
    let body = response.text().await.unwrap();
    assert!(body.contains("matomo-mcp"), "unexpected body: {body}");
}
