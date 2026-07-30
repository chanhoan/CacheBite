# Beta testing guide

CacheBite is a desktop pet that shows how much of your Claude and Codex
subscription you have used. This guide covers installing a beta build, what the
first beta is looking for, and what must never go into a report.

## Before you install

**Nothing is code-signed yet.** Every platform will warn you, and macOS will
refuse outright until you clear the quarantine flag. That is expected at this
stage, not a sign the download is broken — but it also means you should verify
what you downloaded.

```bash
# from the folder holding the downloaded file and SHA256SUMS.txt
sha256sum --ignore-missing --check SHA256SUMS.txt      # Linux
shasum -a 256 --ignore-missing --check SHA256SUMS.txt  # macOS
```

```powershell
# Windows
(Get-FileHash .\CacheBite_0.1.0_x64_en-US.msi -Algorithm SHA256).Hash.ToLower()
# compare against the matching line in SHA256SUMS.txt
```

**There is no auto-updater.** Moving to a newer beta means installing it over
the old build.

## What CacheBite needs to show data

CacheBite reads usage through tools you have already signed in. It never asks
for a password, and it does not read browser cookies.

- **Claude** — Claude Code signed in on this machine.
- **Codex** — the `codex` CLI on your `PATH` and signed in.

A provider you are not signed into reports as unavailable. That is correct
behavior, not a bug. A provider you *are* signed into reporting unavailable is
worth reporting.

## Install

### Windows

Two installers are attached; either is fine.

- **MSI** — `CacheBite_<version>_x64_en-US.msi`
- **NSIS** — `CacheBite_<version>_x64-setup.exe`

SmartScreen will show "Windows protected your PC". Choose **More info → Run
anyway**.

Uninstall from **Settings → Apps → Installed apps → CacheBite**.

### macOS

The DMG is unsigned and un-notarized, so Gatekeeper blocks it by default. After
dragging CacheBite to Applications:

```bash
xattr -dr com.apple.quarantine /Applications/CacheBite.app
```

Then open it normally. Right-click → Open also works on some macOS versions.

Uninstall by moving the app to the Trash.

### Linux

The AppImage needs WebKitGTK 4.1 present on the host:

```bash
chmod +x CacheBite_<version>_amd64.AppImage
./CacheBite_<version>_amd64.AppImage
```

On Debian/Ubuntu the runtime dependency is `libwebkit2gtk-4.1-0`. CacheBite
ships no tray icon — the pet and its panel are the whole surface.

Uninstall by deleting the AppImage.

## First run

- A small pet appears on screen. Drag it anywhere — the position is saved per
  display.
- **Double-click** the pet to open the usage panel.
- The panel has a **Settings** view: primary provider (which provider drives
  the ring), pet selection, bubbles, notifications, start at login.
- The pet you choose is independent of the primary provider. Changing one must
  never change the other.
- The panel stays on top until you close it. Click the **×** at its top-right to
  hide it — CacheBite keeps running. Double-click the pet to bring the panel back
  to the front.
- **Quit** in the panel footer exits CacheBite. That is the supported way to stop
  it; you should never need Task Manager.

## What this beta is looking for

1. **Drag behavior after a drop.** Drag the pet, release it, then move the
   mouse. The pet must return to its usage animation. If it stays stuck in a
   drag pose, or never shows a bubble again after a drag, that is the bug this
   beta most needs confirmed. Windows and Linux X11 are the important cases.
2. **Multi-monitor and DPI.** Mixed-scaling setups, displays arranged to the
   left of or above the primary one, and unplugging a display while the pet
   sits on it.
3. **Provider states.** Whether the usage numbers match what the provider
   itself reports, and how failures are presented when a provider is signed
   out, offline, or rate limited.
4. **Panel placement.** The panel should stay inside the work area of the
   display holding the pet, flipping rather than overflowing near an edge.

## Reporting

Open an issue with the **Beta report** template. Include your OS and version,
the build you installed, which provider is primary, and the steps that led to
what you saw. Screenshots of the pet or panel are welcome.

### Never include

CacheBite is built so that credentials never reach the screen or the logs, and
a report must not undo that.

- Credential files, API keys, OAuth tokens, or session cookies — in any form,
  including screenshots of a terminal showing them.
- Raw provider API responses.
- Account identifiers or email addresses belonging to your provider accounts.

If a report seems to need any of those to be understood, say so in the issue
and we will find another way to reproduce it.

## Known limits at this stage

- No code signing on any platform.
- No auto-update.
- Fullscreen detection reports unavailable, so the pet does not yet hide during
  presentations.
- macOS is validation-only until the signing and notarization environment is
  configured.
