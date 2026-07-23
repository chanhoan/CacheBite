import { describe, expect, it } from 'vitest';
import { absoluteShort, capturedAgo, relativeFromNow } from './time';

const NOW = Date.parse('2026-07-16T12:00:00Z');
const at = (offsetMs: number) => new Date(NOW + offsetMs).toISOString();

describe('relativeFromNow', () => {
  it.each([
    [0, 'now'],
    [30_000, 'now'],
    [60_000, '1m'],
    [59 * 60_000, '59m'],
    [60 * 60_000, '1h 0m'],
    [72 * 60_000, '1h 12m'],
    [(23 * 60 + 59) * 60_000, '23h 59m'],
    [24 * 60 * 60_000, '1d 0h 0m'],
    [(6 * 24 * 60 + 22 * 60 + 10) * 60_000, '6d 22h 10m'],
    [-5 * 60_000, 'now'],
  ])('formats %i ms remaining as "%s"', (offsetMs, expected) => {
    expect(relativeFromNow(at(offsetMs), NOW)).toBe(expected);
  });

  it.each(['not-a-date', ''])(
    'returns null for the unparsable timestamp %j',
    (isoTimestamp) => {
      expect(relativeFromNow(isoTimestamp, NOW)).toBeNull();
    },
  );

  it('returns null when the reference clock is not finite', () => {
    expect(relativeFromNow(at(60_000), Number.NaN)).toBeNull();
  });
});

describe('absoluteShort', () => {
  it('renders a weekday and a 24-hour clock in the pinned zone', () => {
    expect(absoluteShort('2026-07-20T09:00:00Z', 'UTC')).toBe('Mon 09:00');
  });

  it('renders midnight as 00:00 rather than 24:00', () => {
    expect(absoluteShort('2026-07-20T00:00:00Z', 'UTC')).toBe('Mon 00:00');
  });

  it.each(['not-a-date', ''])(
    'returns null for the unparsable timestamp %j',
    (isoTimestamp) => {
      expect(absoluteShort(isoTimestamp, 'UTC')).toBeNull();
    },
  );
});

describe('capturedAgo', () => {
  it.each([
    [0, 'just now'],
    [-30_000, 'just now'],
    [-60_000, '1 min ago'],
    [-2 * 60_000, '2 min ago'],
    [-59 * 60_000, '59 min ago'],
    [-60 * 60_000, '1 hr ago'],
  ])('formats a capture %i ms from now as "%s"', (offsetMs, expected) => {
    expect(capturedAgo(at(offsetMs), NOW)).toBe(expected);
  });

  it('clamps a future capture to "just now" instead of going negative', () => {
    expect(capturedAgo(at(5 * 60_000), NOW)).toBe('just now');
  });

  it.each(['not-a-date', ''])(
    'returns null for the unparsable timestamp %j',
    (isoTimestamp) => {
      expect(capturedAgo(isoTimestamp, NOW)).toBeNull();
    },
  );

  it('returns null when the reference clock is not finite', () => {
    expect(capturedAgo(at(-60_000), Number.POSITIVE_INFINITY)).toBeNull();
  });
});
