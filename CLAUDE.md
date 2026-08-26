# Continuum

High-performance GTK4 / Libadwaita manhwa & manga (.cbz) reader with continuous scrolling.

## Commands

- **Run:** `cargo run`
- **Test:** `cargo test`
- **Lint:** `cargo clippy --all-targets -- -D warnings`
- **Format:** `cargo fmt --check`
- **Preflight:** `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`

## Architecture

- `src/ui/reader.rs` — Viewport scroll orchestration, lazy loading, continuous chapter navigation.
- `src/ui/page_widget.rs` — Individual page rendering (`GtkPicture` + placeholder) & reading mode layouts.
- `src/ui/window.rs` — Main window shell, header bar, keyboard shortcuts, Dewey JSON exit payload.
- `src/cbz/archive.rs` — CBZ archive parsing, chapter number detection, threaded image decoding.
- `src/cache/memory_manager.rs` — Distance-based texture memory cache with bounded eviction.

## Invariants

- Keep `cargo clippy` and `cargo test` clean at all times.
- No `unsafe` and no `unwrap()` in production paths — use explicit error handling (`Result<T, String>`).
- Favor standard library / native platform capabilities over adding new dependencies.
- Do not modify `piolium/` (security audit artifact directory).
