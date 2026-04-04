# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-04-04

### Added

#### Bridge & Remote System
- WebSocket-based communication bridge for remote connections
- `BridgeConnection` with automatic reconnection logic
- `BridgeServer` with multi-client session management
- `BridgeManager` for coordinating client/server operations
- Authentication support with API keys and tokens

#### MCP Client (Model Context Protocol)
- Full JSON-RPC 2.0 compliant client implementation
- `tools/list` and `tools/call` operations
- `resources/list` and `resources/read` operations
- `prompts/list` and `prompts/get` operations
- `McpClientManager` for managing multiple MCP servers
- Async request/response correlation with timeout handling

#### Multi-Agent System
- `Agent` trait for implementing custom agents
- `LlmAgent` base implementation with agentic loop
- Tool filtering and turn limits per agent
- `AgentOrchestrator` for multi-agent coordination with dependency tracking
- `AgentRegistry` for agent discovery and management
- Built-in agents:
  - `GeneralPurposeAgent` - Full capabilities for complex tasks
  - `ExploreAgent` - Codebase exploration and Q&A
  - `PlanAgent` - Task planning and decomposition
  - `TaskAgent` - Single task execution
  - `CodeReviewAgent` - Code review with high signal-to-noise ratio

#### Plugin System
- `PluginManifest` for plugin metadata and capabilities
- `Plugin` trait with lifecycle hooks (`on_load`, `on_unload`)
- `PluginManager` for discovery, loading, and hot-reload
- `PluginContext` for runtime access to tools and state
- Optional native plugin support via `libloading` feature

#### Workflow DSL Engine
- Declarative workflow definitions in YAML/JSON
- `Workflow` and `WorkflowStep` types
- Step types: `Tool`, `Agent`, `Condition`, `Parallel`, `Loop`, `Prompt`
- `WorkflowExecutor` with full execution engine
- Variable interpolation between steps
- Triggers: Manual, Schedule (cron), File-based

#### Tools & Core
- `ToolRegistry::execute()` method for direct tool invocation
- Tool result builders: `ToolResult::success()`, `ToolResult::error()`
- Message helper methods in core types
- 5 new unit tests for tools crate

### Changed

- Updated deprecated `set_cursor` to `set_cursor_position` in UI
- Improved iterator usage in permissions system
- Removed unreachable match arms in omik_provider
- Updated README with new crates documentation

### Fixed

- All clippy warnings resolved across workspace
- Async recursion issue in workflow executor using `BoxFuture`

## [0.1.0] - 2026-04-04

### Added

- Initial release of Bodhi AI Terminal
- Multi-provider LLM support (OpenAI, Anthropic, Ollama, Custom)
- Terminal UI with ratatui
- Tool system with bash, file_read, file_write, grep, glob
- Slash commands: `/help`, `/clear`, `/exit`, `/status`, `/model`, `/tools`
- Configuration system with JSON storage
- Permission system with rule-based access control
- Query engine with message history

[Unreleased]: https://github.com/RavenRepo/Omikai-Bodhi/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/RavenRepo/Omikai-Bodhi/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/RavenRepo/Omikai-Bodhi/releases/tag/v0.1.0
