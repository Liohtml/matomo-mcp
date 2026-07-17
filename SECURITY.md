# Security Policy

## Supported versions

The latest release receives security fixes.

## Reporting a vulnerability

Please **do not** open a public issue for security problems. Use GitHub's
[private vulnerability reporting](https://github.com/Liohtml/matomo-mcp/security/advisories/new)
instead. You will get a response within a few days.

## Design notes relevant to security

- The server is **read-only by design**: the curated tools only call Matomo
  reporting methods. The `matomo_api` escape hatch can technically reach any
  API method the token permits — scope your `token_auth` to **view access** on
  the sites you need, nothing more.
- The token is sent exclusively via `POST` form bodies, never in URLs, so it
  does not end up in server access logs.
- The token is **redacted** from every error message before it reaches the
  model or the logs.
- Reserved parameters (`token_auth`, `module`, `method`, `format`) cannot be
  overridden through tool arguments.
- TLS certificate verification is **on by default**; `--insecure` must be an
  explicit choice.
