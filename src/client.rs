//! HTTP client for the Matomo Reporting API.
//!
//! All requests go through `POST index.php` with form-encoded parameters so the
//! `token_auth` never appears in URLs or server access logs. Transient failures
//! (429/5xx, timeouts, connection errors) are retried with exponential backoff.

use std::time::Duration;

use reqwest::header::{HeaderValue, USER_AGENT};
use reqwest::{Client, StatusCode};
use serde_json::Value;
use thiserror::Error;
use tracing::{debug, warn};
use url::Url;

use crate::config::Config;

const MAX_ATTEMPTS: u32 = 3;
const BACKOFF_BASE_MS: u64 = 250;
/// Cap error bodies quoted back to the model.
const ERROR_BODY_PREVIEW: usize = 500;

#[derive(Debug, Error)]
pub enum MatomoError {
    #[error("Matomo API error: {message}{}", hint_suffix(.hint))]
    Api {
        message: String,
        hint: Option<&'static str>,
    },

    #[error("HTTP {status} from Matomo: {body}{}", hint_suffix(.hint))]
    Http {
        status: StatusCode,
        body: String,
        hint: Option<&'static str>,
    },

    #[error("could not reach Matomo: {0}. Check the URL, your network/VPN, and that the instance is up.")]
    Network(String),

    #[error("Matomo returned a non-JSON response: {0}")]
    InvalidResponse(String),
}

fn hint_suffix(hint: &Option<&'static str>) -> String {
    hint.map(|h| format!(" Hint: {h}")).unwrap_or_default()
}

/// Client for a single Matomo instance.
#[derive(Debug, Clone)]
pub struct MatomoClient {
    http: Client,
    endpoint: Url,
    token: Option<String>,
}

impl MatomoClient {
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        // Preserve sub-directory installs (https://example.com/matomo/ → .../matomo/index.php).
        let mut base = config.base_url.clone();
        if !base.path().ends_with('/') {
            base.set_path(&format!("{}/", base.path()));
        }
        let endpoint = base.join("index.php")?;

        let mut headers = config.extra_headers.clone();
        headers.insert(
            USER_AGENT,
            HeaderValue::try_from(format!("matomo-mcp/{}", env!("CARGO_PKG_VERSION")))
                .expect("static user agent is valid"),
        );

        let mut builder = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .default_headers(headers);
        if config.insecure {
            warn!("TLS certificate verification is DISABLED (--insecure)");
            builder = builder.danger_accept_invalid_certs(true);
        }

        Ok(Self {
            http: builder.build()?,
            endpoint,
            token: config.token.clone(),
        })
    }

    /// Call a Matomo API method (e.g. `VisitsSummary.get`) with query parameters.
    pub async fn call(
        &self,
        method: &str,
        params: &[(String, String)],
    ) -> Result<Value, MatomoError> {
        let mut form: Vec<(&str, &str)> =
            vec![("module", "API"), ("method", method), ("format", "JSON")];
        if let Some(token) = &self.token {
            form.push(("token_auth", token));
        }
        for (k, v) in params {
            form.push((k.as_str(), v.as_str()));
        }

        debug!(method, params = params.len(), "calling Matomo API");

        let mut attempt = 0;
        loop {
            attempt += 1;
            match self.send_once(&form).await {
                Err(err) if attempt < MAX_ATTEMPTS && is_retryable(&err) => {
                    let delay = BACKOFF_BASE_MS * 2u64.pow(attempt - 1);
                    warn!(
                        method,
                        attempt,
                        delay_ms = delay,
                        "retrying after transient error: {err}"
                    );
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
                result => return result,
            }
        }
    }

    async fn send_once(&self, form: &[(&str, &str)]) -> Result<Value, MatomoError> {
        let response = self
            .http
            .post(self.endpoint.clone())
            .form(form)
            .send()
            .await
            .map_err(|e| MatomoError::Network(scrub(&e.to_string(), &self.token)))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| MatomoError::Network(scrub(&e.to_string(), &self.token)))?;

        if !status.is_success() {
            let hint = match status {
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Some(
                    "check that MATOMO_TOKEN is valid and has at least 'view' access to this site",
                ),
                StatusCode::NOT_FOUND => {
                    Some("check that MATOMO_URL points at the Matomo root (the folder containing index.php)")
                }
                _ => None,
            };
            return Err(MatomoError::Http {
                status,
                body: scrub(preview(&text), &self.token),
                hint,
            });
        }

        let json: Value = serde_json::from_str(&text)
            .map_err(|_| MatomoError::InvalidResponse(scrub(preview(&text), &self.token)))?;

        // Matomo signals errors as HTTP 200 + {"result": "error", "message": "..."}.
        if let Some(obj) = json.as_object() {
            if obj.get("result").and_then(Value::as_str) == Some("error") {
                let message = obj
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown error")
                    .to_string();
                let lower = message.to_lowercase();
                let hint = if lower.contains("token_auth") || lower.contains("authenticate") {
                    Some("check that MATOMO_TOKEN is valid and has at least 'view' access")
                } else if lower.contains("idsite") {
                    Some("the site_id may be wrong — call matomo_list_sites to see available sites")
                } else if lower.contains("segment") {
                    Some("the segment expression is invalid — see https://matomo.org/docs/segmentation/")
                } else {
                    None
                };
                return Err(MatomoError::Api {
                    message: scrub(&message, &self.token),
                    hint,
                });
            }
        }

        Ok(json)
    }
}

fn is_retryable(err: &MatomoError) -> bool {
    match err {
        MatomoError::Network(_) => true,
        MatomoError::Http { status, .. } => matches!(
            *status,
            StatusCode::TOO_MANY_REQUESTS
                | StatusCode::INTERNAL_SERVER_ERROR
                | StatusCode::BAD_GATEWAY
                | StatusCode::SERVICE_UNAVAILABLE
                | StatusCode::GATEWAY_TIMEOUT
        ),
        _ => false,
    }
}

fn preview(text: &str) -> &str {
    let end = text
        .char_indices()
        .nth(ERROR_BODY_PREVIEW)
        .map(|(i, _)| i)
        .unwrap_or(text.len());
    &text[..end]
}

/// Never let the auth token leak into error messages shown to the model or logs.
fn scrub(text: &str, token: &Option<String>) -> String {
    match token {
        Some(t) if !t.is_empty() => text.replace(t.as_str(), "[REDACTED]"),
        _ => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrub_removes_token() {
        let token = Some("supersecret".to_string());
        assert_eq!(
            scrub("error near supersecret in query", &token),
            "error near [REDACTED] in query"
        );
        assert_eq!(scrub("clean", &None), "clean");
    }

    #[test]
    fn preview_respects_char_boundaries() {
        let s = "ä".repeat(1_000);
        let p = preview(&s);
        assert_eq!(p.chars().count(), ERROR_BODY_PREVIEW);
    }

    #[test]
    fn retryable_classification() {
        assert!(is_retryable(&MatomoError::Network("timeout".into())));
        assert!(is_retryable(&MatomoError::Http {
            status: StatusCode::SERVICE_UNAVAILABLE,
            body: String::new(),
            hint: None,
        }));
        assert!(!is_retryable(&MatomoError::Http {
            status: StatusCode::UNAUTHORIZED,
            body: String::new(),
            hint: None,
        }));
        assert!(!is_retryable(&MatomoError::Api {
            message: "bad segment".into(),
            hint: None,
        }));
    }
}
