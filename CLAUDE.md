# Continuum — Agent Context

<!-- BEGIN bigpowers:project -->
## Project

Continuum — GTK4/Libadwaita manhwa (.cbz) reader with continuous vertical scrolling and seamless multi-archive navigation.

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
<!-- END bigpowers:project -->

<!-- BEGIN bigpowers:context-routing -->
## Context Routing

| Glob | Sub-AGENTS.md |
|------|---------------|
| `src/ui/*.rs` | GTK4/Libadwaita UI patterns, widget lifecycle |
| `src/cbz/*.rs` | ZIP archive handling, image decoding |
| `src/cache/*.rs` | Memory management, eviction policies |
<!-- END bigpowers:context-routing -->

<!-- BEGIN bigpowers:learned-preferences -->
## Learned User Preferences

- User prefers concise output (ponytail mode active)
- User values safety around infrastructure and secrets (from global AGENTS.md)
- User wants maintainability and small reviewable changes

## Workspace Facts

- Working directory: `/home/omary/Dev/continuum`
- Git branch: `main`
- Last audit: 2026-08-13 (piolium deep audit, commit 4b8bf32e)
<!-- END bigpowers:learned-preferences -->

## Conventions

- Follow idiomatic Rust: `cargo fmt`, `cargo clippy` clean
- Modules mirror `src/` layout
- No `unsafe` in application code (already the case)
- All public functions should have doc comments
- Error handling: use `Result<T, String>` for user-facing errors, `?` operator throughout
- Prefer `eprintln!` for user-visible errors, `tracing` for debug logs

## Never Do

- Do NOT modify `piolium/` directory (security audit artifacts)
- Do NOT change `Cargo.toml` dependencies without explicit approval
- Do NOT add new dependencies for functionality that can be implemented in <50 lines
- Do NOT use `unwrap()` or `expect()` in production code paths — use proper error handling
- Do NOT introduce `unsafe` blocks without explicit approval and documentation

## Defensive Code

Apply these patterns where relevant:

- **Rate limit:** Cap concurrent decode workers (currently unbounded rayon spawn)
- **Graceful degradation:** Skip failed pages, continue loading others (already partially implemented)
- **Timeout:** Consider decode timeouts for malicious archives (zip bomb mitigation)

## Output Convention

All planning documents go to `specs/` at project root. See CONVENTIONS.md for details.
