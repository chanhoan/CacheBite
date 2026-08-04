# Repository Guidelines

## Project Structure & Module Organization

CacheBite is a Tauri 2 desktop app. The Svelte 5/TypeScript renderer is in `src/`, with reusable APIs, state, interactions, components, and styles under `src/lib/`. Rust collectors, storage, refresh, IPC authorization, and window policy live in `src-tauri/src/`. Bundled pets are in `src-tauri/resources/pets/`; product docs and artwork are in `docs/`. Unit tests are colocated with source, WebdriverIO tests are in `tests/e2e/`, and fixtures are in `tests/fixtures/`. Do not commit `dist/`, `coverage/`, or `src-tauri/target/`.

## Build, Test, and Development Commands

Use the pinned pnpm version from `package.json`.

- `pnpm install --frozen-lockfile` installs pinned dependencies.
- `pnpm dev` runs the renderer in Vite; `pnpm tauri dev` runs the desktop app.
- `pnpm check`, `pnpm lint`, and `pnpm test` run type checks, formatting/lint rules, and Vitest.
- `pnpm test:coverage` enforces the 80% coverage gates.
- `pnpm test:e2e:renderer` runs browser smoke tests. Native `pnpm test:e2e` requires a `webdriver`-feature Tauri build and the documented fixture environment.
- `pnpm test:ci` runs type checks, linting, formatting, coverage, and the renderer build.
- `cargo test --manifest-path src-tauri/Cargo.toml --all-features` runs native tests.
- `pnpm audit:ci` checks high-severity dependency advisories.

## Coding Style & Naming Conventions

Prettier uses single quotes and trailing commas. Use two-space indentation in TypeScript, Svelte, JSON, and config files, and standard `rustfmt` output for Rust. Name Svelte components `PascalCase`, TypeScript identifiers `camelCase`, and Rust modules/files `snake_case`. Prefer focused modules, immutable updates, explicit errors, and typed boundary validation.

## Testing Guidelines

Vitest uses `*.test.ts`, WebdriverIO uses `*.spec.ts`, Python uses `*_test.py`, and Rust uses colocated `tests.rs` or `*_test.rs`. Test policy/parsing changes, IPC authorization boundaries, and critical window flows. Keep all coverage metrics at 80% or higher and fixtures credential-free.

## Commit & Pull Request Guidelines

Use Conventional Commits: `feat:`, `fix:`, `test:`, `docs:`, `ci:`, or `chore:` plus an imperative summary. Feature, fix, and docs PRs target `develop`; releases target `main`. Explain what and why, link issues or plans, list validation, and attach screenshots for UI changes. CI, native smoke, Rust checks, audits, and secret safeguards must pass.

## Security & Configuration

Keep credentials, raw provider responses, account identifiers, and local paths out of renderer DTOs, logs, fixtures, and screenshots. Preserve the per-window capability split in `src-tauri/capabilities/` and the command authorization boundary in `src-tauri/src/window/`. Never hardcode tokens; use environment variables for local-only configuration.
