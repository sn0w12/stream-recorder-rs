# AGENTS.md

## Agent Workflow and Best Practices for `stream-recorder-rs`

This file defines conventions, best practices, and required steps for all agents (human or AI) working on this Rust project.

---

## General Rust Agent Guidelines

- **Always run `cargo check` before considering a task complete.**
    - This ensures that the code compiles and catches most errors early.
- Use `cargo fmt` to enforce code style before submitting or committing changes.
- Use `cargo clippy` for linting and to catch common mistakes or non-idiomatic Rust code.
- Write and run tests with `cargo test` for any logic changes, especially in core modules.
- Prefer explicit error handling (using `Result`, `anyhow`, etc.) over panics.
- Use idiomatic Rust patterns (ownership, borrowing, lifetimes) and prefer crates from the Rust ecosystem for common tasks.
- Document public functions and modules with Rustdoc comments (`///`).
- When adding dependencies, prefer minimal, well-maintained crates and update `Cargo.toml` accordingly.
- Keep functions small and focused; refactor large functions into smaller helpers when possible.
- Use feature flags for optional functionality (e.g., Discord integration).

---

## Project-Specific Agent Instructions

- **Testing:**
    - Add or update tests for new features, especially for parsing, config, and CLI logic.
    - Use the provided test suite in `src/main.rs` as an example and add new tests as needed.

- **Error Handling:**
    - Use `anyhow::Result` for error propagation in async and CLI code.
    - Log errors with context for easier debugging.

---

## Required Steps Before Finishing Any Task

1. **Run `cargo check`** to ensure the code compiles.
2. **Run `cargo fmt`** to format the code.
3. **Run `cargo clippy`** and address warnings where possible.
4. **Run `cargo test`** to verify that all tests pass.
5. **Update documentation** (README, config comments, etc.) for any user-facing changes.

---

## File/Module Structure Reference

- `src/main.rs`: CLI entry point, command parsing, and main logic
- `src/config.rs`: Configuration loading, saving, and schema
- `src/stream/monitor.rs`: Stream monitoring, recording, and post-processing
- `src/uploaders/`: Upload service integrations
- `src/template.rs`: Template rendering and helpers
- `src/platform.rs`: Platform definitions and pipeline logic
- `platforms/`: Platform schemas and examples
- `templates/`: Handlebars templates for notifications

---
