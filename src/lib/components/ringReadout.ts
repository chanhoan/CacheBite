/**
 * What a usage ring actually puts on screen, decided away from any component.
 *
 * `models.ts` next door is type-only and excluded from the coverage gate on
 * that basis; branching logic living there would slip past the gate unnoticed.
 * It belongs here for the same reason the pointer and bubble policies live in
 * `interaction/` — the rule is the thing worth testing, not the markup that
 * happens to render it.
 */

import type { Severity } from '../state/engine';
import type { RingWindowModel, SatelliteRingModel } from './models';

/**
 * A window's usage as a 0-100 number, or `null` when the provider did not
 * report one. Callers decide what absence looks like: an arc draws zero length,
 * the satellite's readout draws an em dash. Collapsing both into `0` here would
 * make "no data" and "0% used" indistinguishable on screen.
 */
export const clampPercent = (window: RingWindowModel): number | null =>
  window.usedPercent === null || !Number.isFinite(window.usedPercent)
    ? null
    : Math.min(100, Math.max(0, window.usedPercent));

/**
 * What the satellite's puck shows, and which window it came from.
 *
 * Providers occasionally lift the 5-hour cap for a promotion, and the window
 * then reports nothing at all rather than zero. Pinning the readout to `5H`
 * would blank the puck for the whole event, so it falls back to the weekly
 * figure — the one limit still in force.
 *
 * The severity travels with the value so the number always takes the colour of
 * the arc it belongs to, which is what lets a viewer tell the two apart without
 * a label: an empty upper arc beside a number tinted like the lower one can
 * only be the weekly reading.
 */
export interface SatelliteReading {
  readonly percent: number | null;
  readonly severity: Severity;
  readonly window: 'session' | 'weekly';
}

/**
 * Switches on **absence only, never on severity.** A readout that moved to
 * whichever window was worse would be unreadable at this size, where the
 * `5H`/`WK` labels are hidden — nothing on screen would say which limit the
 * number meant. Switching only when a window goes silent leaves the empty arc
 * beside it as the cue.
 */
export const satelliteReading = (
  model: SatelliteRingModel,
): SatelliteReading => {
  const session = clampPercent(model.session);
  if (session !== null) {
    return {
      percent: session,
      severity: model.session.severity,
      window: 'session',
    };
  }
  return {
    percent: clampPercent(model.weekly),
    severity: model.weekly.severity,
    window: 'weekly',
  };
};
