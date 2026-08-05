<p align="center">
  <img src="docs/assets/logo.png" alt="CacheBite logo" width="160" />
</p>

<h1 align="center">CacheBite</h1>

<p align="center">
  A small desktop pet for Claude and Codex usage, with local-only credentials and a beta release cadence.
</p>

<p align="center">
  <a href="README.md">한국어</a> · English
</p>

<p align="center">
  <img alt="Beta" src="https://img.shields.io/badge/status-beta-orange" />
  <img alt="Local only" src="https://img.shields.io/badge/privacy-local--only-success" />
  <img alt="Windows" src="https://img.shields.io/badge/platform-Windows-blue" />
  <img alt="macOS" src="https://img.shields.io/badge/platform-macOS-lightgrey" />
  <img alt="Linux" src="https://img.shields.io/badge/platform-Linux-yellow" />
  <img alt="Tauri 2" src="https://img.shields.io/badge/runtime-Tauri%202-24C8DB" />
  <img alt="Docs language" src="https://img.shields.io/badge/docs-English%20%2B%20Korean-111827" />
</p>

<p align="center">
  <a href="#quick-start">Quick Start</a> ·
  <a href="#using-cachebite">Using CacheBite</a> ·
  <a href="#how-it-works">How It Works</a> ·
  <a href="#features">Features</a> ·
  <a href="#updates">Updates</a> ·
  <a href="#privacy">Privacy</a> ·
  <a href="#beta-status-and-limits">Beta Status</a> ·
  <a href="#release-engineering">Release Engineering</a>
</p>

<p align="center">
  <img
    src="docs/assets/screenshots/hero.png"
    alt="CacheBite desktop pet overlay beside the opened usage panel"
    width="960"
  />
</p>

---

## Quick Start

> **Unsigned beta warning:** CacheBite builds are not code-signed yet. Windows will show SmartScreen warnings, macOS Gatekeeper will block the unsigned DMG until you clear quarantine, and Linux uses an AppImage that needs WebKitGTK 4.1 on the host.

Download the latest beta from [GitHub Releases](https://github.com/chanhoan/CacheBite/releases).

| Platform | Artifact                          | Note                                                                      |
| -------- | --------------------------------- | ------------------------------------------------------------------------- |
| Windows  | MSI and NSIS installers           | Install over the previous beta when you upgrade.                          |
| macOS    | Unsigned universal validation DMG | One file for Apple Silicon and Intel, validation-only, not notarized yet. |
| Linux    | AppImage                          | Requires WebKitGTK 4.1 and related desktop prerequisites.                 |

Verify downloads against `SHA256SUMS.txt` before installing. Platform-specific install steps and beta reporting rules live in [docs/beta-testing.md](docs/beta-testing.md).

Downloading by hand is normally a one-time step. From `v0.1.0-beta.4` onward CacheBite offers newer releases inside the app — see [Updates](#updates). Builds up to and including `v0.1.0-beta.3` predate the updater and cannot update themselves, so install a newer build by hand once.

### Before first launch

- Claude needs Claude Code signed in on this machine.
- Codex needs the `codex` CLI on your `PATH` and signed in.
- One provider alone is fine. If only one provider is signed in, CacheBite will still show the other as unavailable instead of pretending it has data.

---

## Using CacheBite

1. Install and launch CacheBite.
2. A small pet appears on your desktop. Drag it anywhere. The position is saved per display.
3. Double-click the pet, or press Enter while it has focus, to show or hide the usage panel. A single click does not open it.
4. Right-click the pet for a menu with `Show usage panel` or `Hide usage panel`, `Hide pet`, and `Quit CacheBite`.
5. The panel shows Claude and Codex as tabs, not side by side. Each tab shows a session arc and a weekly arc. For the currently supported providers, the session arc is surfaced as a 5-hour window.
6. Switch tabs to inspect the other provider.
7. Click `Refresh now` to fetch fresh usage, or `Set as primary` to choose the provider whose usage drives the pet ring and status.
8. Open `Settings` to change appearance, primary provider, pet, bubbles, notifications, secondary provider notifications, and start at login.

Use `Ctrl+Shift+H` on Windows and Linux, or `Cmd+Shift+H` on macOS, to hide or show the pet globally. Hiding the pet also closes the usage panel. The shortcut is fixed rather than user-configurable; if another application already owns it, CacheBite reports the conflict in Settings and keeps running.

Three pets ship with the app — Tabby, Corgi, and Momo. The pet you pick is independent from the primary provider. Changing one does not change the other.

The ring's two arcs represent the session window and the weekly window. The pet's mood follows the primary provider's usage, so the desktop state tracks the provider you care about most.

UI states shown in the app:

- `Fresh` means the snapshot is current.
- `Stale` means the data is still shown, but it is past the fresh window.
- If a provider is signed out or unavailable, CacheBite shows that state instead of pretending it has usage data.
- A dot beside `Settings` in the panel footer means a newer release is waiting.
- On Windows, CacheBite hides the pet and panel while another application is fullscreen. Leaving fullscreen restores the pet unless you explicitly hid it with the global shortcut. Other platforms report fullscreen detection as unavailable instead of pretending it works.

---

## How It Works

1. CacheBite collects provider usage locally on your machine.
2. Rust normalizes the provider-specific responses into a shared usage model.
3. The Svelte renderer displays the pet, ring, panel, and settings from that normalized state.
4. Speech bubbles and notifications are driven from local usage transitions, not from a remote CacheBite service.

---

## Features

| Area                 | What CacheBite does                                                                                 |
| -------------------- | --------------------------------------------------------------------------------------------------- |
| Desktop pet          | Keeps a movable pet on screen and remembers its position per display.                               |
| Usage view           | Toggles a Claude and Codex tabbed panel with session and weekly windows.                            |
| Ring state           | Uses the primary provider to drive the pet's mood and ring state.                                   |
| Pet menu             | Opens a right-click menu on the pet to toggle the panel, hide the pet, or quit.                     |
| Global visibility    | Hides or shows the pet with `Ctrl/Cmd+Shift+H`; hiding it also closes the panel.                    |
| Presentation privacy | Hides the pet and panel when a foreground application is fullscreen on Windows.                     |
| In-app updates       | Checks GitHub Releases for a newer signed build and installs it from Settings when you ask it to.   |
| Settings             | Lets you switch appearance, provider focus, pet choice, bubbles, notifications, and start at login. |

---

## Updates

CacheBite reads the update manifest published on GitHub Releases and offers a newer signed build inside the app.

- Checks are automatic: roughly 30 seconds after launch, once an hour in the background, and when the usage panel is revealed, at most once every 15 minutes. There is nothing to poll by hand.
- When a newer build is published, a dot appears beside `Settings` in the panel footer, and `Settings` shows `Update available` with an `Install and restart` button.
- Installing downloads the artifact for your platform, verifies its signature, installs it over the current build, and restarts CacheBite. Settings, history, and installed pets are kept.
- If any step fails, CacheBite says so in Settings and the build you are running is left untouched. You can always install the release by hand instead.
- The channel follows the build you are running. A pre-release build such as `0.1.0-beta.5` sees beta releases; a release build only ever sees stable ones. The channel is derived from the running version rather than stored, so there is no beta opt-in to get stuck in.
- In-app installation is verified on Windows. macOS and Linux artifacts are published and signed and CacheBite will attempt the install, but replacing an unsigned macOS bundle and rewriting a running AppImage are not confirmed on real hardware yet.

The renderer never talks to the updater directly. Checking, downloading, signature verification, and installation all live in the Rust core, and the panel only receives the resulting status.

---

## Privacy

> CacheBite keeps provider credentials local. Nothing is sent through a CacheBite server, and the renderer only receives normalized state.

- There is no CacheBite cloud service.
- Credential access, collection, refresh scheduling, and persistence live in the Rust core.
- The Svelte renderer only receives normalized state.
- CacheBite does not read browser cookies.
- Provider raw responses, account identifiers, and credential paths do not belong in logs or screenshots.
- Credential files are handled read-only.

---

## Beta Status and Limits

- The beta is still unsigned. Update artifacts carry an updater signature, which is not the same thing as platform code signing.
- Release publishing still needs a manual step.
- In-app updates are verified on Windows. macOS and Linux receive signed artifacts, but the install path there is unverified.
- Builds up to and including `v0.1.0-beta.3` cannot update themselves and have to be replaced by hand once.
- Fullscreen detection currently works on Windows only. macOS and Linux expose the limitation in Settings.
- The global hide/show shortcut is fixed. If the operating system or another application owns it, CacheBite reports it as unavailable.
- When only one provider is signed in, the other provider remains unavailable.

---

## Release Engineering

- Tag pushes create a draft GitHub release with the platform installers, their checksums, and the signed updater artifacts attached.
- Publishing the release stays a manual step. A draft advertises nothing to the updater.
- Publishing regenerates the channel manifests attached to the `updater` release. Every published release refreshes `beta.json`, so a beta tester is still offered the stable build that supersedes their pre-release; only a non-pre-release version refreshes `stable.json`.
- macOS ships a single universal DMG rather than one build per architecture.
- A signed and notarized macOS build is only produced through the protected `production-macos-signing` workflow input.

---

## License

No project license has been selected yet. Until one is added, all rights are reserved.
