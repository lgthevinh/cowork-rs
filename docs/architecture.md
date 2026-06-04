# Cowork RS Architecture

Cowork RS is a Rust 2024 desktop chat app using `iced`, an OpenAI-compatible agent, MCP tools, and local SQLite persistence.

## Modules

- `src/main.rs`: opens local storage, initializes record schemas, creates the agent orchestrator, and starts the UI.
- `src/app/`: iced state, views, components, theme, session UI, message streaming, and persistence calls.
- `src/agent/`: agent preset, OpenAI-compatible client orchestration, streaming callbacks, builtin tools, and MCP tool adapters.
- `src/record/`: reusable local storage primitives.
- `src/storage/`: Cowork-specific persisted record types.
- `src/log/`: lightweight logging helper.

## Startup

1. `main.rs` opens `data.db` through `record::RecordSqlite`.
2. `RecordSqlite::init::<SessionRecord>()` and `init::<MessageRecord>()` ensure tables exist.
3. `agent_orchestrator::init()` loads provider config, creates the OpenAI-compatible client, registers builtin tools, and discovers MCP tools from `preference/mcp-servers.json` if present.
4. `app::run(db, agent_orchestrator)` launches the iced UI.

## Storage Boundary

`record` is the reusable storage layer:

- `RecordSqlite`: app-facing SQLite DAO. Owns the single SQLite connection.
- `SqliteRecord`: trait implemented by records that know their SQL, row conversion, and upsert bindings.
- `RecordSchema`: table creation contract.
- `RecordFilter`: typed query filter values.
- `RecordFile`: one-file-per-DAO JSON helper.
- `SqliteDb`: internal SQLite connection wrapper used by `RecordSqlite`.

`storage` is app-specific:

- `chat_record.rs`: `SessionRecord`, `MessageRecord`, role constants, and their `RecordSchema` / `SqliteRecord` implementations.

Preferred usage:

```rust
let db = RecordSqlite::open("data.db")?;
db.init::<SessionRecord>()?;
db.init::<MessageRecord>()?;

let sessions = db.read_all::<SessionRecord>()?;
let messages = db.read::<MessageRecord>(&[RecordFilter::text("session_id", id)])?;
db.upsert(message)?;
db.delete::<MessageRecord>(&filters)?;
```

## Data Model

`SessionRecord` stores session metadata: `session_id`, `title`, `model`, timestamps, and sampling parameters.

`MessageRecord` stores ordered chat messages: `message_id`, `session_id`, `sequence`, numeric `role`, `content`, and `created_at`.

Message roles:

- `0`: system
- `1`: assistant
- `2`: user
- `3`: tool

## Message Flow

When a user sends a message:

1. The UI appends and persists the user message.
2. The UI upserts the current session metadata.
3. The UI builds OpenAI chat messages from the transcript.
4. The agent streams tokens back through `AgentStreamCallback`.
5. The UI appends streamed tokens to the assistant message.
6. On completion, the final assistant message and session update are persisted.

## Configuration

LLM provider config lives at `preference/llm-provider.json` and is managed through `RecordFile`. The Agent settings tab can edit it; saving calls `AgentOrchestrator::load_provider_config()` to swap the client/model into the existing agent.

MCP server config lives at `preference/mcp-servers.json` and is managed through `RecordFile`. The Tools settings tab can edit the JSON directly; saving calls `AgentOrchestrator::reload_mcp_servers_config()` to reconnect MCP servers and replace the running agent tools. If an older root `mcp-servers.json` exists and the preference file does not, startup imports the legacy file once without deleting it.

`.env` can still provide fallback values:

```env
OPENAI_API_KEY=
OPENAI_BASE_URL=https://api.openai.com/v1
```

`COWORK_EMOJI_FONT` can override emoji font probing. `preference/mcp-servers.json` is loaded when present and is ignored by Git. Use `preference/mcp-servers.example.json` as the template.

## Design Notes

- Keep app-specific records out of `src/record/`.
- Keep SQLite connection ownership inside `RecordSqlite`.
- Put SQL and row conversion beside the record type via `SqliteRecord`.
- Use parameterized SQL only.
- Keep agent presets compile-time for now.
- MCP failures should log warnings and not prevent app startup.
