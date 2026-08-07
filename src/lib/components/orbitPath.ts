/**
 * Geometry for the primary provider's mark walking the outside of the overlay.
 *
 * The pet sits inside the big ring and says nothing about which provider drives
 * it, while the satellite carries a logo — which reads as if the smaller ring
 * mattered more. The walking mark puts the primary's identity back on top
 * without crowding the pet.
 *
 * Everything here is in the overlay's own percentage space (0-100 on both axes,
 * matching the ring's `viewBox`), scaled to pixels at the call site. These
 * constants are the single source of truth for the satellite's placement: the
 * components read them through CSS custom properties rather than repeating the
 * numbers, because drift between the two would put the walker's feet somewhere
 * other than the edge it is supposed to be standing on.
 */

import type { OrbitDirection } from './models';

/**
 * Which way round the outline the mark walks, from a roll in [0, 1).
 *
 * The randomness stays at the call site so the mapping itself is testable and
 * so nothing rerolls on a recompute — a `$derived` here would make the mark
 * stutter between directions on every state change.
 */
export const orbitDirection = (roll: number): OrbitDirection =>
  roll < 0.5 ? 'forward' : 'reverse';

/** Outer edge of the ring: arc radius 42 plus half of the 6.5 stroke. */
export const RING_OUTER_RADIUS = 45.25;
/** Edge length of the walking mark. */
export const WALKER_SIZE = 14;
const WALKER_RADIUS = WALKER_SIZE / 2;

export const SATELLITE_SIZE = 40;
export const SATELLITE_RIGHT = -4.3;
export const SATELLITE_BOTTOM = -5.5;
const SATELLITE_RADIUS = SATELLITE_SIZE / 2;
const SATELLITE_CENTER = {
  x: 100 - SATELLITE_RIGHT - SATELLITE_RADIUS,
  y: 100 - SATELLITE_BOTTOM - SATELLITE_RADIUS,
};

/** The walker's centre line: its feet rest on the edge it orbits. */
const BIG_ORBIT = RING_OUTER_RADIUS + WALKER_RADIUS;
const SMALL_ORBIT = SATELLITE_RADIUS + WALKER_RADIUS;

/**
 * How far the outermost element reaches from the overlay's centre, doubled —
 * the overlay is centred in its window, so a rendered size divided by this
 * stays inside.
 *
 * The far side of the satellite's orbit wins: it sits low and right of centre
 * already, so the walker rounding its outside reaches well past anything on the
 * big ring. Applied in both ring modes rather than only in `double`, because a
 * mode switch must not resize the pet.
 */
const BIG_REACH = BIG_ORBIT + WALKER_RADIUS;
const SATELLITE_REACH = Math.max(
  SATELLITE_CENTER.x + SMALL_ORBIT + WALKER_RADIUS - 50,
  SATELLITE_CENTER.y + SMALL_ORBIT + WALKER_RADIUS - 50,
);
export const OVERLAY_BOUNDS_FACTOR =
  (2 * Math.max(BIG_REACH, SATELLITE_REACH)) / 100;

const RADIANS = Math.PI / 180;
const point = (cx: number, cy: number, r: number, degrees: number) => ({
  x: cx + r * Math.cos(degrees * RADIANS),
  y: cy + r * Math.sin(degrees * RADIANS),
});
/**
 * `Math.acos` outside [-1, 1] is `NaN`, and a `NaN` here would flow straight
 * into the path string, which browsers then drop in silence. The constants
 * above keep the two orbits intersecting, but they are exported to be tuned.
 */
const acosDegrees = (value: number) =>
  Math.acos(Math.min(1, Math.max(-1, value))) / RADIANS;

/**
 * Emit `from` → `to` as SVG arcs. Split into segments below a half turn so the
 * large-arc flag is always 0; the sweep flag is 1 because angles increase
 * clockwise once y points down.
 */
function arcTo(
  cx: number,
  cy: number,
  r: number,
  from: number,
  to: number,
  scale: number,
): string {
  const segments = Math.max(2, Math.ceil(Math.abs(to - from) / 120));
  const step = (to - from) / segments;
  const radius = (r * scale).toFixed(3);
  return Array.from({ length: segments }, (_, index) => {
    const { x, y } = point(cx, cy, r, from + step * (index + 1));
    return `A ${radius} ${radius} 0 0 1 ${(x * scale).toFixed(3)} ${(y * scale).toFixed(3)}`;
  }).join(' ');
}

/**
 * The closed loop the walker follows, in pixels, for an overlay `size` px wide.
 *
 * With a satellite the loop is the **union outline** of the two circles: the
 * walker rounds the big ring, steps across at the intersection, walks the
 * exposed part of the satellite, and steps back. Both arcs run clockwise, which
 * is what keeps the mark's feet pointing at whichever centre it is currently
 * orbiting — including upside down along the bottom.
 */
export function orbitPath(size: number, hasSatellite: boolean): string {
  const scale = size / 100;
  const px = (value: number) => (value * scale).toFixed(3);
  if (!hasSatellite) {
    const start = point(50, 50, BIG_ORBIT, 0);
    return `M ${px(start.x)} ${px(start.y)} ${arcTo(50, 50, BIG_ORBIT, 0, 360, scale)} Z`;
  }

  const dx = SATELLITE_CENTER.x - 50;
  const dy = SATELLITE_CENTER.y - 50;
  const distance = Math.hypot(dx, dy);
  // Standard circle-circle intersection: `along` is how far down the centre
  // line the shared chord sits.
  const along =
    (distance * distance + BIG_ORBIT * BIG_ORBIT - SMALL_ORBIT * SMALL_ORBIT) /
    (2 * distance);
  const bearing = Math.atan2(dy, dx) / RADIANS;
  const halfBig = acosDegrees(along / BIG_ORBIT);
  const halfSmall = acosDegrees((distance - along) / SMALL_ORBIT);

  // Leave the big circle where the satellite starts covering it, come back one
  // turn later; the satellite's own hidden span faces the big ring's centre.
  const leaveBig = bearing + halfBig;
  const rejoinBig = bearing - halfBig + 360;
  const enterSmall = bearing + 180 + halfSmall;
  const leaveSmall = bearing + 180 - halfSmall + 360;

  const start = point(50, 50, BIG_ORBIT, leaveBig);
  return [
    `M ${px(start.x)} ${px(start.y)}`,
    arcTo(50, 50, BIG_ORBIT, leaveBig, rejoinBig, scale),
    arcTo(
      SATELLITE_CENTER.x,
      SATELLITE_CENTER.y,
      SMALL_ORBIT,
      enterSmall,
      leaveSmall,
      scale,
    ),
    'Z',
  ].join(' ');
}
