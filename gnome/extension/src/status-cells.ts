import type {
    ProviderView,
    ResetPrecision,
    ServiceState,
    Settings,
    StatusCell,
    StatusEntry,
    TemplateToken,
    Window,
} from './service-state.js';
import {compactCountdown, countdown} from './time-text.js';

function percent(value: number, decimals: number): string {
    return `${value.toFixed(decimals)}%`;
}

function fullCountdown(resetAt: number | null, now: number): string {
    return resetAt === null ? '—' : countdown(resetAt - now);
}

function shortCountdown(resetAt: number | null, now: number): string {
    return resetAt === null ? '--' : compactCountdown(resetAt - now);
}

function resetClock(resetAt: number | null, precision: ResetPrecision, now: number): string {
    if (resetAt === null) return '—';
    if (resetAt <= now) return 'now';
    const date = new Date(resetAt * 1000);
    if (precision === 'day') return date.toLocaleDateString('en-US', {timeZone: 'UTC', month: 'short', day: 'numeric'});
    const current = new Date(now * 1000);
    const options: Intl.DateTimeFormatOptions = {hour: '2-digit', minute: '2-digit', hour12: false};
    const sameDay = date.toDateString() === current.toDateString();
    if (sameDay) return date.toLocaleTimeString('en-GB', options);
    if (resetAt - now < 6 * 86400)
        return `${date.toLocaleDateString('en-US', {weekday: 'short'})} ${date.toLocaleTimeString('en-GB', options)}`;
    return (
        date.toLocaleDateString('en-US', {month: 'short', day: 'numeric'}) +
        ' ' +
        date.toLocaleTimeString('en-GB', options)
    );
}

function renderTemplate(
    tokens: TemplateToken[],
    values: Record<string, string | null>,
    limited: boolean,
    used: number,
    resetKnown: boolean,
): string {
    let text = '';
    let showsLimit = false;
    for (const token of tokens) {
        if (token === 'Newline') {
            text += '\n';
            continue;
        }
        if (token.Text !== undefined) {
            text += token.Text;
            continue;
        }
        const name = token.Placeholder;
        let value = values[name];
        if (value === undefined || value === null) continue;
        const usage = ['pct', 'pct0', 'pct1', 'pct2', 'remaining', 'pctOrReset'].includes(name);
        if (
            limited &&
            (['pct', 'pct0', 'pct1', 'pct2', 'remaining'].includes(name) || (name === 'pctOrReset' && !resetKnown))
        )
            value = 'Limit';
        showsLimit ||= usage && ((limited && value === 'Limit') || used === 100);
        text += value;
    }
    if (limited && !showsLimit) {
        const start = text.search(/[^\n]/);
        const end = start < 0 ? 0 : text.indexOf('\n', start);
        const position = end < 0 ? text.length : end;
        text = `${text.slice(0, position)} Limit${text.slice(position)}`;
    }
    return text;
}

export function windowTag(id: string, scope: string | null | undefined): string {
    switch (id) {
        case 'session':
            return '5h';
        case 'weekly':
            return '7d';
        case 'monthly':
            return '1mo';
        default: {
            const source = scope?.split(' ', 1)[0] ?? id.slice(id.lastIndexOf(':') + 1);
            return Array.from(source).slice(0, 3).join('').toUpperCase();
        }
    }
}

function renderCell(
    entry: StatusEntry,
    provider: ProviderView,
    window: Window,
    settings: Settings,
    multiple: boolean,
    tokens: TemplateToken[],
    now: number,
): StatusCell {
    const limited = entry.restrictions.length > 0;
    const used = entry.used;
    const shown = settings.show_as === 'remaining' ? 100 - used : used;
    const pct = limited ? 'Limit' : percent(shown, settings.decimals);
    const resetKnown = entry.reset_at !== null;
    const reset = ['countdown', 'exhausted', 'custom'].includes(settings.format)
        ? shortCountdown(entry.reset_at, now)
        : null;
    const tag = windowTag(entry.window_id, entry.window_scope);
    let text: string;
    switch (settings.format) {
        case 'inline':
            text = `${entry.short_label}:${pct}`;
            break;
        case 'mini-bars': {
            const filled = Math.max(0, Math.min(5, Math.round(used / 20)));
            text = `${entry.short_label} ${'▰'.repeat(filled)}${'▱'.repeat(5 - filled)} ${pct}`;
            break;
        }
        case 'countdown':
            text = `${entry.short_label} ${pct} · ${reset}`;
            break;
        case 'exhausted':
            text = `${entry.short_label}\n${used >= 100 ? (limited && !resetKnown ? 'Limit' : reset) : pct}`;
            break;
        case 'custom': {
            const values: Record<string, string | null> = {
                cell: multiple ? `${entry.provider_code} ${tag}` : entry.provider_code,
                provider: entry.provider_code,
                providerName: entry.provider_name,
                window: tag,
                label: entry.short_label,
                pct,
                pct0: limited ? 'Limit' : percent(shown, 0),
                pct1: limited ? 'Limit' : percent(shown, 1),
                pct2: limited ? 'Limit' : percent(shown, 2),
                remaining: limited ? 'Limit' : percent(100 - used, settings.decimals),
                pctOrReset: used >= 100 ? (limited && !resetKnown ? 'Limit' : reset) : pct,
                reset,
                resetClock: resetClock(entry.reset_at, window.reset_precision, now),
                plan: provider.plan,
                credits: provider.credit_balance?.toFixed(2) ?? '',
            };
            text = renderTemplate(tokens, values, limited, used, resetKnown);
            break;
        }
        default:
            text = `${entry.short_label}\n${pct}`;
    }
    const rawShown =
        settings.show_as === 'remaining' ? `${percent(100 - window.used, 0)} left` : percent(window.used, 0);
    const rawReset = window.reset ? Date.parse(window.reset) / 1000 : null;
    let tooltip = `${provider.name} ${window.label}: ${rawShown}, resets ${fullCountdown(
        Number.isNaN(rawReset) ? null : rawReset,
        now,
    )}`;
    if (limited) {
        const limits = entry.restrictions.map(
            restriction => `${restriction.label} (100% used; resets ${fullCountdown(restriction.reset_at, now)})`,
        );
        tooltip += `\nIncluded quota limited by ${limits.join('; ')}.`;
    }
    const stale = provider.status === 'stale';
    if (stale) tooltip = `Stale, last known values. ${tooltip}`;
    return {
        id: `${entry.provider}:${entry.window_id}`,
        label: entry.short_label,
        text,
        tooltip,
        used,
        provider: entry.provider,
        restricted: entry.restricted,
        limited,
        stale,
    };
}

export function statusCells(state: ServiceState): StatusCell[] {
    const seen = new Set<string>();
    const multiple = new Set<string>();
    for (const entry of state.status_entries) {
        if (seen.has(entry.provider)) multiple.add(entry.provider);
        seen.add(entry.provider);
    }
    const output: StatusCell[] = [];
    for (const entry of state.status_entries) {
        const provider = state.providers[entry.provider];
        if (!provider) continue;
        const window = provider.windows.find(row => row.id === entry.window_id);
        if (window)
            output.push(
                renderCell(
                    entry,
                    provider,
                    window,
                    state.settings,
                    multiple.has(entry.provider),
                    state.template_tokens,
                    state.now,
                ),
            );
    }
    return output;
}
