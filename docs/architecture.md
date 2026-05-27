# Cowork RS Architecture

## Overview

Cowork RS is a Rust 2024 desktop application providing a local-first AI agent chat interface. It uses the **iced** GUI framework, communicates with LLMs via the OpenAI-compatible chat completions API, and persists all sessions and messages in a local SQLite database.

The project follows a three-layer architecture:

```
┌─────────────────────────────────────────────────────────────┐
│                        main.rs                              │
│   (wires SqliteDb + AgentOrchestrator → launches iced app)  │
└─────────────┬───────────────────────────────────┬───────────┘
              │                                   │
    ┌─────────▼──────────┐             ┌──────────▼───────────┐
    │      src/app/      │             │     src/agent/       │
    │   (UI layer)       │────────────▶│   (LLM layer)        │
    │   iced widgets,    │  streaming  │   OpenAI client,     │
    │   theme, state     │  callbacks  │   presets, tools     │
    └─────────┬──────────┘             └──────────────────────┘
              │
    ┌─────────▼──────────┐
    │     src/repo/      │
    │  (Persistence)     │
    │  SQLite, records,  │
    │  repositories      │
    └────────────────────┘
```

## Configuration

**Environment variables** (`.env` file, gitignored):

```env
OPENAI_API_KEY=           # Required
OPENAI_BASE_URL=https://api.openai.com/v1   # Optional, defaults shown
COWORK_EMOJI_FONT=        # Optional, overrides emoji font path
```

**Rust toolchain**: Pinned to Rust 1.95.0 via `rust-toolchain.toml`, targeting `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`.

## Entry Point: `main.rs`

The `main()` function performs three sequential initialization steps:

1. **Database init** — Opens `data.db` via `repo::SqliteDb::open()`, then calls `db.init_record::<SessionRecord>()` and `db.init_record::<MessageRecord>()` to ensure tables exist.
2. **Agent orchestrator init** — Calls `agent_orchestrator::init()` which loads env vars, creates an OpenAI client, and builds an `AgentOrchestrator` with a single default agent.
3. **UI launch** — Calls `app::run(db, agent_orchestrator)` which starts the iced event loop.

## Module: `src/agent/` — Agent System

### Submodules

| File | Purpose |
|------|---------|
| `agent.rs` | Core `Agent` struct, streaming methods, `AgentStreamCallback` trait |
| `agent_orchestrator.rs` | `AgentOrchestrator` container, `init()` entry point, env config |
| `agent_preset.rs` | Compile-time `AgentPreset` constants |
| `agent_tool.rs` | `AgentTool` trait for tool execution |
| `agent_tool_mcp.rs` | Placeholder for MCP tool integration |
| `tool/mcp/mcp_tool_adapter.rs` | Placeholder for MCP adapter |

### `Agent` (`agent.rs`)

The core LLM interaction unit. Holds:
- `id`, `name`, `system_instruction`, `model` — identity and configuration
- `llm_client: Client<OpenAIConfig>` — the OpenAI-compatible API client
- `tools: Vec<Box<dyn AgentTool + Send + Sync>>` — registered tools (currently empty)

**Key methods:**
- `call()` — non-streaming chat completion
- `call_stream()` — streaming completion, returns final text only
- `call_stream_response()` — streaming completion, returns full `AgentResponse` with usage and tool call data

**`AgentStreamCallback`** (async trait) — the bridge between agent and UI:
- `on_token(&str)` — called for each streamed token
- `on_complete(&str)` — called when stream finishes
- `on_error(&anyhow::Error)` — called on failure
- `on_usage(&CompletionUsage)` — called with token usage stats

### `AgentOrchestrator` (`agent_orchestrator.rs`)

Container holding `agents: Vec<Agent>`. Provides:
- `agents()` — returns slice of all agents
- `default_agent()` — returns the first agent

The `init()` function reads `OPENAI_API_KEY` and `OPENAI_BASE_URL` from environment, creates the OpenAI client, and builds an `AgentOrchestrator` with a single agent from `DEFAULT_AGENT_PRESET`.

### `AgentPreset` (`agent_preset.rs`)

Compile-time constant configuration:

```rust
pub struct AgentPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub system_instruction: &'static str,
    pub model: &'static str,
}
```

Default preset: id `"default"`, name `"Cowork Agent"`, model `"mimo-v2.5"`.

### `AgentTool` (`agent_tool.rs`)

Trait for synchronous tool execution:

```rust
pub trait AgentTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_json(&self) -> &str;      // JSON schema
    fn execute(&self, json_input: &str) -> anyhow::Result<String>;
}
```

No concrete implementations exist yet. Tool call deltas from the API are accumulated but not executed.

## Module: `src/app/` — UI System

### Submodules

| File | Purpose |
|------|---------|
| `mod.rs` | `CoworkApp` state machine, message handling, persistence logic (938 lines) |
| `theme/mod.rs` | Custom "Soft White Green" theme, style functions |
| `components/chat.rs` | Chat area, message rendering, composer |
| `components/sidebar.rs` | Session list, new chat button, pagination |
| `components/settings.rs` | Modal settings dialog with tabs |

### Application State (`mod.rs`)

**`CoworkApp`** holds:
- `db: Rc<SqliteDb>` — shared database connection
- `agent_orchestrator: Arc<AgentOrchestrator>` — shared agent system
- Session state: `session_id`, `session_title`, `session_model`, `next_sequence`
- UI state: `draft`, `messages: Vec<ChatMessage>`, `recent_sessions`
- Streaming state: `streaming_assistant_index`, `is_waiting_for_agent`

**`Message`** enum (21 variants) — the iced event type:
- Input: `DraftChanged`, `Send`
- Sessions: `NewSession`, `SessionSelected`, `DeleteSession`, `LoadMoreSessions`
- Settings: `OpenSettings`, `CloseSettings`, `SettingsTabSelected`
- Streaming: `ChatStreamToken`, `ChatStreamCompleted`, `ChatStreamFailed`, `ChatStreamUsage`
- Fonts: `IconFontLoaded`, `EmojiFontBytesLoaded`, `EmojiFontLoaded`

### Chat Messages

**`ChatMessage`** — rendered message with:
- `kind: ChatMessageKind` — `User`, `Assistant`, or `System`
- `body: String` — raw text content
- `markdown: Vec<markdown::Item>` — parsed markdown
- `blocks: Vec<ChatMessageBlock>` — parsed blocks with icon directives

**Icon directives** — agents can embed icons in responses:
- `[icon:name] rest of line` or `:name: rest of line`
- Supported icons: `Alert`, `Check`, `Code`, `Info`, `Link`, `Rocket`, `Star`, `Tools`, `X`, `Zap`

### Theme (`theme/mod.rs`)

"Soft White Green" palette:

| Token | Hex | Usage |
|-------|-----|-------|
| `ACCENT` | `#2ec27e` | Green accent, buttons, highlights |
| `BACKGROUND` | `#f5f4ef` | Main background |
| `SURFACE` | `#fbfaf7` | Content areas |
| `PANEL` | `#fffffc` | Elevated surfaces |
| `SIDEBAR` | `#ecefeb` | Sidebar background |
| `USER_BUBBLE` | `#eef8f2` | User message background |
| `BORDER` | `#dddbd2` | Borders and dividers |
| `TEXT` | `#242622` | Primary text |
| `MUTED_TEXT` | `#626861` | Secondary text |
| `SUBTLE_TEXT` | `#8a8d86` | Tertiary text |

Window size: 1100×720 pixels.

### Components

**`chat_area()`** — scrollable transcript with composer:
- User messages: right-aligned green bubble, max 620px wide
- Agent messages: left-aligned with icon header
- Composer: text input + send button, shows "Waiting" state
- Emoji rendering: detects emoji characters and applies dedicated emoji font

**`sidebar()`** — 280px fixed sidebar:
- Header with title and subtitle
- "New chat" button
- Recent sessions list with pagination (5 initial, 5 per page)
- Active session highlighted with green accent, delete button

**`settings_dialog()`** — 760×500px modal:
- **General** tab: theme, window, font status
- **Agent** tab: preset info, model, parameters (read-only)
- **Tools** tab: runtime status, MCP (planned)
- **Storage** tab: SQLite info, message count

## Module: `src/repo/` — Persistence System

### Submodules

| File | Purpose |
|------|---------|
| `sqlite_db.rs` | `SqliteDb` connection wrapper |
| `record/record.rs` | `RecordSchema` trait |
| `record/record_impl.rs` | `SessionRecord`, `MessageRecord` definitions |
| `repo.rs` | `Repo<T>` trait |
| `repo_filter.rs` | `RepoFilter` query builder |
| `repo_impl.rs` | `SessionRepo`, `MessageRepo` implementations |

### `SqliteDb` (`sqlite_db.rs`)

Wraps `rusqlite::Connection` with:
- `open(path)` — opens connection, enables foreign keys
- `init_record<T: RecordSchema>()` — ensures table exists
- `conn()` — returns `&Connection`
- `resolved_path()` — canonical database file path

Shared via `Rc<SqliteDb>` throughout the app (single-threaded access).

### Records (`record/record_impl.rs`)

**`SessionRecord`** — `sessions` table:

| Column | Type | Notes |
|--------|------|-------|
| `session_id` | TEXT | Primary key |
| `title` | TEXT | Auto-derived from first message |
| `model` | TEXT | Model identifier |
| `created_at` | INTEGER | Unix millis |
| `updated_at` | INTEGER | Unix millis, indexed |
| `temperature` | REAL | Sampling temperature |
| `top_p` | INTEGER | Top-p sampling |
| `top_k` | INTEGER | Top-k sampling |

**`MessageRecord`** — `messages` table:

| Column | Type | Notes |
|--------|------|-------|
| `message_id` | TEXT | Primary key (`"{session_id}-{sequence}"`) |
| `session_id` | TEXT | FK to sessions, CASCADE delete |
| `sequence` | INTEGER | Message order within session |
| `role` | INTEGER | 0=system, 1=assistant, 2=user, 3=tool |
| `content` | TEXT | Message text |
| `created_at` | INTEGER | Unix millis |

Composite index on `(session_id, sequence)`.

### Repository Trait (`repo.rs`)

```rust
pub trait Repo<T> {
    fn read_all(&self) -> anyhow::Result<Vec<T>>;
    fn read(&self, filters: &[RepoFilter]) -> anyhow::Result<Vec<T>>;
    fn upsert(&self, item: T) -> anyhow::Result<()>;
    fn delete(&self, filters: &[RepoFilter]) -> anyhow::Result<()>;
}
```

All writes use `INSERT ... ON CONFLICT DO UPDATE` for idempotency.

### `RepoFilter` (`repo_filter.rs`)

Type-safe query filter builder:
- `RepoFilter::text(column, value)` — `Value::Text`
- `RepoFilter::integer(column, value)` — `Value::Integer`
- `RepoFilter::real(column, value)` — `Value::Real`
- `RepoFilter::null(column)` — `Value::Null`

## Data Flow

### Message Send Flow

```
User types message → Message::Send
    │
    ├─→ Add user message to self.messages
    ├─→ Auto-title session from first user message
    ├─→ Persist user message via MessageRepo::upsert()
    ├─→ Persist/update session via SessionRepo::upsert()
    ├─→ Build chat_request_messages() from transcript
    ├─→ Spawn async task via stream::channel()
    │     │
    │     └─→ AgentOrchestrator::chat_completion_stream_response()
    │           │
    │           └─→ Agent::call_stream_response()
    │                 │
    │                 └─→ OpenAI API streaming call
    │                       │
    │                       ├─→ UiAgentStreamCallback::on_token()
    │                       │     └─→ Message::ChatStreamToken
    │                       │
    │                       ├─→ UiAgentStreamCallback::on_usage()
    │                       │     └─→ Message::ChatStreamUsage
    │                       │
    │                       └─→ on_complete() / on_error()
    │                             └─→ Message::ChatStreamCompleted / ChatStreamFailed
    │
    ├─→ Push empty assistant message
    └─→ Set streaming_assistant_index
```

### Token Streaming Flow

```
ChatStreamToken(token)
    │
    ├─→ Find assistant message at streaming_assistant_index
    ├─→ Append token to message body
    └─→ Reparse markdown and blocks
```

### Completion Flow

```
ChatStreamCompleted(final_content)
    │
    ├─→ Set final body on assistant message
    ├─→ Persist assistant message via MessageRepo::upsert()
    ├─→ Update session via SessionRepo::upsert()
    └─→ Clear streaming_assistant_index
```

## Ownership Model

| Resource | Wrapper | Access Pattern |
|----------|---------|----------------|
| `SqliteDb` | `Rc<SqliteDb>` | Single-threaded, borrowed by repos |
| `AgentOrchestrator` | `Arc<AgentOrchestrator>` | Shared across tokio tasks |
| `Connection` | `&Connection` | Lightweight borrow per operation |
| Repositories | Created on demand | No persistent state |

## Key Design Decisions

1. **Compile-time presets** — Agent configuration is hardcoded as `&'static str` constants. No runtime configuration exists yet.

2. **Tool system defined but unused** — The `AgentTool` trait is fully specified, and `Agent` holds a tools vec, but no tools are registered and tool call deltas are accumulated but not executed.

3. **Synchronous tool execution** — `AgentTool::execute()` is synchronous despite the async architecture, suggesting tools are expected to be fast local operations.

4. **Upsert-based persistence** — All writes use `INSERT ... ON CONFLICT DO UPDATE`, making operations idempotent. Message IDs encode session and sequence.

5. **Streaming via iced channels** — Bridge between tokio async tasks and iced's event loop uses `stream::channel(100, ...)` with an `mpsc::Sender<Message>`.

6. **Custom icon parsing** — Agents can embed icon directives in responses using `[icon:name]` or `:name:` syntax, rendered with Octicons badges.

7. **Emoji font probing** — Rather than bundling a font, the app probes ~12 candidate paths at startup, with `assets/fonts/emoji.ttf` as fallback.

8. **Schema-on-read pattern** — `RecordSchema::create_table_sql()` returns full DDL strings. `init_record<T>()` ensures tables exist before operations.

9. **MCP placeholders** — Empty files exist for future Model Context Protocol tool integration.

10. **No LLM provider abstraction** — The agent is tightly coupled to OpenAI-compatible APIs via `async-openai`.

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `iced` | 0.14.0 | GUI framework (tokio + markdown features) |
| `iced_aw` | 0.14.0 | Additional iced widgets |
| `iced_fonts` | 0.3.0 | Octicons icon font |
| `async-openai` | 0.29 | OpenAI-compatible API client |
| `rusqlite` | 0.39.0 | SQLite database (bundled) |
| `tokio` | 1 | Async runtime (rt-multi-thread) |
| `serde` / `serde_json` | 1 | Serialization |
| `schemars` | 1 | JSON schema generation (unused) |
| `jsonschema` | 0.46 | JSON schema validation (unused) |
| `anyhow` | 1 | Error handling |
| `thiserror` | 2 | Error derive macros |
| `async-trait` | 0.1 | Async trait support |
| `dotenvy` | 0.15 | .env file loading |
| `futures-util` | 0.3 | Stream utilities |

## Build Targets

```bash
make check          # cargo check
make test           # cargo test
make fmt            # cargo fmt
make build          # cargo build
make build-linux-x86_64    # cross-compile for x86_64
make build-linux-aarch64   # cross-compile for aarch64
make build-linux-all       # both architectures
```
