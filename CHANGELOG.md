# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versions follow
[SemVer](https://semver.org/).

## [Unreleased]

## [0.1.0] - 2026-07-17

### Added

- Initial release: MCP server over stdio with 14 curated, read-only Matomo
  tools (sites, visits summary, pages, referrers, events, goals, e-commerce,
  geo, devices, visit times, site search, real-time, page performance) plus
  the `matomo_api` escape hatch for the full Reporting API.
- Instant startup — no introspection round-trips against the Matomo instance.
- Default site support (`--default-site-id` / `MATOMO_DEFAULT_SITE_ID`).
- Automatic retries with exponential backoff for transient HTTP failures.
- Response truncation budget (`--max-response-chars`) to protect the model's
  context window, with actionable guidance appended.
- Token redaction in all error messages; token sent via POST body only.
- Sub-directory Matomo installs supported (`https://example.com/matomo/`).
- `--check` command to verify URL, token and site access.
- Extra HTTP headers via `--header` / `MATOMO_EXTRA_HEADERS` (auth proxies,
  multi-tenant setups).
- Cross-platform binaries, Docker image, and crates.io packaging.

[Unreleased]: https://github.com/Liohtml/matomo-mcp/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Liohtml/matomo-mcp/releases/tag/v0.1.0
