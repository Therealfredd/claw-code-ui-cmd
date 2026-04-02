# Claw Code

Claw Code is a Claude Code-inspired, clean-room local coding agent CLI written in Rust. It provides an interactive REPL and one-shot prompt execution with workspace-aware tools, session persistence, and a built-in web UI.

---

## Features

- **Interactive REPL** — stateful multi-turn conversations with an AI coding agent
- **One-shot prompts** — run a single prompt and exit, suitable for scripting
- **Multi-provider support** — Anthropic (Claude), X.AI (Grok), and Ollama (local models)
- **Built-in tools** — shell execution, file read/write/edit, grep, web fetch/search, todos, notebooks
- **Slash commands** — session management, compaction, config inspection, diff, export, and more
- **Plugin system** — local plugin discovery and management
- **Session persistence** — save, resume, and inspect past sessions
- **Web UI** — browser-based chat interface via `claw serve`

---

## Prerequisites

- Rust stable toolchain and Cargo
- At least one of the following:
  - An `ANTHROPIC_API_KEY` for Claude models
  - An `XAI_API_KEY` for Grok models
  - A running [Ollama](https://ollama.com) instance for local models

---

## Installation

### Install locally with Cargo

```bash
cd rust
cargo install --path crates/claw-cli --locked
```

### Build from source

```bash
cd rust
cargo build --release -p claw-cli
# Binary will be at rust/target/release/claw
```

---

## Authentication

### Anthropic (Claude)

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
# Optional: override the API endpoint
export ANTHROPIC_BASE_URL="https://api.anthropic.com"
```

### X.AI (Grok)

```bash
export XAI_API_KEY="..."
# Optional: override the API endpoint
export XAI_BASE_URL="https://api.x.ai"
```

### Ollama (local models)

Start Ollama locally — no API key required. Claw will discover available models automatically via `http://localhost:11434`.

### OAuth login

```bash
claw login
```

---

## Usage

### Interactive REPL

```bash
claw
```

### One-shot prompt

```bash
claw prompt "summarize this workspace"
```

### Select a model

```bash
claw --model sonnet "review the latest changes"
claw --model grok-3 "explain crates/runtime"
claw --model ollama:llama3 "what does this function do?"
```

### Web UI

Start the local server and open the chat UI in your browser:

```bash
claw serve
# or on a custom port:
claw serve --port 8080
```

The web UI provides a chat window, session list, and model selector (including Ollama local models).

### Other commands

```bash
claw agents          # list available agents
claw skills          # list available skills
claw plugins         # list and manage plugins
claw --help          # full CLI reference
```

---

## Supported Models

| Model ID | Provider | Notes |
|---|---|---|
| `claude-opus-4-6` | Anthropic | Most capable |
| `claude-sonnet-4-6` | Anthropic | Balanced |
| `claude-haiku-4-5-20251213` | Anthropic | Fast |
| `grok-3` | X.AI | Flagship |
| `grok-3-mini` | X.AI | Compact |
| `grok-2` | X.AI | Legacy |
| `ollama:<model>` | Ollama | Any locally installed model |

The default model is `claude-opus-4-6`.

---

## Slash Commands

Inside the REPL, use `/` commands to control the session:

| Command | Description |
|---|---|
| `/compact` | Compact the current session to save tokens |
| `/diff` | Show workspace changes |
| `/export` | Export the current conversation |
| `/config` | Inspect the current configuration |
| `/agents` | List available agents |
| `/skills` | List available skills |
| `/plugins` | List plugins |
| `/reload-plugins` | Refresh plugin state |
| `/version` | Show version info |

---

## Configuration

### Workspace instructions

Create a `CLAW.md` file in your project root to give Claw persistent instructions and context about your workspace. It is loaded automatically at startup.

### Config files

| File | Purpose |
|---|---|
| `.claw.json` | Workspace-wide settings (model, permissions, tool defaults) |
| `.claw/settings.local.json` | Machine-local overrides (not committed) |
| `.claw/plugins/installed.json` | Plugin registry |
| `.claw/sessions/` | Persisted session snapshots |

---

## Built-in Tools

| Tool | Description |
|---|---|
| `shell` | Run bash commands |
| `read_file` | Read a file |
| `write_file` | Write a file |
| `edit_file` | Make targeted edits to a file |
| `glob` | Find files by pattern |
| `grep` | Search file contents |
| `web_fetch` | Fetch a URL |
| `web_search` | Search the web |
| `todo_write` | Manage a todo list |
| `notebook_edit` | Edit notebook cells |

---

## Repository Layout

```
.
├── rust/                   # Rust workspace (main implementation)
│   ├── crates/
│   │   ├── claw-cli/       # User-facing binary
│   │   ├── api/            # Provider clients and streaming
│   │   ├── runtime/        # Sessions, config, MCP, prompt loop
│   │   ├── tools/          # Built-in tool implementations
│   │   ├── commands/       # Slash-command registry
│   │   ├── plugins/        # Plugin system
│   │   ├── server/         # HTTP/SSE server for web UI
│   │   └── lsp/            # LSP client support
│   └── docs/releases/      # Release notes
├── web/                    # React/Vite/TypeScript web UI
│   └── src/components/     # Chat, session list, model selector
├── src/                    # Python porting workspace (reference layer)
└── tests/                  # Python verification tests
```

---

## License

MIT
