//! CLI arguments and runtime configuration.

use anyhow::{bail, Context, Result};
use clap::Parser;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use url::Url;

/// MCP server for Matomo Analytics.
#[derive(Parser, Debug)]
#[command(
    name = "matomo-mcp",
    version,
    about = "MCP server for Matomo Analytics — curated analytics tools over stdio",
    long_about = "Exposes a curated set of Matomo Analytics tools to MCP clients \
                  (Claude, Cursor, VS Code, ...) plus a generic escape hatch for the full API.\n\n\
                  Minimal setup:\n\
                    matomo-mcp --url https://matomo.example.com --token YOUR_TOKEN"
)]
pub struct Args {
    /// Matomo instance URL, e.g. https://matomo.example.com or https://example.com/matomo/
    #[arg(short, long, env = "MATOMO_URL")]
    pub url: String,

    /// Matomo API token (token_auth). Create one under Settings → Personal → Security.
    #[arg(short, long, env = "MATOMO_TOKEN", hide_env_values = true)]
    pub token: Option<String>,

    /// Default site ID. When set, tools no longer require an explicit site_id argument.
    #[arg(short = 's', long, env = "MATOMO_DEFAULT_SITE_ID")]
    pub default_site_id: Option<u64>,

    /// Extra HTTP header sent with every request ("Name:Value"). Repeatable.
    /// Via env: comma-separated list in MATOMO_EXTRA_HEADERS.
    #[arg(
        short = 'H',
        long = "header",
        env = "MATOMO_EXTRA_HEADERS",
        value_delimiter = ','
    )]
    pub headers: Vec<String>,

    /// HTTP timeout per request, in seconds.
    #[arg(long, env = "MATOMO_TIMEOUT_SECS", default_value_t = 30)]
    pub timeout_secs: u64,

    /// Accept invalid/self-signed TLS certificates. Off by default; enable deliberately.
    #[arg(long, env = "MATOMO_INSECURE", default_value_t = false)]
    pub insecure: bool,

    /// Maximum characters of a tool response before it is truncated (protects the LLM context).
    #[arg(long, env = "MATOMO_MAX_RESPONSE_CHARS", default_value_t = 50_000)]
    pub max_response_chars: usize,

    /// Verify connectivity and token against the Matomo instance, print a summary, then exit.
    #[arg(long)]
    pub check: bool,
}

/// Validated runtime configuration.
#[derive(Debug, Clone)]
pub struct Config {
    pub base_url: Url,
    pub token: Option<String>,
    pub default_site_id: Option<u64>,
    pub extra_headers: HeaderMap,
    pub timeout_secs: u64,
    pub insecure: bool,
    pub max_response_chars: usize,
}

impl Config {
    pub fn from_args(args: &Args) -> Result<Self> {
        let base_url =
            Url::parse(&args.url).with_context(|| format!("invalid Matomo URL: {}", args.url))?;
        if !matches!(base_url.scheme(), "http" | "https") {
            bail!("Matomo URL must start with http:// or https://");
        }

        Ok(Self {
            base_url,
            token: args.token.clone(),
            default_site_id: args.default_site_id,
            extra_headers: parse_headers(&args.headers)?,
            timeout_secs: args.timeout_secs,
            insecure: args.insecure,
            max_response_chars: args.max_response_chars.max(1_000),
        })
    }
}

/// Parse "Name:Value" pairs into a `HeaderMap`.
pub fn parse_headers(pairs: &[String]) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    for pair in pairs {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let (name, value) = pair
            .split_once(':')
            .with_context(|| format!("invalid header '{pair}', expected 'Name:Value'"))?;
        let name = HeaderName::try_from(name.trim())
            .with_context(|| format!("invalid header name in '{pair}'"))?;
        let value = HeaderValue::try_from(value.trim())
            .with_context(|| format!("invalid header value in '{pair}'"))?;
        headers.insert(name, value);
    }
    Ok(headers)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_args() -> Args {
        Args {
            url: "https://matomo.example.com".into(),
            token: Some("secret".into()),
            default_site_id: None,
            headers: vec![],
            timeout_secs: 30,
            insecure: false,
            max_response_chars: 50_000,
            check: false,
        }
    }

    #[test]
    fn accepts_valid_url() {
        let cfg = Config::from_args(&base_args()).unwrap();
        assert_eq!(cfg.base_url.host_str(), Some("matomo.example.com"));
    }

    #[test]
    fn rejects_non_http_scheme() {
        let mut args = base_args();
        args.url = "ftp://example.com".into();
        assert!(Config::from_args(&args).is_err());
    }

    #[test]
    fn parses_headers() {
        let headers = parse_headers(&[
            "X-Auth:token123".into(),
            " X-Tenant : acme ".into(),
            "Authorization:Bearer:a:b".into(),
        ])
        .unwrap();
        assert_eq!(headers.get("X-Auth").unwrap(), "token123");
        assert_eq!(headers.get("X-Tenant").unwrap(), "acme");
        assert_eq!(headers.get("Authorization").unwrap(), "Bearer:a:b");
    }

    #[test]
    fn rejects_header_without_colon() {
        assert!(parse_headers(&["NoColonHere".into()]).is_err());
    }

    #[test]
    fn enforces_minimum_response_budget() {
        let mut args = base_args();
        args.max_response_chars = 10;
        let cfg = Config::from_args(&args).unwrap();
        assert_eq!(cfg.max_response_chars, 1_000);
    }
}
