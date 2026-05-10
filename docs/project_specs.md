# Cowork RS Project Specs

## Overview

Cowork RS is a Rust desktop application for working with AI agents in a local-first environment. The project is inspired by Claude-style coworking workflows, but it uses a custom GUI, compile-time agent presets, local SQLite persistence, and an extensible tool system.

The goal is to provide a focused desktop app where a user can chat with a built-in agent, persist sessions locally, and eventually expand the agent with custom tools, knowledge, and structured workflows.

## Core Goals

- Provide a native desktop chat UI using `iced`.
- Run a built-in AI agent backed by OpenAI chat completions.
- Store chat sessions and messages locally in SQLite.
- Keep agent definitions controlled in code through presets.
- Build a foundation for future tools, knowledge documents, embeddings, and richer orchestration.

## Architecture

The application is split into three main subsystems:

- `src/app/`: iced UI state, components, and theme configuration.
- `src/agent/`: agent definition, presets, OpenAI orchestration, and tool traits.
- `src/repo/`: SQLite connection handling, record schemas, repository traits, filters, and concrete repo implementations.

`src/main.rs` wires these pieces together by opening the local database, initializing schemas, creating the agent orchestrator, and launching the UI.

## Current Behavior

On startup, the app opens `data.db`, creates the `sessions` and `messages` tables if needed, initializes the default compile-time agent preset, and launches the chat UI.

When the user sends a message, the UI:

1. Adds the user message to the transcript.
2. Persists the user message to SQLite.
3. Calls OpenAI chat completion asynchronously.
4. Adds the assistant response to the transcript.
5. Persists the assistant response to SQLite.

## Data Model

`SessionRecord` stores chat thread metadata such as `session_id`, `title`, `model`, timestamps, and model parameters.

`MessageRecord` stores ordered text messages with `message_id`, `session_id`, `sequence`, numeric `role`, `content`, and `created_at`.

Message roles use constants:

- `0`: system
- `1`: assistant
- `2`: user
- `3`: tool

## Configuration

Development configuration can be provided with `.env`:

```env
OPENAI_API_KEY=
OPENAI_BASE_URL=https://api.openai.com/v1
```

Real `.env` files are ignored by Git. Production builds should prefer environment variables or a future settings/keychain flow.

## Roadmap

- Load previous sessions from SQLite into the sidebar.
- Add real tool execution through `AgentTool`.
- Add JSON schema validation for tool inputs.
- Add knowledge document records and embedding storage.
- Support streaming responses and cancellation.
- Move user-facing configuration into an app settings screen.
