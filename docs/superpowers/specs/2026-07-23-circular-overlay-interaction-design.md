# Circular Overlay Interaction Design

## Goal

Make the pet overlay easier to read and interact with while removing the
impression that the whole square window is clickable.

## Approved behavior

- Increase the `5H` and `WK` usage-ring labels from 4 px to 9 px and strengthen
  their contrast.
- Keep the existing pet and ring visuals unchanged and unclipped.
- Place a transparent circular interaction surface over the visible circular
  overlay area. Square corner pixels must not respond to pointer interaction.
- Preserve drag-to-move behavior when dragging from the circular surface.
- A single click does not open the information panel.
- A double-click on the circular surface opens the information panel.
- Clicking the speech bubble only dismisses the bubble. Its existing automatic
  dismissal remains unchanged.
- Add a close control to the information panel. It hides only the panel and
  does not quit CacheBite.

## Native boundary

Add a `hide_panel` Tauri command and expose it through `AppGateway.hidePanel()`.
Only the panel window may invoke this command. The command hides the current
panel window; it must not exit the process or alter the overlay.

## Verification

- Component tests cover the larger labels, circular interaction surface, and
  panel close callback.
- Application tests cover single-click, double-click, drag, bubble dismissal,
  and panel-only close behavior.
- Gateway and Rust authorization tests cover `hide_panel`.
- Native E2E selectors use double-click to open the panel.
