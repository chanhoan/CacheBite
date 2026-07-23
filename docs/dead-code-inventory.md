# Dead-code inventory

Symbols with no production call site, recorded on 2026-07-22 against the
stabilization pass branch and **decided on the same day**.

Reference counts were measured with `grep -rn` over `src/` and `src-tauri/src/`,
excluding the defining module and its own test file.

Guiding rule for the decisions below: an abstraction that exists to hold a
platform seam open is worth more than the reference count suggests, while code
that has been *superseded* by a different mechanism is just a second way to be
wrong. Git history is the archive — "keep it in case we roll back" is not a
reason to keep a symbol compiled.

| # | Subject | Decision |
| --- | --- | --- |
| 1 | `src-tauri/src/window/mod.rs` platform layer | Keep (one removal candidate) |
| 2 | `src/lib/assets/manifest.ts` bare severity keys | **Removed** |
| 3 | `always_on_top` capability field | Keep |
| 4 | `src/lib/state/engine.ts` `tickResetTimers` | **Removed** |
| 5 | `src/lib/stores/interaction.ts` `setFullscreen` | Keep (blocked on IPC) |

## 1. `src-tauri/src/window/mod.rs` — platform abstraction layer

Thirteen symbols with 0 production references, all exercised only by
`src-tauri/src/window/tests.rs`. `lib.rs` and `refresh/ipc.rs` use just
`clamp_window`, `anchor_panel`, `platform_os`, `command_allowed`, and
`foreground_window_is_fullscreen`.

**Decision: keep.** This is the seam that lets `window/tests.rs` exercise DPI
conversion, clamping (including negative-coordinate displays), and panel
anchoring without a live Tauri runtime — which is precisely what makes the
headless X11/Wayland jobs in `native-smoke.yml` able to verify geometry policy
at all. The DPI helpers encode rounding rules that are expensive to rediscover
and cheap to keep.

**One genuine duplicate:** `set_autostart` is superseded. `refresh/ipc.rs:220`
drives autostart through `tauri_plugin_autostart::ManagerExt` directly and never
calls the helper. It is the only entry here that is redundant rather than
seam-preserving.

> Not removed in this pass: the Rust toolchain could not build in this
> environment (`pkg-config` and `libdbus-1-dev` absent, so `libdbus-sys`'s build
> script panics). Deleting Rust that cannot be compile-checked trades a cosmetic
> win for real risk. Remove it in a pass that can run
> `cargo clippy -- -D warnings && cargo test`.

## 2. `src/lib/assets/manifest.ts` — unreachable pet state keys

`PET_STATES` declared `ok`, `warn`, `critical`, `exhausted` alongside the
`idle_*` forms. `RequestedAnimationKey` (`src/lib/assets/resolver.ts`) only ever
emits `idle`, `idle_warn`, `idle_critical`, `idle_exhausted`, `sleep`,
`dragging`, so the four bare names passed validation and were then never
requested.

**Decision: removed.** A package declaring `warn` rendered nothing and reported
nothing — a silent miss. Validation now rejects the bare names, which turns that
into a clear error at load time, and matches ui-contract §6.

- Bundled `cat` and `corgi` manifests declare only `idle`, `idle_warn`,
  `idle_critical`, `idle_exhausted` — verified unaffected.
- This *is* a breaking manifest change for any third-party package that used the
  old names. That is the intent: such a package was already broken, silently.

## 3. `src-tauri/src/refresh/ipc.rs` — `always_on_top` capability

`get_platform_capabilities` always reports
`Unavailable { reason: "always-on-top support is unverified on this platform build" }`,
and no renderer surface displays it, so the field is transmitted and never read.

**Decision: keep.** Per CLAUDE.md an unverified capability *must* report
`unavailable`, so the value is correct — what is dead is the delivery, not the
policy. Dropping the field is a wire-DTO change on both sides of the boundary
that would be re-added the moment detection lands, and it cannot be
compile-verified here (see the note in §1). The real follow-up is implementing
detection, not trimming the DTO.

## 4. `src/lib/state/engine.ts` — `tickResetTimers`

Referenced only by `src/lib/state/domain.test.ts`. The backend `reset_pending`
flag drives optimistic window resets (`App.svelte` → `markResetPending`), so the
renderer-side timer never ran.

**Decision: removed**, along with the `ProviderState.resetKeys` set it was the
sole consumer of, and its unit test. Two mechanisms for one behaviour is how the
renderer and backend end up disagreeing about whether a window has reset; the
backend is the authority here, as it is for snapshot expiry.

## 5. `src/lib/stores/interaction.ts` — `setFullscreen`

Referenced only by `src/lib/stores/settings.test.ts`. Nothing calls it in
production, so `InteractionState.fullscreen` is permanently `false` and
ui-contract §7.1-5 ("no bubbles while a fullscreen app is in front") does not
take effect.

**Decision: keep — blocked on a missing IPC command, not on the store.**
`get_platform_capabilities` reports only whether *detection* is available
(`refresh/ipc.rs:168-174`: `Available` on Windows, `Unavailable` elsewhere).
There is no command that reports the *current* foreground-fullscreen state, so
the renderer has nothing to subscribe to. Wiring §7.1-5 needs a new native
command plus a push channel; the store action is the already-correct landing
point for it, and deleting it would only have to be rewritten.

Practical impact stays small — the overlay window is hidden during fullscreen
anyway.

> Documentation drift spotted while deciding this: CLAUDE.md states "Fullscreen
> detection is currently reported unavailable", but `ipc.rs:168` reports
> `Available` on Windows. Worth correcting the invariant text.
