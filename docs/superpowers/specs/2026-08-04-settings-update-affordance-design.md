# Settings Update Affordance Design

## Goal

Move update installation out of the main usage panel and into Settings. Keep
the main panel quiet while still making an available release discoverable.

## Main panel

- Remove the update notice rendered above `UsagePanel`.
- When and only when the update state is `available`, place a small red status
  dot at the upper-right of the footer `Settings` label.
- The dot is decorative. The existing `Settings` button remains the single
  accessible control and its accessible name must communicate that an update
  is available.
- Do not show a `Later` action. Leaving Settings is sufficient dismissal of the
  surface; it does not dismiss or mutate the native update state.

## Settings screen

- When and only when the update state is `available`, insert an update row
  immediately below the `Settings` heading and above `Appearance`.
- The row contains exactly:
  - `Update available — {version}`
  - `Install and restart`
- The update text uses the same typography, color, spacing, and alignment as
  the other Settings rows. It is not a banner, card, alert-colored block, or
  enlarged headline.
- The install button reuses the filled shape and theme-driven colors of the
  existing `Refresh now` primary action. Its font size and dimensions may be
  reduced to fit the denser Settings layout without wrapping at the 312 px
  panel width.
- Use existing semantic theme tokens rather than literal light/dark colors so
  the control keeps the `Refresh now` contrast in system, light, and dark
  themes.
- Activating the button calls the existing install command once. Disable it
  while an update operation is busy to prevent duplicate requests.
- Preserve the existing Version and manual `Check for updates` rows below the
  other Settings controls.

## State and component boundaries

- `App.svelte` continues to own native update state and update actions.
- `UsagePanel` receives a boolean indicating whether an update is available
  and renders only the Settings notification dot.
- `SettingsPanel` receives the available version and install callback needed
  for the new row. It does not invoke native APIs directly.
- Remove the main-panel `UpdateNotice` composition and the session dismissal
  state that existed only for `Later`. The update presentation mapping may be
  simplified only as far as needed by the remaining Settings status UI.
- Downloading, installing, failed, checking, and up-to-date states continue to
  use the existing Settings update-status line; they do not show the red dot or
  the available-update row.

## Accessibility

- The red dot must not become a separate focus target.
- When the dot is present, the Settings button exposes an accessible label such
  as `Settings, update available` while retaining its visible `Settings` text.
- The update row follows the existing Settings reading order, immediately
  before Appearance.
- Focus, disabled, hover, light-theme, and dark-theme behavior follow the
  existing primary-action contract.

## Tests

- Component tests cover the dot's available/hidden states and accessible name.
- Settings component tests cover row placement, version text, the single
  install action, absence when unavailable, and disabled behavior while busy.
- App tests prove the available state reaches both components and that the old
  main-panel notice and `Later` action are absent.
- Native E2E opens the panel with `CACHEBITE_E2E_UPDATE=available`, verifies the
  dot, opens Settings, and verifies `Update available — 9.9.9` plus the enabled
  `Install and restart` button.
- Existing renderer checks continue to cover both light and dark theme token
  application; add a computed-style assertion only if the current suite does
  not already exercise the shared primary-action tokens.
