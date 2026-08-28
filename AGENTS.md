# Continuum — Agent Instructions

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

Single-process GTK4/Libadwaita desktop app. Reads .cbz (zip) archives, decodes images via the `image` crate on rayon worker threads, sends RGBA bytes through a crossbeam channel to the GTK main loop for texture upload. A memory manager caps resident textures (64 MiB per-entry decompression cap, bounded in-flight dispatch, distance-based eviction).

Key modules:
- `src/ui/reader.rs` — scroll orchestration, lazy loading, chapter navigation
- `src/ui/page_widget.rs` — single page display (picture + placeholder + spinner)
- `src/cbz/archive.rs` — .cbz reading, image decoding, texture creation
- `src/cache/memory_manager.rs` — texture cache with eviction

## Conventions

- Keep `cargo clippy` and `cargo test` clean at all times.
- No `unsafe` and no `unwrap()` in production paths — explicit error handling (`Result<T, String>`).
- Favor stdlib / native platform capabilities over adding dependencies.

## Never Do

- Do NOT modify `Cargo.toml` dependencies without explicit approval.
- Do NOT add dependencies for functionality implementable in <50 lines.

## Agent skills

### Issue tracker

Issues live in GitHub Issues (github.com/omarys/continuum), used via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default five-role vocabulary: needs-triage, needs-info, ready-for-agent, ready-for-human, wontfix. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout: one `CONTEXT.md` + `docs/adr/` at repo root. See `docs/agents/domain.md`.
