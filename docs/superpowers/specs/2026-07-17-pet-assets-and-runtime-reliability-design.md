# Pet Assets and Runtime Reliability Design

## Goal

Make every Pet asset path portable and predictable, then prevent malformed Pet URLs or packages from trapping CacheBite in its startup screen.

## Naming Contract

- All Pet image filenames use lowercase ASCII snake case: `{pet}_{state}_{frame}.png`.
- Pet identifiers are `cat` and `corgi`; documentation `dog` paths become `corgi`.
- States are `idle`, `warn`, `critical`, and `exhausted`; the misspelling `exhusted` is removed.
- Frame numbers are two digits from `01` through `04`.
- Runtime manifests and every HTML/JavaScript/document reference move with the files.

## Runtime URL Contract

Tauri emits `asset://localhost/...` on some platforms and `http://asset.localhost/...` on Windows. The renderer accepts only those two exact origins, rejects credentials/query/hash/traversal, and resolves manifest-relative frame paths beneath the validated package root.

## Failure Handling

Pet package validation and animation resolution must not throw from Svelte derived rendering. A package error produces the existing `Pet package unavailable` diagnostic while the application reaches `ready`. Optional native listener registration remains outside the readiness gate. Diagnostic builds retain stage logging and automatically open DevTools.

## Verification

- Unit tests cover both valid Tauri asset origins and hostile origins.
- A composition test covers invalid package roots without an indefinite startup state.
- A repository scan rejects Pet filenames containing whitespace, non-ASCII characters, parentheses, or inconsistent state/Pet names.
- Frontend tests, Svelte checks, lint, Rust tests, Release MSI, and Debug MSI must pass.
