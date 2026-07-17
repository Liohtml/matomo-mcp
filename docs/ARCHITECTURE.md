# Architecture & Design Decisions

_Design record for matomo-mcp, 2026-07-17._

## Goal

An MCP server that lets any LLM answer real analytics questions against a
Matomo instance — fast, safe, and without drowning the model in tools.

## The core decision: curated tools, not generated ones

Prior art (FGRibreau/mcp-matomo) introspects the Matomo instance at startup and
generates one MCP tool per API method (~70+ tools). That approach has costs:

- **Context flooding.** Every tool definition is sent to the model on every
  request. 70+ near-identical tools with guessed parameter types measurably
  hurt tool selection quality.
- **Fragility.** The method list comes from `API.getReportMetadata` plus
  regex-parsing the `API.listAllAPI` HTML page; parameter types are inferred
  from parameter *names*.
- **Slow startup.** Multiple network round-trips before the server can answer
  its first request.

matomo-mcp inverts this: **14 hand-written tools** modeled on the questions
people actually ask (traffic? top pages? where from? which devices? live right
now?), each mapping to one or more Matomo reporting methods via a small
declarative catalog (`src/tools.rs`). One escape hatch — `matomo_api` — keeps
the entire Reporting API reachable, so curation never becomes a cage.

Consequences:

- Startup does zero network I/O; schemas are precomputed once.
- Parameter types, enums and defaults are exact, not inferred.
- Descriptions are written for an LLM choosing between tools.
- New Matomo features don't break the server; they're reachable via
  `matomo_api` immediately and get a curated tool when they earn one.

## Module map

```
src/
├── main.rs      CLI entry: parse args, wire everything, serve stdio. Also `--check`.
├── config.rs    clap Args → validated Config (URL scheme, header parsing).
├── client.rs    Matomo HTTP client: POST-only auth, retries, error mapping,
│                token redaction, response-size preview caps.
├── tools.rs     Declarative tool catalog + Registry (schema build, dispatch).
└── server.rs    rmcp ServerHandler: list_tools / call_tool, response shaping.
```

Data flow for a tool call:

```
MCP client ──stdio──▶ server.rs (call_tool)
                        │  Registry::resolve → Invocation {method, params}
                        ▼
                      client.rs ──POST index.php──▶ Matomo
                        │  retry 429/5xx, map errors, redact token
                        ▼
                      server.rs shape_response (truncate at budget) ──▶ MCP client
```

## Security posture

- Read-only by intent: curated tools only call reporting methods.
- `token_auth` travels exclusively in POST bodies (never URLs → never in
  access logs) and is redacted from every error string.
- `module`, `method`, `format`, `token_auth` cannot be overridden via the
  `matomo_api` params object.
- TLS verification on by default; `--insecure` is an explicit opt-in
  (the prior art hard-coded `danger_accept_invalid_certs(true)` for
  introspection).

## Context-window protection

Matomo can return megabytes of JSON. Two layers keep tool results small:

1. Server-side: every list tool sends `filter_limit` (default 20).
2. Client-side: `shape_response` truncates at `--max-response-chars`
   (default 50k) and appends guidance on how to narrow the query.

## Error UX

Errors are written for the model to *act on*: Matomo's `{"result":"error"}`
payloads are mapped to messages with hints ("call matomo_list_sites", "check
the segment syntax"), and transient failures (429/5xx/network) are retried
with exponential backoff before surfacing.

## Testing strategy

- Unit tests cover the dispatch table: defaults, select-case routing, coercion,
  reserved-key stripping, schema generation (33 tests total).
- Integration tests run the real client against an in-process wiremock server:
  form encoding, sub-directory installs, retry behavior, redaction. No live
  Matomo needed; the suite is offline and deterministic.

## Deliberate non-goals (for now)

- **HTTP/SSE transport** — stdio covers the dominant client setups; a
  streamable-HTTP mode is on the roadmap.
- **Write operations** (creating goals/segments) — read-only is a feature.
- **Response schema inference** — high effort, low LLM benefit.
