<div align="center">

<img src="assets/logo.svg" width="120" alt="matomo-mcp logo"/>

# matomo-mcp

**Talk to your Matomo Analytics.** From Claude, Cursor, VS Code, or any MCP client.

[![CI](https://github.com/Liohtml/matomo-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/Liohtml/matomo-mcp/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/matomo-mcp.svg)](https://crates.io/crates/matomo-mcp)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![MCP](https://img.shields.io/badge/MCP-compatible-8A2BE2)](https://modelcontextprotocol.io)

*14 curated, read-only analytics tools + a full-API escape hatch. Single binary, instant startup, context-friendly.*

[Quickstart](#-quickstart) · [Clients](#-connect-your-client) · [Tools](#-tools) · [Configuration](#%EF%B8%8F-configuration) · [FAQ](#-troubleshooting)

</div>

---

```text
You  ▸ How was traffic yesterday, and where did it come from?

Claude ▸ Yesterday you had 14,472 visits (11,416 unique visitors, 66% bounce rate).
         Top acquisition channels:
         1. Organic search — 6,120 visits (Google 92%)
         2. Direct — 4,890 visits
         3. AI assistants — 1,204 visits (↑ 31% vs. last week)
         Want me to break down which landing pages converted best?
```

Every question your Matomo dashboard can answer, your AI assistant can now answer too — including follow-ups, comparisons, and "why?".

## ✨ Why matomo-mcp?

| | |
|---|---|
| 🎯 **Curated, not generated** | 14 hand-crafted tools modeled on real analytics questions — not 70+ auto-generated API mirrors that flood the model's context and degrade tool selection. |
| ⚡ **Instant startup** | No introspection round-trips. One static binary, no Node, no Python, no runtime. Starts in milliseconds. |
| 🔒 **Safe by default** | Read-only reporting tools. Token sent via POST only (never in URLs/logs), redacted from every error. TLS verification on by default. |
| 🧠 **Context-friendly** | Row limits on every report and a hard response budget with actionable guidance — one tool call can never blow up the context window. |
| 📡 **Real-time included** | Live visitor counters and a visit log (`matomo_realtime`) — see what's happening *right now*. |
| 🧰 **Never a cage** | `matomo_api` reaches **any** Reporting API method (funnels, heatmaps, custom dimensions, …) when the curated tools don't cover it. |
| 🔁 **Resilient** | Automatic retries with backoff on 429/5xx/network hiccups. Helpful, hint-annotated error messages the model can act on. |

## 🚀 Quickstart

### 1. Install

**Prebuilt binary** (Linux, macOS, Windows) — grab it from [Releases](https://github.com/Liohtml/matomo-mcp/releases), or:

```bash
# Cargo
cargo install matomo-mcp

# From source
cargo install --git https://github.com/Liohtml/matomo-mcp

# Docker
docker pull ghcr.io/liohtml/matomo-mcp
```

### 2. Get a Matomo API token

Matomo → **Settings** (⚙) → **Personal** → **Security** → **Auth tokens** → *Create new token*.
View-only permissions are all it needs.

### 3. Verify the connection

```bash
matomo-mcp --url https://your-matomo.example.com --token YOUR_TOKEN --check
```

```text
✓ Connected — Matomo version 5.2.1
✓ Token grants access to 3 site(s):
    #1 My Shop (https://shop.example.com)
    #2 Blog (https://blog.example.com)
    #3 Docs (https://docs.example.com)
```

### 4. Connect your client ⬇

## 🔌 Connect your client

<details>
<summary><b>Claude Code</b></summary>

```bash
claude mcp add matomo \
  --env MATOMO_URL=https://your-matomo.example.com \
  --env MATOMO_TOKEN=YOUR_TOKEN \
  --env MATOMO_DEFAULT_SITE_ID=1 \
  -- matomo-mcp
```

</details>

<details>
<summary><b>Claude Desktop</b></summary>

Add to `claude_desktop_config.json` (macOS: `~/Library/Application Support/Claude/`, Windows: `%APPDATA%\Claude\`):

```json
{
  "mcpServers": {
    "matomo": {
      "command": "matomo-mcp",
      "env": {
        "MATOMO_URL": "https://your-matomo.example.com",
        "MATOMO_TOKEN": "YOUR_TOKEN",
        "MATOMO_DEFAULT_SITE_ID": "1"
      }
    }
  }
}
```

</details>

<details>
<summary><b>Cursor</b></summary>

`.cursor/mcp.json` (project) or `~/.cursor/mcp.json` (global):

```json
{
  "mcpServers": {
    "matomo": {
      "command": "matomo-mcp",
      "env": {
        "MATOMO_URL": "https://your-matomo.example.com",
        "MATOMO_TOKEN": "YOUR_TOKEN",
        "MATOMO_DEFAULT_SITE_ID": "1"
      }
    }
  }
}
```

</details>

<details>
<summary><b>VS Code (GitHub Copilot)</b></summary>

`.vscode/mcp.json`:

```json
{
  "servers": {
    "matomo": {
      "type": "stdio",
      "command": "matomo-mcp",
      "env": {
        "MATOMO_URL": "https://your-matomo.example.com",
        "MATOMO_TOKEN": "${input:matomo-token}",
        "MATOMO_DEFAULT_SITE_ID": "1"
      }
    }
  },
  "inputs": [
    {
      "id": "matomo-token",
      "type": "promptString",
      "description": "Matomo API token",
      "password": true
    }
  ]
}
```

</details>

<details>
<summary><b>Windsurf / Zed / other MCP clients</b></summary>

Any client that speaks MCP over stdio works with the generic shape:

```json
{
  "command": "matomo-mcp",
  "args": [],
  "env": {
    "MATOMO_URL": "https://your-matomo.example.com",
    "MATOMO_TOKEN": "YOUR_TOKEN",
    "MATOMO_DEFAULT_SITE_ID": "1"
  }
}
```

</details>

<details>
<summary><b>Docker (any client)</b></summary>

```json
{
  "mcpServers": {
    "matomo": {
      "command": "docker",
      "args": [
        "run", "-i", "--rm",
        "-e", "MATOMO_URL", "-e", "MATOMO_TOKEN", "-e", "MATOMO_DEFAULT_SITE_ID",
        "ghcr.io/liohtml/matomo-mcp"
      ],
      "env": {
        "MATOMO_URL": "https://your-matomo.example.com",
        "MATOMO_TOKEN": "YOUR_TOKEN",
        "MATOMO_DEFAULT_SITE_ID": "1"
      }
    }
  }
}
```

</details>

> [!TIP]
> Set `MATOMO_DEFAULT_SITE_ID` and the model never has to ask which site you mean.
> No token at hand? Try it against the public demo: `--url https://demo.matomo.cloud --default-site-id 1` (no token needed).

## 🧭 Tools

| Tool | Answers questions like |
|------|------------------------|
| `matomo_list_sites` | *"Which sites do we track?"* |
| `matomo_visits_summary` | *"How much traffic did we get last week?"* |
| `matomo_pages` | *"What are our top pages? Where do people exit?"* |
| `matomo_referrers` | *"Where do visitors come from? Which campaigns work? What do AI assistants send us?"* |
| `matomo_events` | *"How often was the configurator opened?"* |
| `matomo_goals` | *"What's our conversion rate per goal?"* |
| `matomo_ecommerce` | *"Revenue this month? Best-selling products?"* |
| `matomo_geo` | *"Which countries/cities do visitors come from?"* |
| `matomo_devices` | *"Mobile vs. desktop? Which browsers?"* |
| `matomo_visit_times` | *"When during the day/week do people visit?"* |
| `matomo_site_search` | *"What do people search for on our site — and find nothing?"* |
| `matomo_realtime` | *"Who's on the site right now?"* |
| `matomo_page_performance` | *"Which pages load slowly?"* |
| `matomo_api` | Everything else — funnels, heatmaps, custom dimensions, any `Module.action` of the Reporting API |

All tools accept `site_id`, `period` (`day`/`week`/`month`/`year`/`range`), `date`
(`today`, `yesterday`, `2026-07-01`, `last30`, or `start,end` ranges), an optional
`segment` (e.g. `deviceType==mobile;country==DE`), and a row `limit`.

### Prompts to try

- *"Compare this week's traffic with last week — what changed and why?"*
- *"Top 10 landing pages by conversions this month, with bounce rates."*
- *"Are we getting traffic from ChatGPT or Perplexity? Trend over 3 months."*
- *"Which internal searches return no results? Suggest content we should create."*
- *"Anything unusual in the visitor log right now?"*

## ⚙️ Configuration

| Flag | Env | Default | Description |
|------|-----|---------|-------------|
| `--url` | `MATOMO_URL` | — *(required)* | Matomo instance URL (sub-directory installs like `https://example.com/matomo/` work) |
| `--token` | `MATOMO_TOKEN` | — | API token (`token_auth`), view access is enough |
| `--default-site-id` | `MATOMO_DEFAULT_SITE_ID` | — | Site used when the model doesn't specify one |
| `--header` | `MATOMO_EXTRA_HEADERS` | — | Extra HTTP headers (`Name:Value`, repeatable / comma-separated) — for auth proxies, Zero-Trust, multi-tenant setups |
| `--timeout-secs` | `MATOMO_TIMEOUT_SECS` | `30` | Per-request timeout |
| `--max-response-chars` | `MATOMO_MAX_RESPONSE_CHARS` | `50000` | Response budget before truncation |
| `--insecure` | `MATOMO_INSECURE` | `false` | Accept self-signed TLS certificates (explicit opt-in) |
| `--check` | — | — | Verify URL + token + site access, then exit |

## 🆚 How is this different from `FGRibreau/mcp-matomo`?

[mcp-matomo](https://github.com/FGRibreau/mcp-matomo) (which inspired this project — thanks! 🙏) introspects your Matomo instance at startup and generates one MCP tool per API method. matomo-mcp takes the opposite approach:

| | matomo-mcp | mcp-matomo |
|---|---|---|
| Tool set | 14 curated tools + escape hatch | ~70+ generated tools |
| Model context cost | Small, stable | Large, instance-dependent |
| Parameter types | Exact, hand-written enums/defaults | Inferred from parameter names |
| Startup | Instant (no network I/O) | Introspection round-trips (or cached spec file) |
| TLS verification | On by default | Disabled for introspection |
| Sub-directory installs | ✅ | Path is overwritten |
| Response size guard | Row limits + hard budget | — |
| Retries on transient errors | ✅ | — |
| Real-time (Live) tools | ✅ | — (not part of report metadata) |

If you want *every* API method as its own tool, use mcp-matomo. If you want the model to reliably pick the right tool and never flood its context, use matomo-mcp.

## 🩺 Troubleshooting

<details>
<summary><b>"site_id is required"</b></summary>

Either pass `--default-site-id 1` (recommended) or let the model call `matomo_list_sites` first.

</details>

<details>
<summary><b>401 / "cannot be authenticated"</b></summary>

Run `matomo-mcp --url ... --token ... --check`. If it fails: regenerate the token (Settings → Personal → Security), make sure it has at least **view** access to the site.

</details>

<details>
<summary><b>404 or HTML instead of JSON</b></summary>

`MATOMO_URL` must point at the Matomo root — the folder containing `index.php`. For `https://example.com/matomo/index.php`, use `https://example.com/matomo/`.

</details>

<details>
<summary><b>Behind Cloudflare Access / OAuth2 proxy / Zero Trust?</b></summary>

Inject the bypass headers: `--header "CF-Access-Client-Id:..." --header "CF-Access-Client-Secret:..."` (or via `MATOMO_EXTRA_HEADERS`).

</details>

<details>
<summary><b>Responses feel truncated</b></summary>

That's the context guard doing its job. Ask for fewer rows, a shorter date range, or raise `--max-response-chars`.

</details>

## 🗺️ Roadmap

- [ ] Streamable HTTP transport (host it once, connect many clients)
- [ ] `matomo_annotations` — read & correlate deploy markers with traffic
- [ ] Multi-instance support (one server, several Matomo installations)
- [ ] Homebrew tap & winget manifest
- [ ] MCP registry listing

Want one of these sooner? [Open an issue](https://github.com/Liohtml/matomo-mcp/issues) — or a PR, see [CONTRIBUTING.md](CONTRIBUTING.md).

## 🛠️ Development

```bash
cargo test                                   # 33 tests, fully offline (wiremock)
cargo clippy --all-targets -- -D warnings
cargo run -- --url https://demo.matomo.cloud --default-site-id 1 --check
```

Architecture and design decisions: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## 📄 License & Credits

[MIT](LICENSE). Not affiliated with or endorsed by [Matomo](https://matomo.org) — Matomo is a registered trademark of InnoCraft Ltd.

Built with [rmcp](https://crates.io/crates/rmcp), the official Rust MCP SDK. Inspired by [FGRibreau/mcp-matomo](https://github.com/FGRibreau/mcp-matomo).

- MCP Registry name: `mcp-name: io.github.liohtml/matomo-mcp`

---

<div align="center">

**If matomo-mcp saves you a dashboard visit, a ⭐ helps others find it.**

</div>
