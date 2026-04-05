# Theasus Rust Migration — Complete Parity Audit

> **Audit Date**: 2026-04-04  
> **Source**: `ClaudeTerminal/src/` (TypeScript) → `Theasus/crates/` (Rust)

---

## Executive Summary

The Theasus Rust workspace has **solid architectural foundations** — the 22-crate structure, trait abstractions, and core engine loop are well-designed and production-ready. However, the migration is **approximately 12-15% complete by feature count**. The TypeScript codebase contains **1,332 source files** across 40 tools and 80+ commands, while the Rust workspace has **32 files / 8,700 LOC** with 5 tools and 12 commands.

> [!IMPORTANT]
> The architectural skeleton is correct. The gap is in **feature coverage**, not design. Every missing module has a clear place to land in the existing crate structure.

---

## Module-by-Module Parity Matrix

### 🔧 Tools (`crates/tools/`)

| TypeScript Tool        | Rust Status    | Priority             |
| ---------------------- | -------------- | -------------------- |
| `BashTool`             | ✅ Implemented | —                    |
| `FileReadTool`         | ✅ Implemented | —                    |
| `FileWriteTool`        | ✅ Implemented | —                    |
| `GlobTool`             | ✅ Implemented | —                    |
| `GrepTool`             | ✅ Implemented | —                    |
| `FileEditTool`         | ❌ Missing     | 🔴 P0 — Core editing |
| `AgentTool`            | ❌ Missing     | 🔴 P0 — Multi-agent  |
| `MCPTool`              | ❌ Missing     | 🔴 P0 — MCP bridge   |
| `WebFetchTool`         | ❌ Missing     | 🔴 P0 — HTTP fetch   |
| `WebSearchTool`        | ❌ Missing     | 🔴 P0 — Web search   |
| `TaskCreateTool`       | ❌ Missing     | 🟡 P1 — Task system  |
| `TaskGetTool`          | ❌ Missing     | 🟡 P1 — Task system  |
| `TaskListTool`         | ❌ Missing     | 🟡 P1 — Task system  |
| `TaskStopTool`         | ❌ Missing     | 🟡 P1 — Task system  |
| `TaskUpdateTool`       | ❌ Missing     | 🟡 P1 — Task system  |
| `TaskOutputTool`       | ❌ Missing     | 🟡 P1 — Task system  |
| `SendMessageTool`      | ❌ Missing     | 🟡 P1 — Multi-agent  |
| `TodoWriteTool`        | ❌ Missing     | 🟡 P1 — Planning     |
| `ListMcpResourcesTool` | ❌ Missing     | 🟡 P1 — MCP          |
| `ReadMcpResourceTool`  | ❌ Missing     | 🟡 P1 — MCP          |
| `McpAuthTool`          | ❌ Missing     | 🟡 P1 — MCP auth     |
| `AskUserQuestionTool`  | ❌ Missing     | 🟡 P1 — Interaction  |
| `ToolSearchTool`       | ❌ Missing     | 🟡 P1 — Discovery    |
| `EnterPlanModeTool`    | ❌ Missing     | 🟢 P2 — Plan mode    |
| `ExitPlanModeTool`     | ❌ Missing     | 🟢 P2 — Plan mode    |
| `EnterWorktreeTool`    | ❌ Missing     | 🟢 P2 — Worktree     |
| `ExitWorktreeTool`     | ❌ Missing     | 🟢 P2 — Worktree     |
| `RemoteTriggerTool`    | ❌ Missing     | 🟢 P2 — Remote       |
| `LSPTool`              | ❌ Missing     | 🟢 P2 — Code intel   |
| `SkillTool`            | ❌ Missing     | 🟢 P2 — Skills       |
| `BriefTool`            | ❌ Missing     | 🟢 P2 — Output       |
| `ConfigTool`           | ❌ Missing     | 🟢 P2 — Config       |
| `SleepTool`            | ❌ Missing     | 🟢 P2 — Utility      |
| `SyntheticOutputTool`  | ❌ Missing     | 🟢 P2 — Synthetic    |
| `REPLTool`             | ❌ Missing     | 🟢 P2 — REPL         |
| `NotebookEditTool`     | ❌ Missing     | 🟢 P2 — Notebooks    |
| `PowerShellTool`       | ❌ Missing     | 🟢 P2 — Windows      |
| `ScheduleCronTool`     | ❌ Missing     | 🟢 P2 — Cron         |
| `TeamCreateTool`       | ❌ Missing     | 🟢 P2 — Teams        |
| `TeamDeleteTool`       | ❌ Missing     | 🟢 P2 — Teams        |

**Score: 5 / 40 tools implemented (12.5%)**

---

### ⌨️ Commands (`crates/commands/`)

| Command    | Rust Status    | Notes                                   |
| ---------- | -------------- | --------------------------------------- |
| `/help`    | ✅ Implemented | Aliases: `h`, `?`                       |
| `/clear`   | ✅ Implemented | Alias: `c`                              |
| `/exit`    | ✅ Implemented | Aliases: `e`, `quit`, `q`               |
| `/status`  | ✅ Implemented | Shows session/cwd                       |
| `/model`   | ✅ Implemented | Get/set model                           |
| `/compact` | ⚠️ Stub        | Returns "not yet implemented"           |
| `/tools`   | ✅ Implemented | Lists 5 tools                           |
| `/agents`  | ✅ Implemented | Lists agents                            |
| `/config`  | ⚠️ Partial     | Read-only, update "not yet implemented" |
| `/env`     | ✅ Implemented | Shows env vars                          |
| `/pwd`     | ✅ Implemented | Working directory                       |
| `/history` | ⚠️ Stub        | Returns "coming soon"                   |

**70+ commands missing**: add-dir, branch, bridge, btw, chrome, color, commit, context, copy, cost, desktop, diff, doctor, effort, export, extra-usage, fast, feedback, files, heapdump, hooks, ide, init, install-github-app, install-slack-app, keybindings, login, logout, mcp, memory, mobile, output-style, passes, permissions, plan, plugin, pr_comments, privacy-settings, rate-limit-options, release-notes, reload-plugins, remote-env, remote-setup, rename, resume, review, rewind, sandbox-toggle, session, skills, stats, stickers, tag, tasks, terminalSetup, theme, thinkback, thinkback-play, upgrade, usage, vim, voice, ...

**Score: 9 active + 3 stubs / 80+ commands (≈12%)**

---

### 🏗️ Architectural Crates

| Crate                             | Parity Status | Detail                                                                        |
| --------------------------------- | ------------- | ----------------------------------------------------------------------------- |
| `core`                            | ✅ Solid      | Types, AppState, Message, ContentBlock, QueryEngine shell                     |
| `language_model`                  | ✅ Solid      | `LanguageModel` trait with streaming                                          |
| `omik_provider`                   | ✅ Solid      | OpenAI, Anthropic, Ollama adapters                                            |
| `http_client`                     | ✅ Solid      | Trait abstraction                                                             |
| `reqwest_client`                  | ✅ Solid      | Implementation                                                                |
| `fs` / `fs_real`                  | ✅ Solid      | Filesystem trait + real impl                                                  |
| `terminal` / `terminal_crossterm` | ✅ Solid      | Terminal trait + crossterm impl                                               |
| `ui`                              | ⚠️ Shell      | Ratatui scaffolding, missing React-equivalent hooks/state                     |
| `cli`                             | ✅ Solid      | clap-based entry point                                                        |
| `agents`                          | ⚠️ Shell      | `LlmAgent` loop + `AgentOrchestrator` exist, task decomposition incomplete    |
| `mcp`                             | ⚠️ Shell      | JSON-RPC protocol handler, missing SDK/auth/OAuth flows                       |
| `settings`                        | ✅ Solid      | Strongly-typed config with serde                                              |
| `bridge`                          | ⚠️ Shell      | WebSocket frame, missing 25+ bridge modules from TS                           |
| `permissions`                     | ⚠️ Shell      | `PermissionManager` exists, missing bash classifier, rule parser, 15+ modules |
| `plugins`                         | ⚠️ Shell      | `Plugin` trait, missing installation manager + operations                     |
| `workflows`                       | ⚠️ Shell      | YAML DSL engine, basic step execution                                         |
| `commands`                        | ⚠️ Minimal    | 12 commands vs 80+ in TS                                                      |
| `tools`                           | ⚠️ Minimal    | 5 tools vs 40 in TS                                                           |
| `extension_api`                   | ❌ Planned    | Not started                                                                   |
| `settings_macros`                 | ❌ Planned    | Not started                                                                   |

---

### 🔌 Services Layer (Not Yet Ported)

The TypeScript `src/services/` directory contains critical systems that don't have Rust equivalents yet:

| Service            | TS Files  | Rust Crate        | Status                               |
| ------------------ | --------- | ----------------- | ------------------------------------ |
| MCP Client/Server  | ~15 files | `crates/mcp/`     | ⚠️ Protocol only, no SDK/auth/OAuth  |
| OAuth/Auth         | ~5 files  | —                 | ❌ No auth system                    |
| Plugin Manager     | ~3 files  | `crates/plugins/` | ⚠️ Trait only, no install/operations |
| Session Memory     | ~3 files  | `crates/core/`    | ❌ Not ported                        |
| Team Memory Sync   | ~5 files  | —                 | ❌ Not ported                        |
| Voice/STT          | ~3 files  | —                 | ❌ Not ported                        |
| Tool Orchestration | ~4 files  | —                 | ❌ Not ported                        |
| Rate Limiting      | ~3 files  | —                 | ❌ Not ported                        |
| Tips Engine        | ~3 files  | —                 | ❌ Not ported                        |
| Notifications      | ~1 file   | —                 | ❌ Not ported                        |
| Cost Tracking      | ~2 files  | —                 | ❌ Not ported                        |

### 🌉 Bridge System (Minimal)

| TS Module                      | Description                | Rust                           |
| ------------------------------ | -------------------------- | ------------------------------ |
| `bridgeMain.ts`                | Core bridge orchestration  | ⚠️ Partial in `crates/bridge/` |
| `bridgeMessaging.ts`           | Message protocol           | ⚠️ Frame types only            |
| `bridgeApi.ts`                 | REST API endpoints         | ❌ Missing                     |
| `bridgeConfig.ts`              | Configuration              | ❌ Missing                     |
| `bridgePermissionCallbacks.ts` | Permission integration     | ❌ Missing                     |
| `jwtUtils.ts`                  | JWT token handling         | ❌ Missing                     |
| `remoteBridgeCore.ts`          | Remote execution           | ❌ Missing                     |
| `replBridge*.ts`               | REPL integration (4 files) | ❌ Missing                     |
| `trustedDevice.ts`             | Device trust system        | ❌ Missing                     |
| + 15 more files                | Various bridge utilities   | ❌ Missing                     |

### 🔒 Permissions System (Minimal)

| TS Module                  | Description                | Rust                                |
| -------------------------- | -------------------------- | ----------------------------------- |
| `permissions.ts`           | Core permission logic      | ⚠️ Partial in `crates/permissions/` |
| `PermissionMode.ts`        | Mode enum                  | ✅ In `PermissionMode` enum         |
| `PermissionRule.ts`        | Rule types                 | ⚠️ Basic `PermissionRule` struct    |
| `bashClassifier.ts`        | Command safety analysis    | ❌ Missing                          |
| `yoloClassifier.ts`        | Auto-approve classifier    | ❌ Missing                          |
| `permissionRuleParser.ts`  | Rule parsing               | ❌ Missing                          |
| `pathValidation.ts`        | Path safety checks         | ❌ Missing                          |
| `dangerousPatterns.ts`     | Dangerous command patterns | ❌ Missing                          |
| `denialTracking.ts`        | Permission denial tracking | ❌ Missing                          |
| `shadowedRuleDetection.ts` | Rule conflict detection    | ❌ Missing                          |
| `shellRuleMatching.ts`     | Shell pattern matching     | ❌ Missing                          |
| + 10 more files            | Permission utilities       | ❌ Missing                          |

---

## 📊 Summary Scorecard

| Dimension                       | Coverage      | Grade |
| ------------------------------- | ------------- | ----- |
| **Architecture & Trait Design** | 100%          | 🟢 A  |
| **Core Types & Engine**         | ~80%          | 🟢 B+ |
| **LLM Provider Integration**    | ~90%          | 🟢 A  |
| **Tool System**                 | 12.5% (5/40)  | 🔴 F  |
| **Command System**              | ~12% (12/80+) | 🔴 F  |
| **Services Layer**              | ~5%           | 🔴 F  |
| **Bridge System**               | ~10%          | 🔴 F  |
| **Permission System**           | ~15%          | 🔴 F  |
| **Plugin System**               | ~20%          | 🔴 D  |
| **UI/State Management**         | ~15%          | 🔴 F  |
| **Testing**                     | 0 test files  | 🔴 F  |
| **Documentation**               | 90%           | 🟢 A  |

### Overall Migration Progress: **~15%**

---

## 🎯 Recommended Implementation Roadmap

### Phase 1 — Core Tools (Critical Path)

> **Goal**: Achieve minimum viable agent execution

1. `FileEditTool` — diff-based file editing (the most-used tool)
2. `WebFetchTool` — HTTP content retrieval
3. `WebSearchTool` — Web search integration
4. `AgentTool` — Multi-agent spawning
5. `MCPTool` — MCP tool proxy
6. `AskUserQuestionTool` — User interaction

### Phase 2 — Task System

> **Goal**: Enable background task management

7. `TaskCreateTool` / `TaskGetTool` / `TaskListTool` / `TaskStopTool` / `TaskUpdateTool`
8. `SendMessageTool` — Inter-agent messaging
9. `TodoWriteTool` — Planning support

### Phase 3 — Commands

> **Goal**: Feature-complete command system

10. Port all P0 commands: `/plan`, `/context`, `/diff`, `/review`, `/session`, `/resume`, `/permissions`
11. Port P1 commands: `/branch`, `/cost`, `/usage`, `/memory`, `/mcp`, `/login`

### Phase 4 — Services

> **Goal**: Production-ready runtime

12. Session Memory system
13. OAuth/Authentication flow
14. Rate limiting & cost tracking
15. Permission system (bash classifier, rule parser, pattern matching)

### Phase 5 — Bridge & Advanced

> **Goal**: Remote and collaborative capabilities

16. Full bridge messaging protocol
17. JWT/device trust
18. Plugin installation manager
19. Voice/STT (optional)

---

> [!WARNING]
> The Rust codebase compiles and the architecture is sound, but it is **not production-ready** for end users. The 5 implemented tools (bash, file_read, file_write, glob, grep) provide basic functionality, but the 35 missing tools and 70+ missing commands mean most user workflows from the TypeScript version will fail silently or error out.
