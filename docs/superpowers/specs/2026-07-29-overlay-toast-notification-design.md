# Overlay Toast Notification Design

## Goal

Replace the speech-bubble metaphor with a short-lived notification card below
the circular usage gauge. The notification must remain readable without
overlapping the gauge, implying that the pet is speaking, or being clipped by
the fixed overlay window.

## Approved behavior

- Keep the existing 128 px pet and circular usage gauge unchanged.
- Render the transient message below the gauge in the remaining space inside
  the fixed 240 x 240 overlay window.
- Present the message as a centered toast card without a speech-bubble tail.
- Keep a visible gap between the gauge and toast; their rectangles must never
  overlap.
- Keep the toast inside the overlay viewport so it does not extend into the
  separately anchored click panel's space when that panel is visible.
- Allow ordinary text to wrap naturally to two or three lines.
- Prefer whole-word wrapping for Korean and English text. Only unbroken strings
  that exceed the available width may break at an arbitrary character.
- Size the toast from its content within the available width. Do not truncate,
  ellipsize, or hide message text.
- Preserve the existing eight-second expiry and click-to-dismiss behavior.
- Keep the existing bubble policy and settings wire format; this is a visual
  presentation change, not a notification-policy migration.

## Component design

`App.svelte` groups `PetOverlay` and the transient notification in one
overlay-only stack. The stack owns vertical spacing and ensures the toast is
laid out after the fixed-size gauge instead of as an unrelated root grid item.

`SpeechBubble.svelte` keeps its current public props so policy and composition
code do not churn, but its rendered class and styling become a tail-free overlay
toast. Its button semantics remain intentional: clicking the toast dismisses it
without opening the usage panel.

The toast uses a viewport-aware maximum inline size, normal white-space, pretty
text wrapping where supported, whole-word breaking, and an emergency overflow
wrap for a single oversized token.

## Panel interaction

The click panel remains a separate native window positioned beside the 240 px
overlay bounds by the existing native placement policy. Because the toast stays
within those bounds, it introduces no new panel overlap and opening or closing
the panel requires no alternate toast placement.

## Verification

- Component tests preserve click dismissal and policy-owned expiry behavior.
- Component tests assert the notification presentation hook and multiline-safe
  content structure.
- Application tests assert that the toast remains an overlay-only child and
  never opens the click panel.
- Renderer E2E coverage compares element rectangles to prove that the toast is
  below the gauge, does not overlap it, stays inside the viewport, and wraps a
  long synthetic message without horizontal overflow.
- Run the full renderer unit suite, type check, lint/format checks, and renderer
  build before completion.
