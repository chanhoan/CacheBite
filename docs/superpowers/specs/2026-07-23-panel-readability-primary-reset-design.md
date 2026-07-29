# Panel Readability, Primary Action, and Reset Countdown Design

## Goal

Bring three usage-panel behaviors in line with the approved UI:

1. Keep freshness copy on one line and stop exposing collector implementation
   names.
2. Make `Set as primary` the only control that changes the primary provider.
3. Display both reset timestamps as relative countdowns, including days.

## Display Contract

The freshness line contains only the state and capture age:

- `Fresh · captured just now`
- `Stale · captured 24 min ago`

Collector source values such as `oauth_api` and `cli_rpc`, and the cached marker,
are not rendered. The line uses the UI-plan's compact mono treatment and does
not wrap.

Both usage gauges use `resets in <countdown>`. The countdown uses the largest
relevant units while retaining hours and minutes:

- `6d 22h 10m`
- `1d 0h 5m`
- `3h 20m`
- `23m`
- `now`

Invalid reset timestamps continue to omit the reset line.

## Primary Provider Interaction

Selecting a provider tab changes only the panel's selected provider. It does not
persist settings or change the primary provider.

When the selected provider is not primary, `Set as primary` is enabled. Clicking
it uses the existing serialized settings update path. On success, the selected
provider receives the primary marker and the button becomes disabled. Existing
settings rollback and user-visible failure handling remain responsible for a
rejected save.

## Implementation Boundaries

- `UsagePanel` owns freshness copy and compact one-line styling.
- `UsageGauge` renders one relative reset format for both windows.
- The time formatter owns day/hour/minute countdown calculation.
- `App` keeps tab selection local to the provider store and changes settings
  only from the explicit primary action.

No backend DTO, persistence schema, or collector behavior changes.

## Verification

- Formatter unit tests cover day boundaries, hours, minutes, expired values,
  and invalid input.
- Gauge tests require relative weekly countdown copy.
- Panel component tests require source-free, single-line freshness copy and the
  primary callback.
- App tests prove tab selection does not persist primary and the button does.
- Renderer E2E checks the source-free freshness line remains one visual line and
  exercises `Set as primary`.
- Full TypeScript CI and native tests remain green.
