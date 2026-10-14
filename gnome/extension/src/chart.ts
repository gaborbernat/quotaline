import Clutter from 'gi://Clutter';
import St from 'gi://St';
import {COLORS, historyValue, readingAt} from './chart-data.js';
import type {HistorySeries} from './service-state.js';

export interface Highlight {
    id: string | null;
}

const DASH_PATTERNS: [number[], number[], number[], number[], number[]] = [
    [7, 3],
    [2, 3],
    [8, 2, 2, 2],
    [4, 2, 1, 2],
    [10, 3, 2, 3],
];

export function createLegendSwatch(index: number, kind: string): St.DrawingArea {
    const swatch = new St.DrawingArea({style_class: 'tmb-legend-swatch', width: 18, height: 12});
    swatch.connect('repaint', area => {
        const context = area.get_context();
        const [width, height] = area.get_surface_size();
        const variant = Math.floor(index / COLORS.length);
        const color = COLORS[(index % COLORS.length) as 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7];
        context.setSourceRGBA(color[0], color[1], color[2], 1);
        if (kind !== 'usage' && kind !== 'surfaceUsagePercent' && kind !== 'creditBalance') {
            context.rectangle(0, 2, width, height - 4);
            context.fill();
            context.$dispose();
            return;
        }
        context.setLineWidth(variant ? 1.75 + (Math.floor(variant / DASH_PATTERNS.length) % 3) * 0.25 : 2);
        context.setDash(
            variant ? DASH_PATTERNS[(variant % DASH_PATTERNS.length) as 0 | 1 | 2 | 3 | 4] : [],
            Math.floor(variant / (DASH_PATTERNS.length * 3)) % 7,
        );
        context.moveTo(1, height / 2);
        context.lineTo(width - 1, height / 2);
        context.stroke();
        context.setDash([], 0);
        if (variant % 3 === 1) context.rectangle(width / 2 - 2.5, height / 2 - 2.5, 5, 5);
        else if (variant % 3 === 2) {
            context.moveTo(width / 2, height / 2 - 3);
            context.lineTo(width / 2 + 3, height / 2);
            context.lineTo(width / 2, height / 2 + 3);
            context.lineTo(width / 2 - 3, height / 2);
            context.closePath();
        } else context.arc(width / 2, height / 2, 2.5, 0, 2 * Math.PI);
        context.fill();
        context.$dispose();
    });
    return swatch;
}

export function createChart(
    series: HistorySeries[],
    start: number,
    end: number,
    shown: Set<string>,
    kind = 'usage',
    stacked = false,
    selectedAt: number | null = null,
    utc = false,
    onSelect: ((timestamp: number | null) => void) | null = null,
    highlighted: Highlight | null = null,
    unit = '%',
): St.DrawingArea {
    const visible = series.filter(item => shown.has(item.id) && item.points.length);
    const line = kind === 'usage' || kind === 'surfaceUsagePercent' || kind === 'creditBalance';
    const totals = new Map<number, number>();
    if (stacked)
        for (const item of visible)
            for (const [timestamp, value] of item.points)
                totals.set(timestamp, (totals.get(timestamp) ?? 0) + Math.max(0, value));
    let maximum = 0;
    if (kind === 'usage' || kind === 'surfaceUsagePercent') maximum = 100;
    else if (stacked) for (const value of totals.values()) maximum = Math.max(maximum, value);
    else for (const item of visible) for (const point of item.points) maximum = Math.max(maximum, point[1]);
    if (kind !== 'usage' && kind !== 'surfaceUsagePercent') maximum = maximum > 0 ? maximum * 1.08 : 1;
    const fractions = [0, 0.25, 0.5, 0.75, 1];
    const axisTicks = fractions.map(fraction => historyValue(maximum * fraction, unit));
    const dateTicks = fractions.map(fraction => {
        const date = new Date((start + fraction * (end - start)) * 1000);
        return end - start > 172800
            ? date.toLocaleDateString(undefined, {month: 'numeric', day: 'numeric', timeZone: utc ? 'UTC' : undefined})
            : date.toLocaleTimeString(undefined, {
                  hour: '2-digit',
                  minute: '2-digit',
                  hour12: false,
                  timeZone: utc ? 'UTC' : undefined,
              });
    });
    const interactive = Boolean(onSelect && visible.length);
    const chart = new St.DrawingArea({
        style_class: 'tmb-chart',
        width: 690,
        height: 250,
        reactive: interactive,
        accessible_name: `History chart with ${visible.length} series; Y-axis ${historyValue(0, unit)} to ${historyValue(maximum, unit)}; use the series list for values`,
    });
    if (onSelect && visible.length) {
        const click = new Clutter.ClickGesture();
        click.set_required_button(Clutter.BUTTON_PRIMARY);
        click.connect('recognize', () => {
            onSelect(readingAt(series, shown, start, end, chart.width, click.get_coords().x));
        });
        chart.add_action(click);
    }
    chart.connect('repaint', area => {
        const context = area.get_context();
        const [width, height] = area.get_surface_size();
        const left = 12;
        const top = 12;
        const right = width - 38;
        const bottom = height - 24;
        context.setLineWidth(1);
        context.setSourceRGBA(0.5, 0.5, 0.5, 0.35);
        for (const fraction of fractions) {
            const y = bottom - fraction * (bottom - top);
            context.moveTo(left, y);
            context.lineTo(right, y);
            context.stroke();
        }
        context.setSourceRGBA(0.68, 0.68, 0.72, 0.9);
        context.setFontSize(9);
        for (const [index, tick] of axisTicks.entries()) {
            const fraction = index / (axisTicks.length - 1);
            context.moveTo(right + 5, bottom - fraction * (bottom - top) + 3);
            context.showText(tick);
        }
        for (const [index, tick] of dateTicks.entries()) {
            const fraction = index / (dateTicks.length - 1);
            const size = context.textExtents(tick).width;
            context.moveTo(
                Math.max(left, Math.min(right - size, left + fraction * (right - left) - size / 2)),
                height - 6,
            );
            context.showText(tick);
        }
        if (selectedAt !== null) {
            const x = left + ((selectedAt - start) / Math.max(1, end - start)) * (right - left);
            context.setSourceRGBA(0.9, 0.9, 0.9, 0.6);
            context.moveTo(x, top);
            context.lineTo(x, bottom);
            context.stroke();
        }
        const stackBase = new Map<number, number>();
        // The first reading may come from before the period so a line starts at the left edge; keep it inside.
        context.rectangle(left, top, right - left, bottom - top);
        context.clip();
        series.forEach((item, index) => {
            if (!shown.has(item.id) || item.points.length === 0) return;
            const color = COLORS[(index % COLORS.length) as 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7];
            const variant = Math.floor(index / COLORS.length);
            context.setSourceRGBA(
                color[0],
                color[1],
                color[2],
                highlighted?.id && highlighted.id !== item.id ? 0.2 : 1,
            );
            if (!line) {
                const slot = (right - left) / Math.max(1, (end - start) / 86400);
                const bar = Math.max(1, Math.min(18, (slot * 0.8) / (stacked ? 1 : Math.max(1, visible.length))));
                const position = visible.indexOf(item);
                for (const [timestamp, value] of item.points) {
                    const center = left + ((timestamp + 43200 - start) / Math.max(1, end - start)) * (right - left);
                    const base = stacked ? (stackBase.get(timestamp) ?? 0) : 0;
                    const upper = base + Math.max(0, value);
                    const y = bottom - (upper / maximum) * (bottom - top);
                    const lower = bottom - (base / maximum) * (bottom - top);
                    context.rectangle(
                        stacked ? center - bar / 2 : center - (visible.length * bar) / 2 + position * bar,
                        y,
                        bar,
                        lower - y,
                    );
                    context.fill();
                    if (stacked) stackBase.set(timestamp, upper);
                }
                return;
            }
            context.setLineWidth(
                highlighted?.id === item.id
                    ? 3
                    : variant
                      ? 1.75 + (Math.floor(variant / DASH_PATTERNS.length) % 3) * 0.25
                      : 2,
            );
            context.setDash(
                variant ? DASH_PATTERNS[(variant % DASH_PATTERNS.length) as 0 | 1 | 2 | 3 | 4] : [],
                Math.floor(variant / (DASH_PATTERNS.length * 3)) % 7,
            );
            let lastY: number | null = null;
            const points = item.points;
            points.forEach(([timestamp, used]) => {
                const x = left + ((timestamp - start) / Math.max(1, end - start)) * (right - left);
                const y = bottom - (Math.max(0, Math.min(maximum, used)) / maximum) * (bottom - top);
                if (lastY === null) context.moveTo(x, y);
                else if (kind === 'usage') {
                    context.lineTo(x, lastY);
                    context.lineTo(x, y);
                } else {
                    context.lineTo(x, y);
                }
                lastY = y;
            });
            context.stroke();
            context.setDash([], 0);
            const markers = Math.min(16, item.points.length);
            const markerIndices = new Set(
                Array.from({length: markers}, (_, marker) =>
                    markers === item.points.length
                        ? marker
                        : Math.round((marker * (item.points.length - 1)) / (markers - 1)),
                ),
            );
            for (const [index, point] of item.points.entries()) {
                if (!markerIndices.has(index)) continue;
                const x = left + ((point[0] - start) / Math.max(1, end - start)) * (right - left);
                const y = bottom - (Math.max(0, Math.min(maximum, point[1])) / maximum) * (bottom - top);
                if (variant % 3 === 1) context.rectangle(x - 2.5, y - 2.5, 5, 5);
                else if (variant % 3 === 2) {
                    context.moveTo(x, y - 3.5);
                    context.lineTo(x + 3.5, y);
                    context.lineTo(x, y + 3.5);
                    context.lineTo(x - 3.5, y);
                    context.closePath();
                } else context.arc(x, y, 2.5, 0, 2 * Math.PI);
                context.fill();
            }
        });
        context.$dispose();
    });
    return chart;
}
