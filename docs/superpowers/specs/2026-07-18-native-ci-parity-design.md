# Native CI Parity Design

## Goal

Make PR native E2E exercise one WebDriver architecture across Windows, macOS, Linux X11, and Linux Wayland, while running credential-free production composition on both Linux and macOS. Restore the Rust advisory gate without weakening dependency checks.

## Architecture

All native smoke binaries are test-only builds with the `webdriver` Cargo feature and use the embedded WebDriver provider. This removes the external `tauri-driver`, EdgeDriver, and WebKitWebDriver integration boundary that currently differs by operating system. Display setup remains platform-specific: Xvfb for Linux X11 and production, Weston for Linux Wayland, and the native window server on Windows and macOS.

Fixture composition remains a Windows/macOS matrix plus Linux X11/Wayland jobs. Production composition becomes an Ubuntu/macOS matrix that executes the same `tests/e2e/native.spec.ts`; only the Linux launch is wrapped with Xvfb.

The Rust advisory gate continues to fail on genuine advisories, but pins `cargo-audit 0.22.2`, which parses CVSS 4.0 advisory records, instead of the incompatible `0.21.2`. It audits the repository's actual lockfile at `src-tauri/Cargo.lock`.

## Invariants

- Production release builds must not enable the embedded WebDriver feature.
- Every native smoke build must enable `--features webdriver`.
- `wdio.conf.ts` must select `embedded` on every operating system.
- Credential-free production composition must run on `ubuntu-latest` and `macos-latest` with identical collector expectations.
- Linux X11 and Wayland remain separate required checks.
- The advisory gate must not ignore vulnerabilities or advisory parse failures.
- macOS production E2E must isolate app data with a step-local `HOME`; Linux uses step-local `HOME` and `XDG_DATA_HOME`.

## Verification

Static workflow contract tests lock the provider, feature, and production OS matrix. Then run the frontend unit suite, renderer checks, Rust formatting, clippy, and Rust tests. Review the final diff for test-only WebDriver exposure and secrets before committing.
