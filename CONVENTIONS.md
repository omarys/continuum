# Conventions

Shared rules for all AI agents working on this project.

## Project

Continuum — GTK4/Libadwaita manhwa (.cbz) reader.

## Stack

- **Language:** Rust (edition 2021)
- **Framework:** GTK4 0.9 + Libadwaita 0.7
- **Runtime:** Native Linux desktop (no web, no server)
- **Build:** cargo

## Commands

| Action | Command |
|--------|---------|
| Run | `cargo run` |
| Test | `cargo test` |
| Build | `cargo build` |
| Lint | `cargo clippy --all-targets -- -D warnings` |
| Format | `cargo fmt --check` |
| Preflight | `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo build` |

## Architecture

Single-process GTK4/Libadwaita desktop app. Reads .cbz (zip) archives, decodes images via `image` crate on rayon worker threads, sends RGBA bytes through crossbeam channel to GTK main loop for texture upload. Memory manager caps resident textures at 1 GiB with distance-based eviction.

Key modules:
- `src/ui/reader.rs` — scroll orchestration, lazy loading, chapter navigation
- `src/ui/page_widget.rs` — single page display (picture + placeholder + spinner)
- `src/cbz/archive.rs` — .cbz reading, image decoding, texture creation
- `src/cache/memory_manager.rs` — LRU-ish texture cache with eviction

## Conventions

- Follow idiomatic Rust: `cargo fmt`, `cargo clippy` clean
- Modules mirror `src/` layout
- No `unsafe` in application code
- All public functions should have doc comments
- Error handling: use `Result<T, String>` for user-facing errors, `?` operator throughout
- Prefer `eprintln!` for user-visible errors, `tracing` for debug logs

## Never Do

- Do NOT modify `piolium/` directory (security audit artifacts)
- Do NOT change `Cargo.toml` dependencies without explicit approval
- Do NOT add new dependencies for functionality that can be implemented in <50 lines
- Do NOT use `unwrap()` or `expect()` in production code paths
- Do NOT introduce `unsafe` blocks without explicit approval and documentation

## Defensive Code Categories

Apply these patterns where relevant:

- **Rate limit:** Cap concurrent decode workers
- **Graceful degradation:** Skip failed pages, continue loading others
- **Timeout:** Consider decode timeouts for malicious archives

## Output Convention

All planning documents go to `specs/` at project root:

```
specs/
├── product/           → vision, scope, glossary
├── tech-architecture/ → tech stack, security plan, test plan
├── epics/             → epic capsules (stories + tasks)
├── bugs/              → bug reports and registry
├── verifications/     → UAT evidence
├── adr/               → architecture decision records
├── state.yaml         → session state, active flow
├── release-plan.yaml  → release index
└── execution-status.yaml → story/epic status
```

## Git

- Branch naming: `feat/<description>`, `fix/<description>`, `refactor/<description>`
- Commit messages: Conventional Commits (feat:, fix:, refactor:, docs:, test:, chore:)
- Do NOT commit unless explicitly asked
- Do NOT push without explicit approval

## Testing

- Unit tests in `#[cfg(test)] mod tests` within each module
- Integration tests in `tests/` directory for cross-module behavior
- Every epic task must have `verify: <runnable command>` in its YAML

## Code Style

- Prefer small, focused functions (<50 lines)
- Prefer composition over inheritance
- Prefer explicit error handling over panics
- Prefer immutable data where possible
- Document complex algorithms or non-obvious logic
