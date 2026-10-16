import type {
    MiniBarGroup,
    Provider,
    Settings,
    StatusCell,
    StatusGroup,
    StatusIconTone,
    StatusVariant,
} from './service-state.js';
import {COPY} from './shared-copy.js';
export function emptyUsageTitle(settings: Settings): string {
    return Object.values(settings.providers).some(Boolean) ? COPY.noUsageAvailable : COPY.noProvidersEnabled;
}

export function miniBarGroups(cells: StatusCell[], groups: StatusGroup[]): MiniBarGroup[] {
    return groups.map(group => ({
        provider: group.provider,
        bars: group.indices.flatMap(index => cells.slice(index, index + 1)),
        used: group.used,
        restricted: group.restricted,
    }));
}

export function statusTooltip(cells: StatusCell[]): string {
    const [first] = cells;
    if (!first) return COPY.noUsageYet;
    const restriction = first.tooltip.split('\n')[1];
    if (restriction && cells.every(cell => cell.tooltip.split('\n')[1] === restriction))
        return `${cells.map(cell => cell.tooltip.split('\n')[0]).join('\n')}\n${restriction}`;
    return cells.map(cell => cell.tooltip).join('\n');
}

export function statusIconTone(
    cells: StatusCell[],
    providers: Record<string, Provider>,
    settings: Settings,
): StatusIconTone {
    if (cells.some(cell => cell.restricted)) return 'attention';
    const statuses = Object.entries(providers)
        .filter(([id]) => settings.providers[id] === true)
        .map(([, provider]) => provider.recovery_status ?? provider.status);
    if (statuses.includes('sign-in')) return 'attention';
    return statuses.includes('offline') ? 'offline' : 'normal';
}

export function statusVariants(cells: StatusCell[], settings: Settings, providerIndices: number[]): StatusVariant[] {
    const stacked = cells.map(cell => ({
        ...cell,
        text: `${cell.label}\n${
            cell.limited
                ? 'Limit'
                : `${(settings.show_as === 'remaining' ? 100 - cell.used : cell.used).toFixed(settings.decimals)}%`
        }`,
    }));
    return [
        {tier: 'configured', cells},
        {tier: 'stacked', cells: stacked},
        {tier: 'providers', cells: providerIndices.flatMap(index => stacked.slice(index, index + 1))},
        {tier: 'icon', cells: []},
    ];
}
