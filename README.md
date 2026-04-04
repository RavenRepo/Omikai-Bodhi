# Bodhi - AI-Native Terminal Application

```
   _____ _     _       _    _   _ _   _      _   _
  |  ___(_)___| |__   \ \  / / | | | | | |    | \ | |
  | |_  | / __| '_ \   \ \/ /  | | | | | |    |  \| |
  |  _| | \__ \ | | |   \  /   | | | | | |    | |\  |
  |_|   |_|___/_| |_|    |_|   |_|_| |_|___  |_| \_|
                                                    
  ____                 _                       _   
 / ___|  __ _ _ __     | |    _____  _____  __  | |  
| |  _ / _` | '__|    | |   / _ \ \/ / _ \ \ \ | |  
| |_| || (_| | |      | |__|  __/>  <  __/  | ||_| 
 \____| \__,_|_|      |_____\___/_/\_\___|  |_| (_)
                                                   
 _   _      _ _    __ _                       _    
| \ | | ___| | |  / _| | ____      __        | |   
|  \| |/ _ \ | | | |_| |/ _ \    / /        | |   
| |\  |  __/ | | |  _| | (_) |  / /         |_|   
|_| \_|\___|_|_| |_| |_|\___/  /_/          (_)  
```

<p align="center">
  <img src="https://img.shields.io/badge/version-0.1.0-blue.svg" alt="version">
  <img src="https://img.shields.io/badge/rust-1.75+-informational.svg" alt="rust">
  <img src="https://img.shields.io/badge/license-MIT-green.svg" alt="license">
  <img src="https://img.shields.io/badge/status-active-success.svg" alt="status">
  <a href="https://github.com/RavenRepo/Omikai-Bodhi/stargazers">
    <img src="https://img.shields.io/github/stars/RavenRepo/Omikai-Bodhi?style=social" alt="stars">
  </a>
</p>

<p align="center">
  <a href="https://github.com/RavenRepo/Omikai-Bodhi">Repository</a> ·
  <a href="#installation">Install</a> ·
  <a href="#quick-start">Quick Start</a> ·
  <a href="#configuration">Configure</a> ·
  <a href="#contributing">Contribute</a>
</p>

---

## What is Bodhi?

Bodhi is an AI-native terminal application built in **Rust** that brings the power of large language models to your command line. With a plugin-style architecture, Bodhi supports multiple LLM providers, MCP (Model Context Protocol) integration, multi-agent orchestration, and a rich tool system.

> 🤖 **Powered by AI** · ⚡ **Built in Rust** · 🔧 **Extensible**

---

## Key Features

| Feature | Description |
|---------|-------------|
| 🔮 **Multi-Provider LLM** | Connect to OpenAI, Anthropic, Ollama, or custom endpoints |
| 🛠️ **Tool System** | Execute bash, file ops, grep, glob, and more |
| ⌨️ **Slash Commands** | `/help`, `/clear`, `/status`, `/model`, and more |
| 🤖 **Multi-Agent** | Specialized agents for exploration, planning, general tasks |
| 🔗 **MCP Integration** | Connect to Model Context Protocol servers |
| 🔒 **Permission System** | Rule-based access control for tool execution |
| 🎨 **Terminal UI** | Rich TUI built with ratatui |
| 📦 **Extensible** | Architecture inspired by Zed editor patterns |

---

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Architecture](#architecture)
- [Commands](#commands)
- [Tools](#tools)
- [Development](#development)
- [License](#license)

---

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/RavenRepo/Omikai-Bodhi.git
cd Omikai-Bodhi

# Build the project
cargo build --release

# Install the binary
cargo install --path crates/cli
```

### Using Cargo

```bash
cargo install theasus-cli
```

### Pre-built Binaries

Download pre-built binaries from the [releases page](https://github.com/RavenRepo/Omikai-Bodhi/releases).

---

## Quick Start

### 1. Configure Your LLM Provider

```bash
# Configure OpenAI
bodhi config-llm --provider openai --api-key YOUR_OPENAI_KEY --model gpt-4o

# Configure Anthropic
bodhi config-llm --provider anthropic --api-key YOUR_ANTHROPIC_KEY --model claude-3-5-sonnet-20241022

# Configure Ollama (local)
bodhi config-llm --provider ollama --model llama2

# Configure Custom Endpoint
bodhi config-llm --provider custom --api-key YOUR_KEY --base-url https://your-api.com/v1 --model your-model
```

### 2. Start the Interactive Terminal

```bash
bodhi run
```

### 3. Using Bodhi

```
╔════════════════════════════════════════════════════════════╗
║  Welcome to Bodhi! AI-Native Terminal                   ║
║  Type /help for available commands                      ║
╚════════════════════════════════════════════════════════════╝

> What is Rust's ownership system?

┌─────────────────────────────────────────────────────────────┐
│                                                             │
│   Rust's ownership system is a memory safety feature that  │
│   eliminates the need for a garbage collector while        │
│   preventing memory safety bugs like null pointer         │
│   dereferences and data races.                              │
│                                                             │
│   Key concepts:                                             │
│   • Ownership - Each value has a single owner               │
│   • Borrowing - References can be borrowed                 │
│   • Lifetimes - Ensures references are valid               │
│                                                             │
└─────────────────────────────────────────────────────────────┘

[Tokens: 287] >
```

---

## Configuration

### Configuration File

Bodhi stores configuration at `~/.omikai/bodhi/config.json`:

```json
{
  "model": "gpt-4o",
  "api_key": "your-api-key",
  "llm_provider": "openai",
  "llm_base_url": null,
  "theme": "dark",
  "max_budget_usd": null,
  "permission_mode": "default",
  "mcp_servers": [],
  "custom_tools": [],
  "shortcuts": {}
}
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `OPENAI_API_KEY` | OpenAI API key |
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `OLLAMA_BASE_URL` | Ollama base URL (default: http://localhost:11434) |
| `BODHI_CONFIG_PATH` | Custom config file path |

### Permission Modes

| Mode | Description |
|------|-------------|
| `default` | Ask for permission on potentially dangerous operations |
| `accept_edits` | Accept file edit requests automatically |
| `bypass_permissions` | Skip all permission checks |
| `dont_ask` | Deny all permission prompts silently |
| `plan` | Run in planning mode with limited capabilities |
| `auto` | Automatically determine permissions |
| `bubble` | Bubble permission decisions up to parent context |

---

## Architecture

Bodhi follows a workspace-based architecture with specialized crates:

```
┌─────────────────────────────────────────────────────────────┐
│                      Bodhi Architecture                    │
└─────────────────────────────────────────────────────────────┘

                            ┌─────────────┐
                            │    CLI      │
                            │  (cli)      │
                            └──────┬──────┘
                                   │
         ┌─────────────────────────┼─────────────────────────┐
         │                         │                         │
    ┌────▼─────┐            ┌─────▼─────┐            ┌──────▼──────┐
    │  Core    │            │  Tools    │            │ Commands   │
    │ (engine) │            │           │            │            │
    └────┬─────┘            └─────┬─────┘            └──────┬──────┘
         │                        │                        │
    ┌────▼────────────────────────▼─────────────────────────▼────┐
    │                    language_model                         │
    │         (LLM abstraction + provider implementations)       │
    └────┬────────────────────────▲─────────────────────────┬────┘
         │                        │                            │
    ┌────▼─────┐            ┌─────▼─────┐              ┌──────▼──────┐
    │  OpenAI  │            │ Anthropic │              │  Ollama     │
    └──────────┘            └───────────┘              └─────────────┘
```

### Crate Overview

| Crate | Purpose |
|-------|---------|
| `cli` | CLI entry point and interactive terminal |
| `core` | Core types, QueryEngine, state management |
| `language_model` | LLM trait abstraction |
| `omik_provider` | Multi-provider LLM client (OpenAI, Anthropic, Ollama, Custom) |
| `http_client` | HTTP abstraction trait |
| `reqwest_client` | reqwest implementation |
| `fs` / `fs_real` | Filesystem abstraction |
| `terminal` / `terminal_crossterm` | Terminal trait and implementation |
| `ui` | Ratatui terminal UI |
| `tools` | Tool implementations (bash, file ops, grep, glob) |
| `commands` | Slash command implementations |
| `agents` | Multi-agent system (Explore, Plan, Task, CodeReview) |
| `mcp` | MCP client (JSON-RPC 2.0 protocol) |
| `plugins` | Dynamic plugin system |
| `workflows` | Workflow DSL engine |
| `settings` | Configuration system |
| `bridge` | Remote connections (WebSocket) |
| `permissions` | Permission system |

---

## Commands

### Built-in Commands

| Command | Aliases | Description |
|---------|---------|-------------|
| `/help` | `/h` | Show available commands |
| `/clear` | `/c` | Clear conversation history |
| `/exit` | `/q`, `/quit` | Exit the application |
| `/status` | `/s` | Show current session status |
| `/model` | `/m` | Change AI model |
| `/compact` | | Compact conversation history |
| `/tools` | `/t` | List available tools |
| `/agents` | `/a` | List available agents |

### Configuration Commands

```bash
# Configure LLM provider
bodhi config-llm --provider openai --api-key KEY --model gpt-4o

# View current configuration
bodhi config

# Check configuration
bodhi config --check
```

---

## Tools

### Available Tools

| Tool | Description |
|------|-------------|
| `bash` | Execute shell commands |
| `file_read` | Read files and directories |
| `file_write` | Create or overwrite files |
| `grep` | Search file contents |
| `glob` | Find files by pattern |

### Tools in Action

```
> Read all Rust files in src/

┌─────────────────────────────────────────────────────────────┐
│  TOOL: file_read                                           │
│  ─────────────────────────────────────────────────────────│
│  ✓ src/main.rs (245 lines)                                │
│  ✓ src/lib.rs (156 lines)                                 │
│  ✓ src/config.rs (89 lines)                                │
│                                                             │
│  Total: 3 files, 490 lines                                 │
└─────────────────────────────────────────────────────────────┘

> Search for "TODO" in the codebase

┌─────────────────────────────────────────────────────────────┐
│  TOOL: grep                                                │
│  ─────────────────────────────────────────────────────────│
│  ✓ src/main.rs:42: // TODO: Implement auth                │
│  ✓ src/core.rs:156: // TODO: Add caching                  │
│  ✓ src/tools.rs:89: // TODO: Error handling               │
│                                                             │
│  Total: 3 matches in 3 files                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Development

### Prerequisites

- Rust 1.75 or later
- Cargo

### Building

```bash
# Build debug version
cargo build

# Build release version
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

### Code Quality

```bash
# Format code
cargo fmt

# Run clippy
cargo clippy

# Run all checks
cargo check
```

### Project Structure

```
crates/
├── cli/                    # Entry point and UI
├── core/                   # Core types and QueryEngine
├── language_model/         # LLM trait abstraction
├── omik_provider/          # Multi-provider client
├── http_client/            # HTTP trait
├── reqwest_client/         # HTTP implementation
├── fs/                     # Filesystem trait
├── fs_real/                # Filesystem implementation
├── terminal/               # Terminal trait
├── terminal_crossterm/     # crossterm implementation
├── ui/                     # Ratatui UI
├── tools/                  # Tool implementations
├── commands/              # Slash commands
├── agents/                # Agent system
├── mcp/                    # MCP client
├── settings/              # Configuration
├── bridge/                # Remote connections
└── permissions/           # Permission system
```

---

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for details.

### Ways to Contribute

- 🐛 Report bugs
- 💡 Suggest features
- 📝 Improve documentation
- 🔧 Submit pull requests
- 🛠️ Create tools and agents

---

## License

MIT License - see [LICENSE](LICENSE) for details.

---

## Acknowledgments

- Inspired by [Zed](https://zed.dev/) - High-performance code editor in Rust
- Built with [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI library
- Architecture patterns from [Tokio](https://tokio.rs/) - Async runtime

---

## Support

- [GitHub Issues](https://github.com/RavenRepo/Omikai-Bodhi/issues)
- [Discussions](https://github.com/RavenRepo/Omikai-Bodhi/discussions)

---

<p align="center">
  Made with 🔥 by <a href="https://omikai.io">Omikai</a>
</p>
