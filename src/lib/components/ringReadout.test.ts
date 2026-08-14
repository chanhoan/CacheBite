import { describe, expect, it } from 'vitest';

import { clampPercent, satelliteReading } from './ringReadout';
import type { RingWindowModel, SatelliteRingModel } from './models';

const window = (
  usedPercent: number | null,
  severity: RingWindowModel['severity'],
): RingWindowModel => ({ usedPercent, severity });

const satellite = (
  session: RingWindowModel,
  weekly: RingWindowModel,
): SatelliteRingModel => ({
  provider: 'codex',
  providerName: 'Codex',
  system: 'active',
  stale: false,
  session,
  weekly,
});

describe('clampPercent', () => {
  it('keeps a reported figure inside the window', () => {
    expect(clampPercent(window(41, 'ok'))).toBe(41);
    expect(clampPercent(window(0, 'ok'))).toBe(0);
    expect(clampPercent(window(137, 'exhausted'))).toBe(100);
    expect(clampPercent(window(-4, 'ok'))).toBe(0);
  });

  it('preserves absence rather than collapsing it to zero', () => {
    // A `0` here would make "no data" and "0% used" the same on screen. The arc
    // renders absence as an empty track and the satellite's puck as an em dash;
    // neither can tell the cases apart if this decides for them.
    expect(clampPercent(window(null, 'unknown'))).toBeNull();
    expect(clampPercent(window(Number.NaN, 'unknown'))).toBeNull();
    expect(
      clampPercent(window(Number.POSITIVE_INFINITY, 'unknown')),
    ).toBeNull();
  });
});

describe('satelliteReading', () => {
  it('reports the 5-hour window when it has a figure', () => {
    expect(
      satelliteReading(satellite(window(41, 'ok'), window(58, 'warn'))),
    ).toEqual({ percent: 41, severity: 'ok', window: 'session' });
  });

  it('falls back to the weekly window when the 5-hour cap is lifted', () => {
    // Providers run promotions that remove the 5-hour limit outright, and the
    // window then reports nothing rather than zero. Pinning to 5H would blank
    // the puck for the whole event.
    expect(
      satelliteReading(satellite(window(null, 'unknown'), window(62, 'warn'))),
    ).toEqual({ percent: 62, severity: 'warn', window: 'weekly' });
  });

  it('switches on absence only, never on severity', () => {
    // A readout that moved to whichever window was worse would be unreadable:
    // the 5H / WK labels are hidden at this size, so nothing on screen would
    // say which limit the number meant.
    expect(
      satelliteReading(satellite(window(12, 'ok'), window(97, 'critical')))
        .window,
    ).toBe('session');
  });

  it('reports absence when neither window has a figure', () => {
    expect(
      satelliteReading(
        satellite(window(null, 'unknown'), window(null, 'unknown')),
      ),
    ).toEqual({ percent: null, severity: 'unknown', window: 'weekly' });
  });

  it('clamps whichever window it ends up reporting', () => {
    expect(
      satelliteReading(satellite(window(137, 'exhausted'), window(4, 'ok')))
        .percent,
    ).toBe(100);
    expect(
      satelliteReading(
        satellite(window(null, 'unknown'), window(212, 'exhausted')),
      ).percent,
    ).toBe(100);
  });
});
