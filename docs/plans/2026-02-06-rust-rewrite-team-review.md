# Rust Rewrite Team Review — Consolidated Synthesis

**Date:** 2026-02-06
**Decision:** Proceed with Option A (GPUI)
**Reviewed by:** 4-agent team (UX, Systems, Architecture, Devil's Advocate)

---

## Executive Summary

Four specialists independently reviewed the [Rust rewrite research plan](2026-02-05-rust-rewrite-research.md). The plan is architecturally sound with genuine performance/UX gains. The primary risks are ecosystem immaturity (GPUI pre-1.0, gpui-ghostty 5 weeks old, Zig build dep) — not architectural flaws. Below: what all four agree on, where they diverge, and concrete actions to de-risk the build.

---

## Consensus Findings (All 4 Agree)

### 1. gpui-ghostty immaturity is the #1 risk
- 81 commits, 1 contributor (Xuanwo), 5 weeks old, zero releases
- Bus factor of 1 — if abandoned, you own Zig+Rust+C FFI maintenance
- **Mitigation:** Fork and mirror the repo. Test multi-terminal rendering in Phase 3 (not Phase 5). Define explicit pivot criteria.

### 2. Zig 0.14.1 build dependency adds significant friction
- Non-standard toolchain requirement for all developers + CI
- Zig is pre-1.0 with breaking changes between point releases
- **Mitigation:** Pin exact Zig version in CI (`mlugg/setup-zig@v2`), document in README. Evaluate `gpui-terminal` (alacritty-based, no Zig) as fallback.

### 3. Database migration is the easiest part
- rusqlite opens existing `conescope.db` directly — same SQLite format
- `rusqlite_migration` crate (v1.0) handles schema versioning via `user_version` pragma
- Zero data export/import needed

### 4. Overview mini-terminals need early GPU profiling
- 6-10 live gpui-ghostty instances per window = unknown GPU memory cost
- **Mitigation:** Profile in Phase 3. If too expensive, use static text snapshots for unfocused tiles (update every 500ms) and full rendering only for focused terminal.

### 5. Cargo workspace structure is correct
- 5 crates (bin, core, pty, ui, platform) is right granularity
- No circular dependency risk (Rust prevents it at compile time)
- `conescope-core` purity (no UI deps) is achievable

---

## Key Technical Insights by Area

### PTY Management (Systems Engineer)
- portable-pty uses **blocking `Read`/`Write` traits** — need `std::thread::spawn` for reader loops, NOT `cx.background_spawn()`
- Output flow: reader thread → `flume::channel` → GPUI entity update → terminal view
- Define `PtyEvent` enum (Output, Exit, Error) in `conescope-pty` — don't let GPUI types leak into PTY crate
- `Drop` on `InstanceManager` should kill all PTYs (portable-pty's `Child` already implements `Drop`)

### State Management (Architect)
- **Critical:** Don't use a single `Entity<InstanceManager>` — use **per-instance entities**: `Entity<Instance>` each + `Entity<InstanceList>` for the collection
- Reason: GPUI's `cx.observe()` fires on ANY `notify()`, no selector optimization. Single entity = O(n) re-renders on every terminal output chunk
- **Terminal output batching** needed: accumulate in buffer, `notify()` on frame boundary
- **Re-entrancy panic risk**: updating entity A from within entity B's update closure. Use `cx.emit()` + subscriber pattern for cross-entity updates
- `terminalTabStore` should merge into PTY management, not be a separate entity

### Build System (Systems + Architect)
- GPUI git dep pulls ~300+ transitive deps from Zed monorepo (internal crates `util`, `refineable`, etc. not on crates.io)
- **Mirror git deps into controlled forks** — don't depend on upstream GitHub directly
- CI needs: macOS (primary, Metal works), Linux (`libvulkan-dev` + `xvfb`), Windows (compile check only initially)
- Fresh build estimate: 5-15 min. Add `sccache` + `actions/cache` for CI.
- Consider shipping aarch64 macOS only initially (skip universal binary to halve CI time)

### Async Runtime (Systems Engineer)
- GPUI has its own executor (wraps GCD on macOS) — do NOT mix with tokio
- For PTY I/O: `std::thread::spawn` (blocking), not async
- For database: `cx.background_spawn()` with rusqlite (sync) is fine given light workload
- If a dependency needs tokio: use `async-compat` bridge or prefer non-tokio alternatives (e.g., `ureq` over `reqwest`)

---

## UX Wins and Losses

### Clear Wins (+)
| Area | Current | After Rewrite |
|------|---------|---------------|
| Themes | 1 dark theme | 20+ themes with dark/light mode |
| Code editor | CodeMirror basic | Tree-sitter + LSP (mini Zed editor) |
| Keyboard shortcuts | Global `keydown` listener | Context-aware action dispatch, remappable |
| Panel management | Manual CSS resize | gpui-component `Resizable` + `Dock` |
| Terminal rendering | xterm.js canvas | GPU-accelerated Metal, better VT compliance |
| Memory | 150-300MB | 30-60MB projected |
| Startup | 2-5s | <1s |

### Known Losses (-)
| Area | Impact | Mitigation |
|------|--------|------------|
| **Accessibility** | Screen readers lose all access (GPU rendering = no DOM/accessibility tree) | Document as known limitation. Keyboard-only users covered by gpui-component focus system. |
| **File type icons** | 60+ custom icons need recreation | Reduce to ~15 categories initially, expand later |
| **Copy/paste** | Browser-native → GPU terminal clipboard (unverified in gpui-ghostty) | Test explicitly in Phase 3 |
| **Font rendering** | GPU subpixel differs from Chromium | Users will notice but Ghostty's rendering is high quality |

---

## Revised Phase Ordering

Based on architect's analysis, reorder to front-load risk validation:

```
Phase 0: Setup (1-2 weeks)
  - Cargo workspace skeleton
  - Verify GPUI builds, verify gpui-ghostty renders a PTY
  - Pin git deps to working commits, fork repos
  - GATE: If gpui-ghostty can't render a basic terminal → evaluate gpui-terminal or pivot

Phase 1a: Core types + database (1-2 weeks)     ← parallel
Phase 1b: PTY manager basics (1-2 weeks)         ← parallel

Phase 2: Minimal GPUI + single terminal (2-4 weeks) ← HIGHEST RISK
  - Single terminal window with input/output
  - Test: escape sequences, mouse, selection/copy, scrollback, rapid output
  - Test: 6+ terminal instances in one window (multi-terminal gut check)
  - GATE: If multi-terminal fails or frame drops → pivot to static tiles or framework

Phase 3: State entities + DB wiring (2-3 weeks)
  - Per-instance entities (Entity<Instance>)
  - Entity<InstanceList> for collection
  - Terminal output batching
  - Session persistence

Phase 4: Overview mode (3-4 weeks)
Phase 5: Focus mode (3-4 weeks)
Phase 6: Navigation + modals (2-3 weeks)
Phase 7: Polish + parity (3-4 weeks)
Phase 8: Testing + packaging (2-3 weeks)
```

**Total estimate: 5-8 months** (realistic, includes buffer for GPUI friction)

---

## Explicit Pivot Criteria

Define these BEFORE starting. If any trigger fires, evaluate alternatives:

| Trigger | Gate Phase | Action |
|---------|-----------|--------|
| gpui-ghostty can't render >3 terminals/window without frame drops | Phase 2 | Try static text tiles; if still bad → evaluate gpui-terminal |
| GPUI git dep breaks twice in one month, >2 days each to fix | Any | Mirror + freeze deps; if persists → evaluate Iced |
| Phase 2 (single terminal) exceeds 4 weeks | Phase 2 | Evaluate gpui-terminal (no Zig) or Tauri as escape hatch |
| Zig build dep blocks a contributor for >1 week | Any | Switch to gpui-terminal (alacritty-based) |
| GPUI drops macOS Metal support or makes breaking change that takes >1 week to resolve | Any | Evaluate Iced (COSMIC-validated) |

---

## Corrections to the Original Plan

| Item | Original | Corrected |
|------|----------|-----------|
| PTY reader threading | `cx.background_spawn()` | `std::thread::spawn` (blocking I/O) + `flume::channel` |
| State architecture | Single `InstanceManager` model | Per-instance `Entity<Instance>` + `Entity<InstanceList>` |
| Terminal output | Direct entity notify | Batch in buffer, notify on frame boundary |
| Clippy nursery lints | `warn` globally | Drop nursery initially, cherry-pick later |
| IPC channel count | 39 | 42 (38 invoke + 4 events) |
| Phase ordering | Phases 1→2→3→4 linear | Phase 1a‖1b parallel, multi-terminal test in Phase 2 |
| Git dependencies | Direct upstream | Mirror into controlled forks |
| Migration crate | Hand-rolled | Use `rusqlite_migration` v1.0 |
| Colors in core | Unspecified | Keep as hex strings in core, convert to `gpui::Hsla` in UI |
| Terminal history | Unspecified format | Raw `Vec<Vec<u8>>` in PTY crate, parsing in UI layer |

---

## Testing Strategy (Gap-Fill)

| Crate | Approach |
|-------|----------|
| `conescope-core` | `#[test]` with `:memory:` SQLite. High coverage. Straightforward. |
| `conescope-pty` | Real shell spawn/kill/resize tests behind timeout. Accept slow CI. Mock PTY for unit tests. |
| `conescope-ui` | Entity logic tests via `gpui::TestAppContext` (if available). Skip view rendering tests initially. |
| Integration | Smoke test: boot app → open terminal → type command → verify output. Run with `#[ignore]` in CI. |
| Compatibility | `compat-test/` workspace that builds minimal GPUI window + terminal — run before bumping git dep pins. |

---

## Final Assessment

**The plan is good. The architecture is sound. The risks are real but manageable with the mitigations above.** The biggest danger isn't the technical architecture — it's the ecosystem immaturity of GPUI + gpui-ghostty. The gated phase approach with explicit pivot criteria turns this from "betting the project on experimental tech" into "validating experimental tech early and having escape hatches."

Proceed with Phase 0. The first real decision point is Phase 2: can gpui-ghostty handle multiple terminals in one window at acceptable performance? Everything else follows from that answer.
