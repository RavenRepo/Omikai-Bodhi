# Bodhi - AI-Native Terminal Application

<p align="center">
  <img src="https://img.shields.io/badge/version-0.1.0-blue.svg" alt="version">
  <img src="https://img.shields.io/badge/rust-1.75+-informational.svg" alt="rust">
  <img src="https://img.shields.io/badge/license-MIT-green.svg" alt="license">
  <img src="https://img.shields.io/badge/status-active-success.svg" alt="status">
</p>

<p align="center">
  <a href="https://github.com/RavenRepo/Omikai-Bodhi">Repository</a> ·
  <a href="#installation">Install</a> ·
  <a href="#quick-start">Quick Start</a> ·
  <a href="#configuration">Configure</a> ·
  <a href="#contributing">Contribute</a>
</p>

---

## Overview

Bodhi is an AI-native terminal application built in Rust that brings the power of large language models to your command line. Built with a plugin architecture, it supports multiple LLM providers, MCP (Model Context Protocol) integration, multi-agent orchestration, and a rich tool system.

### Key Features

- **Multi-Provider LLM Support** - Connect to OpenAI, Anthropic, Ollama, or custom endpoints
- **Tool System** - Execute file operations, shell commands, grep, glob, and more
- **Slash Commands** - Intuitive commands like `/help`, `/clear`, `/status`, `/model`
- **Multi-Agent System** - Specialized agents for exploration, planning, and general tasks
- **MCP Integration** - Connect to Model Context Protocol servers for extended capabilities
- **Permission System** - Rule-based access control for tool execution
- **Terminal UI** - Built with ratatui for a rich terminal experience
- **Extensible Architecture** - Design inspired by Zed editor patterns

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
Welcome to Bodhi! Type /help for available commands.

> What is the purpose of Rust's ownership system?
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

- `default` - Ask for permission on potentially dangerous operations
- `accept_edits` - Accept file edit requests automatically
- `bypass_permissions` - Skip all permission checks (use with caution)
- `dont_ask` - Deny all permission prompts silently
- `plan` - Run in planning mode with limited capabilities
- `auto` - Automatically determine permissions
- `bubble` - Bubble permission decisions up to parent context

---

## Architecture

Bodhi follows a workspace-based architecture with specialized crates:

```
Omikai-Bodhi/
├── Cargo.toml                 # Workspace manifest
├── crates/
│   ├── cli/                   # CLI entry point
│   ├── core/                  # Core types and QueryEngine
│   ├── language_model/        # LLM trait abstraction
│   ├── omik_provider/         # Multi-provider LLM client
│   ├── http_client/           # HTTP abstraction trait
│   ├── reqwest_client/         # reqwest implementation
│   ├── fs/                    # Filesystem abstraction
│   ├── fs_real/               # Real filesystem implementation
│   ├── terminal/              # Terminal trait
│   ├── terminal_crossterm/    # crossterm implementation
│   ├── ui/                    # Ratatui UI
│   ├── tools/                 # Tool implementations
│   ├── commands/              # Slash commands
│   ├── agents/                # Multi-agent system
│   ├── mcp/                   # MCP client
│   ├── settings/              # Settings system
│   ├── bridge/                # Remote connections
│   └── permissions/           # Permission system
```

### Key Concepts

- **QueryEngine** - Processes user input and manages conversation flow
- **Tool** - Executable actions (bash, file read/write, grep, glob)
- **Command** - User-initiated actions via `/command` syntax
- **Agent** - Specialized AI assistants with specific capabilities
- **MCP** - Model Context Protocol for external tool integration

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

### Using Tools

Tools are automatically invoked based on LLM responses. The system handles tool execution and result processing transparently.

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

- `crates/cli/` - Command-line interface
- `crates/core/` - Core types and logic
- `crates/tools/` - Tool implementations
- `crates/commands/` - Slash command implementations
- `crates/agents/` - Agent implementations
- `crates/ui/` - Terminal UI components

---

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for details on how to contribute.

### Ways to Contribute

- Report bugs
- Suggest new features
- Improve documentation
- Submit pull requests
- Create tools and agents

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

<p align="center">Made with ❤️ by <a href="https://omikai.io">Omikai</a></p>
