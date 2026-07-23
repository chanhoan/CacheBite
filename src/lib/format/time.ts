const MINUTE_MS = 60_000;

/**
 * Every formatter takes `nowMs` as an argument instead of reading the clock.
 * Calling `Date.now()` in here would make results untestable and would break
 * Svelte reactivity: a `$derived` that reads the clock never re-runs on time.
 */
function parseTimestamp(isoTimestamp: string): number | null {
  const parsed = Date.parse(isoTimestamp);
  return Number.isNaN(parsed) ? null : parsed;
}

function humanizeMinutes(totalMinutes: number): string {
  if (totalMinutes < 60) return `${totalMinutes}m`;
  const totalHours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  if (totalHours < 24) return `${totalHours}h ${minutes}m`;
  const days = Math.floor(totalHours / 24);
  return `${days}d ${totalHours % 24}h ${minutes}m`;
}

/** Time remaining until `isoTimestamp`, for short rolling windows. */
export function relativeFromNow(
  isoTimestamp: string,
  nowMs: number,
): string | null {
  const target = parseTimestamp(isoTimestamp);
  if (target === null || !Number.isFinite(nowMs)) return null;
  const remainingMs = target - nowMs;
  if (remainingMs < MINUTE_MS) return 'now';
  return humanizeMinutes(Math.floor(remainingMs / MINUTE_MS));
}

/**
 * Wall-clock label for distant resets, e.g. `Mon 09:00`.
 *
 * Deviates from the plan's `(iso, nowMs)` signature: this label does not depend
 * on the current time, and an unused parameter fails lint. `timeZone` takes its
 * place so tests can pin a zone instead of inheriting the runner's.
 */
export function absoluteShort(
  isoTimestamp: string,
  timeZone?: string,
): string | null {
  const target = parseTimestamp(isoTimestamp);
  if (target === null) return null;
  const weekday = new Intl.DateTimeFormat('en-US', {
    weekday: 'short',
    timeZone,
  }).format(target);
  const clock = new Intl.DateTimeFormat('en-US', {
    hour: '2-digit',
    minute: '2-digit',
    hourCycle: 'h23',
    timeZone,
  }).format(target);
  return `${weekday} ${clock}`;
}

/** Elapsed time since a snapshot was captured, for the panel freshness line. */
export function capturedAgo(
  isoTimestamp: string,
  nowMs: number,
): string | null {
  const captured = parseTimestamp(isoTimestamp);
  if (captured === null || !Number.isFinite(nowMs)) return null;
  const minutes = Math.floor(Math.max(0, nowMs - captured) / MINUTE_MS);
  if (minutes < 1) return 'just now';
  if (minutes < 60) return `${minutes} min ago`;
  return `${Math.floor(minutes / 60)} hr ago`;
}
