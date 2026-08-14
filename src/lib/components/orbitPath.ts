/**
 * Geometry for the two provider marks walking the outside of the overlay rings.
 *
 * The pet sits inside the big ring and says nothing about which provider drives
 * it, so each ring carries its own mark instead: the big one walks the primary,
 * the satellite walks the secondary. Neither ring holds a static logo.
 *
 * The satellite sits down and to the right of the big ring, far enough out that
 * the two circles no longer touch. Neither the distance nor the direction is
 * arbitrary: the distance is chosen so the two **outer common tangents**
 * converge at `TANGENT_HALF_ANGLE`, which is what makes the pair read as one
 * circle receding rather than as a badge stuck to the side of another, and the
 * bearing tips that recede toward the lower right.
 *
 * The construction is rotationally symmetric about the line joining the two
 * centres, so `SATELLITE_BEARING` moves the whole thing without disturbing the
 * tangency — `TANGENT_HALF_ANGLE` depends only on the two radii and the
 * distance. A diagonal also costs *less* than a straight sideways offset: the
 * overlay window is a square with the overlay centred in it, so a horizontal
 * placement spends the entire budget on one axis while a diagonal splits it.
 *
 * Everything here is in the overlay's own percentage space (0-100 on both axes,
 * matching the ring's `viewBox`), scaled to pixels at the call site. These
 * constants are the single source of truth for the satellite's placement: the
 * components read them through CSS custom properties rather than repeating the
 * numbers, because drift between the two would put a walker's feet somewhere
 * other than the edge it is supposed to be standing on.
 */

import type { OrbitDirection } from './models';

const RADIANS = Math.PI / 180;

/**
 * Which way round its circle the marks walk, from a roll in [0, 1).
 *
 * The randomness stays at the call site so the mapping itself is testable and
 * so nothing rerolls on a recompute — a `$derived` here would make the marks
 * stutter between directions on every state change. Both marks share the one
 * roll: two circles turning opposite ways stop reading as a single scene.
 */
export const orbitDirection = (roll: number): OrbitDirection =>
  roll < 0.5 ? 'forward' : 'reverse';

/** Outer edge of the ring: arc radius 42 plus half of the 6.5 stroke. */
export const RING_OUTER_RADIUS = 45.25;
/** Edge length of the primary's walking mark. */
export const WALKER_SIZE = 14;
const WALKER_RADIUS = WALKER_SIZE / 2;

export const SATELLITE_SIZE = 36;
/** Centre-to-centre distance, along `SATELLITE_BEARING`. */
export const SATELLITE_DISTANCE = 70;
/**
 * The secondary's mark. Deliberately smaller than `WALKER_SIZE`: matching the
 * big mark would make it 39% of the satellite's diameter against the primary's
 * 15% of the big ring, and the pair would stop looking like the same creature
 * seen at two distances.
 */
export const SATELLITE_WALKER_SIZE = 9;
const SATELLITE_RADIUS = SATELLITE_SIZE / 2;
const SATELLITE_WALKER_RADIUS = SATELLITE_WALKER_SIZE / 2;

/**
 * Half the angle at which the two outer common tangents converge, in degrees.
 *
 * Nothing renders these lines — they are the placement rule, not a mark. The
 * value is exported so a test can assert the tangency still holds after someone
 * tunes `SATELLITE_SIZE` or `SATELLITE_DISTANCE`; a pair that stops being
 * tangent stops reading as a recede, and no assertion about coordinates alone
 * would notice.
 *
 * Note the contact points are *not* the circles' extreme points across the
 * centre line. Joining those would give two parallel lines, which cannot
 * converge at all — each sits `TANGENT_HALF_ANGLE` round from the point square
 * to the bearing, i.e. at `SATELLITE_BEARING + TANGENT_HALF_ANGLE - 90°`.
 */
/**
 * Broken out so a failure lands on the cause rather than the symptom. Outside
 * [-1, 1] `Math.asin` returns `NaN`, and from here that `NaN` reaches
 * `SATELLITE_CENTER`, `OVERLAY_BOUNDS_FACTOR` and finally `overlaySize` — the
 * pet's rendered width, which would silently stop being a number. Two circles
 * only have outer common tangents when their centres are further apart than
 * their radii differ.
 */
export const TANGENT_RATIO =
  (RING_OUTER_RADIUS - SATELLITE_RADIUS) / SATELLITE_DISTANCE;
export const TANGENT_HALF_ANGLE = Math.asin(TANGENT_RATIO) / RADIANS;

/**
 * Direction from the big ring's centre to the satellite's, in degrees clockwise
 * from due right — y points down here, so this tips the pair toward the lower
 * right.
 *
 * It equals `TANGENT_HALF_ANGLE`, and that is not a coincidence. Keeping the
 * satellite's lowest point from dropping below the big ring's requires
 * `SATELLITE_DISTANCE * sin(bearing) <= RING_OUTER_RADIUS - SATELLITE_RADIUS`,
 * and the right-hand side over the distance is exactly `sin` of the half-angle.
 * So the constraint is `bearing <= TANGENT_HALF_ANGLE`, and taking the equality
 * puts both circles' lowest points on **one horizontal line**: the lower of the
 * two tangents comes out flat, and the pair reads as two circles standing on the
 * same floor with the far one set back. Steepening this past the half-angle
 * would hang the satellite below the pet; flattening it spends the size budget
 * below on the horizontal axis for nothing.
 */
export const SATELLITE_BEARING = TANGENT_HALF_ANGLE;
export const SATELLITE_CENTER = {
  x: 50 + SATELLITE_DISTANCE * Math.cos(SATELLITE_BEARING * RADIANS),
  y: 50 + SATELLITE_DISTANCE * Math.sin(SATELLITE_BEARING * RADIANS),
};

/** Each walker's centre line: its feet rest on the edge it orbits. */
const BIG_ORBIT = RING_OUTER_RADIUS + WALKER_RADIUS;
const SMALL_ORBIT = SATELLITE_RADIUS + SATELLITE_WALKER_RADIUS;

/**
 * Height of the satellite's readout digits as a fraction of its box, so the
 * caller can turn it into the pixels CSS needs. A percentage `font-size` would
 * resolve against the inherited font size, which knows nothing about how wide
 * the overlay was clamped to.
 *
 * The clear space inside the puck is the arc radius less half the stroke —
 * 38.75% of the box, i.e. 77.5% across. At this ratio three tabular digits stay
 * inside it, which is what `100` needs.
 */
export const SATELLITE_READOUT_RATIO = 0.33;

/**
 * Diameter of the badge that stands in for the readout when the satellite's
 * provider is not reporting, in the same currency and for the same reason:
 * `SystemBadge` sizes its chip in `rem`, which knows nothing about how wide the
 * overlay was clamped to, so at the puck's 36% a fixed 2rem chip covers nearly
 * the whole disc.
 *
 * Matched to the reading it replaces rather than to the puck, so the middle of
 * the puck carries the same weight whichever one is showing: the widest
 * three-digit readout spans about 0.61 of the box, which also keeps the badge
 * inside the 77.5% clear space the readout is measured against.
 */
export const SATELLITE_BADGE_RATIO = 0.6;

/** One full lap for the primary's mark. */
export const WALK_DURATION_S = 26;
/**
 * The secondary's lap, matched on **linear** speed rather than angular. Handing
 * the smaller circle the same 26s would make its mark crawl at 43% of the big
 * one's pace, and the two would stop looking like one creature at two distances.
 * Derived rather than written out so tuning either radius keeps them in step.
 */
export const SATELLITE_WALK_DURATION_S =
  WALK_DURATION_S * (SMALL_ORBIT / BIG_ORBIT);

/**
 * How far the outermost element reaches from the overlay's centre, doubled —
 * the overlay is centred in its window, so a rendered size divided by this
 * stays inside.
 *
 * The far side of the satellite's orbit wins, and because the bearing is under
 * 45° the horizontal axis is the binding one. Applied in both ring modes rather
 * than only in `double`, because a mode switch must not resize the pet.
 *
 * Every bundled pet manifest declares `defaultSize` 128, and a 240px window
 * clamped by this factor allows about 131px — roughly 1.9 points of reach to
 * spare. Growing `SATELLITE_DISTANCE` or `SATELLITE_SIZE`, or flattening
 * `SATELLITE_BEARING` toward horizontal, all spend that margin, and past it the
 * pet shrinks — which would turn a display preference into a layout change.
 */
/**
 * How far a mark's *corner* gets from the centre it orbits, along one axis.
 *
 * `offset-rotate: auto` turns the mark with the path, so a square of side `s`
 * at orbit angle θ spans `(s/2)(|sin θ| + |cos θ|)` on the x axis rather than a
 * flat `s/2`. Treating it as axis-aligned understates the reach, and the mark
 * then clips at exactly the size this factor exists to make safe.
 *
 * The extreme is not at θ = 0 where the orbit itself reaches furthest: giving
 * up a little orbit buys more corner. Differentiating
 * `r·cos θ + h·(sin θ + cos θ)` and solving gives `tan θ = h / (r + h)`.
 */
const rotatedReach = (orbit: number, half: number): number => {
  const theta = Math.atan(half / (orbit + half));
  return orbit * Math.cos(theta) + half * (Math.sin(theta) + Math.cos(theta));
};

const BIG_REACH = rotatedReach(BIG_ORBIT, WALKER_RADIUS);
const SATELLITE_WALKER_REACH = rotatedReach(
  SMALL_ORBIT,
  SATELLITE_WALKER_RADIUS,
);
const SATELLITE_REACH = Math.max(
  SATELLITE_CENTER.x + SATELLITE_WALKER_REACH - 50,
  SATELLITE_CENTER.y + SATELLITE_WALKER_REACH - 50,
);
export const OVERLAY_BOUNDS_FACTOR =
  (2 * Math.max(BIG_REACH, SATELLITE_REACH)) / 100;

const point = (cx: number, cy: number, r: number, degrees: number) => ({
  x: cx + r * Math.cos(degrees * RADIANS),
  y: cy + r * Math.sin(degrees * RADIANS),
});

/**
 * Thirds, because a single SVG arc cannot express a full turn and each third
 * stays under a half turn — which keeps the large-arc flag at 0 throughout.
 */
const CIRCLE_SEGMENTS = 3;

/**
 * A closed clockwise circle, in pixels, for an overlay `size` px wide. Clockwise
 * is what keeps a mark's feet pointing at the centre it orbits — including
 * upside down along the bottom, the way a walker on a globe looks from outside.
 * The sweep flag is 1 for the same reason: angles increase clockwise once y
 * points down.
 */
const circleOrbit = (
  size: number,
  cx: number,
  cy: number,
  radius: number,
): string => {
  const scale = size / 100;
  const px = (value: number) => (value * scale).toFixed(3);
  const r = px(radius);
  const start = point(cx, cy, radius, 0);
  const arcs = Array.from({ length: CIRCLE_SEGMENTS }, (_, index) => {
    const { x, y } = point(
      cx,
      cy,
      radius,
      (360 / CIRCLE_SEGMENTS) * (index + 1),
    );
    return `A ${r} ${r} 0 0 1 ${px(x)} ${px(y)}`;
  }).join(' ');
  return `M ${px(start.x)} ${px(start.y)} ${arcs} Z`;
};

/**
 * The primary's loop: the big ring's own circle, identical in both ring modes.
 * It does not grow an extra lobe when a satellite appears — the two rings are
 * separate objects, each carrying its own mark.
 */
export const orbitPath = (size: number): string =>
  circleOrbit(size, 50, 50, BIG_ORBIT);

/** The secondary's loop, around the satellite. Only drawn in `double` mode. */
export const satelliteOrbitPath = (size: number): string =>
  circleOrbit(size, SATELLITE_CENTER.x, SATELLITE_CENTER.y, SMALL_ORBIT);
