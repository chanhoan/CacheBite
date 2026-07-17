export const VISUAL_POINT_CAP = 240;

export type HistoryWindow = 'session' | 'weekly';

export interface HistoryPoint {
  readonly usedPercent: number;
  readonly startsNewSegment: boolean;
}

export interface HistorySample {
  readonly capturedAt: string;
  readonly session: HistoryPoint | null;
  readonly weekly: HistoryPoint | null;
}

export interface GraphPoint extends HistoryPoint {
  readonly capturedAt: string;
  readonly index: number;
  readonly key: string;
  readonly x: number;
  readonly y: number;
}

export interface GraphSegment {
  readonly points: readonly GraphPoint[];
}

export function buildGraphSegments(
  samples: readonly HistorySample[],
  window: HistoryWindow,
): readonly GraphSegment[] {
  const segments: GraphSegment[] = [];
  let current: GraphPoint[] = [];

  for (let index = 0; index < samples.length; index += 1) {
    const sample = samples[index];
    if (sample === undefined) continue;
    const point = sample[window];
    if (point === null) continue;
    if (point.startsNewSegment && current.length > 0) {
      segments.push({ points: current });
      current = [];
    }
    current.push({
      ...point,
      capturedAt: sample.capturedAt,
      index,
      key: `${index}:${sample.capturedAt}`,
      x: samples.length === 1 ? 50 : 5 + (index * 90) / (samples.length - 1),
      y: 95 - point.usedPercent * 0.9,
    });
  }
  if (current.length > 0) segments.push({ points: current });
  return segments;
}

function sampleSegment(
  points: readonly GraphPoint[],
  budget: number,
): readonly GraphPoint[] {
  if (points.length <= budget) return points;
  if (budget <= 1) return [points[0]!];

  const selected = new Set<number>([0, points.length - 1]);
  const innerSlots = budget - 2;
  const bucketCount = Math.floor(innerSlots / 2);
  for (let bucket = 0; bucket < bucketCount; bucket += 1) {
    const start = 1 + Math.floor((bucket * (points.length - 2)) / bucketCount);
    const end =
      1 + Math.floor(((bucket + 1) * (points.length - 2)) / bucketCount);
    let min = start;
    let max = start;
    for (let index = start + 1; index < end; index += 1) {
      if (points[index]!.usedPercent < points[min]!.usedPercent) min = index;
      if (points[index]!.usedPercent > points[max]!.usedPercent) max = index;
    }
    selected.add(min);
    selected.add(max);
  }
  if (innerSlots % 2 === 1) {
    selected.add(1 + Math.floor((points.length - 2) / 2));
  }
  return [...selected]
    .sort((left, right) => left - right)
    .slice(0, budget)
    .map((index) => points[index]!);
}

export function sampleGraphSegments(
  segments: readonly GraphSegment[],
  maxPoints = VISUAL_POINT_CAP,
): readonly GraphSegment[] {
  const totalPoints = segments.reduce(
    (total, segment) => total + segment.points.length,
    0,
  );
  if (totalPoints <= maxPoints) return segments;

  const minimums = segments.map((segment) =>
    Math.min(segment.points.length, 2),
  );
  const minimumTotal = minimums.reduce((total, minimum) => total + minimum, 0);
  if (minimumTotal > maxPoints) {
    return segments.slice(0, maxPoints).map((segment) => ({
      points: [segment.points[0]!],
    }));
  }

  const available = maxPoints - minimumTotal;
  const expandable = segments.map(
    (segment, index) => segment.points.length - minimums[index]!,
  );
  const expandableTotal = expandable.reduce((total, value) => total + value, 0);
  const budgets = minimums.map(
    (minimum, index) =>
      minimum + Math.floor((available * expandable[index]!) / expandableTotal),
  );
  let remaining =
    maxPoints - budgets.reduce((total, budget) => total + budget, 0);
  for (let index = 0; remaining > 0 && index < budgets.length; index += 1) {
    if (budgets[index]! < segments[index]!.points.length) {
      budgets[index] = budgets[index]! + 1;
      remaining -= 1;
    }
  }
  return segments.map((segment, index) => ({
    points: sampleSegment(segment.points, budgets[index]!),
  }));
}
