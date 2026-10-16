import type {PaceEstimate, PaceLabels, PaceStatus, QuotaPresentation, Window} from './service-state.js';
import {COPY} from './shared-copy.js';

function paceTitle(status: PaceStatus, compact: boolean): string {
    switch (status) {
        case 'on_track':
            return 'On pace';
        case 'ahead':
            return compact ? 'Ahead' : COPY.aheadOfPace;
        case 'behind':
            return COPY.underPace;
        case 'exhausted':
            return COPY.limitReached;
        case 'unknown':
            return COPY.learningPace;
    }
}

function paceComparison(pace: PaceEstimate, countdown: string | null, compact: boolean): string {
    if (pace.status === 'exhausted') return paceTitle(pace.status, compact);
    const parts = [paceTitle(pace.status, compact)];
    if (pace.expected_percent !== null)
        parts.push(`${compact ? 'exp' : 'expected'} ${Math.round(pace.expected_percent)}%`);
    if (pace.ratio !== null) parts.push(`${pace.ratio.toFixed(1)}×`);
    if (countdown !== null) parts.push(`hits 100% in ${countdown}`);
    else if (pace.expected_percent !== null) parts.push(COPY.lastsToReset);
    return parts.join(' · ');
}

export function paceLabels(pace: PaceEstimate, now: number): PaceLabels {
    let countdown: string | null = null;
    if (pace.projected_exhaustion !== null) {
        const seconds = Math.max(0, pace.projected_exhaustion - now);
        countdown =
            seconds >= 86400
                ? `${Math.trunc(seconds / 86400)}d ${Math.trunc((seconds % 86400) / 3600)}h`
                : `${Math.trunc(seconds / 3600)}h ${Math.trunc((seconds % 3600) / 60)}m`;
    }
    const compact = paceComparison(pace, countdown, true);
    return {
        full: paceComparison(pace, countdown, false),
        compact,
        brief: compact.split(' · ').slice(0, 2).join(' · '),
    };
}

function shortDuration(seconds: number): string {
    const hours = Math.trunc(seconds / 3600);
    if (hours >= 24 && hours % 24 === 0) return hours === 24 ? '24h' : `${hours / 24}d`;
    return hours > 0 ? `${hours}h` : `${Math.trunc(seconds / 60)}m`;
}

export function quotaPresentation(window: Window, now: number): QuotaPresentation {
    const seconds = window.duration_seconds ?? 0;
    const duration = seconds > 0 ? shortDuration(seconds) : null;
    const detail =
        window.detail ||
        (['session', 'weekly', 'monthly'].includes(window.id)
            ? duration
                ? `window · ${duration}`
                : 'window'
            : window.id);
    const pace_labels = paceLabels(window.pace, now);
    const used = Math.max(0, Math.min(100, window.used));
    const help = [`Used ${used.toFixed(1)}%; ${(100 - used).toFixed(1)}% remains.`];
    if (duration) help.push(`Window duration: ${duration}.`);
    if (window.pace.expected_percent !== null)
        help.push(
            `Even pace would be ${Math.round(Math.max(0, Math.min(100, window.pace.expected_percent)))}% by now.`,
        );
    if (window.pace.ratio !== null) help.push(`Pace ratio: ${window.pace.ratio.toFixed(2)}×.`);
    help.push(`${pace_labels.full}.`);
    if (window.severity) help.push(`Severity: ${window.severity}.`);
    return {detail, help: help.join(' '), pace_labels};
}
