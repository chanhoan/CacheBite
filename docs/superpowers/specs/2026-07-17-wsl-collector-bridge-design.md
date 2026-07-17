# CacheBite WSL Collector Bridge Design

## Goal

Allow the native Windows CacheBite application to collect Claude and Codex usage when the tools are installed and authenticated only inside the user's default WSL distribution. Preserve the current native Windows behavior and require no distribution picker or manual path configuration.

## Scope

The first WSL release supports Windows only and always targets the distribution configured as the WSL default. It adds automatic fallback for Claude credentials and the Codex app-server. It does not add distribution selection, simultaneous account aggregation, WSL installation management, login flows, or arbitrary user-supplied commands.

## Selection Policy

Each provider uses a native-first composite collector:

1. Attempt the existing Windows collector.
2. Fall back to WSL only when the native result is `CredentialsMissing` for Claude or `CliMissing` for Codex.
3. Do not mask native network, provider, parse, protocol, timeout, or internal failures by trying a second environment.
4. Cache WSL availability and executable discovery for the application session, but perform provider collection on the existing refresh schedule.

This preserves native behavior for existing users and prevents an unrelated WSL account from replacing a configured native account.

## WSL Boundary

All WSL access goes through a small Windows-only process adapter. The adapter invokes the absolute Windows system `wsl.exe` resolved from the Windows system directory rather than searching the current working directory. Omitting `--distribution` intentionally selects the user's configured default distribution.

The adapter:

- creates child processes without a console window;
- uses fixed executable names and fixed arguments;
- never interpolates settings, paths, provider responses, or other user-controlled text into a shell command;
- bounds stdout and stderr reads;
- applies an operation timeout;
- kills and reaps the child on success, error, timeout, cancellation, and application shutdown;
- maps missing WSL and missing tools separately from network failures;
- emits only redacted diagnostic metadata such as provider, source, phase, and error class.

`wsl.exe --exec` is used directly for Codex. Claude credential discovery requires a fixed, repository-owned POSIX script passed to `sh -c`; the script accepts no positional parameters or interpolated data.

## Claude Data Flow

The existing native Claude collector remains unchanged. When it reports missing credentials, the WSL credential source runs a fixed script in the default distribution that checks, in order:

1. `$CLAUDE_CONFIG_DIR/.credentials.json`, when `CLAUDE_CONFIG_DIR` is non-empty.
2. `$HOME/.claude/.credentials.json`.

The script returns the first regular credential file through stdout and otherwise exits with a dedicated not-found code. CacheBite limits the output to the existing 64 KiB credential limit, parses the bytes in memory with the existing credential schema, and immediately discards the raw bytes after creating the secret value. It never copies the credential file into Windows AppData and never sends a token across Tauri IPC.

The existing Claude HTTP usage collector then performs the provider request from the Windows process. This avoids running a second HTTP implementation in WSL while still using the WSL login.

Outcomes are mapped as follows:

- WSL unavailable or credential file absent: `CredentialsMissing` (`auth_required`).
- Credential file malformed or oversized: parse/credential error, not offline.
- Provider request timeout or network failure: network failure (`offline`).
- Valid response: normal Claude snapshot with the existing `oauth_api` source.

## Codex Data Flow

When native Codex resolution reports that the CLI is missing, the WSL collector launches:

```text
wsl.exe --exec codex -s read-only -a untrusted app-server
```

The existing JSON-RPC session implementation is reused against the child's stdin and stdout. It performs initialization and `account/rateLimits/read`, preserves the existing response-size and ten-second protocol timeout limits, and normalizes the result through the existing Codex parser.

Before selection, a bounded probe uses `wsl.exe --exec sh -c 'command -v codex >/dev/null 2>&1'`. The script is a fixed literal and receives no user data. A missing command maps to `CliMissing` (`not_installed`), not offline. Protocol and provider failures retain their current classifications.

Killing `wsl.exe` alone does not always prove that a Linux descendant exited. The adapter therefore launches Codex through a fixed WSL process wrapper that records the Linux process group and terminates that group on timeout or cancellation before reaping the Windows child. Tests must demonstrate that a deliberately hanging fake app-server is not left running.

## State and Diagnostics

The renderer contract does not gain credentials, paths, distribution names, or raw process errors. Provider status semantics remain:

- Claude credentials absent in both environments: `auth_required`.
- Codex absent in both environments: `unavailable/not_installed`.
- WSL unavailable: the same missing-provider result as native-only operation.
- Network or request timeout after successful provider selection: `offline`.

Debug logging may identify `environment=native` or `environment=wsl` and a stable failure class. It must not log access tokens, credential contents, account identifiers, home paths, WSL distribution names, JSON-RPC bodies, or HTTP bodies.

No settings UI is required. A future distribution selector can extend the adapter with an allowlisted distribution identifier without changing collector behavior.

## Security Constraints

- Resolve `wsl.exe` from the Windows system directory and reject non-file or unexpected executable paths.
- Do not invoke PowerShell, `cmd /c`, or a user-configurable shell command.
- Keep all shell snippets fixed constants with no interpolation.
- Treat all WSL output as untrusted input and apply byte limits before parsing.
- Preserve HTTPS endpoint allowlisting and existing response validation.
- Keep credentials in secret wrappers and memory only; do not persist or expose them.
- Redact paths and process output from UI errors and production logs.
- Do not fall back from a real native authentication or provider error to a different account in WSL.

## Components

- `collectors/wsl.rs`: Windows-only WSL executable resolution, bounded process execution, fixed credential read, availability/tool probe, and lifecycle management.
- `collectors/fallback.rs`: provider-aware native-first fallback policy with no platform process details.
- Claude broker extension: accepts an asynchronous WSL secret source after native locations are exhausted.
- Codex process abstraction: allows the current RPC session to operate on either a native Codex child or the WSL-managed child.
- Startup wiring: constructs the composite collectors only on Windows production builds; fixtures and non-Windows builds remain unchanged.

## Testing

Tests are written before implementation and use fake executables or an injected process boundary; CI does not require a real WSL installation or real credentials.

Required unit coverage:

- native success never invokes WSL;
- only Claude credential-missing and Codex CLI-missing outcomes trigger fallback;
- native network, provider, parse, timeout, and internal failures do not trigger fallback;
- fixed WSL commands contain no user-controlled arguments;
- missing WSL, missing credentials, missing Codex, malformed credentials, oversized output, timeout, and cancellation map correctly;
- secrets and paths are absent from diagnostics;
- Codex JSON-RPC works over the WSL child transport;
- hanging children and descendants are terminated and reaped.

Windows integration coverage uses a fake `wsl.exe` fixture to simulate the default distribution, Claude credential output, and Codex app-server protocol. A manual smoke test on a Windows host with tools installed only in the default WSL distribution verifies both providers become active and that no console window remains open.

## Acceptance Criteria

- A native Windows CacheBite build automatically uses the default WSL distribution when the corresponding native provider is missing.
- WSL-only Claude authentication produces usage without copying or exposing credentials.
- WSL-only Codex produces usage through app-server RPC.
- Missing WSL or tools yields `auth_required`/`not_installed`, never a false offline state.
- Native configured providers retain precedence.
- No command injection surface, unbounded output, leaked secret, visible console, or orphaned child remains.
- Existing native, fixture, frontend, and provider state tests continue to pass.
