import { describe, expect, it } from 'vitest';

import {
  orbitDirection,
  orbitPath,
  OVERLAY_BOUNDS_FACTOR,
  RING_OUTER_RADIUS,
  SATELLITE_BOTTOM,
  SATELLITE_RIGHT,
  SATELLITE_SIZE,
  WALKER_SIZE,
} from './orbitPath';

/** Every coordinate pair in the emitted path, in overlay-percentage space. */
function points(path: string, size: number): { x: number; y: number }[] {
  const numbers = (path.match(/-?\d+(\.\d+)?/g) ?? []).map(Number);
  const commands = path.match(/[MA]/g) ?? [];
  // `M` carries 2 numbers, `A` carries 7, and the coordinate pair is last.
  const pairs: { x: number; y: number }[] = [];
  let cursor = 0;
  for (const command of commands) {
    const width = command === 'M' ? 2 : 7;
    const slice = numbers.slice(cursor, cursor + width);
    const y = slice.pop() ?? 0;
    const x = slice.pop() ?? 0;
    pairs.push({ x: (x / size) * 100, y: (y / size) * 100 });
    cursor += width;
  }
  return pairs;
}

const distance = (
  a: { x: number; y: number },
  b: { x: number; y: number },
): number => Math.hypot(a.x - b.x, a.y - b.y);

const CENTRE = { x: 50, y: 50 };
const SATELLITE_CENTRE = {
  x: 100 - SATELLITE_RIGHT - SATELLITE_SIZE / 2,
  y: 100 - SATELLITE_BOTTOM - SATELLITE_SIZE / 2,
};
// The walker is centred on the path, so its feet reach half its size inward.
const BIG_ORBIT = RING_OUTER_RADIUS + WALKER_SIZE / 2;
const SMALL_ORBIT = SATELLITE_SIZE / 2 + WALKER_SIZE / 2;

describe('orbitPath', () => {
  it('keeps the walker a constant step outside the ring in single mode', () => {
    const path = orbitPath(160, false);

    for (const p of points(path, 160)) {
      // Feet on the ring's outer edge, all the way round.
      expect(distance(p, CENTRE)).toBeCloseTo(BIG_ORBIT, 2);
    }
    expect(path.endsWith('Z')).toBe(true);
  });

  it('scales with the overlay so the loop is size-independent', () => {
    const small = points(orbitPath(100, false), 100);
    const large = points(orbitPath(226, false), 226);

    expect(large).toHaveLength(small.length);
    large.forEach((p, index) => {
      const reference = small.at(index) ?? p;
      expect(p.x).toBeCloseTo(reference.x, 1);
      expect(p.y).toBeCloseTo(reference.y, 1);
    });
  });

  it('walks the union outline of both circles in double mode', () => {
    const traced = points(orbitPath(160, true), 160);

    for (const p of traced) {
      const onBig = Math.abs(distance(p, CENTRE) - BIG_ORBIT) < 0.05;
      const onSmall =
        Math.abs(distance(p, SATELLITE_CENTRE) - SMALL_ORBIT) < 0.05;
      // Every anchor rides one of the two circles...
      expect(onBig || onSmall).toBe(true);
      // ...and never dips inside the other, which is what makes this an
      // outline rather than two overlapping loops.
      expect(distance(p, CENTRE)).toBeGreaterThanOrEqual(BIG_ORBIT - 0.05);
      expect(distance(p, SATELLITE_CENTRE)).toBeGreaterThanOrEqual(
        SMALL_ORBIT - 0.05,
      );
    }

    // Both circles actually contribute; the satellite is not skipped.
    expect(
      traced.some((p) => Math.abs(distance(p, CENTRE) - BIG_ORBIT) < 0.05),
    ).toBe(true);
    expect(
      traced.some(
        (p) => Math.abs(distance(p, SATELLITE_CENTRE) - SMALL_ORBIT) < 0.05,
      ),
    ).toBe(true);
  });

  it('hands off between the circles without a jump', () => {
    const path = orbitPath(160, true);
    const traced = points(path, 160);

    // One `M` and a closing `Z` mean a single subpath: the walker never lifts
    // off and reappears somewhere else.
    expect(path.match(/M/g)).toHaveLength(1);
    expect(path.trimEnd().endsWith('Z')).toBe(true);

    // Exactly two anchors sit on both circles at once. Those crossings are the
    // handover points, and landing on both is what makes the seam continuous.
    // The loop starts on one of them, so it is emitted twice — dedupe first.
    const crossings = new Set(
      traced
        .filter(
          (p) =>
            Math.abs(distance(p, CENTRE) - BIG_ORBIT) < 0.05 &&
            Math.abs(distance(p, SATELLITE_CENTRE) - SMALL_ORBIT) < 0.05,
        )
        .map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`),
    );

    expect(crossings.size).toBe(2);
  });

  it('reserves enough room for the walker, the outermost element', () => {
    const half = WALKER_SIZE / 2;
    // Clipping is axis-aligned — the window is a square with the overlay
    // centred in it — so the radial extreme is not what matters. The far side
    // of the satellite's orbit is the binding constraint on both axes, since
    // the satellite already sits low and right of centre.
    const reach = Math.max(
      BIG_ORBIT + half - 50 + 50,
      SATELLITE_CENTRE.x + SMALL_ORBIT + half - 50,
      SATELLITE_CENTRE.y + SMALL_ORBIT + half - 50,
    );

    expect(OVERLAY_BOUNDS_FACTOR).toBeCloseTo((2 * reach) / 100, 5);
    // The consequence that actually matters: clamping a 240px window by this
    // factor leaves the whole loop on screen.
    expect(
      (reach / 100) * (240 / OVERLAY_BOUNDS_FACTOR) * 2,
    ).toBeLessThanOrEqual(240);
  });

  // The roll is supplied by the caller so the mapping is pinnable here and the
  // composition root does not bake `Math.random()` into a derived value.
  it('splits the direction roll evenly across the unit interval', () => {
    expect(orbitDirection(0)).toBe('forward');
    expect(orbitDirection(0.499)).toBe('forward');
    expect(orbitDirection(0.5)).toBe('reverse');
    expect(orbitDirection(0.999)).toBe('reverse');
  });

  // The two orbits intersect for the constants above, but they are exported to
  // be tuned, and `Math.acos` out of domain would emit `NaN` coordinates that
  // browsers drop without a word.
  it('never emits NaN coordinates', () => {
    for (const size of [64, 128, 172, 240]) {
      for (const hasSatellite of [true, false]) {
        expect(orbitPath(size, hasSatellite)).not.toMatch(/NaN/);
      }
    }
  });
});
