import type {History, MetricDefinition, ProviderDefinition} from './service-state.js';
import {COPY} from './shared-copy.js';

export interface PresentedHistory extends History {
    attribution: string | null;
    display_labels: Record<string, string>;
}

export function historyAttribution(
    metric: string,
    providers: string[],
    definitions: MetricDefinition[],
    providerDefinitions: ProviderDefinition[],
): string | null {
    const definition = definitions.find(item => item.id === metric);
    if (!definition) return null;
    const active = definition.suppliers.filter(id => providers.includes(id));
    if (!active.length) return COPY.noEnabledProviderData;
    const names = active.map(id => providerDefinitions.find(provider => provider.id === id)?.name ?? id).join(' + ');
    if (metric === 'windowUsagePercent') return `${names} · enabled models · step line · selected time zone`;
    if (
        definition.suppliers.length === 2 &&
        definition.suppliers.includes('claude') &&
        definition.suppliers.includes('codex')
    ) {
        const detail =
            active.length === 2 ? COPY.claudeByModelCodexTotal : active[0] === 'claude' ? COPY.byModel : 'total';
        return `${names} · ${detail} · daily UTC`;
    }
    return `${names} · ${definition.attribution.split(' · ').slice(1).join(' · ')}`;
}

export function historySeriesLabel(
    metric: string,
    provider: string,
    raw: string,
    definitions: MetricDefinition[],
    providerDefinitions: ProviderDefinition[],
): string {
    const name = raw.startsWith('model:')
        ? `Model · ${raw.slice(6)}`
        : raw.startsWith('surface:')
          ? `Surface · ${raw.slice(8)}`
          : raw;
    const providerName = providerDefinitions.find(item => item.id === provider)?.name ?? provider;
    if (metric === 'usage' || metric === 'windowUsagePercent') return `${providerName} ${name}`;
    if (definitions.find(item => item.id === metric)?.suppliers.length === 2) return `${providerName} · ${name}`;
    return name;
}

export function presentHistory(
    history: History,
    definitions: MetricDefinition[],
    providerDefinitions: ProviderDefinition[],
): PresentedHistory {
    const providers = [
        ...new Set(history.series.filter(series => series.points.length).map(series => series.provider)),
    ];
    const metric = history.metric === 'usage' ? 'windowUsagePercent' : history.metric;
    return {
        ...history,
        attribution: historyAttribution(metric, providers, definitions, providerDefinitions),
        display_labels: Object.fromEntries(
            history.series.map(series => {
                const raw = history.project_labels[series.label] ?? series.label;
                return [
                    series.id,
                    historySeriesLabel(history.metric, series.provider, raw, definitions, providerDefinitions),
                ];
            }),
        ),
    };
}
