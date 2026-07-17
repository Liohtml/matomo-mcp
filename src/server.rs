//! MCP server handler wiring the tool registry to the Matomo client.

use std::sync::Arc;

use rmcp::handler::server::ServerHandler;
use rmcp::model::*;
use rmcp::service::{RequestContext, RoleServer};
use rmcp::ErrorData;
use tracing::debug;

use crate::client::MatomoClient;
use crate::tools::Registry;

#[derive(Clone)]
pub struct MatomoServer {
    client: Arc<MatomoClient>,
    registry: Arc<Registry>,
    matomo_url: String,
    max_response_chars: usize,
}

impl MatomoServer {
    pub fn new(
        client: MatomoClient,
        registry: Registry,
        matomo_url: String,
        max_response_chars: usize,
    ) -> Self {
        Self {
            client: Arc::new(client),
            registry: Arc::new(registry),
            matomo_url,
            max_response_chars,
        }
    }
}

/// Compact-serialize a response, truncating at a character budget so a single
/// tool call can never flood the model's context window.
pub fn shape_response(value: &serde_json::Value, max_chars: usize) -> String {
    let text = value.to_string();
    if text.chars().count() <= max_chars {
        return text;
    }
    let cut: String = text.chars().take(max_chars).collect();
    format!(
        "{cut}\n... [response truncated at {max_chars} characters — narrow the query with \
         'limit', a shorter date range, or a segment]"
    )
}

impl ServerHandler for MatomoServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "matomo-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                title: Some("Matomo Analytics".to_string()),
                website_url: Some("https://github.com/Liohtml/matomo-mcp".to_string()),
            },
            instructions: Some(format!(
                "Matomo Analytics for {url}. {count} read-only tools.\n\n\
                 Workflow:\n\
                 1. If the site_id is unknown, call matomo_list_sites first.\n\
                 2. Use the dedicated matomo_* tools for common questions (traffic, pages, \
                 referrers, goals, e-commerce, devices, locations, real-time).\n\
                 3. For anything else, call matomo_api with any 'Module.action' method; \
                 discover methods via method='API.getReportMetadata'.\n\n\
                 Dates accept 'today', 'yesterday', 'YYYY-MM-DD', 'last7'/'last30', or a \
                 'start,end' range with period=range. Most tools default to yesterday/day.",
                url = self.matomo_url,
                count = self.registry.tool_count(),
            )),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            tools: self.registry.mcp_tools(),
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let tool_name = request.name.as_ref();
        let args = request.arguments.unwrap_or_default();
        debug!(tool = tool_name, "tool call");

        let invocation = match self.registry.resolve(tool_name, &args) {
            Ok(inv) => inv,
            Err(message) => return Ok(error_result(message)),
        };

        match self
            .client
            .call(&invocation.method, &invocation.params)
            .await
        {
            Ok(value) => Ok(CallToolResult {
                content: vec![Content::text(shape_response(
                    &value,
                    self.max_response_chars,
                ))],
                is_error: Some(false),
                meta: None,
                structured_content: None,
            }),
            Err(err) => Ok(error_result(err.to_string())),
        }
    }
}

fn error_result(message: String) -> CallToolResult {
    CallToolResult {
        content: vec![Content::text(message)],
        is_error: Some(true),
        meta: None,
        structured_content: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn shape_passes_small_responses_through() {
        let value = json!({"nb_visits": 42});
        assert_eq!(shape_response(&value, 1_000), value.to_string());
    }

    #[test]
    fn shape_truncates_large_responses_with_guidance() {
        let value = json!(vec!["row"; 10_000]);
        let shaped = shape_response(&value, 500);
        assert!(shaped.contains("truncated at 500"));
        assert!(shaped.contains("limit"));
        assert!(shaped.chars().count() < 700);
    }

    #[test]
    fn shape_is_utf8_safe() {
        let value = json!("ü".repeat(2_000));
        let shaped = shape_response(&value, 100);
        assert!(shaped.contains("truncated"));
    }
}
