import type {ServiceState, TimePresentation, Window} from './service-state.js';
import {COPY} from './shared-copy.js';
import {relativeAge as ageText} from './time-text.js';

interface ProviderTimes {
    status: string;
    recovery_status: string | null;
    windows: Window[];
    updated: number | null;
}

export function availabilityTitle(status: string): string {
    return (
        (
            {
                connected: COPY.upToDate,
                stale: COPY.showingLastKnownValues,
                checking: 'Loading',
                'sign-in': COPY.signInRequired,
                offline: 'Offline',
                'rate-limited': COPY.rateLimited,
                disabled: 'Disabled',
            } as Record<string, string>
        )[status] ?? 'Unavailable'
    );
}

export function relativeAge(timestamp: number | null | undefined, now: number): string {
    return timestamp === null || timestamp === undefined ? 'never' : ageText(now - timestamp);
}

export function nextAgeUpdateMs(timestamp: number | null | undefined, now: number): number | null {
    if (timestamp === null || timestamp === undefined) return null;
    const seconds = Math.max(0, now - timestamp);
    const boundary =
        seconds < 5
            ? 5
            : seconds < 60
              ? Math.floor(seconds) + 1
              : seconds < 3600
                ? (Math.floor(seconds / 60) + 1) * 60
                : seconds < 86_400
                  ? (Math.floor(seconds / 3600) + 1) * 3600
                  : (Math.floor(seconds / 86_400) + 1) * 86_400;
    return Math.ceil(Math.max(1, (boundary - (now - timestamp)) * 1000)) + 1;
}

export function timePresentation(state: ServiceState, now: number): TimePresentation {
    const selected = new Set(state.selected_window_keys);
    const running = new Set(state.refreshing_providers);
    const statuses: Record<string, string> = {};
    const help: Record<string, string> = {};
    let nextUpdate = nextAgeUpdateMs(state.last_refresh, now);
    for (const {id} of state.provider_definitions) {
        const provider: ProviderTimes = state.providers[id] ?? {
            status: running.has(id) ? 'checking' : 'sign-in',
            recovery_status: null,
            windows: [],
            updated: null,
        };
        const effective = provider.recovery_status ?? provider.status;
        const title = availabilityTitle(effective);
        const hasRows = provider.windows.some(window => selected.has(`${id}:${window.id}`));
        const refreshing = running.has(id);
        const age = relativeAge(provider.updated, now);
        if (hasRows) {
            const delay = nextAgeUpdateMs(provider.updated, now);
            if (delay !== null) nextUpdate = nextUpdate === null ? delay : Math.min(nextUpdate, delay);
        }
        if (!hasRows) statuses[id] = refreshing ? 'Refreshing…' : title;
        else if (refreshing) {
            const recovery = ['sign-in', 'offline', 'rate-limited', 'error', 'unreadable'].includes(effective);
            statuses[id] = `${recovery ? `${title} · ` : ''}refreshing… · showing ${age}`;
        } else statuses[id] = effective === 'connected' ? `fetched ${age}` : `${title} · ${age}`;
        help[id] = refreshing
            ? hasRows
                ? COPY.refreshingStaleValues
                : COPY.fetchingFirstValues
            : provider.updated !== null
              ? COPY.lastSuccessfulValues
              : COPY.noSuccessfulReading;
    }
    const refreshAge = relativeAge(state.last_refresh, now);
    return {
        header_updated: `Updated ${refreshAge}`,
        refresh_age: refreshAge,
        header_activity: state.settings.demo ? COPY.demoData : running.size ? 'Refreshing…' : '',
        provider_statuses: statuses,
        provider_help: help,
        next_update_ms: nextUpdate,
    };
}
