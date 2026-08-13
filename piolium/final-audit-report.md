# Final Security Audit Report — Continuum

**Repository:** omarys/continuum — GTK4/Libadwaita manhwa (.cbz) reader, Rust
**Commit audited:** `4b8bf32e8cb145ddbf880f3b42143560b89746c3` (HEAD)
**Methodology:** piolium deep audit (Phases 1–12)
**Date:** 2026-08-13

---

## Executive Summary

Continuum is a single-process, single-user desktop application with **no network surface,
no IPC, no `unsafe` code in `src/`, and no cross-user/cross-tenant trust boundary**. Its
entire attack surface is a crafted `.cbz`/`.zip` archive that the user opens (or that is
planted as a sibling of a comic the user opens).

The audit confirmed **one HIGH-severity finding** — an **unbounded decompression (zip-bomb)
path** that lets a small, valid-CRC archive force multi-GiB allocations and crash the process
via OOM (the release profile sets `panic = "abort"`, so the failure is a hard abort). Ten
**MEDIUM** findings follow, all in the resource-exhaustion / state-machine / trust-scope
families, with no memory-safety defects and no privilege escalation.

**Bottom line:** Continuum is not remotely exploitable and contains no memory-safety
vulnerability. Its risks are (1) availability — a malicious comic can crash the reader or
consume transient memory well past the intended 1 GiB cap — and (2) correctness — a
chapter-id state-machine bug can render/display the wrong archive's pages. Both are fixable
with small, well-scoped budget and lifecycle changes.

---

## Methodology Summary

| Phase | Description | Result |
|---|---|---|
| P1 | Advisory intelligence + SBOM | 236-crate tree; no open advisories on locked decoder versions at audit time |
| P2 | Patch-bypass analysis | No applicable advisories; no patches to bypass |
| P3 | Knowledge base + threat model | 8 runtime entry points, 4 trust boundaries, 7 abuse paths ranked |
| P4 | Static analysis | Semgrep OSS (Pro unavailable — documented fallback) + `cargo clippy` (clean) + manual structural extraction (CodeQL N/A for Rust); 15 drafts |
| P5 | Authorization audit | 3 findings (single-file grant widened to sibling directory) |
| P6 | Spec gap / state machine | 5 findings (chapter-id divergence, TOCTOU, lifecycle) |
| P7 | Deep bug hunting | 3 findings |
| P8 | FP check + contradiction | 6 findings; p8-005 falsified most of p4-007 |
| P9 | Variant analysis | Structural variants enumerated across all 8 entry points |
| P10 | Review Chamber synthesis | Dedup/merge into 11 root causes (see KB `## Phase 10 Addendum`) |
| P11 | FP elimination + cold verification | 3 HIGH candidates cold-verified from scratch: p4-001 **CONFIRMED**; p6-001 reclassified **correctness-only** (dropped); p8-003 downgraded **HIGH→MEDIUM** |
| P12 | Variant analysis | Confirmed patterns propagated; no additional findings |

CodeQL was not run (no Rust extractor exists; binary not installed). Semgrep Pro was not
available (auth required) and OSS was used as the documented fallback; cross-file taint was
compensated by full manual source-to-sink reconstruction.

---

## Summary of Findings

| ID | Severity | Title | CWE | Cold-verified |
|---|---|---|---|---|
| H1 | **HIGH** | Unbounded decompression / zip bomb → OOM abort | CWE-400, CWE-770 | ✅ CONFIRMED |
| M1 | MEDIUM | Chapter-id state-machine divergence → wrong-archive decode | CWE-440, CWE-362 | ✅ (static) |
| M2 | MEDIUM | Transient decode memory exceeds 1 GiB cache cap | CWE-400, CWE-770 | ✅ CONFIRMED (bounded) |
| M3 | MEDIUM | Per-page ZIP reopen: CPU amplification + TOCTOU | CWE-400, CWE-367 | ✅ (static) |
| M4 | MEDIUM | Main-thread texture upload stall | CWE-400, CWE-770 | ✅ (static) |
| M5 | MEDIUM | Decode request burst / channel backlog / stale accumulation | CWE-400, CWE-770 | ✅ (static) |
| M6 | MEDIUM | Sibling auto-load trust-scope widening | CWE-829, CWE-20 | ✅ (static) |
| M7 | MEDIUM | Entry-count DoS at open (O(n) directory walk) | CWE-400 | ✅ (static) |
| M8 | MEDIUM | Window lifecycle pump leak (per open signal) | CWE-404, CWE-400 | ✅ (static) |
| M9 | MEDIUM | Untrusted image decode (contingent on future advisories) | CWE-400 | ⚠️ contingent |
| M10 | MEDIUM | CWD appended to icon search path | CWE-427, CWE-400 | ✅ (static) |

**Dropped (LOW / by-design / informational):** log-terminal injection (p4-007, narrowed to
LOW by p8-005), banner non-UTF-8 cosmetic bypass (p4-008), file-dialog filter advisory
(p4-010), AVIF feature mismatch (p4-011), 32-bit size truncation (p4-015), desktop-entry
install artifact (p5-003), stale-decode cross-archive correctness bug (p6-001 — same-user
mis-render, no trust-boundary break).

---

## Technical Findings Detail

### H1 — Unbounded decompression / zip bomb → OOM abort

- **File:** `src/ui/reader.rs:1053-1054`; `src/cbz/archive.rs:91-98`; `Cargo.toml:29` (`panic = "abort"`)
- **CWE:** 400 (uncontrolled resource consumption), 770 (unbounded allocation)
- **Status:** CONFIRMED by independent cold verification.

**Mechanism.** The lazy-decode worker reads a ZIP entry with no size cap:

```rust
let mut buf = Vec::with_capacity(entry.size() as usize); // entry.size() = untrusted header field
std::io::Read::read_to_end(&mut entry, &mut buf)         // no .take(), no cap
```

`ZipFile::size()` (zip 2.4.2) returns the `uncompressed_size` field parsed verbatim from the
local/central header — it is never cross-checked against actual bytes. The zip crate applies
**no decompression-ratio limit** (`Crc32Reader` only validates CRC at EOF). The only
downstream bound is `image`'s 512 MiB `Limits`, which runs **after** full decompression, and
the 1 GiB cache cap, which bounds only post-decode resident textures.

**Impact.** A ~10 KB archive whose entry declares (and, for a real bomb, genuinely expands to)
4 GiB forces a 4 GiB allocation and full inflate before any cap applies — ≥4× the 1 GiB cap,
with no ceiling. A lying ZIP64 header (`u64::MAX`) overflows the `Vec` layout and aborts.
Because `panic = "abort"`, OOM is a hard crash, not a catchable panic.

**Reachability.** Any of the 8 entry points (CLI, GTK open, FileDialog, sibling auto-load)
→ `dispatch_requests_around` → `rayon::spawn` → `read_to_end`. No privileges required.

**Fix.** Cap before allocation: reject `entry.size() > N` (e.g. 256 MiB), use
`Read::take(cap)`, and `Vec::with_capacity(min(size, cap))`. Add a decompressed-total budget
across the archive.

---

### M1 — Chapter-id state-machine divergence → wrong-archive decode

- **Drafts merged:** p6-002 + p7-001 + p8-004
- **File:** `src/ui/reader.rs:210-211, 245-246, 865, 919, 1030, 928`
- **CWE:** 440 (intended-code-path bypass), 362 (race)

Two divergent chapter-id allocators: the `append_chapter`/`prepend_chapter` methods allocate
from the monotonic `next_chapter_id` counter, while the scroll-listener inline auto-load paths
allocate `chapters.len()` and never advance the counter. After an auto-load, a later manual
`append_chapter` reuses an already-assigned id. The `page_widgets` HashMap silently
**overwrites** the earlier chapter's `PageKey`s, and decode dispatch resolves with
`.find(|ch| ch.chapter_id == key.chapter_idx)` — returning the **first** (older) chapter — so
the new chapter's pages are decoded from the **wrong archive's bytes**.

**Impact.** Wrong-content rendering and attacker-controlled bytes (from a sibling the
attacker may plant) flowing into the image pipeline under the victim chapter's page keys;
permanently blank/orphaned page widgets; corrupted eviction ordering.

**Fix.** Single allocator: use `next_chapter_id` everywhere (make the inline paths advance it),
or allocate `chapter_id = next_chapter_id` centrally.

---

### M2 — Transient decode memory exceeds 1 GiB cache cap

- **Draft:** p8-003 (downgraded HIGH→MEDIUM by cold verification)
- **File:** `src/ui/reader.rs:124, 984-1085`; `src/cache/memory_manager.rs:65-93`
- **CWE:** 400, 770

The 1 GiB cap (`evict_if_needed`) bounds only **resident textures after** `set_loaded`. Three
transient pools are uncapped: the unbounded crossbeam channel backlog, the in-flight
worker RGBA buffers, and the raw zip-byte buffers. Cold verification quantified the ceiling:
the dispatch window is fixed (~20 keys), `in_flight` dedups same-key, and `image`'s 512 MiB
`Limits` caps each decode — so realistic content stays ~10–20% of the cap, while a crafted
bomb of max-size images reaches ~8–16× (finite, not unbounded).

**Impact.** Bounded memory amplification; no unbounded growth. Downgraded from HIGH because
the amplification is finite and requires a crafted near-limit bomb.

**Fix.** Global in-flight byte budget + bounded channel (`try_send` with drop) + total
decoded-bytes budget.

---

### M3 — Per-page ZIP reopen: CPU amplification + TOCTOU

- **Drafts merged:** p8-001 + p6-003
- **File:** `src/ui/reader.rs:1047-1050`
- **CWE:** 400, 367 (TOCTOU)

Every decode worker re-opens and re-parses the entire ZIP archive (`File::open` →
`ZipArchive::new` → `by_name`) instead of reusing a per-chapter handle. This is O(pages ×
archive-size) parsing work. Separately, the entry-name list is a snapshot taken at open, but
each page re-opens the archive path — a file replaced between open and decode (TOCTOU) is
decoded under the original entry names.

**Impact.** CPU amplification on large archives; content confusion if the file is swapped
mid-read.

**Fix.** Keep the opened `ZipArchive` in `ChapterState` (or re-open once per chapter, not per
page) and guard against path replacement.

---

### M4 — Main-thread texture upload stall

- **Draft:** p8-002
- **File:** `src/ui/reader.rs:787-841` (decode pump)
- **CWE:** 400, 770

`CbzArchive::create_texture` (pixbuf construction + `Texture::for_pixbuf` GPU upload) runs on
the GTK main thread inside the 16 ms pump, which drains **all** queued payloads in one tick.
A batch of large decodes stalls the UI for the duration of the uploads.

**Impact.** UI jank/stall proportional to queued decoded bytes; complements M2.

**Fix.** Bound uploads per tick, or upload off the critical path where the GTK texture API
permits.

---

### M5 — Decode request burst / channel backlog / stale accumulation

- **Drafts merged:** p4-003 + p4-006 + p4-013 + p6-004
- **File:** `src/ui/reader.rs:984-1024, 1098-1132`
- **CWE:** 400, 770

`dispatch_requests_around` fans out one `rayon::spawn` per unloaded page with only per-key
dedup; there is no global in-flight budget. `G`/`Shift+G` jump-to-end and fast scroll enqueue
many decodes at once. The unbounded channel accumulates payloads faster than the 16 ms pump
drains them, and `clear()` never invalidates already-dispatched workers, so stale workers from
a previous archive accumulate and double-dispatch.

**Impact.** Transient memory spike and UI stall; worker accumulation across archive switches.

**Fix.** Global in-flight decode budget (max concurrent bytes), bounded channel, generation
token to discard stale completions on `clear()`.

---

### M6 — Sibling auto-load trust-scope widening

- **Drafts merged:** p4-004 + p4-012 + p4-009 + p5-001 + p5-002
- **File:** `src/cbz/archive.rs:156-180`; `src/ui/reader.rs:790-924`
- **CWE:** 829 (inclusion of functionality from untrusted sphere), 20 (insufficient validation)

`DirectorySeries::new` scans the **parent directory** of the opened file and lists every
`.cbz`/`.zip` sibling; the scroll listener auto-opens the previous/next sibling on scroll
threshold crossings, with no authorization beyond the single user-granted file. Identification
is extension-only (no content/magic-byte validation). In a Flatpak/sandboxed context where the
user grants access to a single file, this widens the grant to the entire directory.

**Impact.** A planted sibling `evil.cbz` is auto-loaded and decoded without explicit user
choice, broadening the zip-bomb/image-decode attack reach.

**Fix.** Scope auto-load to a user-confirmed series, require explicit opt-in for siblings, and
validate archive magic before opening.

---

### M7 — Entry-count DoS at open

- **Draft:** p4-005
- **File:** `src/cbz/archive.rs:48-63`
- **CWE:** 400

`CbzArchive::open` iterates the **entire** ZIP entry table on the GTK main thread
(`for i in 0..zip.len()`), with no entry-count cap. A crafted archive with a very large central
directory stalls the UI at open.

**Impact.** Main-thread stall proportional to entry count.

**Fix.** Reject archives with > K entries before the walk; move enumeration off the main thread.

---

### M8 — Window lifecycle pump leak

- **Drafts merged:** p7-002 + p8-006 + p6-005
- **File:** `src/main.rs:24, 37`; `src/ui/reader.rs:763-973`
- **CWE:** 404 (improper resource shutdown), 400

Both `connect_activate` and `connect_open` unconditionally construct a **new** `ManhwaWindow`
instead of reusing the existing primary window. Each new window starts its own 16 ms pump
(`timeout_add_local`) whose `SourceId` is never stored or removed on destroy — the pump keeps
running (and the decode pipeline keeps dispatching) after the window is closed.

**Impact.** Resource leak multiplied per open/activate signal; orphaned pumps keep the process
alive.

**Fix.** Reuse `app.windows().first()`; store and remove the `SourceId` on window destroy.

---

### M9 — Untrusted image decode (contingent)

- **Draft:** p4-014
- **File:** `src/cbz/archive.rs:91-98`
- **CWE:** 400

Every page's bytes (from attacker-controlled ZIP entries) are decoded by the `image` crate
(`image::load_from_memory` → `into_rgba8`). No open advisories affect the locked versions at
audit time, but image-decoder vulnerabilities are a recurring CRIT/HIGH category (libpng,
libwebp, libjpeg). This is flagged **contingent** — severity rises if a decoder advisory lands
on a locked version.

**Impact.** Memory-safety RCE as the launching user if a decoder bug is ever triggered.

**Fix.** Pin and track decoder versions; add `cargo audit` to CI; keep distro image libraries
patched.

---

### M10 — CWD appended to icon search path

- **Draft:** p7-003
- **File:** `src/main.rs:124-125`
- **CWE:** 427 (uncontrolled search path), 400

`load_custom_styles` appends the process working directory to the GTK icon theme search path.
Icon lookups resolve against CWD, so an attacker who can place a crafted icon in the
launch directory can influence icon decoding (via gdk-pixbuf) — a secondary decode channel with
no size limits.

**Impact.** Secondary untrusted-decode channel; supply-chain/planted-file influence.

**Fix.** Remove the CWD search-path append (use the app's installed icon path instead).

---

## Conclusion

Continuum is a **well-scoped, memory-safe Rust application** with a deliberately minimal attack
surface. The audit found **no remote attack vector, no memory-safety defect, and no privilege
escalation**. The dominant risk class is **availability**: the unbounded-decompression path
(H1) is a genuine, trivially-exploitable zip bomb that can crash the reader and pressure the
host, and the MEDIUM findings are all resource-budget, state-machine, and lifecycle gaps that
share a single mitigation family — **budgets** (decompression cap, in-flight byte budget,
bounded channel, entry-count cap) plus **single-window lifecycle** and a **single chapter-id
allocator**. All are small, local changes with no architectural redesign required.

Recommended fix priority: **H1** (decompression cap) → **M1** (chapter-id allocator) →
**M2/M5** (in-flight + channel budgets) → **M8** (window lifecycle) → remaining budget/scope
hardening.
