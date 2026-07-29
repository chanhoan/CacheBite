# Repository Guidelines

## Project Structure & Module Organization

CacheBite is a Tauri 2 desktop application. The Svelte 5 and TypeScript renderer lives in `src/`; shared API adapters, state, components, and styles are grouped under `src/lib/`. Native collection, persistence, refresh, and window policy are implemented in Rust under `src-tauri/src/`. Bundled pet packages are in `src-tauri/resources/pets/`, while their source artwork is under `docs/UI-plan/`. Browser and native end-to-end tests live in `tests/e2e/`; unit tests are colocated with source files. Generated `dist/`, `coverage/`, and `src-tauri/target/` content should not be committed.

## Build, Test, and Development Commands

Use the pinned pnpm version from `package.json`.

- `pnpm install --frozen-lockfile` installs exact JavaScript dependencies.
- `pnpm dev` runs the renderer in Vite; `pnpm tauri dev` runs the desktop app.
- `pnpm test` runs Vitest unit and component tests.
- `pnpm test:coverage` enforces the configured coverage gates.
- `pnpm test:e2e:renderer` and `pnpm test:e2e` run WebdriverIO flows.
- `pnpm test:ci` runs type checks, linting, formatting, coverage, and the renderer build.
- `cargo test --manifest-path src-tauri/Cargo.toml --all-features` runs native tests.

## Coding Style & Naming Conventions

Prettier uses single quotes and trailing commas; run `pnpm lint` before submitting. Use two-space indentation in TypeScript, Svelte, JSON, and configuration files, and standard `rustfmt` output for Rust. Name Svelte components in `PascalCase`, functions and variables in `camelCase`, and Rust modules/files in `snake_case`. Keep provider credentials and native-only details out of renderer DTOs and logs. Prefer focused modules, explicit error handling, immutable updates, and typed boundary validation.

## Testing Guidelines

Vitest tests use `*.test.ts`; WebdriverIO specs use `*.spec.ts`; Rust tests are colocated in `tests.rs` or `*_test.rs`. Add unit tests for policy and parsing changes, integration coverage for native boundaries, and E2E coverage for critical user flows. Maintain at least 80% coverage and use credential-free fixtures for automated tests.

## Commit & Pull Request Guidelines

Follow Conventional Commits seen in history: `feat:`, `fix:`, `test:`, `docs:`, `ci:`, or `chore:` with a concise imperative summary. Pull requests should explain behavior changes, link relevant issues or plans, list validation commands, and include screenshots for visible UI changes. Ensure CI, Rust checks, dependency audits, and secret safeguards pass before requesting review.
