//! Integration tests for the Matomo HTTP client against a mock server.

use matomo_mcp::client::MatomoClient;
use matomo_mcp::config::{Args, Config};
use serde_json::json;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn config_for(url: &str) -> Config {
    let args = Args {
        url: Some(url.to_string()),
        token: Some("test-token".to_string()),
        default_site_id: None,
        headers: vec!["X-Test-Header:hello".to_string()],
        timeout_secs: 5,
        insecure: false,
        max_response_chars: 50_000,
        check: false,
    };
    Config::from_args(&args).unwrap()
}

#[tokio::test]
async fn calls_api_with_form_encoded_token_and_params() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/index.php"))
        .and(body_string_contains("module=API"))
        .and(body_string_contains("method=VisitsSummary.get"))
        .and(body_string_contains("format=JSON"))
        .and(body_string_contains("token_auth=test-token"))
        .and(body_string_contains("idSite=1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"nb_visits": 123})))
        .expect(1)
        .mount(&mock)
        .await;

    let client = MatomoClient::new(&config_for(&mock.uri())).unwrap();
    let result = client
        .call(
            "VisitsSummary.get",
            &[("idSite".to_string(), "1".to_string())],
        )
        .await
        .unwrap();

    assert_eq!(result, json!({"nb_visits": 123}));
}

#[tokio::test]
async fn sends_user_agent_and_extra_headers() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/index.php"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&mock)
        .await;

    let client = MatomoClient::new(&config_for(&mock.uri())).unwrap();
    client.call("API.getMatomoVersion", &[]).await.unwrap();

    let requests = mock.received_requests().await.unwrap();
    let request: &Request = &requests[0];
    let ua = request.headers.get("user-agent").unwrap().to_str().unwrap();
    assert!(ua.starts_with("matomo-mcp/"));
    assert_eq!(request.headers.get("x-test-header").unwrap(), "hello");
}

#[tokio::test]
async fn preserves_subdirectory_installs() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/matomo/index.php"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .expect(1)
        .mount(&mock)
        .await;

    let client = MatomoClient::new(&config_for(&format!("{}/matomo", mock.uri()))).unwrap();
    let result = client.call("API.getMatomoVersion", &[]).await.unwrap();
    assert_eq!(result, json!({"ok": true}));
}

#[tokio::test]
async fn maps_matomo_error_payload_with_hint_and_redacts_token() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/index.php"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": "error",
            "message": "The token_auth 'test-token' cannot be authenticated"
        })))
        .mount(&mock)
        .await;

    let client = MatomoClient::new(&config_for(&mock.uri())).unwrap();
    let err = client
        .call("VisitsSummary.get", &[])
        .await
        .unwrap_err()
        .to_string();

    assert!(err.contains("cannot be authenticated"));
    assert!(err.contains("Hint:"), "expected a hint, got: {err}");
    assert!(
        !err.contains("test-token"),
        "token leaked into error: {err}"
    );
    assert!(err.contains("[REDACTED]"));
}

#[tokio::test]
async fn retries_transient_5xx_then_succeeds() {
    let mock = MockServer::start().await;

    // First two attempts fail with 503, third succeeds.
    Mock::given(method("POST"))
        .and(path("/index.php"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(2)
        .expect(2)
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path("/index.php"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"recovered": true})))
        .expect(1)
        .mount(&mock)
        .await;

    let client = MatomoClient::new(&config_for(&mock.uri())).unwrap();
    let result = client.call("API.getMatomoVersion", &[]).await.unwrap();
    assert_eq!(result, json!({"recovered": true}));
}

#[tokio::test]
async fn does_not_retry_auth_failures() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/index.php"))
        .respond_with(ResponseTemplate::new(401).set_body_string("nope"))
        .expect(1)
        .mount(&mock)
        .await;

    let client = MatomoClient::new(&config_for(&mock.uri())).unwrap();
    let err = client
        .call("API.getMatomoVersion", &[])
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("401"));
    assert!(err.to_lowercase().contains("token"));
}

#[tokio::test]
async fn non_json_response_is_a_clear_error() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/index.php"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>login page</html>"))
        .mount(&mock)
        .await;

    let client = MatomoClient::new(&config_for(&mock.uri())).unwrap();
    let err = client
        .call("API.getMatomoVersion", &[])
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("non-JSON"));
}
