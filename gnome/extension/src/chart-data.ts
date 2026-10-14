import type {HistoryPoint, HistorySeries} from './service-state.js';

export const COLORS = [
    [0.19, 0.59, 1],
    [1, 0.58, 0.19],
    [0.2, 0.78, 0.36],
    [0.73, 0.34, 0.9],
    [0.85, 0.66, 0.68],
    [0.1, 0.77, 0.78],
    [0.43, 0.45, 0.93],
    [0.68, 0.49, 0.38],
] as const;
const CURRENCY_FORMAT = new Intl.NumberFormat(undefined, {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
});
const INTEGER_FORMAT = new Intl.NumberFormat(undefined, {maximumFractionDigits: 0});
const COMPACT_FORMAT = new Intl.NumberFormat(undefined, {notation: 'compact'});

export function historyValue(value: number, unit: string): string {
    if (unit === '%') return `${Math.round(value)}%`;
    if (unit === 'USD') return CURRENCY_FORMAT.format(value);
    return (Math.abs(value) < 1_000 ? INTEGER_FORMAT : COMPACT_FORMAT).format(value);
}

export function readingAt(
    series: HistorySeries[],
    shown: Set<string>,
    start: number,
    end: number,
    width: number,
    x: number,
): number | null {
    const readings = [
        ...new Set(
            series
                .filter(item => shown.has(item.id))
                .flatMap(item => item.points.filter(point => !point[3]).map(point => point[0])),
        ),
    ].sort((a, b) => a - b);
    if (!readings.length) return null;
    const fraction = Math.max(0, Math.min(1, (x - 12) / Math.max(1, width - 50)));
    const target = start + fraction * (end - start);
    return readings.reduce((left, right) => (Math.abs(right - target) < Math.abs(left - target) ? right : left));
}

export function valueAt(points: HistoryPoint[], timestamp: number, kind: string): number | null {
    const after = points.findIndex(point => point[0] >= timestamp);
    const next = points[after];
    if (!next) return null;
    if (next[0] === timestamp) return next[1];
    const before = points[after - 1];
    if (!before || (kind !== 'usage' && kind !== 'surfaceUsagePercent' && kind !== 'creditBalance')) return null;
    if (kind === 'usage') return before[1];
    const fraction = (timestamp - before[0]) / (next[0] - before[0]);
    return before[1] + fraction * (next[1] - before[1]);
}
