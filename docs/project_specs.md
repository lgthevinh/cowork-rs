# Cowork RS Project Specs

## Overview

Cowork RS is a local-first Rust desktop app for chatting with a built-in AI agent. It uses `iced` for the UI, OpenAI-compatible chat completions for the agent, MCP for tools, and SQLite for local persistence.

## Core Goals

- Provide a native desktop chat UI using `iced`.
- Run a built-in AI agent backed by OpenAI chat completions.
- Store chat sessions and messages locally in SQLite.
- Keep agent definitions controlled in code through presets.
- Build a foundation for future tools, knowledge documents, embeddings, and richer orchestration.

## Architecture

- `src/app/`: iced UI state, components, and theme configuration.
- `src/agent/`: agent definition, presets, OpenAI orchestration, and tool traits.
- `src/record/`: reusable local storage primitives such as `RecordSqlite`, `SqliteRecord`, `RecordSchema`, `RecordFilter`, and `RecordFile`.
- `src/storage/`: app-specific persisted record types.

`src/main.rs` opens local storage, initializes schemas, creates the agent orchestrator, and launches the UI.

## Current Behavior

On startup, the app opens `data.db` through `RecordSqlite`, creates the chat tables if needed, initializes the default agent preset, and launches the chat UI.

When the user sends a message, the UI:

1. Adds the user message to the transcript.
2. Persists the user message to SQLite.
3. Calls OpenAI chat completion asynchronously.
4. Adds the assistant response to the transcript.
5. Persists the assistant response to SQLite.

## Data Model

`SessionRecord` and `MessageRecord` live in `src/storage/chat_record.rs`. They implement `RecordSchema` for table creation and `SqliteRecord` for SQL, row conversion, and upsert bindings. `RecordSqlite` owns the single SQLite connection and provides DAO-style calls such as `read_all::<SessionRecord>()`, `upsert(record)`, and `delete::<MessageRecord>(&filters)`.

Message roles use constants:

- `0`: system
- `1`: assistant
- `2`: user
- `3`: tool

## Configuration

LLM provider configuration is stored in `preference/llm-provider.json`, created from defaults on first startup and ignored by Git. Use `preference/llm-provider.example.json` as the template.

`.env` remains supported as fallback for secrets and base URL:

```env
OPENAI_API_KEY=
OPENAI_BASE_URL=https://api.openai.com/v1
```

Real `.env` files are ignored by Git. Production builds should prefer environment variables or a future settings/keychain flow.

MCP servers are stored in `preference/mcp-servers.json`, created on first startup, and ignored by Git. Use `preference/mcp-servers.example.json` as the template. If an older root `mcp-servers.json` exists and the preference file does not, the app imports it once without deleting the old file.

## Roadmap

- Add JSON schema validation for tool inputs.
- Add knowledge document records and embedding storage.
- Support streaming responses and cancellation.
- Add richer user-facing forms for nested MCP server configuration.
