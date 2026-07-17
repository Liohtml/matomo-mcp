# Announcement drafts

Ready-to-paste posts. Personal accounts required, so these are published
manually. Adjust tone freely — they're written to be honest, not salesy.

---

## 1. Matomo Forum — forum.matomo.org

**Category:** "Community Plugins & Integrations" (or "General Discussion")
**Title:** `matomo-mcp — talk to your Matomo instance from Claude, Cursor & other AI assistants (open source, Rust)`

I built an open-source [MCP](https://modelcontextprotocol.io) server for Matomo and wanted to share it here:

**https://github.com/Liohtml/matomo-mcp**

MCP is the protocol that lets AI assistants (Claude, Cursor, VS Code Copilot, …) call external tools. With matomo-mcp connected, you can ask things like:

- *"How was traffic yesterday, and where did it come from?"*
- *"Which internal searches return no results?"*
- *"Are we getting visitors from ChatGPT or Perplexity? Trend over 3 months."*
- *"Compare this week to last week and tell me what changed."*

…and the assistant answers straight from your Matomo instance — including follow-up questions.

Design decisions that might interest this forum:

- **Read-only by design.** The 14 curated tools only call Reporting API methods; a view-only `token_auth` is all it needs.
- **Token safety.** The token travels exclusively in POST bodies (never in URLs → never in access logs) and is redacted from every error message.
- **Curated, not generated.** Instead of mirroring all ~70+ API methods as tools (which overwhelms the models), it exposes 14 well-described tools + one escape hatch for the full Reporting API.
- Single static binary (Rust), Docker image, works with self-hosted and cloud Matomo, sub-directory installs supported.

Install: `cargo install matomo-mcp`, prebuilt binaries, or `ghcr.io/liohtml/matomo-mcp`. You can try it against `https://demo.matomo.cloud` without a token.

MIT licensed. Feedback very welcome — especially which reports you'd want as dedicated tools next (funnels and annotations are on the roadmap).

---

## 2. Reddit — r/rust

**Flair:** 🛠️ project
**Title:** `matomo-mcp: an MCP server for Matomo Analytics in Rust — curated tools instead of API mirroring`

Repo: https://github.com/Liohtml/matomo-mcp — MIT, `cargo install matomo-mcp`

I rebuilt an existing Matomo MCP server from scratch and the interesting part is less "it's in Rust" (the original was too) and more the design questions along the way:

**Tool curation vs. generation.** The common pattern for API-wrapping MCP servers is to introspect the API and generate one tool per method. For Matomo that's 70+ tools whose definitions get sent to the LLM on every request — measurably worse tool selection, guessed parameter types, fragile HTML parsing at startup. I went the opposite way: a declarative catalog of 14 hand-written tools (`&'static` specs + a small dispatch layer), each mapping real analytics questions to one or more API methods, plus one `matomo_api` escape hatch so nothing is unreachable. Schemas are exact and built once at startup; zero network I/O before the first request.

**Rust bits that pulled their weight:**

- `rmcp` (the official Rust MCP SDK) + tokio for the stdio transport
- a `cases!` macro for the report→method dispatch tables, so the whole catalog is data, not code
- wiremock for fully offline integration tests (retry behavior, token redaction, sub-directory URL handling) — 33 tests, no live Matomo needed
- 6-target release matrix incl. musl static builds; ~4 MB stripped binary, starts in milliseconds — nice fit for a process an MCP client spawns per session

**Safety posture** (it talks to production analytics): read-only tool set, `token_auth` only in POST bodies and scrubbed from every error string, TLS verification on by default, reserved params (`token_auth`, `module`, …) can't be overridden through tool arguments, and responses are hard-capped so one tool call can't flood the model's context.

Happy to answer questions about rmcp, the dispatch design, or what I'd do differently.

---

## 3. mcpservers.org submission form (wong2's list — no PRs accepted)

**URL:** https://mcpservers.org/submit

| Field | Value |
|---|---|
| Server Name | `matomo-mcp` |
| Short Description | `Curated read-only Matomo Analytics tools — traffic, pages, referrers, goals, e-commerce, devices, real-time visitors — plus a full-API escape hatch. Rust, single binary.` |
| Link | `https://github.com/Liohtml/matomo-mcp` |
| Category | Monitoring (or Analytics, whichever the dropdown offers) |
| Contact Email | *(your private email)* |

The free tier is sufficient; the $39 "premium" only buys faster review.
