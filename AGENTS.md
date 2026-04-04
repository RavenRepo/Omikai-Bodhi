# AGENTS Instructions

Bodhi is an AI-native terminal application in Rust with a TUI interface, multi-agent orchestration, MCP integration, and an extensible plugin/tool system. Migrated from TypeScript.

## Setup

```bash
# Install Rust 1.75+
rustup update stable

# Clone and build
git clone https://github.com/RavenRepo/Omikai-Bodhi.git
cd Omikai-Bodhi
cargo build
```

## Commands

### Build

```bash
cargo build                    # debug build
cargo build --release          # release build
cargo install --path crates/cli  # install binary locally
```

### Test

```bash
cargo test                     # all workspace tests
cargo test -p theasus-core     # specific crate
cargo test -p theasus-tools    # tools crate
cargo test -p theasus-agents   # agents crate
```

### Lint/Format

```bash
cargo fmt                      # format all crates
cargo fmt -- --check           # check formatting (CI)
cargo clippy --all-targets -- -D warnings
```

### Run

```bash
cargo run                      # run CLI (debug)
RUST_LOG=debug cargo run       # run with tracing output
bodhi run                      # run installed binary
bodhi config-llm --provider openai --api-key KEY --model gpt-4o
```

## Structure

```
crates/
├── core/                  # QueryEngine, types, state, config — the brain
├── language_model/        # LLM trait abstraction (trait LanguageModel)
├── omik_provider/         # Multi-provider client (OpenAI, Anthropic, Ollama, Custom)
├── http_client/           # HTTP abstraction trait (trait HttpClient)
├── reqwest_client/        # reqwest implementation of HttpClient
├── fs/                    # Filesystem abstraction trait (trait Fs)
├── fs_real/               # Real filesystem implementation
├── terminal/              # Terminal abstraction trait (trait Terminal)
├── terminal_crossterm/    # crossterm implementation of Terminal
├── ui/                    # Ratatui terminal UI components
├── cli/                   # CLI entry point (binary: bodhi)
├── tools/                 # Tool implementations (bash, file_read, file_write, grep, glob)
├── commands/              # Slash command system (/help, /clear, /model, etc.)
├── agents/                # Multi-agent orchestration system
├── mcp/                   # MCP (Model Context Protocol) client
├── settings/              # Configuration system + settings traits
├── settings_macros/       # Settings proc macros (planned)
├── bridge/                # Remote connection / WebSocket transport
├── permissions/           # Rule-based permission system
├── plugins/               # Plugin system (trait Plugin)
├── workflows/             # Workflow DSL and execution engine
├── extension_api/         # Extension API (planned)

examples/                  # Usage examples
tests/                     # Integration tests
```

## Architecture Patterns

Bodhi follows Zed-inspired trait abstraction patterns:

```
CLI (cli) → Commands + UI
     ↓
   Core (QueryEngine, State)
     ↓
Language Model trait ← omik_provider (OpenAI, Anthropic, Ollama)
     ↓
HttpClient trait ← reqwest_client
Fs trait ← fs_real
Terminal trait ← terminal_crossterm
```

**Every external dependency is behind a trait.** This enables:

- Easy mocking in tests
- Swappable implementations
- Future WASM support

## Development Loop

```bash
# 1. Make changes
# 2. cargo fmt
# 3. cargo build
```

### Run these only if building/testing your changes:

```bash
# 1. cargo build
# 2. cargo test -p <crate>
# 3. cargo clippy --all-targets -- -D warnings
```

## Rules

### Traits & Abstraction

Trait: All external I/O must be behind a trait (HTTP, filesystem, terminal, LLM)
Trait: Trait crate = interface only, no implementation. Implementation crate = separate (e.g., `fs` + `fs_real`)
Trait: Use `async_trait` for async trait methods
Trait: Traits must be `Send + Sync`

### Error Handling

Error: Use `anyhow::Result` for application-level errors
Error: Use `thiserror` for library/crate-specific error enums
Error: Implement `Display` and `Error` for all custom error types

### Testing

Test: Prefer `tests/` directory for integration tests, e.g., `crates/core/tests/`
Test: Use `#[cfg(test)] mod tests` for unit tests in the same file
Test: Mock traits for isolation — never hit real APIs in tests
Test: All new tools and commands must have corresponding tests

### Crate Design

Crate: Each crate has a single `lib.rs` (libraries) or `main.rs` (binaries)
Crate: Only `cli` produces a binary, everything else is a library crate
Crate: Use workspace dependencies from root `Cargo.toml` — never pin versions in sub-crates

### Providers & LLM

Provider: Implement `LanguageModel` trait — see `crates/language_model/`
Provider: New providers go in `crates/omik_provider/`
Provider: Streaming responses use `impl Stream<Item = Result<StreamEvent>>`

### Tools

Tool: Implement `Tool` trait — see `crates/tools/`
Tool: Every tool must implement `check_permission` alongside `execute`
Tool: Register new tools in the `ToolRegistry`

### Commands

Command: Implement `Command` trait — see `crates/commands/`
Command: Commands support aliases (e.g., `/h` → `/help`, `/q` → `/quit`)

### MCP

MCP: Client implementation in `crates/mcp/`
MCP: Supports stdio and SSE transports
MCP: Tool proxying bridges MCP tools into Bodhi's tool registry

### Permissions

Permission: Check permissions before tool execution, never after
Permission: Permission modes: Default, AcceptEdits, BypassPermissions, DontAsk, Plan, Auto, Bubble

### Config

Config: Application config lives at `~/.omikai/bodhi/config.json`
Config: Use `directories` crate for platform-appropriate paths
Config: Settings are strongly typed with serde — no raw string access

## Code Quality

Comments: Write self-documenting code — prefer clear names over comments
Comments: Never add comments that restate what code does
Comments: Only comment for complex algorithms, non-obvious business logic, or "why" not "what"
Simplicity: Don't make things `Option` that don't need to be — the compiler will enforce
Simplicity: Booleans should default to false, not be `Option<bool>`
Errors: Don't add error context that doesn't add useful information
Simplicity: Avoid overly defensive code — trust Rust's type system
Logging: Use `tracing` macros (`tracing::info!`, `tracing::error!`) — not `println!`
Logging: Only log errors, security events, or significant state transitions

## Git

### Commit Message Format

```
<type>(<scope>): <description>

[optional body]

[optional footer: Closes #123]
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

### Branch Naming

```
feature/*     # New features
fix/*         # Bug fixes
refactor/*    # Refactoring
docs/*        # Documentation
test/*        # Tests
chore/*       # Maintenance
```

## Never

Never: Edit `Cargo.lock` manually — it's auto-generated
Never: Pin dependency versions in sub-crate `Cargo.toml` — use `[workspace.dependencies]`
Never: Add deps with `cargo add` from a sub-crate without adding to workspace first
Never: Skip `cargo fmt`
Never: Merge without running clippy
Never: Commit directly to `main`
Never: Put implementation logic in trait crates (`fs`, `http_client`, `terminal`)
Never: Use `println!` for logging — use `tracing`
Never: Comment self-evident operations (`// Initialize`, `// Return result`)
Never: Comment getters/setters, constructors, or standard Rust idioms
Never: Use `unwrap()` in library code — propagate errors with `?`

## Entry Points

- **CLI**: `crates/cli/src/main.rs`
- **Core Engine**: `crates/core/src/lib.rs`
- **Agents**: `crates/agents/src/lib.rs`
- **Tools**: `crates/tools/src/lib.rs`
- **Commands**: `crates/commands/src/lib.rs`
- **LLM Trait**: `crates/language_model/src/lib.rs`
- **Provider**: `crates/omik_provider/src/lib.rs`
- **MCP Client**: `crates/mcp/src/lib.rs`
- **Permissions**: `crates/permissions/src/lib.rs`
- **UI**: `crates/ui/src/lib.rs`
- **Settings**: `crates/settings/src/lib.rs`
- **Plugins**: `crates/plugins/src/lib.rs`
- **Workflows**: `crates/workflows/src/lib.rs`

## Key Dependencies

| Purpose       | Crate                            | Notes                               |
| ------------- | -------------------------------- | ----------------------------------- |
| Async Runtime | `tokio` (full)                   | All async code runs on tokio        |
| Serialization | `serde` + `serde_json`           | Derive-based, workspace-wide        |
| HTTP          | `reqwest`                        | Behind `HttpClient` trait           |
| CLI           | `clap` (derive)                  | CLI argument parsing                |
| TUI           | `ratatui` + `crossterm`          | Behind `Terminal` trait             |
| Errors        | `anyhow` + `thiserror`           | anyhow for apps, thiserror for libs |
| Tracing       | `tracing` + `tracing-subscriber` | Structured logging                  |
| Async Traits  | `async-trait`                    | For async trait methods             |
| WebSocket     | `tokio-tungstenite`              | Bridge transport                    |
| IDs           | `uuid` v4                        | Session and entity IDs              |
| Time          | `chrono`                         | Timestamps with serde support       |
