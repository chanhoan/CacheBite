# Plan: Runtime Refactoring and Performance Hardening

## Summary

Refactor the verified duplicated and high-cost paths without changing IPC commands, persisted schemas, provider protocols, or visible product behavior. The work prioritizes bounded history rendering and I/O, correct animation lifecycle, reusable collector setup, and single-source validation.

## User Story

As a CacheBite desktop-app user, I want usage updates, history, and pet animations to remain responsive after long use or transient storage failures, so the monitor stays dependable.

## Problem to Solution

- Up to 3,000 retained history samples can cause quadratic segmentation and thousands of SVG nodes. Use linear segment generation and bounded, segment-safe visual sampling.
- Every live provider update queues a full history IPC read. Use one in-flight reload and at most one dirty follow-up.
- PetAnimation creates its interval only on mount. Use a reactive effect that owns and cleans timers after prop changes.
- Refresh actor performs full-file synchronous persistence and can replay 3,000 samples as independent rewrites. Use a serialized persistence writer plus repository batch append.
- Claude creates a reqwest client for every poll. Build one secure client per collector.
- Pet-id validation and Codex child configuration are duplicated. Extract only exact shared behavior with behavior-locking tests.

## Metadata

- Complexity: XL
- Source PRD: N/A
- PRD Phase: standalone
- Estimated Files: 21

---

## UX Design

### Before

    event burst -> full history read per event
    3,000 samples -> copied nested arrays plus 3,000 SVG circles
    changed animation -> old mount-time timer may run against new props
    persistence retry -> scheduler waits for sync disk I/O

### After

    event burst -> one active history read and one follow-up maximum
    3,000 samples -> linear transform plus bounded visual points
    changed animation -> old timer cleaned, frame reset, correct timer installed
    persistence retry -> scheduler publishes state while writer batches I/O

### Interaction Changes

| Touchpoint | Before | After | Notes |
|---|---|---|---|
| Usage history | Can slow with full retention | Same graph, bounded visual density | Raw history remains unchanged |
| Pet animation | Frames/image changes can retain stale timer | Correctly changes immediately | No new controls |
| Provider updates | Full history reads can multiply | Reads are coalesced | No API change |
| Storage outage | Scheduler can wait on disk work | State/TTL/reset stay responsive | Existing warning remains |

---

## Mandatory Reading

| Priority | File | Lines | Why |
|---|---|---:|---|
| P0 | src-tauri/src/refresh/actor.rs | 142-417 | Scheduler and current synchronous retry flow |
| P0 | src-tauri/src/store/history.rs | 104-153 | History ordering, pruning, atomic write boundary |
| P0 | src/App.svelte | 126-212, 235-519 | Startup, queues, events, duplicated mappings |
| P0 | src/lib/components/HistoryGraph.svelte | 21-112 | Quadratic segmentation and unbounded SVG |
| P0 | src/lib/components/PetAnimation.svelte | 1-23 | Mount-only timer lifecycle |
| P1 | src-tauri/src/collectors/claude.rs | 11-146 | Collector ownership and secure client |
| P1 | src-tauri/src/collectors/codex.rs | 190-273 | Duplicate app-server child setup |
| P1 | src-tauri/src/store/settings.rs | 117-227 | Settings I/O and pet-id validation |
| P1 | src-tauri/src/store/pets.rs | 48-145 | Package validation and allocation |
| P1 | src/main.ts | 1-20 | Full font-subset imports |
| P2 | src/lib/components/HistoryGraph.test.ts | 1-59 | Renderer accessibility test idiom |
| P2 | src-tauri/src/store/tests.rs | 1-250 | Native repository test idiom |

## External Documentation

No external research needed. This plan uses established internal Svelte 5, Tokio, reqwest, Tauri IPC, and repository patterns. Confirm the installed Svelte 5.38.7 supports the intended reactive effect before replacing onMount.

---

## Unified Discovery Table

| Category | File:Lines | Pattern | Evidence |
|---|---|---|---|
| Naming | src/lib/assets/resolver.ts:61-94 | Named pure helper exports | resolvePetAnimation |
| Error handling | src/App.svelte:235-251 | Expected native failure becomes local status | catch to null |
| Logging | src-tauri/src/refresh/actor.rs:79-87 | Structured warning fields | provider and category |
| Persistence | src-tauri/src/store/mod.rs:56-78 | Temp plus sync plus atomic replace | write_json_atomically |
| State | src-tauri/src/refresh/actor.rs:117-132 | Cloned state via watch | ProviderState |
| Tests | src/lib/components/HistoryGraph.test.ts:31-42 | Test observable DOM and accessibility | render then expect |
| Configuration | package.json:8-17 | check, lint, test:ci are canonical | scripts |
| Dependencies | src-tauri/src/collectors/claude.rs:140-146 | Explicit secure reqwest builder | https_only |

---

## Patterns to Mirror

### NAMING_CONVENTION

SOURCE: src/lib/assets/resolver.ts:61-94

    export function resolvePetAnimation(
      manifest: PetManifest,
      packageRoot: string,
      requested: RequestedAnimationKey,
    ): ResolvedAnimation { /* ... */ }

New renderer transforms are named pure exports and components are thin consumers.

### ERROR_HANDLING

SOURCE: src-tauri/src/refresh/actor.rs:375-384

    if persistence.snapshots.save_provider(record.clone()).is_err() {
        (persistence.diagnostic)(PersistenceDiagnostic { provider, category: PersistenceCategory::Snapshot });
        *pending_snapshot = Some(record);
    }

Persistence failure retains work and emits the existing typed diagnostic. No sensitive paths, credentials, or payloads are logged.

### LOGGING_PATTERN

SOURCE: src-tauri/src/refresh/actor.rs:79-87

    tracing::warn!(
        provider = ?diagnostic.provider,
        category = ?diagnostic.category,
        "persistence write failed; retained for retry"
    );

Keep the structured fields and the diagnostic intent.

### REPOSITORY_PATTERN

SOURCE: src-tauri/src/store/history.rs:121-153

    let mut store = self.load_locked(now)?;
    prune(provider, now);
    write_json_atomically(&self.path, &store)?;

Batch append must take one lock, load once, prune once, and write atomically once.

### TEST_STRUCTURE

SOURCE: src/lib/components/HistoryGraph.test.ts:31-42

    render(HistoryGraph, { props: { samples, window: 'session', onWindowChange } });
    expect(screen.getAllByTestId('history-segment')).toHaveLength(2);

Test DOM/accessibility and pure-helper contracts, not component internals.

---

## Files to Change

| File | Action | Justification |
|---|---|---|
| src/lib/components/historyGraph.ts | CREATE | Pure linear segment and sampling transforms |
| src/lib/components/historyGraph.test.ts | CREATE | Large-input, boundary, extrema, cap tests |
| src/lib/components/HistoryGraph.svelte | UPDATE | Use helper and bounded rendered points |
| src/lib/components/HistoryGraph.test.ts | UPDATE | DOM cap/accessibility coverage |
| src/lib/components/PetAnimation.svelte | UPDATE | Reactive timer lifecycle |
| src/lib/components/PetAnimation.test.ts | CREATE | Fake-timer prop-transition coverage |
| src/lib/state/presentation.ts | CREATE | Shared provider/settings projections |
| src/lib/state/presentation.test.ts | CREATE | Projection contracts |
| src/App.svelte | UPDATE | Coalesced history, projections, overlay-only pet loading |
| src/App.test.ts | UPDATE | Coalescing and no panel pet IPC |
| src/main.ts | UPDATE | Latin-only installed font imports |
| src-tauri/src/domain.rs | UPDATE | Shared crate-visible pet-id predicate |
| src-tauri/src/store/settings.rs | UPDATE | Reuse predicate |
| src-tauri/src/store/pets.rs | UPDATE | Reuse predicate and no temporary path vectors |
| src-tauri/src/store/history.rs | UPDATE | Ordered batch append |
| src-tauri/src/refresh/actor.rs | UPDATE | Serialized bounded persistence writer |
| src-tauri/src/refresh/tests.rs | UPDATE | Writer/backpressure/scheduler tests |
| src-tauri/src/store/tests.rs | UPDATE | Batch/ordering/validation tests |
| src-tauri/src/collectors/claude.rs | UPDATE | Client reuse |
| src-tauri/src/collectors/codex.rs | UPDATE | Shared spawn configuration |
| src-tauri/src/collectors/tests.rs | UPDATE | Security/configuration parity tests |

## NOT Building

- No persisted JSON schema, IPC command, event name, DTO casing, or provider protocol changes.
- No retention-policy reduction, destructive migration, database, telemetry, or new background service.
- No WSL, asset-security, authentication, or position-save behavior change.
- No merge of resolveIdleAnimation and resolvePetAnimation because their idle-state semantics differ unless new tests prove equivalence.
- No speculative position-save optimization; frontend already debounces it at src/lib/api/gateway.ts:183-225.

---

## Step-by-Step Tasks

### Task 1: Test and create linear bounded graph transforms

- ACTION: Add pure historyGraph tests before editing the component.
- IMPLEMENT: Define readonly sample, point, and segment types plus buildGraphSegments(samples, window) and sampleGraphSegments(segments, maxPoints). Build with loops and push, never spread/copy arrays per point. Sample each segment independently and retain first, last, segment boundaries, and per-bucket min/max percentages. Use a 240-point visual cap and stable keys from original index plus timestamp.
- MIRROR: src/lib/assets/resolver.ts:61-94.
- IMPORTS: None in the pure helper.
- GOTCHA: Do not connect across startsNewSegment, mutate raw samples, or key only by timestamp.
- VALIDATE: Empty, one, multi-segment, 3,000 point, cap, order, extrema, and boundary tests.

### Task 2: Apply bounded graph rendering

- ACTION: Replace HistoryGraph lines 22-51 with Task 1 helpers.
- IMPLEMENT: Render sampled segments/circles only; retain tab roles, keyboard behavior, labels, history-segment test id, and empty-state wording. Add SVG desc when sampling is active.
- MIRROR: Existing HistoryGraph markup at lines 71-112.
- IMPORTS: Task 1 helper exports.
- GOTCHA: Existing reset-marker test must still produce two paths.
- VALIDATE: 3,000-input component test proves circles are at most cap and segment/accessibility output is preserved.

### Task 3: Make pet animation timer ownership reactive

- ACTION: Replace the PetAnimation mount-only timer.
- IMPLEMENT: Reactive effect resets frameIndex to zero whenever animation identity/type/sources/duration changes; install an interval only for frames with more than one source and return cleanup. Capture narrowed sources in callback.
- MIRROR: Timer cleanup in src/lib/components/SpeechBubble.svelte:1-14.
- IMPORTS: Remove onMount and use Svelte runes.
- GOTCHA: Frames to image clears before next tick; image to frames starts at zero; one frame creates no interval.
- VALIDATE: Fake timers cover tick, both transition directions, changed duration, and unmount.

### Task 4: Consolidate renderer projections and coalesce history IPC

- ACTION: Extract exact repeated projections and replace all App call sites.
- IMPLEMENT: Add toSettingsStoreState(AppSettings) and a provider presentation mapper that derives UI once and supplies common session, weekly, source, and stale fields. Replace historyQueue with inFlight plus dirty coalescing. Gate startup loadPetPackage to overlay only.
- MIRROR: Notification serialization at App lines 126-140 and attempt checks at 254-307.
- IMPORTS: New presentation helpers and existing model types.
- GOTCHA: Do not coalesce settings saves or notifications. Stale completion must not update after unmount/retry. Preserve revision rejection in providersStore.
- VALIDATE: Deferred getHistory burst test, no panel getPetPackage, overlay reload on pet change, current settings/notification tests.

### Task 5: Narrow app fonts to installed Latin subsets

- ACTION: Change only import paths in src/main.ts.
- IMPLEMENT: Keep Sans 400/500/600/700 and Mono 400/500/600, importing latin-weight CSS paths.
- MIRROR: Current explicit import order in main.ts:1-9.
- IMPORTS: Installed Fontsource Latin CSS paths.
- GOTCHA: Keep system fallbacks for Korean glyphs; do not use wildcard/full-family imports.
- VALIDATE: pnpm build, lower asset bytes/count than baseline of 80 files and 1,045,031 bytes, and manual Korean fallback check.

### Task 6: Share pet-id validation and eliminate path-vector allocation

- ACTION: Add is_valid_pet_id in domain.rs and reuse it from settings/pets.
- IMPLEMENT: Preserve the exact 64-char lowercase/digit/hyphen rule. Callers retain their current InvalidData versus InvalidInput mapping. Replace paths returning Vec of references with direct match or borrowed iterator.
- MIRROR: UsageWindow validation at domain.rs:42-60.
- IMPORTS: crate::domain::is_valid_pet_id.
- GOTCHA: Do not relax traversal or canonicalization security checks.
- VALIDATE: Table test valid/invalid IDs through both call paths; settings migration and pet tests stay green.

### Task 7: Batch persistence outside the refresh actor

- ACTION: Add a private serialized persistence writer in the refresh module.
- IMPLEMENT: Extend HistoryPersistence with append_success_batch. HistoryRepository locks/loads once, skips cached/non-monotonic samples like today, appends ordered valid records, prunes once, and atomically writes once only when changed. Actor sends coalesced latest snapshot and FIFO history batches over bounded channel. Writer runs filesystem work in spawn_blocking; on failure retains work, emits current PersistenceDiagnostic, and retries bounded batches such as 128. On full channel actor retains local dirty work for next scheduling opportunity instead of awaiting I/O.
- MIRROR: Atomic write at store/mod.rs:56-78 and retry retention at actor.rs:369-417.
- IMPORTS: Tokio mpsc and spawn_blocking, existing VecDeque and Arc.
- GOTCHA: Preserve FIFO, 3,000 pending cap, last-good snapshot behavior, cached/non-increasing no-op behavior, and non-blocking shutdown. Never hold async locks while filesystem work runs.
- VALIDATE: Fake persistence tests: one batch for many successes; failure retains order/diagnostic; blocked writer does not block TTL/reset/revision; cap drops oldest; repository output is sorted/pruned/atomic.

### Task 8: Reuse secure Claude client and centralize Codex child setup

- ACTION: Refactor collector setup only after behavior tests exist.
- IMPLEMENT: Store one client configured with exact existing HTTPS-only, 10-second timeout, no-redirect policy in ClaudeCollector and pass Client reference to internal fetch helper. Extract configure_app_server_command and spawn_managed_app_server from duplicate native/PGID Codex paths. Shared setup owns piped stdio, null stderr, kill-on-drop, Unix process group, Windows no-window, spawn error mapping, and ManagedChild.
- MIRROR: claude.rs:140-146 and cleanup ordering at codex.rs:216-228 and 265-273.
- IMPORTS: Existing reqwest Client and Tokio process types.
- GOTCHA: PGID cleanup runs before error propagation and termination remains guaranteed. Do not change WSL scripts.
- VALIDATE: Existing collector suite plus repeated Claude/fetch seam, secure-client constraints, and both Codex routes for identical setup/cleanup.

---

## Testing Strategy

| Test | Input | Expected Output | Edge Case? |
|---|---|---|---|
| Graph build/sample | 3,000 points with resets | Linear ordered segments; 240 visual max; extrema/boundaries retained | Yes |
| Animation effect | Frames/image changes, fake time | No stale timer, frame reset, cleanup | Yes |
| History coalescer | Burst during deferred read | One active plus one follow-up max | Yes |
| Presentation mapping | Unknown/reset/no snapshot | Current null/severity/source behavior | Yes |
| Batch repository | Cached/duplicate/ordered samples | One write and correct pruned result | Yes |
| Writer | Slow/failing fake persistence | Scheduler progresses and FIFO retry retained | Yes |
| Collectors | Repeated Claude and two Codex flows | Secure reuse and setup parity | Yes |

### Edge Cases Checklist

- [ ] Empty, one, and 3,000-point history
- [ ] Reset segments, duplicate timestamps, and extrema sampling
- [ ] Frames/image/one-frame transitions
- [ ] IPC rejection/unmount/retry while history reads
- [ ] Writer failure, channel full, and shutdown
- [ ] Cached/non-monotonic snapshots
- [ ] Credentials, Codex exit 127, and WSL cleanup regressions
- [ ] Korean font fallback

---

## Validation Commands

### Static Analysis

    corepack pnpm check
    corepack pnpm lint
    cargo fmt --check --manifest-path src-tauri/Cargo.toml

EXPECT: Zero errors.

### Unit Tests

    corepack pnpm test -- src/lib/components/HistoryGraph.test.ts src/lib/components/PetAnimation.test.ts src/App.test.ts
    cargo test --manifest-path src-tauri/Cargo.toml refresh:: store:: collectors::

EXPECT: All affected tests pass.

### Full Test Suite

    corepack pnpm test:ci
    cargo test --manifest-path src-tauri/Cargo.toml

EXPECT: No regression and coverage remains at least 80%.

### Build and Bundle Validation

    corepack pnpm build
    find dist/assets -type f -printf '%s\n' | awk '{sum += $1} END {print sum}'
    corepack pnpm tauri build --debug --bundles msi

EXPECT: Build succeeds, renderer assets improve over 80 files and 1,045,031 bytes, and Debug MSI builds.

### Manual Validation

- [ ] Change provider/mood repeatedly; no timer exception/freeze.
- [ ] Large history fixture remains responsive and reset gaps remain.
- [ ] Rapid provider events do not make history IPC one-for-one.
- [ ] Persistence failure does not block UI state/recovery.
- [ ] Claude, Codex/WSL, settings, and autostart still behave.

---

## Acceptance Criteria

- [ ] No quadratic graph segmentation, bounded graph DOM, preserved reset/extrema semantics.
- [ ] Animation timers follow prop changes and clean up.
- [ ] History reads coalesce; panel skips pet-package IPC.
- [ ] Font assets are smaller with Korean fallback intact.
- [ ] History batch persistence does not stall scheduler transitions.
- [ ] Claude transport safeguards remain; Codex setup has one implementation.
- [ ] Pet-id validation has one source with caller error contracts preserved.
- [ ] Checks, tests, format, and Debug MSI build pass.

## Completion Checklist

- [ ] Tests precede each behavior change.
- [ ] Existing diagnostics/security validation are preserved.
- [ ] No IPC or persisted-schema change.
- [ ] No sensitive data in new logs.
- [ ] No work beyond listed scope.
- [ ] Plan is self-contained.

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Sampler hides spike | Medium | Medium | Keep min/max plus first/last/boundaries |
| Writer loses work at shutdown | Medium | High | Channel ownership/drop tests and retained local dirty state |
| Async writer changes order | Medium | High | One serialized writer and monotonic repository checks |
| Client reuse weakens transport | Low | High | Reuse exact existing builder and test constraints |
| Shared validation alters errors | Low | Medium | Caller-specific mapping plus table tests |
| Latin imports harm Korean text | Low | Medium | System fallback/manual verification |

## Notes

- Audit baseline: Debug renderer output contains 80 assets totaling 1,045,031 bytes; full Fontsource imports are at src/main.ts:1-7.
- Implement Tasks 1-4 tests first, then persistence isolation. Do not convert Task 7 into a broad store rewrite: atomic write and quarantine are reliability boundaries.

