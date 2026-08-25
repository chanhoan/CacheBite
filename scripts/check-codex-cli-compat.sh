#!/usr/bin/env bash
# Verifies that the installed Codex CLI still accepts CacheBite's fixed argument
# list (collectors::codex::app_server_argv).
#
# The check runs the real invocation with stdin already at EOF: app-server starts,
# reads no request, and exits 0. It is credential-free because authentication is
# only consulted once a JSON-RPC request arrives, and none ever does.
#
# `--help` deliberately is NOT used: clap short-circuits on it and prints usage
# without validating option VALUES, so `-a untrusted app-server --help` still
# exits 0 on a CLI that rejects `-a untrusted` for real.
#
# Only exit code 2 is treated as a failure — that is clap's usage error, the exact
# way codex-cli 0.149.0 dropping `--ask-for-approval untrusted` surfaced. Any
# other nonzero status is reported but not failed, so an unrelated runtime problem
# cannot raise a false alarm about the argument surface.
set -euo pipefail

if ! command -v codex >/dev/null 2>&1; then
  echo "codex not found on PATH; nothing to check." >&2
  exit 0
fi

# A fixed name under /tmp would be a symlink-following hazard on a shared box.
err="$(mktemp "${TMPDIR:-/tmp}/codex-compat.XXXXXX")"
trap 'rm -f "$err"' EXIT

version="$(codex --version 2>&1 || echo unknown)"

# This check assumes app-server terminates at stdin EOF — an upstream behaviour,
# and upstream behaviour changing is the whole reason this guard exists. Bound it
# so the guard can never become the thing that hangs. `timeout` is coreutils and
# is absent from a stock macOS, so run unguarded there rather than failing.
runner=""
if command -v timeout >/dev/null 2>&1; then
  runner="timeout 30"
fi

status=0
# shellcheck disable=SC2086  # $runner is a deliberate word-split prefix or empty.
$runner codex -s read-only -a never app-server </dev/null >/dev/null 2>"$err" || status=$?

if [ "$status" -eq 2 ]; then
  echo "Codex CLI rejected CacheBite's fixed arguments (installed: ${version})." >&2
  echo "--- CLI stderr ---" >&2
  head -20 "$err" >&2
  echo "------------------" >&2
  echo "Update collectors::codex::app_server_argv and both WSL launch scripts." >&2
  exit 1
fi

if [ "$status" -eq 124 ]; then
  echo "Inconclusive: codex did not exit at stdin EOF within 30s (installed: ${version})." >&2
  echo "The check's termination assumption may no longer hold; revisit this script." >&2
  exit 0
fi

if [ "$status" -ne 0 ]; then
  # Note: this prints codex's own stderr, which can name local paths.
  echo "Inconclusive: codex exited ${status} for a reason other than argument parsing (installed: ${version})." >&2
  head -20 "$err" >&2
  exit 0
fi

echo "Codex CLI argument surface OK: ${version}"
