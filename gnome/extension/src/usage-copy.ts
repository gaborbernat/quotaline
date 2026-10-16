import type {
    CloudCreditProjection,
    CreditMetric,
    LocalMetricKind,
    ProviderView,
    ResetCredits,
    ReviewCounts,
    SpendControl,
    SpendMetricKind,
    UsageCredits,
} from './service-state.js';
import {COPY} from './shared-copy.js';

export interface MetricCopy {
    title: string;
    value: string;
    help: string;
}

export interface IdentityChip {
    kind: 'plan' | 'account' | 'workspace' | 'renewal';
    value: string;
}

export function localMetricCopy(kind: LocalMetricKind): [string, string] {
    switch (kind) {
        case 'window_tokens':
            return [COPY.fiveHourBlock, COPY.sessionTokensHelp];
        case 'window_cost':
            return [COPY.blockCost, COPY.apiEquivalentCostHelp];
        case 'cost_per_hour':
            return [COPY.burnRate, COPY.burnRateHelp];
        case 'today_tokens':
            return ['Today', COPY.tokensTodayHelp];
        case 'today_messages':
            return [COPY.messagesToday, COPY.messagesTodayHelp];
        case 'today_cost':
            return [COPY.todayCost, "API-equivalent cost of today's traffic."];
    }
}

export function cloudCreditStatus(projection: CloudCreditProjection): string {
    if (projection.status === 'locked') return 'Unavailable';
    if (projection.status === 'expired') return 'Expired';
    return projection.expiry_epoch === null
        ? ''
        : `Expires ${new Date(projection.expiry_epoch * 1000).toLocaleString()}`;
}

export function creditMetricCopy(metric: CreditMetric, provider: ProviderView, codex: boolean): MetricCopy {
    // The service only lists credit metrics whose source it reported.
    const credits = provider.usage_credits as UsageCredits;
    const resets = provider.reset_credits as ResetCredits;
    switch (metric.kind) {
        case 'credits':
            return {
                title: 'Credits',
                value: credits.unlimited
                    ? 'Unlimited'
                    : provider.credit_balance === null
                      ? 'Balance not reported'
                      : new Intl.NumberFormat(undefined, {style: 'currency', currency: 'USD'}).format(
                            provider.credit_balance,
                        ),
                help: codex
                    ? 'Available usage credits shared by ChatGPT Work and Codex. Credits extend usage beyond plan limits; they are separate from limit-reset credits.'
                    : 'Credits extend usage beyond plan limits.',
            };
        case 'overage':
            return {title: 'Overage', value: COPY.limitReached, help: 'The credit overage cap has been hit.'};
        case 'local_messages':
            return {
                title: 'Local messages',
                value: `~${(credits.local_messages as [number, number]).join('–')}`,
                help: 'Estimated local task messages the balance covers.',
            };
        case 'cloud_messages':
            return {
                title: 'Cloud messages',
                value: `~${(credits.cloud_messages as [number, number]).join('–')}`,
                help: 'Estimated cloud task messages the balance covers.',
            };
        case 'limit_resets':
            return {
                title: 'Limit resets',
                value: `${resets.available} available`,
                help: `Usage-limit resets available to redeem; ${resets.applicable} apply now.`,
            };
        case 'next_expiry':
            return {
                title: 'Next reset-credit expiry',
                value: new Date(metric.epoch * 1000).toLocaleString(),
                help: 'The earliest reported expiry among available reset credits. Expired credits cannot reset a limit.',
            };
        case 'reset_purchase':
            return {
                title: 'Reset purchase',
                value: 'Eligible',
                help: 'The provider reports that an immediate reset can be purchased. This app does not make purchases.',
            };
        case 'resets_earned':
            return {title: 'Resets earned', value: String(resets.total_earned), help: 'Reset credits earned so far.'};
    }
}

export function spendStatus(spend: SpendControl): string | null {
    if (spend.enabled === null) return 'Enablement not reported';
    if (spend.enabled) return null;
    if (!spend.disabled_reason) return 'Off';
    return `Off (${humanizeIdentifier(spend.disabled_reason)})`;
}

export function humanizeIdentifier(value: string): string {
    return value
        .split(/[_-]+/)
        .filter(Boolean)
        .map(part => part.charAt(0).toUpperCase() + part.slice(1))
        .join(' ');
}

export function spendMetricCopy(kind: SpendMetricKind): [string, string] {
    switch (kind) {
        case 'remaining':
            return ['Remaining', 'Provider-reported monthly credits, not currency.'];
        case 'allowance_source':
            return ['Allowance source', 'Who sets this credit allowance.'];
        case 'balance':
            return ['Balance', 'Prepaid credit balance.'];
        case 'resets':
            return ['Resets', 'When the monthly spend counter resets.'];
        case 'auto_reload':
            return ['Auto-reload', 'Whether credits top up automatically.'];
        case 'purchase':
            return [
                'Purchase',
                'The provider reports that this account can purchase credits. This app does not make purchases.',
            ];
    }
}

export function autoReloadText(enabled: boolean | null): string {
    return enabled ? 'On' : 'Off';
}

export function identityChips(provider: ProviderView): IdentityChip[] {
    const chips: IdentityChip[] = [];
    if (provider.plan) chips.push({kind: 'plan', value: provider.plan});
    if (provider.identity) chips.push({kind: 'account', value: provider.identity});
    if (provider.show_organization && provider.organization)
        chips.push({kind: 'workspace', value: provider.organization});
    if (provider.subscription_active_until) chips.push({kind: 'renewal', value: provider.subscription_active_until});
    return chips;
}

export function reviewCountText(counts: ReviewCounts): string {
    return `${counts.today} today · ${counts.week} this week`;
}
