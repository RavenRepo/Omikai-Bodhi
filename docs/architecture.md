# Bodhi Architecture

This document describes the architecture of Bodhi, an AI-native terminal application built in Rust.

## Overview

Bodhi follows a modular, trait-based architecture inspired by the Zed editor. Every external dependency is behind a trait, enabling easy testing, swappable implementations, and future WASM support.

```
┌─────────────────────────────────────────────────────────────────┐
│                          CLI (bodhi)                             │
│                    Entry point & user interface                  │
└─────────────────────────┬───────────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────────┐
│                         Core Engine                              │
│              QueryEngine, State, Types, Config                   │
└─────────┬───────────────┬───────────────┬───────────────────────┘
          │               │               │
┌─────────▼─────┐ ┌───────▼───────┐ ┌─────▼─────────────┐
│   Commands    │ │    Tools      │ │     Agents        │
│ Slash command │ │ File, Bash,   │ │ Multi-agent       │
│   system      │ │ Web, MCP      │ │ orchestration     │
└───────────────┘ └───────────────┘ └───────────────────┘
          │               │               │
┌─────────▼───────────────▼───────────────▼───────────────────────┐
│                      Abstraction Layer                           │
│         Traits: LanguageModel, Fs, Terminal, HttpClient          │
└─────────┬───────────────┬───────────────┬───────────────────────┘
          │               │               │
┌─────────▼─────┐ ┌───────▼───────┐ ┌─────▼─────────────┐
│  Providers    │ │ Filesystem    │ │   HTTP Client     │
│ OpenAI,       │ │ fs_real       │ │ reqwest_client    │
│ Anthropic,    │ │               │ │                   │
│ Ollama        │ │               │ │                   │
└───────────────┘ └───────────────┘ └───────────────────┘
```

## Crate Structure

### Core Crates

| Crate | Description | Key Types |
|-------|-------------|-----------|
| `theasus-core` | Core types and query engine | `QueryEngine`, `AppState`, `Message`, `Config` |
| `theasus-language-model` | LLM trait abstraction | `trait LanguageModel` |
| `theasus-omik-provider` | Multi-provider LLM client | `OpenAiProvider`, `AnthropicProvider`, `OllamaProvider` |

### Abstraction Crates (Traits)

| Crate | Description | Trait |
|-------|-------------|-------|
| `theasus-fs` | Filesystem abstraction | `trait Fs` |
| `theasus-http-client` | HTTP abstraction | `trait HttpClient` |
| `theasus-terminal` | Terminal abstraction | `trait Terminal` |

### Implementation Crates

| Crate | Description | Implements |
|-------|-------------|-----------|
| `theasus-fs-real` | Real filesystem | `Fs` |
| `theasus-reqwest-client` | reqwest HTTP | `HttpClient` |
| `theasus-terminal-crossterm` | crossterm terminal | `Terminal` |

### Feature Crates

| Crate | Description |
|-------|-------------|
| `theasus-tools` | Tool implementations (bash, file_read, file_write, grep, glob, web_fetch) |
| `theasus-commands` | Slash command system |
| `theasus-agents` | Multi-agent orchestration |
| `theasus-mcp` | MCP client |
| `theasus-session` | SQLite session persistence |
| `theasus-bridge` | Remote connection system |
| `theasus-permissions` | Permission system |
| `theasus-plugins` | Plugin system |
| `theasus-workflows` | Workflow DSL |
| `theasus-knowledge` | Knowledge system |
| `theasus-settings` | Configuration |
| `theasus-ui` | Terminal UI (ratatui) |

## Data Flow

### Query Flow

```
User Input
    │
    ▼
┌───────────────┐
│  QueryEngine  │◄──────────────────┐
└───────┬───────┘                   │
        │                           │
        ▼                           │
┌───────────────┐                   │
│ LanguageModel │ (LLM API call)    │
└───────┬───────┘                   │
        │                           │
        ▼                           │
┌───────────────┐                   │
│ Tool Calls?   │───Yes────┐        │
└───────┬───────┘          │        │
        │No                ▼        │
        │          ┌───────────────┐│
        │          │ Tool Registry ││
        │          └───────┬───────┘│
        │                  │        │
        │                  ▼        │
        │          ┌───────────────┐│
        │          │ Execute Tool  ││
        │          └───────┬───────┘│
        │                  │        │
        │                  ▼        │
        │          ┌───────────────┐│
        │          │ Tool Result   │┘
        │          └───────────────┘
        │
        ▼
┌───────────────┐
│   Response    │
└───────────────┘
```

### Permission Flow

```
Tool Execution Request
    │
    ▼
┌───────────────────┐
│ Permission Check  │
└───────┬───────────┘
        │
        ▼
┌───────────────────┐
│ Match Rules       │
│ (Path, Command)   │
└───────┬───────────┘
        │
   ┌────┴────┐
   │         │
   ▼         ▼
┌──────┐  ┌──────┐
│Allow │  │ Deny │
└──┬───┘  └──┬───┘
   │         │
   ▼         │
Execute      │
   │         │
   ▼         ▼
Result    Error
```

## Key Abstractions

### LanguageModel Trait

```rust
#[async_trait]
pub trait LanguageModel: Send + Sync {
    async fn generate(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<Response>;
    
    async fn stream(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<impl Stream<Item = Result<StreamEvent>>>;
}
```

### Tool Trait

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> JsonSchema;
    
    async fn execute(&self, input: Value) -> Result<ToolResult>;
    
    fn check_permission(&self, input: &Value) -> PermissionCheck {
        PermissionCheck::Allowed
    }
}
```

### Command Trait

```rust
#[async_trait]
pub trait Command: Send + Sync {
    fn name(&self) -> &str;
    fn aliases(&self) -> &[&str] { &[] }
    fn description(&self) -> &str;
    
    async fn execute(
        &self,
        args: &str,
        context: &CommandContext,
    ) -> Result<CommandResult>;
}
```

## Agent System

### Agent Types

| Agent | Purpose | Tools |
|-------|---------|-------|
| `GeneralPurpose` | Complex tasks | All tools |
| `Explore` | Code exploration | grep, glob, file_read |
| `Plan` | Task planning | None (planning only) |
| `Task` | Background execution | bash, file ops |
| `CodeReview` | Code review | grep, glob, file_read, diff |

### Orchestration

```
┌─────────────────────────────────────────┐
│          AgentOrchestrator              │
├─────────────────────────────────────────┤
│  ┌─────────┐  ┌─────────┐  ┌─────────┐ │
│  │ Agent 1 │  │ Agent 2 │  │ Agent 3 │ │
│  └────┬────┘  └────┬────┘  └────┬────┘ │
│       │            │            │       │
│       ▼            ▼            ▼       │
│  ┌─────────────────────────────────┐   │
│  │      Dependency Tracker         │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

## Knowledge System

The knowledge system enables learning and retention:

```
┌─────────────────────────────────────────┐
│           Knowledge Layer               │
├─────────────────────────────────────────┤
│  PreExecutionQuery                      │
│       │                                 │
│       ▼                                 │
│  ExecutionContract                      │
│       │                                 │
│       ▼                                 │
│  Agent Execution                        │
│       │                                 │
│       ▼                                 │
│  PostExecutionCapture                   │
│       │                                 │
│       ▼                                 │
│  PromotionEvaluation                    │
│       │                                 │
│       ▼                                 │
│  KnowledgeStore                         │
└─────────────────────────────────────────┘
```

### Promotion Scoring (4D)

| Dimension | Weight | Description |
|-----------|--------|-------------|
| Correctness | 0.25 | Was the outcome correct? |
| Generalizability | 0.40 | Does it apply broadly? |
| Completeness | 0.20 | Is the knowledge complete? |
| Independence | 0.15 | Verified across contexts? |

Promotion requires:
- `weighted_score > 0.72`
- `independence > 0.5`

## Session Persistence

```
┌─────────────────────────────────────────┐
│           SessionStore                  │
├─────────────────────────────────────────┤
│  SQLite Database                        │
│  ┌─────────────────────────────────┐   │
│  │ sessions                         │   │
│  │   id, name, model, timestamps   │   │
│  └─────────────────────────────────┘   │
│  ┌─────────────────────────────────┐   │
│  │ messages                         │   │
│  │   id, session_id, role, content │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

## MCP Integration

```
┌─────────────────┐      ┌─────────────────┐
│   Bodhi Core    │      │   MCP Server    │
│                 │      │                 │
│  ┌───────────┐  │      │  ┌───────────┐  │
│  │McpClient  │◄─┼──────┼──┤ JSON-RPC  │  │
│  └───────────┘  │ stdio│  └───────────┘  │
│                 │  or  │                 │
│  ┌───────────┐  │ SSE  │  ┌───────────┐  │
│  │McpManager │  │      │  │   Tools   │  │
│  └───────────┘  │      │  └───────────┘  │
└─────────────────┘      └─────────────────┘
```

## Security Model

### Permission Rules

1. **Path-based**: Allow/deny based on file paths
2. **Command-based**: Allow/deny based on shell commands
3. **Domain-based**: Web fetch URL restrictions

### Permission Modes

| Mode | File Write | Bash | Web Fetch |
|------|------------|------|-----------|
| Default | Ask | Ask | Ask |
| AcceptEdits | Allow | Ask | Ask |
| BypassPermissions | Allow | Allow | Allow |
| DontAsk | Deny | Deny | Deny |

### Audit Trail

All tool executions are logged with:
- Timestamp
- Tool name
- Input parameters
- Permission decision
- Result status

## Extension Points

### 1. Custom Tools

Implement the `Tool` trait and register with `ToolRegistry`.

### 2. Custom Commands

Implement the `Command` trait and register with `CommandRegistry`.

### 3. Custom Agents

Implement agent configuration and register with `AgentOrchestrator`.

### 4. Plugins

Create a plugin with `manifest.json` and implement hooks.

### 5. MCP Servers

Add MCP server configurations to settings.

## Performance Considerations

### Lazy Loading

Tools are loaded on-demand to minimize startup time.

### Parallel Execution

Independent tool calls can execute in parallel.

### Caching

Read-only tool results are cached with TTL:
- `glob`: 30s
- `grep`: 30s
- `file_read`: 10s

### Memory Management

- Conversation history is bounded
- Old sessions are cleaned up automatically
- Knowledge entries have confidence decay

## Testing Strategy

### Unit Tests

Each crate has unit tests in `#[cfg(test)] mod tests`.

### Integration Tests

Located in `crates/<crate>/tests/` directory.

### Mock Implementations

Traits enable mock implementations for testing:
- `MockFs` for filesystem
- `MockHttpClient` for HTTP
- `MockLanguageModel` for LLM

## Dependencies

### Core

| Crate | Purpose |
|-------|---------|
| tokio | Async runtime |
| serde | Serialization |
| anyhow/thiserror | Error handling |
| tracing | Logging |

### HTTP

| Crate | Purpose |
|-------|---------|
| reqwest | HTTP client |

### Terminal

| Crate | Purpose |
|-------|---------|
| ratatui | TUI framework |
| crossterm | Terminal control |
| clap | CLI parsing |

### Database

| Crate | Purpose |
|-------|---------|
| rusqlite | SQLite |

### WebSocket

| Crate | Purpose |
|-------|---------|
| tokio-tungstenite | WebSocket |

---

## Further Reading

- [User Guide](user-guide.md)
- [API Documentation](https://docs.rs/theasus-core)
- [Contributing Guide](../CONTRIBUTING.md)
