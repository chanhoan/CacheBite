import { describe, expect, it } from 'vitest';

import {
  orbitDirection,
  orbitPath,
  OVERLAY_BOUNDS_FACTOR,
  RING_OUTER_RADIUS,
  SATELLITE_BADGE_RATIO,
  SATELLITE_BEARING,
  SATELLITE_CENTER,
  SATELLITE_DISTANCE,
  SATELLITE_READOUT_RATIO,
  SATELLITE_SIZE,
  SATELLITE_WALK_DURATION_S,
  SATELLITE_WALKER_SIZE,
  satelliteOrbitPath,
  TANGENT_HALF_ANGLE,
  TANGENT_RATIO,
  WALK_DURATION_S,
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
// The walkers are centred on their paths, so their feet reach half their size
// inward. Each ring carries its own mark, at its own size.
const BIG_ORBIT = RING_OUTER_RADIUS + WALKER_SIZE / 2;
const SMALL_ORBIT = SATELLITE_SIZE / 2 + SATELLITE_WALKER_SIZE / 2;

describe('orbitPath', () => {
  it('keeps the walker a constant step outside the ring', () => {
    const path = orbitPath(160);

    for (const p of points(path, 160)) {
      // Feet on the ring's outer edge, all the way round.
      expect(distance(p, CENTRE)).toBeCloseTo(BIG_ORBIT, 2);
    }
    expect(path.endsWith('Z')).toBe(true);
  });

  it('keeps the satellite walker a constant step outside the small ring', () => {
    const path = satelliteOrbitPath(160);

    for (const p of points(path, 160)) {
      expect(distance(p, SATELLITE_CENTER)).toBeCloseTo(SMALL_ORBIT, 2);
    }
    expect(path.endsWith('Z')).toBe(true);
  });

  it('scales with the overlay so the loops are size-independent', () => {
    for (const build of [orbitPath, satelliteOrbitPath]) {
      const small = points(build(100), 100);
      const large = points(build(226), 226);

      expect(large).toHaveLength(small.length);
      large.forEach((p, index) => {
        const reference = small.at(index) ?? p;
        expect(p.x).toBeCloseTo(reference.x, 1);
        expect(p.y).toBeCloseTo(reference.y, 1);
      });
    }
  });

  it('places the satellite down and to the right', () => {
    expect(SATELLITE_CENTER.x).toBeGreaterThan(50);
    expect(SATELLITE_CENTER.y).toBeGreaterThan(50);
    expect(SATELLITE_BEARING).toBeGreaterThan(0);
    // Past 45° the vertical axis would start binding the size budget instead of
    // the horizontal, and the satellite would sit under the pet rather than
    // beside it.
    expect(SATELLITE_BEARING).toBeLessThan(45);
  });

  it('rests both circles on one horizontal line', () => {
    // The requirement: the satellite's lowest point must not drop below the big
    // ring's. Taking the equality is what makes the lower of the two tangents
    // come out flat, so the pair reads as two circles on the same floor with
    // the far one set back.
    const bigBottom = 50 + RING_OUTER_RADIUS;
    const smallBottom = SATELLITE_CENTER.y + SATELLITE_SIZE / 2;

    expect(smallBottom).toBeLessThanOrEqual(bigBottom + 1e-9);
    expect(smallBottom).toBeCloseTo(bigBottom, 6);
    // Which is the same statement as the bearing not exceeding the half-angle,
    // since sin(half-angle) is (R - r) / distance by construction.
    expect(SATELLITE_BEARING).toBeCloseTo(TANGENT_HALF_ANGLE, 6);
  });

  it('separates the two rings so neither swallows the other', () => {
    const gap = SATELLITE_DISTANCE - RING_OUTER_RADIUS - SATELLITE_SIZE / 2;

    // Touching or overlapping would put the satellite back to reading as a
    // badge stuck on the big ring, which is the layout this replaced.
    expect(gap).toBeGreaterThan(0);
  });

  it('converges the two outer tangents on both circles at the same angle', () => {
    // The design constraint: one line off the big circle and one off the small
    // one, converging at equal angles and touching both. Tangency means the
    // perpendicular distance from each centre equals that circle's own radius —
    // check it directly rather than trusting the arcsin that produced it.
    const smallRadius = SATELLITE_SIZE / 2;
    // Upper tangent. The whole construction rotates with the bearing, so each
    // contact point sits at `bearing + half-angle - 90°` round its own circle —
    // a quarter turn back from the centre line, then forward by the half-angle.
    const contactAngle =
      ((SATELLITE_BEARING + TANGENT_HALF_ANGLE - 90) * Math.PI) / 180;
    const contact = (cx: number, cy: number, radius: number) => ({
      x: cx + radius * Math.cos(contactAngle),
      y: cy + radius * Math.sin(contactAngle),
    });
    const onBig = contact(CENTRE.x, CENTRE.y, RING_OUTER_RADIUS);
    const onSmall = contact(
      SATELLITE_CENTER.x,
      SATELLITE_CENTER.y,
      smallRadius,
    );
    const perpendicular = (cx: number, cy: number) =>
      Math.abs(
        (onSmall.y - onBig.y) * cx -
          (onSmall.x - onBig.x) * cy +
          onSmall.x * onBig.y -
          onSmall.y * onBig.x,
      ) / Math.hypot(onSmall.x - onBig.x, onSmall.y - onBig.y);

    expect(perpendicular(CENTRE.x, CENTRE.y)).toBeCloseTo(RING_OUTER_RADIUS, 6);
    expect(perpendicular(SATELLITE_CENTER.x, SATELLITE_CENTER.y)).toBeCloseTo(
      smallRadius,
      6,
    );
    // The contact points are not the circles' extreme points across the centre
    // line — joining those would give two parallel lines that never converge.
    expect(onBig.x).not.toBeCloseTo(CENTRE.x, 3);
    expect(onSmall.x).not.toBeCloseTo(SATELLITE_CENTER.x, 3);
    // And the angle is one a person would call a recede, not a megaphone.
    expect(TANGENT_HALF_ANGLE).toBeGreaterThan(20);
    expect(TANGENT_HALF_ANGLE).toBeLessThan(30);
  });

  it('matches the two walkers on linear speed, not angular speed', () => {
    // The same period on a smaller circle would make the satellite's mark crawl
    // at 43% of the primary's pace, and the two would stop looking like one
    // creature seen at two distances.
    expect(SATELLITE_WALK_DURATION_S / WALK_DURATION_S).toBeCloseTo(
      SMALL_ORBIT / BIG_ORBIT,
      6,
    );
    expect(SATELLITE_WALK_DURATION_S).toBeLessThan(WALK_DURATION_S);
  });

  it('keeps the satellite mark proportionate to its own ring', () => {
    // Reusing WALKER_SIZE here would make the mark 39% of the satellite's
    // diameter against the primary's 15% of the big ring.
    const bigRatio = WALKER_SIZE / (RING_OUTER_RADIUS * 2);
    const smallRatio = SATELLITE_WALKER_SIZE / SATELLITE_SIZE;

    expect(SATELLITE_WALKER_SIZE).toBeLessThan(WALKER_SIZE);
    expect(smallRatio).toBeLessThan(bigRatio * 2);
  });

  it('leaves room for a three-digit readout inside the puck', () => {
    // Clear space is the arc radius less half the stroke: 38.75% of the box,
    // so 77.5% across. Tabular digits in the mono fallback advance at roughly
    // 0.62em, the widest of the stacked faces.
    const clearWidth = 2 * (42 - 6.5 / 2);
    const widestReading = 3 * SATELLITE_READOUT_RATIO * 0.62 * 100;

    expect(widestReading).toBeLessThan(clearWidth);
  });

  it('sizes the badge like the reading it stands in for', () => {
    const clearWidth = 2 * (42 - 6.5 / 2);
    const widestReading = 3 * SATELLITE_READOUT_RATIO * 0.62 * 100;
    const badge = SATELLITE_BADGE_RATIO * 100;

    // The badge replaces the number in the same middle, so it takes the number's
    // footprint: a puck whose weight jumped as a provider dropped out would read
    // as a layout change rather than a status change.
    expect(Math.abs(badge - widestReading)).toBeLessThan(2);
    expect(badge).toBeLessThan(clearWidth);
  });

  it('reserves enough room for the walkers, the outermost elements', () => {
    // Clipping is axis-aligned — the window is a square with the overlay
    // centred in it — so the radial extreme is not what matters. The far side
    // of the satellite's orbit is the binding constraint now that the satellite
    // sits a full SATELLITE_DISTANCE out.
    //
    // `offset-rotate: auto` spins each mark with its path, so its axis-aligned
    // half-width breathes between s/2 and s/√2 and the furthest point is not at
    // the orbit's own extreme. Sweep the whole loop rather than reusing the
    // closed form the source solves for — an independent check is the point.
    const sweep = (centre: number, orbit: number, size: number): number => {
      let furthest = 0;
      for (let degrees = 0; degrees < 360; degrees += 0.05) {
        const radians = (degrees * Math.PI) / 180;
        const half =
          (size / 2) *
          (Math.abs(Math.sin(radians)) + Math.abs(Math.cos(radians)));
        furthest = Math.max(
          furthest,
          centre + orbit * Math.cos(radians) + half,
        );
      }
      return furthest - 50;
    };
    const reach = Math.max(
      sweep(50, BIG_ORBIT, WALKER_SIZE),
      sweep(SATELLITE_CENTER.x, SMALL_ORBIT, SATELLITE_WALKER_SIZE),
      sweep(SATELLITE_CENTER.y, SMALL_ORBIT, SATELLITE_WALKER_SIZE),
    );

    // A model that ignored the rotation would land about 0.37 short here, and a
    // package asking for exactly the clamped size would clip on the right.
    expect(OVERLAY_BOUNDS_FACTOR).toBeCloseTo((2 * reach) / 100, 3);
    // The consequence that actually matters: clamping a 240px window by this
    // factor leaves both loops on screen.
    expect(
      (reach / 100) * (240 / OVERLAY_BOUNDS_FACTOR) * 2,
    ).toBeLessThanOrEqual(240);
    // And the one the layout was tuned for: every bundled pet manifest declares
    // defaultSize 128, so a clamp below that would let a display preference
    // silently shrink the pet.
    expect(240 / OVERLAY_BOUNDS_FACTOR).toBeGreaterThanOrEqual(128);
  });

  // The roll is supplied by the caller so the mapping is pinnable here and the
  // composition root does not bake `Math.random()` into a derived value.
  it('splits the direction roll evenly across the unit interval', () => {
    expect(orbitDirection(0)).toBe('forward');
    expect(orbitDirection(0.499)).toBe('forward');
    expect(orbitDirection(0.5)).toBe('reverse');
    expect(orbitDirection(0.999)).toBe('reverse');
  });

  it('keeps the tangent construction inside asin domain', () => {
    // Fails here rather than three derivations downstream. Out of domain, the
    // NaN reaches SATELLITE_CENTER, OVERLAY_BOUNDS_FACTOR and finally the pet's
    // rendered width — the symptom would be an overlay with no width at all.
    expect(Math.abs(TANGENT_RATIO)).toBeLessThan(1);
    expect(SATELLITE_DISTANCE).toBeGreaterThan(
      RING_OUTER_RADIUS - SATELLITE_SIZE / 2,
    );
    expect(Number.isFinite(TANGENT_HALF_ANGLE)).toBe(true);
  });

  // The constants above are exported to be tuned, and a `NaN` reaching a path
  // string is dropped by browsers without a word — the mark would simply stop
  // appearing, with nothing in the console to say why.
  it('never emits NaN coordinates', () => {
    for (const size of [64, 128, 172, 240]) {
      expect(orbitPath(size)).not.toMatch(/NaN/);
      expect(satelliteOrbitPath(size)).not.toMatch(/NaN/);
    }
  });
});
