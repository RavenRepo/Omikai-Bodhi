# Theasus → Rust Migration Plan

## Project Overview

**Theasus** is an AI-native terminal application. It features multi-agent orchestration, rich tool use, MCP (Model Context Protocol) support, and an extensible architecture.

### Current State (TypeScript)
- **1,884 source files** (.ts/.tsx) across **301 directories**
- Built with TypeScript, React (Ink for terminal UI), Bun runtime
- Complex feature set including:
  - AI conversation engine with context management
  - 38+ tools for file ops, search, execution, web, MCP, tasks
  - 75+ slash commands
  - Multi-agent orchestration system
  - Permission system with rules and classifiers
  - Bridge system for remote connections
  - MCP client for external tool integration

---

## Migration Goals

1. **Complete Feature Parity** - All existing features working in Rust
2. **Performance Improvement** - Leverage Rust's speed and memory safety
3. **Better Binary Distribution** - Single native binary, no Node.js dependency
4. **Enhanced Reliability** - Rust's type system catches more bugs at compile time
5. **Advanced Features** - Add new capabilities during migration

---

## Phase 0: Foundation & Setup (Week 1-2)

### 0.1 Project Setup
- [x] Initialize Rust workspace with Cargo
- [ ] Set up CI/CD pipeline (GitHub Actions)
- [ ] Configure linting (clippy), formatting (rustfmt), testing
- [ ] Create directory structure mirroring the architecture

```
theasus/
├── Cargo.toml                 # Workspace manifest
├── crates/
│   ├── cli/                   # CLI entry point & UI
│   ├── core/                  # QueryEngine, types, state
│   ├── tools/                 # Tool implementations
│   ├── commands/              # Slash commands
│   ├── bridge/                # Remote connection system
│   ├── mcp/                   # MCP client
│   ├── permissions/           # Permission system
│   ├── agents/                # Multi-agent system
│   └── utils/                 # Shared utilities
├── tests/                     # Integration tests
└── examples/                  # Usage examples
```

### 0.2 Select Core Dependencies
| Purpose | Rust Crate | Replaces |
|---------|------------|----------|
| Async Runtime | `tokio` | Node.js event loop |
| CLI Framework | `clap` | Commander.js |
| Terminal UI | `ratatui` + `crossterm` | Ink (React) |
| HTTP Client | `reqwest` | Axios |
| JSON | `serde` + `serde_json` | Native JS |
| WebSocket | `tokio-tungstenite` | ws |
| Regex | `regex` | Node regex |
| Glob | `glob` + `globset` | fast-glob |
| Process | `tokio::process` | child_process |
| Config | `config` + `directories` | Node config |
| Logging | `tracing` | Custom logging |
| Error Handling | `thiserror` + `anyhow` | Custom errors |

### 0.3 Documentation
- [ ] Document existing TypeScript architecture
- [ ] Create API contract specifications
- [ ] Write Rust style guide for the project

---

## Phase 1: Core Types & Data Structures (Week 3-4)

### 1.1 Define Core Types
```rust
// crates/core/src/types/mod.rs

// Session and Agent IDs (branded types)
pub struct SessionId(String);
pub struct AgentId(String);

// Task system
pub enum TaskType {
    LocalBash,
    LocalAgent,
    RemoteAgent,
    InProcessTeammate,
    LocalWorkflow,
    MonitorMcp,
    Dream,
}

pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Killed,
}

// Permission types
pub enum PermissionMode {
    Default,
    AcceptEdits,
    BypassPermissions,
    DontAsk,
    Plan,
    Auto,
    Bubble,
}

pub enum PermissionBehavior {
    Allow,
    Deny,
    Ask,
}
```

### 1.2 Message Types
```rust
// Message types for conversation
pub enum Message {
    User(UserMessage),
    Assistant(AssistantMessage),
    System(SystemMessage),
    Progress(ProgressMessage),
    Attachment(AttachmentMessage),
}

pub struct UserMessage {
    pub id: Uuid,
    pub content: Vec<ContentBlock>,
    pub timestamp: DateTime<Utc>,
}
```

### 1.3 Tool Types
```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> JsonSchema;
    
    async fn execute(
        &self,
        input: Value,
        context: &ToolContext,
    ) -> Result<ToolResult, ToolError>;
    
    async fn check_permission(
        &self,
        input: &Value,
        context: &PermissionContext,
    ) -> PermissionResult;
}
```

### Deliverables
- [x] All type definitions in `crates/core/src/types/`
- [x] Serde serialization/deserialization
- [ ] Unit tests for type conversions
- [ ] JSON schema generation for tool inputs

---

## Phase 2: Configuration & State Management (Week 5-6)

### 2.1 Configuration System
```rust
// crates/core/src/config/mod.rs

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub model: String,
    pub theme: Theme,
    pub max_budget_usd: Option<f64>,
    pub permission_mode: PermissionMode,
    pub mcp_servers: Vec<McpServerConfig>,
    // ... more fields
}

impl Config {
    pub fn load() -> Result<Self>;
    pub fn save(&self) -> Result<()>;
    pub fn get_path() -> PathBuf;
}
```

### 2.2 Application State
```rust
// crates/core/src/state/mod.rs

pub struct AppState {
    pub messages: Vec<Message>,
    pub tasks: HashMap<String, TaskState>,
    pub tool_permission_context: ToolPermissionContext,
    pub session_id: SessionId,
    pub cwd: PathBuf,
    // ... more fields
}

// Thread-safe state wrapper
pub type SharedState = Arc<RwLock<AppState>>;
```

### 2.3 Session Persistence
- [ ] Implement session storage (SQLite or JSON files)
- [ ] Session resume functionality
- [ ] Conversation history management

### Deliverables
- [ ] Config loading from `~/.theasus/config.json`
- [ ] Settings file watching for hot reload
- [ ] State management with proper synchronization
- [ ] Session persistence to disk

---

## Phase 3: Claude API Client (Week 7-8)

### 3.1 API Client
```rust
// crates/core/src/api/client.rs

pub struct ClaudeClient {
    http_client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl ClaudeClient {
    pub async fn create_message(
        &self,
        request: MessageRequest,
    ) -> Result<MessageResponse, ApiError>;
    
    pub async fn stream_message(
        &self,
        request: MessageRequest,
    ) -> impl Stream<Item = Result<StreamEvent, ApiError>>;
}
```

### 3.2 Query Engine
```rust
// crates/core/src/engine/query_engine.rs

pub struct QueryEngine {
    client: ClaudeClient,
    tools: ToolRegistry,
    state: SharedState,
    config: QueryEngineConfig,
}

impl QueryEngine {
    pub async fn query(&mut self, input: &str) -> Result<Response>;
    pub async fn process_tool_use(&mut self, tool_use: ToolUse) -> Result<ToolResult>;
    pub fn compact_conversation(&mut self) -> Result<()>;
}
```

### 3.3 Features to Implement
- [ ] Multi-turn conversation handling
- [ ] Token/budget management
- [ ] Streaming responses
- [ ] Tool use processing loop
- [ ] Context compaction
- [ ] Thinking mode support

### Deliverables
- [x] Working API client with streaming
- [ ] QueryEngine with full conversation loop
- [ ] Budget tracking and enforcement
- [ ] Auto-compaction logic

---

## Phase 4: Tool System (Week 9-12)

### 4.1 Tool Framework
```rust
// crates/tools/src/registry.rs

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    deferred_tools: HashMap<String, DeferredToolLoader>,
}

impl ToolRegistry {
    pub fn register<T: Tool + 'static>(&mut self, tool: T);
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>>;
    pub fn list(&self) -> Vec<&str>;
}
```

### 4.2 Tool Implementations (Priority Order)

#### Tier 1: Essential (Week 9-10)
| Tool | Description | Complexity |
|------|-------------|------------|
| FileReadTool | Read files, PDFs, images | Medium |
| FileWriteTool | Create/overwrite files | Medium |
| FileEditTool | Surgical file edits | High |
| BashTool | Shell command execution | High |
| GrepTool | Content search (ripgrep) | Medium |
| GlobTool | File pattern matching | Low |

#### Tier 2: Important (Week 11)
| Tool | Description | Complexity |
|------|-------------|------------|
| AskUserQuestionTool | User prompts | Low |
| ConfigTool | Settings management | Medium |
| LSPTool | Language server ops | High |
| WebFetchTool | HTTP requests | Medium |

#### Tier 3: Advanced (Week 12)
| Tool | Description | Complexity |
|------|-------------|------------|
| AgentTool | Sub-agent spawning | Very High |
| MCPTool | MCP server tools | High |
| TaskTools | Task management | Medium |
| SendMessageTool | Agent messaging | Medium |

### 4.3 BashTool Deep Dive
```rust
// crates/tools/src/bash/mod.rs

pub struct BashTool {
    command_parser: CommandParser,
    sandbox: Option<SandboxConfig>,
}

impl BashTool {
    async fn execute_command(
        &self,
        command: &str,
        timeout: Option<Duration>,
        background: bool,
    ) -> Result<BashResult>;
    
    fn parse_command(&self, command: &str) -> ParsedCommand;
    fn check_safety(&self, parsed: &ParsedCommand) -> SafetyResult;
}
```

### Deliverables
- [ ] Tool trait and registry
- [ ] All Tier 1 tools working
- [ ] All Tier 2 tools working
- [ ] All Tier 3 tools working
- [ ] Tool permission checks
- [ ] Integration tests for each tool

---

## Phase 5: Permission System (Week 13-14)

### 5.1 Permission Framework
```rust
// crates/permissions/src/mod.rs

pub struct PermissionManager {
    rules: PermissionRules,
    mode: PermissionMode,
    classifiers: Vec<Box<dyn Classifier>>,
}

impl PermissionManager {
    pub async fn check_permission(
        &self,
        tool: &str,
        input: &Value,
        context: &PermissionContext,
    ) -> PermissionResult;
    
    pub fn add_rule(&mut self, rule: PermissionRule);
    pub fn remove_rule(&mut self, rule: &PermissionRule);
}
```

### 5.2 Permission Rules
```rust
pub struct PermissionRules {
    pub always_allow: HashMap<PermissionRuleSource, Vec<String>>,
    pub always_deny: HashMap<PermissionRuleSource, Vec<String>>,
    pub always_ask: HashMap<PermissionRuleSource, Vec<String>>,
}

pub enum PermissionRuleSource {
    UserSettings,
    ProjectSettings,
    LocalSettings,
    FlagSettings,
    PolicySettings,
    CliArg,
    Command,
    Session,
}
```

### 5.3 Features
- [ ] Rule-based permission checking
- [ ] Command pattern matching for bash
- [ ] Path-based file permissions
- [ ] User prompt for "ask" decisions
- [ ] Permission persistence

---

## Phase 6: Slash Commands (Week 15-17)

### 6.1 Command Framework
```rust
// crates/commands/src/mod.rs

pub trait Command: Send + Sync {
    fn name(&self) -> &str;
    fn aliases(&self) -> &[&str] { &[] }
    fn description(&self) -> &str;
    fn args_description(&self) -> Option<&str> { None }
    
    async fn execute(
        &self,
        args: &str,
        context: &mut CommandContext,
    ) -> Result<CommandResult>;
}

pub struct CommandRegistry {
    commands: HashMap<String, Arc<dyn Command>>,
}
```

### 6.2 Command Implementations (Priority Order)

#### Tier 1: Essential (Week 15)
- `/help` - Show available commands
- `/clear` - Clear conversation
- `/exit` - Exit application
- `/status` - Show status
- `/config` - Configuration
- `/model` - Set AI model
- `/compact` - Compact conversation

#### Tier 2: Git & Code (Week 16)
- `/commit` - Git commit
- `/diff` - View changes
- `/review` - Code review
- `/branch` - Branch conversation

#### Tier 3: Advanced (Week 17)
- `/mcp` - MCP server management
- `/permissions` - Permission rules
- `/resume` - Resume session
- `/export` - Export conversation
- `/memory` - Memory management
- All remaining commands

### Deliverables
- [ ] Command trait and registry
- [ ] All 75+ commands implemented
- [ ] Command argument parsing
- [ ] Tab completion support

---

## Phase 7: Terminal UI (Week 18-20)

### 7.1 UI Framework Selection
Using **ratatui** + **crossterm** for terminal UI (replaces Ink/React)

### 7.2 UI Components
```rust
// crates/cli/src/ui/mod.rs

pub struct App {
    state: SharedState,
    input: TextInput,
    messages: MessageList,
    status_bar: StatusBar,
    mode: UIMode,
}

impl App {
    pub fn run(&mut self) -> Result<()>;
    pub fn render(&self, frame: &mut Frame);
    pub fn handle_input(&mut self, event: Event) -> Result<Action>;
}
```

### 7.3 UI Features
- [ ] Multi-line text input with history
- [ ] Message display with markdown rendering
- [ ] Syntax highlighting for code blocks
- [ ] Progress indicators and spinners
- [ ] Split panes for tool output
- [ ] Vim mode support
- [ ] Scrolling and pagination
- [ ] Color themes

### 7.4 Input Handling
- [ ] Keyboard shortcuts
- [ ] Mouse support
- [ ] Clipboard integration
- [ ] Unicode support

---

## Phase 8: Bridge & Remote System (Week 21-22)

### 8.1 Bridge Architecture
```rust
// crates/bridge/src/mod.rs

pub struct BridgeManager {
    transport: Box<dyn Transport>,
    session_id: SessionId,
    auth: JwtAuth,
}

pub trait Transport: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn send(&self, message: BridgeMessage) -> Result<()>;
    async fn receive(&mut self) -> Result<BridgeMessage>;
    async fn disconnect(&mut self) -> Result<()>;
}
```

### 8.2 Transport Implementations
- [ ] WebSocket transport
- [ ] SSE (Server-Sent Events) transport
- [ ] Hybrid transport (auto-select)

### 8.3 Remote Features
- [ ] Remote session creation
- [ ] JWT authentication
- [ ] Message routing
- [ ] Attachment handling

---

## Phase 9: MCP Client (Week 23-24)

### 9.1 MCP Protocol Implementation
```rust
// crates/mcp/src/client.rs

pub struct McpClient {
    transport: McpTransport,
    server_info: ServerInfo,
}

impl McpClient {
    pub async fn connect(config: &McpServerConfig) -> Result<Self>;
    pub async fn list_tools(&self) -> Result<Vec<ToolDefinition>>;
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value>;
    pub async fn list_resources(&self) -> Result<Vec<Resource>>;
    pub async fn read_resource(&self, uri: &str) -> Result<ResourceContent>;
}
```

### 9.2 MCP Features
- [ ] Server discovery and connection
- [ ] Tool proxying
- [ ] Resource management
- [ ] OAuth authentication flow
- [ ] Server lifecycle management

---

## Phase 10: Multi-Agent System (Week 25-27)

### 10.1 Agent Framework
```rust
// crates/agents/src/mod.rs

pub trait Agent: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn system_prompt(&self) -> &str;
    
    async fn execute(
        &self,
        query: &str,
        context: AgentContext,
    ) -> Result<AgentResult>;
}

pub struct AgentManager {
    agents: HashMap<String, Arc<dyn Agent>>,
    running: HashMap<AgentId, RunningAgent>,
}
```

### 10.2 Built-in Agents
- [ ] GeneralPurpose agent
- [ ] Explore agent (codebase exploration)
- [ ] Plan agent (task planning)
- [ ] Verification agent (output validation)
- [ ] ClaudeCodeGuide agent

### 10.3 Agent Features
- [ ] Agent forking and resumption
- [ ] Inter-agent messaging
- [ ] Shared memory/context
- [ ] Team orchestration
- [ ] Background execution

---

## Phase 11: Advanced Features (Week 28-30)

### 11.1 New Features (Not in Original)
These are advanced features to add during migration:

#### 11.1.1 Local Model Support
```rust
// crates/core/src/api/local.rs

pub struct LocalModelClient {
    endpoint: String,  // Ollama, llama.cpp, etc.
    model: String,
}

impl ModelClient for LocalModelClient {
    async fn create_message(&self, request: MessageRequest) -> Result<MessageResponse>;
}
```

#### 11.1.2 Plugin System
```rust
// crates/plugins/src/mod.rs

pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn on_load(&self, app: &mut App) -> Result<()>;
    fn on_unload(&self, app: &mut App) -> Result<()>;
}

pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    plugin_dir: PathBuf,
}
```

#### 11.1.3 Workflow DSL
```rust
// crates/workflows/src/mod.rs

pub struct Workflow {
    name: String,
    steps: Vec<WorkflowStep>,
    triggers: Vec<Trigger>,
}

pub enum WorkflowStep {
    Tool { name: String, args: Value },
    Agent { name: String, query: String },
    Condition { expr: String, then: Box<WorkflowStep>, else_: Option<Box<WorkflowStep>> },
    Parallel { steps: Vec<WorkflowStep> },
}
```

#### 11.1.4 Team Collaboration
- Shared sessions
- Real-time collaboration
- Permission delegation

### 11.2 Performance Optimizations
- [ ] Lazy tool loading
- [ ] Parallel tool execution
- [ ] Response caching
- [ ] Memory-mapped file operations

---

## Phase 12: Testing & Quality (Week 31-32)

### 12.1 Test Strategy
```
tests/
├── unit/                 # Unit tests per module
├── integration/          # Integration tests
├── e2e/                  # End-to-end tests
└── fixtures/             # Test data
```

### 12.2 Test Coverage Goals
- Unit tests: 80%+ coverage
- Integration tests: All tools, commands, agents
- E2E tests: Critical user flows

### 12.3 Quality Checklist
- [ ] All clippy warnings addressed
- [ ] Documentation for public APIs
- [ ] Error messages are helpful
- [ ] Graceful degradation on failures
- [ ] Memory leak testing
- [ ] Performance benchmarks

---

## Phase 13: Documentation & Polish (Week 33-34)

### 13.1 Documentation
- [ ] README with installation instructions
- [ ] User guide with examples
- [ ] API documentation (rustdoc)
- [ ] Architecture documentation
- [ ] Contributing guide

### 13.2 Distribution
- [ ] Cross-platform builds (Linux, macOS, Windows)
- [ ] Binary releases on GitHub
- [ ] Homebrew formula
- [ ] Cargo publish
- [ ] Shell completion scripts

### 13.3 Migration Guide
- [ ] Feature comparison table
- [ ] Configuration migration tool
- [ ] Breaking changes documentation

---

---

## 🔬 ZED CASE STUDY: Lessons from a Successful Rust Terminal/Editor

### Zed Overview (Researched April 2026)

Zed is a high-performance code editor written entirely in Rust. It's the best comparable project to study for this migration.

| Metric | Zed | Theasus (Target) |
|--------|-----|-------------------------|
| **Total crates** | 227 | ~15-20 (simpler scope) |
| **Lines of Rust** | 1,247,517 | ~100,000 (estimated) |
| **Age** | 4+ years | New |
| **UI Framework** | Custom (GPUI) | ratatui (TUI) |
| **WASM Support** | Yes (gpui_web) | Should add |
| **Extension System** | WASM + wit-bindgen | Should add |

### Key Architectural Decisions from Zed

#### 1. **Custom UI Framework (GPUI)**
Zed built their own UI framework instead of using existing crates.

**Lesson for us:** We're using ratatui, which is simpler than a GUI. But we SHOULD:
- Wrap ratatui behind an abstraction trait (already planned)
- Consider that ratatui may need replacement in 5+ years

#### 2. **HTTP Client Abstraction (Critical Pattern)**
```rust
// Zed's approach: crates/http_client/src/http_client.rs
pub trait HttpClient: 'static + Send + Sync {
    fn send(&self, req: Request<AsyncBody>) -> BoxFuture<'static, Result<Response<AsyncBody>>>;
    fn get(&self, uri: &str, body: AsyncBody, follow_redirects: bool) -> BoxFuture<...>;
    fn post_json(&self, uri: &str, body: AsyncBody) -> BoxFuture<...>;
}
```
**Lesson:** Abstract ALL external dependencies behind traits. Zed has separate `reqwest_client` crate that implements this trait.

#### 3. **Filesystem Abstraction**
```rust
// Zed's approach: crates/fs/src/fs.rs
#[async_trait::async_trait]
pub trait Fs: Send + Sync {
    async fn create_dir(&self, path: &Path) -> Result<()>;
    async fn create_file(&self, path: &Path, options: CreateOptions) -> Result<()>;
    async fn read_file(&self, path: &Path) -> Result<String>;
    // ... 30+ methods
}
```
**Lesson:** Full filesystem abstraction enables:
- Easy mocking in tests
- Future remote filesystem support
- WASM compatibility (different fs implementation)

#### 4. **Settings System Architecture**
Zed uses a macro-based settings system:
```rust
// settings_macros provides #[derive(RegisterSetting)]
pub trait Settings: 'static + Sized + Clone + Send + Sync {
    fn json_schema(...) -> JsonSchema;
    fn load(sources: &SettingsSources, cx: &mut AppContext) -> Result<Self>;
}
```
**Lesson:** Settings should be:
- Strongly typed with serde
- Schema-validated (schemars)
- Hot-reloadable via file watchers

#### 5. **AI/LLM Client Design**
Zed has MULTIPLE AI provider crates:
- `anthropic` - Claude models
- `open_ai` - OpenAI models
- `ollama` - Local models
- `google_ai` - Gemini models
- `bedrock` - AWS Bedrock
- `deepseek`, `mistral`, `lmstudio`, etc.

**Lesson:** Design for multiple providers from day 1:
```rust
pub trait LanguageModel: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
    async fn stream(&self, request: CompletionRequest) -> impl Stream<Item = Result<Chunk>>;
}
```

#### 6. **Extension System (WASM-based)**
Zed uses WebAssembly for extensions:
```rust
// Extension API uses wit-bindgen for WASM interop
pub trait Extension: Send + Sync {
    fn new() -> Self;
    fn language_server_command(&mut self, ...) -> Result<Command>;
}
```
**Lesson:** WASM extensions enable:
- Safe sandboxing
- Language-agnostic extensions
- Hot-reloading without restart

#### 7. **Test Support Features**
Every major crate has a `test-support` feature:
```toml
[features]
test-support = [
    "gpui/test-support",
    "fs/test-support",
    "settings/test-support",
]
```
**Lesson:** Build testability into the architecture from the start.

#### 8. **Platform-Specific Code Organization**
```
crates/gpui/           # Core, cross-platform
crates/gpui_macos/     # macOS-specific
crates/gpui_linux/     # Linux-specific  
crates/gpui_windows/   # Windows-specific
crates/gpui_web/       # WASM/web
```
**Lesson:** Separate platform code into dedicated crates.

---

### Updated Recommendations Based on Zed

| Original Plan | Updated Based on Zed |
|---------------|----------------------|
| Single `core` crate | Split into `core`, `core_platform` |
| Use reqwest directly | Abstract behind `HttpClient` trait |
| Single AI provider | Design for multiple providers (LLM trait) |
| No WASM support | Add `core_wasm` crate from start |
| Simple plugin system | WASM-based extensions with wit-bindgen |
| Manual testing | `test-support` feature flag pattern |

### Current Crate Structure (Zed-Inspired)

```
theasus/
├── Cargo.toml
├── crates/
│   ├── core/                    # ✅ Pure Rust, no I/O
│   ├── language_model/          # ✅ LLM trait abstraction
│   ├── anthropic/               # ✅ Claude API client
│   ├── http_client/             # ✅ HTTP abstraction trait
│   ├── reqwest_client/          # ✅ reqwest implementation
│   ├── fs/                      # ✅ Filesystem abstraction
│   ├── fs_real/                 # ✅ Real filesystem impl
│   ├── terminal/                # Terminal abstraction
│   ├── terminal_crossterm/      # crossterm implementation
│   ├── ui/                      # UI components (ratatui)
│   ├── tools/                   # Tool implementations
│   ├── commands/                # Slash commands
│   ├── agents/                  # Multi-agent system
│   ├── mcp/                     # MCP client
│   ├── bridge/                  # Remote connections
│   ├── settings/                # Settings system
│   ├── settings_macros/         # #[derive(Setting)]
│   ├── extension_api/           # WASM extension API
│   └── cli/                     # Main entry point
│
├── extensions/                  # Built-in extensions
└── tests/                       # Integration tests
```

---

## 🔬 CRITICAL ADDENDUM: 10-Year Maintenance Strategy

### Honest Assessment of Current Plan

**What IS researched:**
- ✅ Actual codebase analysis (1,884 files explored)
- ✅ Tool/command inventory (38 tools, 75 commands documented)
- ✅ Architecture understanding from source code
- ✅ Crate download statistics (see below)

**What is NOT researched:**
- ❌ Performance benchmarks (TS vs Rust for this workload)
- ❌ Similar migration case studies (Warp, Zed, Helix)
- ❌ Long-term crate abandonment risk analysis
- ❌ Total cost of ownership comparison

---

### Crate Stability Analysis (Real Data - April 2026)

| Crate | Downloads | Age | Version | Risk Level | Backup Plan |
|-------|-----------|-----|---------|------------|-------------|
| **tokio** | 595M | 10 yrs | 1.51.0 | 🟢 LOW | Foundation crate, won't die |
| **serde** | 907M | 12 yrs | 1.0.228 | 🟢 LOW | De-facto standard |
| **clap** | 748M | 11 yrs | 4.6.0 | 🟢 LOW | Could swap to `argh` |
| **reqwest** | 423M | 10 yrs | 0.13.2 | 🟢 LOW | `ureq` or `hyper` |
| **crossterm** | 116M | 8 yrs | 0.29.0 | 🟡 MEDIUM | `termion` fallback |
| **ratatui** | 22M | 3 yrs | 0.30.0 | 🟡 MEDIUM | Fork of tui-rs, active |

**Key Risk: ratatui is only 3 years old.** Mitigation required.

---

### Architecture for 10-Year Maintenance

#### 1. Abstraction Layers (Critical)

```rust
// crates/core/src/abstractions/mod.rs

/// UI backend abstraction - can swap ratatui for something else
pub trait TerminalBackend: Send + Sync {
    fn draw<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut Frame);
    fn size(&self) -> io::Result<Rect>;
    fn clear(&mut self) -> io::Result<()>;
}

/// HTTP client abstraction - can swap reqwest for ureq/hyper
pub trait HttpClient: Send + Sync {
    async fn post(&self, url: &str, body: &[u8]) -> Result<Response>;
    async fn get(&self, url: &str) -> Result<Response>;
    fn stream(&self, url: &str) -> impl Stream<Item = Result<Bytes>>;
}

/// Process spawner abstraction - testable, swappable
pub trait ProcessSpawner: Send + Sync {
    async fn spawn(&self, cmd: &str, args: &[&str]) -> Result<ProcessHandle>;
    async fn spawn_shell(&self, command: &str) -> Result<ProcessHandle>;
}

/// Config storage abstraction - can swap JSON for TOML/SQLite
pub trait ConfigStore: Send + Sync {
    fn load(&self) -> Result<Config>;
    fn save(&self, config: &Config) -> Result<()>;
    fn watch(&self) -> impl Stream<Item = ConfigChange>;
}
```

#### 2. Crate Governance Policy

```markdown
## Dependency Policy (enforce in CI)

### Tier 1: Core (Cannot be abandoned)
- tokio, serde, std - No replacement needed

### Tier 2: Stable (Replacements identified)
- clap → argh, lexopt
- reqwest → ureq, hyper
- crossterm → termion, termwiz

### Tier 3: Risky (Abstraction REQUIRED)
- ratatui → Must wrap behind TerminalBackend trait
- Any crate < 5 years old
- Any crate with < 10M downloads

### Annual Audit Checklist
- [ ] Check all dependencies for CVEs (cargo audit)
- [ ] Review maintenance status of Tier 2/3 crates
- [ ] Update abstraction layers if replacements needed
- [ ] Run full benchmark suite
- [ ] Test on latest Rust stable + MSRV
```

#### 3. Minimum Supported Rust Version (MSRV) Strategy

```toml
# Cargo.toml
[package]
rust-version = "1.75"  # Pin to LTS-style version

# Update policy:
# - Bump MSRV annually in January
# - Always support N-2 stable versions
# - Test in CI: stable, beta, MSRV
```

#### 4. Feature Flags for Future-Proofing

```toml
[features]
default = ["tokio-runtime", "ratatui-ui", "reqwest-http"]

# Runtime alternatives
tokio-runtime = ["tokio"]
async-std-runtime = ["async-std"]  # Future option

# UI alternatives  
ratatui-ui = ["ratatui", "crossterm"]
dioxus-ui = ["dioxus-tui"]  # Future option
headless = []  # Server mode, no UI

# HTTP alternatives
reqwest-http = ["reqwest"]
ureq-http = ["ureq"]  # Sync alternative

# AI providers (not just Claude)
anthropic = []
openai = ["async-openai"]
local-llm = ["llama-cpp-rs"]
```

---

### Additional Components for 10-Year Viability

#### 5. WASM Target Support

```
crates/
├── core/          # Pure Rust, no platform deps → compiles to WASM
├── cli/           # Native-only, TUI
└── web/           # WASM + web-sys for browser target
```

```rust
// crates/core/src/lib.rs
#![cfg_attr(target_arch = "wasm32", no_std)]

// Core logic works everywhere
pub mod query_engine;  // Pure async
pub mod tools;         // Abstract over I/O
pub mod permissions;   // Pure logic
```

#### 6. Error Handling Standardization

```rust
// crates/core/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TheasusError {
    // API errors
    #[error("API error: {message} (status: {status})")]
    Api { status: u16, message: String },
    
    #[error("Rate limited, retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },
    
    #[error("Budget exceeded: ${spent:.2} of ${limit:.2}")]
    BudgetExceeded { spent: f64, limit: f64 },
    
    // Tool errors
    #[error("Tool '{tool}' failed: {reason}")]
    ToolExecution { tool: String, reason: String },
    
    #[error("Permission denied for '{tool}': {reason}")]
    PermissionDenied { tool: String, reason: String },
    
    // IO errors
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },
    
    #[error(transparent)]
    Io(#[from] std::io::Error),
    
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

// Result type alias
pub type Result<T> = std::result::Result<T, TheasusError>;
```

#### 7. Observability & Telemetry

```rust
// crates/core/src/telemetry.rs
use tracing::{info, warn, error, instrument};

#[instrument(skip(client), fields(model = %request.model))]
pub async fn query_api(
    client: &impl HttpClient,
    request: &MessageRequest,
) -> Result<MessageResponse> {
    let start = Instant::now();
    let result = client.post(API_URL, &request.to_bytes()?).await;
    
    info!(
        duration_ms = start.elapsed().as_millis(),
        tokens_in = result.usage.input_tokens,
        tokens_out = result.usage.output_tokens,
        "API call completed"
    );
    
    result
}
```

#### 8. Property-Based Testing & Fuzzing

```rust
// crates/core/tests/property_tests.rs
use proptest::prelude::*;

proptest! {
    #[test]
    fn message_roundtrip(msg in any::<Message>()) {
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, decoded);
    }
    
    #[test]
    fn permission_rules_are_deterministic(
        rules in any::<Vec<PermissionRule>>(),
        input in any::<ToolInput>()
    ) {
        let result1 = check_permission(&rules, &input);
        let result2 = check_permission(&rules, &input);
        assert_eq!(result1, result2);
    }
}

// Fuzzing target
#[cfg(fuzzing)]
fuzz_target!(|data: &[u8]| {
    if let Ok(cmd) = std::str::from_utf8(data) {
        let _ = parse_bash_command(cmd);  // Should never panic
    }
});
```

#### 9. Security Hardening

```rust
// crates/core/src/security.rs

/// Validate paths never escape sandbox
pub fn validate_path(path: &Path, sandbox: &Path) -> Result<PathBuf> {
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(sandbox) {
        return Err(TheasusError::SecurityViolation {
            reason: format!("Path {} escapes sandbox {}", path.display(), sandbox.display()),
        });
    }
    Ok(canonical)
}

/// Sanitize shell commands
pub fn sanitize_command(cmd: &str) -> Result<String> {
    // Reject command injection patterns
    let dangerous = ["$(", "`", "${", "&&", "||", ";", "|", ">", "<"];
    for pattern in dangerous {
        if cmd.contains(pattern) {
            return Err(TheasusError::SecurityViolation {
                reason: format!("Dangerous pattern '{}' in command", pattern),
            });
        }
    }
    Ok(cmd.to_string())
}
```

---

### Revised Timeline with Maintenance

| Phase | Original | With 10-Year Prep | Notes |
|-------|----------|-------------------|-------|
| 0. Foundation | 2 wks | 3 wks | +Abstraction layers |
| 1. Core Types | 2 wks | 2 wks | Same |
| 2. Config/State | 2 wks | 3 wks | +Config abstraction |
| 3. API Client | 2 wks | 3 wks | +HTTP abstraction |
| 4. Tools | 4 wks | 5 wks | +Process abstraction |
| 5. Permissions | 2 wks | 2 wks | Same |
| 6. Commands | 3 wks | 3 wks | Same |
| 7. Terminal UI | 3 wks | 4 wks | +UI abstraction, a11y |
| 8. Bridge | 2 wks | 2 wks | Same |
| 9. MCP | 2 wks | 2 wks | Same |
| 10. Agents | 3 wks | 3 wks | Same |
| 11. Advanced | 3 wks | 4 wks | +WASM, i18n |
| 12. Testing | 2 wks | 3 wks | +Property tests, fuzz |
| 13. Docs | 2 wks | 3 wks | +Maintenance docs |

**New Total: ~42 weeks (10-11 months)**

---

### Annual Maintenance Checklist

```markdown
## Yearly Maintenance (Every January)

### Security
- [ ] Run `cargo audit` - fix all CVEs
- [ ] Review permission system for bypass vectors
- [ ] Update MSRV if needed

### Dependencies  
- [ ] Check Tier 2/3 crate health
- [ ] Update major versions (with testing)
- [ ] Evaluate new alternatives

### Performance
- [ ] Run benchmark suite
- [ ] Compare to previous year
- [ ] Profile memory usage

### Compatibility
- [ ] Test on latest OS versions (Linux, macOS, Windows)
- [ ] Test on new terminal emulators
- [ ] Verify WASM target still builds

### Documentation
- [ ] Update architecture docs
- [ ] Review error messages
- [ ] Update migration guides
```

---

## Appendix A: File-by-File Migration Map

### Core Files
| TypeScript | Rust | Priority |
|------------|------|----------|
| QueryEngine.ts | crates/core/src/engine/query_engine.rs | P0 |
| Tool.ts | crates/tools/src/lib.rs | P0 |
| Task.ts | crates/core/src/task.rs | P0 |
| context.ts | crates/core/src/context.rs | P1 |
| commands.ts | crates/commands/src/lib.rs | P1 |

### Types
| TypeScript | Rust | Priority |
|------------|------|----------|
| types/ids.ts | crates/core/src/types/ids.rs | P0 |
| types/permissions.ts | crates/permissions/src/types.rs | P0 |
| types/message.ts | crates/core/src/types/message.rs | P0 |
| types/tools.ts | crates/tools/src/types.rs | P0 |

### Tools (38 total)
| Tool | Rust Module | Priority |
|------|-------------|----------|
| BashTool | crates/tools/src/bash/ | P0 |
| FileReadTool | crates/tools/src/file_read/ | P0 |
| FileWriteTool | crates/tools/src/file_write/ | P0 |
| FileEditTool | crates/tools/src/file_edit/ | P0 |
| GrepTool | crates/tools/src/grep/ | P0 |
| GlobTool | crates/tools/src/glob/ | P0 |
| AskUserQuestionTool | crates/tools/src/ask_user/ | P1 |
| ConfigTool | crates/tools/src/config/ | P1 |
| LSPTool | crates/tools/src/lsp/ | P1 |
| AgentTool | crates/agents/src/tool.rs | P2 |
| MCPTool | crates/mcp/src/tool.rs | P2 |
| ... | ... | ... |

---

## Appendix B: Rust Crate Recommendations

### Essential Crates
```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.11", features = ["json", "stream"] }
clap = { version = "4", features = ["derive"] }
ratatui = "0.26"
crossterm = "0.27"
tracing = "0.1"
tracing-subscriber = "0.3"
thiserror = "1"
anyhow = "1"
tokio-tungstenite = "0.21"
regex = "1"
glob = "0.3"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
directories = "5"
```

### Development Crates
```toml
[dev-dependencies]
tokio-test = "0.4"
mockall = "0.12"
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

---

## Appendix C: Glossary

| Term | Definition |
|------|------------|
| **QueryEngine** | Core conversation loop that processes user input and tool calls |
| **Tool** | An action Theasus can take (file read, bash, etc.) |
| **Command** | User-initiated action via `/command` syntax |
| **MCP** | Model Context Protocol - standard for AI tool integration |
| **Bridge** | System for remote terminal connections |
| **Agent** | Specialized AI assistant with specific capabilities |
| **Permission** | Access control for tool execution |

---

## Timeline Summary

| Phase | Duration | Weeks |
|-------|----------|-------|
| 0. Foundation | 2 weeks | 1-2 |
| 1. Core Types | 2 weeks | 3-4 |
| 2. Config & State | 2 weeks | 5-6 |
| 3. API Client | 2 weeks | 7-8 |
| 4. Tool System | 4 weeks | 9-12 |
| 5. Permissions | 2 weeks | 13-14 |
| 6. Commands | 3 weeks | 15-17 |
| 7. Terminal UI | 3 weeks | 18-20 |
| 8. Bridge | 2 weeks | 21-22 |
| 9. MCP | 2 weeks | 23-24 |
| 10. Agents | 3 weeks | 25-27 |
| 11. Advanced | 3 weeks | 28-30 |
| 12. Testing | 2 weeks | 31-32 |
| 13. Documentation | 2 weeks | 33-34 |

**Total: ~34 weeks (8-9 months)**

---

## Getting Started (For Interns)

### Prerequisites
1. Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. Install development tools: `rustup component add clippy rustfmt`
3. Clone the repository
4. Read the Rust Book: https://doc.rust-lang.org/book/

### First Tasks
1. **Week 1**: Set up project, implement `SessionId` and `AgentId` types
2. **Week 2**: Implement basic config loading
3. **Week 3**: Create `Tool` trait and `FileReadTool`
4. **Week 4**: Add basic CLI with `clap`

### Code Review Checklist
- [ ] Compiles without warnings
- [ ] Passes `cargo clippy`
- [ ] Formatted with `cargo fmt`
- [ ] Has unit tests
- [ ] Has documentation comments
- [ ] Error handling is appropriate

### Resources
- Rust Book: https://doc.rust-lang.org/book/
- Tokio Tutorial: https://tokio.rs/tokio/tutorial
- Ratatui Examples: https://github.com/ratatui-org/ratatui/tree/main/examples
- Error Handling: https://doc.rust-lang.org/book/ch09-00-error-handling.html

(End of file - total 1496 lines)
