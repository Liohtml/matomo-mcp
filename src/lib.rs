//! matomo-mcp — MCP server for Matomo Analytics.
//!
//! Exposes a curated set of analytics tools (plus a raw-API escape hatch)
//! to MCP clients over stdio. See `README.md` for usage.

pub mod client;
pub mod config;
pub mod server;
pub mod tools;
