# Repository Guidelines

## Project Structure & Module Organization

This is a Rust 2024 Cargo application. Source lives under `src/`.

- `src/main.rs` starts the app, initializes the SQLite database, and launches the iced UI.
- `src/app/` contains GUI state, theme settings, and reusable UI components.
- `src/agent/` contains agent structs, orchestration, and tool traits.
- `src/repo/` contains repository traits, SQLite connection handling, filters, and record schema definitions.
- `docs/` is reserved for project notes and design documentation.

Build output goes to `target/`. The local SQLite file `data.db` is generated at runtime and ignored by Git.

## Build, Test, and Development Commands

Use Cargo from the repository root:

```bash
cargo check
```

Type-checks the project quickly without producing a final binary.

```bash
cargo test
```

Runs unit tests.

```bash
cargo run
```

Starts the desktop app and creates or opens `data.db`.

```bash
cargo fmt
```

Formats Rust code with `rustfmt`.

## Coding Style & Naming Conventions

Use standard Rust formatting with 4-space indentation. Prefer `snake_case` for modules, functions, and fields; `PascalCase` for structs, enums, and traits.

Keep modules explicit with `mod` declarations. Re-export only stable public APIs from `mod.rs`. Prefer small traits for boundaries such as `AgentTool`, `Repo<T>`, and `RecordSchema`.

Use `anyhow::Result` for application-level errors. Keep SQL values parameterized; do not interpolate user input into SQL strings.

## Testing Guidelines

Place focused unit tests near the code under `#[cfg(test)]`. Name tests by behavior, for example:

```rust
fn build_where_clause_rejects_empty_filters()
```

Run `cargo test` before submitting changes. Add tests for repo helpers, schema generation, JSON handling, and agent/tool execution paths as they become concrete.

## Commit & Pull Request Guidelines

Recent history uses Conventional Commit-style messages:

```text
feat: implement repository structure with SQLite integration
refactor: change agent struct fields to private visibility
```

Use short, imperative commits with prefixes such as `feat:`, `fix:`, `refactor:`, `test:`, or `docs:`.

Pull requests should include a concise summary, test results (`cargo check`, `cargo test`), and screenshots or notes for visible UI changes.

## Security & Configuration Tips

Do not commit local databases, secrets, or API keys. `data.db` is ignored. OpenAI credentials should come from environment variables such as `OPENAI_API_KEY`, not source files.
