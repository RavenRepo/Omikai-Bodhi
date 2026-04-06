# Bodhi User Guide

Bodhi is an AI-native terminal application built in Rust with multi-agent orchestration, MCP integration, and an extensible tool system.

## Table of Contents

1. [Installation](#installation)
2. [Quick Start](#quick-start)
3. [Configuration](#configuration)
4. [Using Tools](#using-tools)
5. [Slash Commands](#slash-commands)
6. [Agent System](#agent-system)
7. [Session Management](#session-management)
8. [MCP Integration](#mcp-integration)
9. [Plugins](#plugins)
10. [Workflows](#workflows)
11. [Troubleshooting](#troubleshooting)

---

## Installation

### From Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/RavenRepo/Omikai-Bodhi.git
cd Omikai-Bodhi

# Build and install
cargo install --path crates/cli

# Verify installation
bodhi --version
```

### From Cargo

```bash
cargo install theasus-cli
```

### From Homebrew (macOS/Linux)

```bash
brew tap RavenRepo/tap
brew install bodhi
```

### Shell Completions

Generate shell completions for your shell:

```bash
# Bash
bodhi completions bash > /etc/bash_completion.d/bodhi

# Zsh
bodhi completions zsh > ~/.zfunc/_bodhi

# Fish
bodhi completions fish > ~/.config/fish/completions/bodhi.fish

# PowerShell
bodhi completions powershell > bodhi.ps1
```

---

## Quick Start

### 1. Configure Your API Key

```bash
# Set your OpenAI API key
bodhi config-llm --provider openai --api-key YOUR_API_KEY --model gpt-4o

# Or use environment variable
export OPENAI_API_KEY=your-key
bodhi
```

### 2. Start a Conversation

```bash
bodhi
```

Once in the interactive mode, simply type your questions:

```
> What files are in this directory?
```

Bodhi will automatically use tools like `glob` and `file_read` to answer.

### 3. Use Tools

Ask Bodhi to perform actions:

```
> Create a file called hello.txt with "Hello, World!"
> Search for TODO comments in the codebase
> Run the tests
```

---

## Configuration

Configuration is stored at `~/.config/bodhi/config.json` (Linux/macOS) or `%APPDATA%\bodhi\config.json` (Windows).

### Configuration Options

```json
{
  "model": "gpt-4o",
  "api_key": "sk-...",
  "llm_provider": "openai",
  "llm_base_url": null,
  "theme": "dark",
  "max_budget_usd": 10.0,
  "permission_mode": "default",
  "mcp_servers": [],
  "custom_tools": [],
  "shortcuts": {}
}
```

### LLM Providers

Bodhi supports multiple LLM providers:

| Provider | Config Value | Notes |
|----------|--------------|-------|
| OpenAI | `openai` | GPT-4, GPT-4o, GPT-3.5 |
| Anthropic | `anthropic` | Claude 3 family |
| Ollama | `ollama` | Local models |
| Custom | `custom` | Any OpenAI-compatible API |

### Permission Modes

| Mode | Description |
|------|-------------|
| `default` | Ask before file writes and bash commands |
| `accept_edits` | Auto-approve file edits, ask for bash |
| `bypass_permissions` | Approve everything (use with caution) |
| `dont_ask` | Deny everything that would require permission |
| `plan` | Planning mode - no actual changes |

---

## Using Tools

Bodhi has access to various tools it can use to help you:

### File Operations

| Tool | Description | Example |
|------|-------------|---------|
| `file_read` | Read file contents | "Show me the contents of main.rs" |
| `file_write` | Create or overwrite files | "Create a new config.json" |
| `file_edit` | Edit specific parts of files | "Add a new function to utils.rs" |
| `glob` | Find files by pattern | "Find all Python files" |
| `grep` | Search file contents | "Find usages of `async fn`" |

### Execution

| Tool | Description | Example |
|------|-------------|---------|
| `bash` | Run shell commands | "Run the tests" |
| `task` | Run background tasks | "Build in the background" |

### Web

| Tool | Description | Example |
|------|-------------|---------|
| `web_fetch` | Fetch web pages | "Get the Rust docs for Vec" |

### MCP

| Tool | Description | Example |
|------|-------------|---------|
| `mcp` | Call MCP server tools | "Use the filesystem MCP to list files" |

---

## Slash Commands

Commands start with `/` and provide quick actions:

### General

| Command | Alias | Description |
|---------|-------|-------------|
| `/help` | `/h`, `/?` | Show help |
| `/clear` | `/c` | Clear conversation |
| `/exit` | `/e`, `/quit`, `/q` | Exit Bodhi |
| `/status` | | Show session status |
| `/model` | | Switch LLM model |

### Session Management

| Command | Description |
|---------|-------------|
| `/sessions` | List all saved sessions |
| `/session new [name]` | Create a new session |
| `/session resume <id>` | Resume a saved session |
| `/session delete <id>` | Delete a session |
| `/session rename <id> <name>` | Rename a session |
| `/session save` | Save current session |

### Git Integration

| Command | Alias | Description |
|---------|-------|-------------|
| `/commit` | | Create a commit |
| `/diff` | | Show git diff |
| `/review` | | Review changes |
| `/branch` | `/br` | Show/switch branches |

### Advanced

| Command | Alias | Description |
|---------|-------|-------------|
| `/mcp` | | Manage MCP servers |
| `/permissions` | `/perms` | View/modify permissions |
| `/export` | | Export conversation |
| `/memory` | `/mem`, `/context` | View context usage |
| `/config` | | View/edit configuration |

---

## Agent System

Bodhi includes a multi-agent system for complex tasks:

### Built-in Agents

| Agent | Description | Use Case |
|-------|-------------|----------|
| `GeneralPurpose` | Full-capability agent | Complex multi-step tasks |
| `Explore` | Code exploration | Understanding codebases |
| `Plan` | Planning agent | Breaking down tasks |
| `Task` | Task execution | Running builds/tests |
| `CodeReview` | Code review | Reviewing changes |

### Using Agents

Agents are invoked automatically based on context, or you can request them:

```
> Use the explore agent to understand how authentication works
> Use the code review agent to review my changes
```

---

## Session Management

Sessions persist your conversation history:

### Auto-Save

Sessions are automatically saved after each message exchange.

### Manual Management

```bash
# List sessions
/sessions

# Create named session
/session new my-project

# Resume a session
/session resume my-project

# Delete old sessions
/session delete <session-id>
```

### Session Storage

Sessions are stored in SQLite at:
- Linux/macOS: `~/.local/share/bodhi/sessions.db`
- Windows: `%APPDATA%\bodhi\sessions.db`

---

## MCP Integration

Bodhi can connect to MCP (Model Context Protocol) servers for extended capabilities.

### Configuring MCP Servers

Add servers to your config:

```json
{
  "mcp_servers": [
    {
      "name": "filesystem",
      "command": "npx",
      "args": ["-y", "@anthropic/mcp-server-filesystem"],
      "env": {},
      "timeout_ms": 30000,
      "transport": "stdio"
    }
  ]
}
```

### Using MCP Tools

Once configured, MCP tools appear alongside built-in tools:

```
> List available MCP tools
> Use the filesystem server to read /etc/hosts
```

---

## Plugins

Bodhi supports plugins for extended functionality.

### Plugin Structure

```
my-plugin/
├── manifest.json
├── src/
│   └── lib.rs
└── Cargo.toml
```

### manifest.json

```json
{
  "name": "my-plugin",
  "version": "1.0.0",
  "description": "My custom plugin",
  "entry_point": "libmy_plugin.so",
  "capabilities": ["tools", "commands"]
}
```

### Loading Plugins

Plugins are loaded from `~/.config/bodhi/plugins/`.

---

## Workflows

Define reusable workflows in YAML:

### Example Workflow

```yaml
name: code-review
description: Review code changes
steps:
  - name: get-diff
    tool: bash
    args:
      command: git diff HEAD~1
  
  - name: review
    agent: CodeReview
    input: "{{ steps.get-diff.output }}"
  
  - name: report
    tool: file_write
    args:
      path: review.md
      content: "{{ steps.review.output }}"
```

### Running Workflows

```
> Run the code-review workflow
```

---

## Troubleshooting

### Common Issues

#### "API key not configured"

Set your API key:
```bash
bodhi config-llm --provider openai --api-key YOUR_KEY
# or
export OPENAI_API_KEY=your-key
```

#### "Permission denied" errors

Check your permission mode:
```
/permissions
```

Temporarily allow operations:
```bash
bodhi --permission-mode accept_edits
```

#### "Tool execution failed"

1. Check the error message for details
2. Verify the tool has necessary permissions
3. Check if external dependencies are installed (e.g., `git` for git commands)

#### Session not loading

```bash
# Check session database
ls -la ~/.local/share/bodhi/sessions.db

# List sessions to verify
/sessions
```

### Debug Mode

Run with debug logging:

```bash
RUST_LOG=debug bodhi
```

### Getting Help

- GitHub Issues: https://github.com/RavenRepo/Omikai-Bodhi/issues
- Documentation: https://docs.rs/theasus-core

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+C` | Cancel current operation |
| `Ctrl+D` | Exit Bodhi |
| `Ctrl+L` | Clear screen |
| `Up/Down` | Navigate history |
| `Tab` | Autocomplete |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `OPENAI_API_KEY` | OpenAI API key |
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `BODHI_CONFIG` | Custom config path |
| `RUST_LOG` | Log level (error, warn, info, debug, trace) |

---

## License

MIT License - see [LICENSE](../LICENSE) for details.
