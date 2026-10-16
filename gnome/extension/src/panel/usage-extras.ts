import Clutter from 'gi://Clutter';
import Pango from 'gi://Pango';
import type St from 'gi://St';
import {usageColor} from '../color.js';
import type {TokenPanel} from '../panel.js';
import {until} from '../presentation.js';
import type {
    CreditAllowance,
    LocalMetric,
    LocalUsage,
    PanelProvider,
    PanelState,
    SpendAmount,
} from '../service-state.js';
import {COPY} from '../shared-copy.js';
import {
    autoReloadText,
    cloudCreditStatus,
    creditMetricCopy,
    humanizeIdentifier,
    localMetricCopy,
    spendMetricCopy,
    spendStatus,
} from '../usage-copy.js';
import {box, button, label, meter, percent} from './support.js';

export const extraMethods = {
    _renderUsageExtras(
        this: TokenPanel,
        state: PanelState,
        id: string,
        provider: PanelProvider,
        local: LocalUsage | null,
        body: St.BoxLayout,
    ): void {
        if (['claude', 'codex'].includes(id) && this._costs?.[id]) {
            const summary = this._costs[id];
            const costs = box(true, 'tmb-cost-summary');
            const narrow = this._root.width < 440;
            costs.add_child(
                label(
                    narrow
                        ? 'API-EQUIVALENT ESTIMATES · UTC'
                        : `${provider.name.toUpperCase()} API-EQUIVALENT ESTIMATES · DAILY UTC`,
                    'tmb-eyebrow',
                ),
            );
            const tiles = box(false, 'tmb-cost-tiles');
            for (const [title, value] of [
                [narrow ? 'Today' : COPY.todayUtc, summary.today],
                [narrow ? 'Yesterday' : COPY.yesterdayUtc, summary.yesterday],
                [narrow ? '30 days' : 'Last 30 days', summary.last_window],
            ] satisfies [string, number][]) {
                const tile = box(true, 'tmb-cost-tile');
                tile.x_expand = true;
                tile.add_child(label(title, 'tmb-secondary'));
                tile.add_child(
                    label(
                        new Intl.NumberFormat(undefined, {style: 'currency', currency: 'USD'}).format(value),
                        'tmb-cost-value',
                    ),
                );
                tiles.add_child(tile);
            }
            this._tooltip.attach(
                tiles,
                `${provider.name} API-equivalent estimates · daily UTC`,
                [
                    'Models over the last 30 days:',
                    ...summary.models.map(
                        model =>
                            `${provider.name} · ${model.model}: ${new Intl.NumberFormat(undefined, {
                                style: 'currency',
                                currency: 'USD',
                            }).format(model.cost)}`,
                    ),
                ].join('\n'),
            );
            costs.add_child(tiles);
            const key = `cost:${id}`;
            const disclosure = button(
                `${this._expandedDetails.has(key) ? '▾' : '▸'} Model cost estimates`,
                () => {
                    if (this._expandedDetails.has(key)) this._expandedDetails.delete(key);
                    else this._expandedDetails.add(key);
                    this._renderPage();
                },
                'tmb-disclosure',
            );
            disclosure.x_align = Clutter.ActorAlign.START;
            costs.add_child(disclosure);
            if (this._expandedDetails.has(key)) {
                const explanation = label(COPY.spendEstimateHelp, 'tmb-secondary');
                explanation.width = Math.max(200, this._root.width - 80);
                explanation.clutter_text.set_line_wrap(true);
                costs.add_child(explanation);
                for (const model of summary.models)
                    costs.add_child(
                        this._keyValue(
                            model.model,
                            new Intl.NumberFormat(undefined, {style: 'currency', currency: 'USD'}).format(model.cost),
                        ),
                    );
            }
            body.add_child(costs);
        } else if (['claude', 'codex'].includes(id)) {
            body.add_child(
                label(
                    !this._costs
                        ? COPY.loadingCostHistory
                        : this._costError
                          ? COPY.costHistoryUnavailable
                          : COPY.noCostHistory,
                    'tmb-secondary',
                ),
            );
        }
        if (['claude', 'codex'].includes(id) && this._costError)
            body.add_child(label(this._costError, 'tmb-secondary'));
        if (provider.cloud_credits) {
            const cloud = provider.cloud_credits;
            const projection = provider.cloud_projection;
            const money = (value: number): string =>
                new Intl.NumberFormat(undefined, {style: 'currency', currency: 'USD', maximumFractionDigits: 2}).format(
                    value,
                );
            const section = box(true, 'tmb-spend tmb-cloud-credits');
            section.add_child(label(COPY.cloudSessionCredits, 'tmb-spend-title'));
            const state = cloudCreditStatus(projection);
            section.add_child(
                label(
                    `${money(cloud.remaining)} of ${money(cloud.limit)} ` + `left${state ? ` · ${state}` : ''}`,
                    'tmb-spend-summary',
                ),
            );
            section.add_child(
                meter(
                    Math.max(160, this._root.width - 60),
                    projection.used_percent,
                    projection.status !== 'active' || provider.status === 'stale'
                        ? '#808080'
                        : usageColor(projection.used_percent),
                ),
            );
            body.add_child(section);
        }
        if (provider.spend) {
            const spend = provider.spend;
            const projection = provider.spend_projection;
            // Projection kinds identify which optional source values the service reported.
            const allowance = spend.credit_allowance as CreditAllowance;
            const credits = (value: number): string =>
                `${new Intl.NumberFormat(undefined, {maximumFractionDigits: 6}).format(value)} credits`;
            const money = (value: number): string =>
                new Intl.NumberFormat(
                    undefined,
                    spend.currency ? {style: 'currency', currency: spend.currency} : {maximumFractionDigits: 2},
                ).format(value);
            const amount = (source: SpendAmount, used: boolean): string => {
                switch (source) {
                    case 'credits':
                        return credits(used ? (allowance.used as number) : (allowance.limit as number));
                    case 'money':
                        return money(used ? (spend.used as number) : (spend.limit as number));
                    case 'unavailable':
                        return 'Unavailable';
                    case 'unreported_limit':
                        return COPY.unreportedLimit;
                }
            };
            const summary = [spendStatus(spend)];
            if (projection.used) summary.push(`${amount(projection.used, true)} of ${amount(projection.limit, false)}`);
            if (projection.percent !== null) summary.push(`${percent(projection.percent)} used`);
            const section = box(true, 'tmb-spend');
            const heading = box(false, 'tmb-spend-heading');
            heading.add_child(label(id === 'claude' ? COPY.usageCredits : COPY.spendControl, 'tmb-spend-title'));
            if (spend.limit_reached) heading.add_child(label(COPY.limitReached, 'tmb-spend-reached'));
            section.add_child(heading);
            section.add_child(label(summary.filter(Boolean).join(' · ') || COPY.usageUnavailable, 'tmb-spend-summary'));
            if (projection.percent !== null) {
                section.add_child(
                    meter(
                        Math.max(160, this._root.width - 60),
                        projection.percent,
                        provider.status === 'stale' ? '#808080' : usageColor(projection.percent),
                    ),
                );
            }
            if (projection.metrics.length) {
                const key = `credit:${id}`;
                const disclosure = button(
                    `${this._expandedDetails.has(key) ? '▾' : '▸'} Credit details`,
                    () => {
                        if (this._expandedDetails.has(key)) this._expandedDetails.delete(key);
                        else this._expandedDetails.add(key);
                        this._renderPage();
                    },
                    'tmb-disclosure',
                );
                disclosure.x_align = Clutter.ActorAlign.START;
                section.add_child(disclosure);
                if (this._expandedDetails.has(key)) {
                    for (const metric of projection.metrics) {
                        const [title, help] = spendMetricCopy(metric.kind);
                        let value: string;
                        if (metric.kind === 'remaining') value = credits(allowance.remaining as number);
                        else if (metric.kind === 'balance')
                            value = new Intl.NumberFormat(undefined, {
                                style: 'currency',
                                currency: spend.currency ?? 'USD',
                            }).format(spend.balance as number);
                        else if (metric.kind === 'resets') value = until(spend.reset);
                        else if (metric.kind === 'allowance_source')
                            value = humanizeIdentifier(allowance.source as string);
                        else if (metric.kind === 'auto_reload') value = autoReloadText(spend.auto_reload);
                        else value = 'Available';
                        const row = this._keyValue(title, value);
                        this._tooltip.attach(row, title, help);
                        section.add_child(row);
                    }
                }
            }
            body.add_child(section);
        }
        if (provider.usage_credits || provider.reset_credits) {
            const resets = provider.reset_credits;
            const primary: ReturnType<typeof creditMetricCopy>[] = [];
            const supporting: ReturnType<typeof creditMetricCopy>[] = [];
            for (const metric of provider.credit_metrics) {
                (metric.supporting ? supporting : primary).push(creditMetricCopy(metric, provider, id === 'codex'));
            }
            const section = box(true, 'tmb-credit-section');
            const columns = this._root.width >= 760 ? primary.length : this._root.width >= 340 ? 2 : 1;
            const metrics = box(columns < primary.length, 'tmb-credit-metrics');
            let row!: St.BoxLayout;
            for (const [index, item] of primary.entries()) {
                if (index % columns === 0) {
                    row = columns < primary.length ? box(false, 'tmb-credit-metrics-row') : metrics;
                    if (row !== metrics) metrics.add_child(row);
                }
                const tile = box(true, 'tmb-credit-tile');
                tile.add_child(label(item.title, 'tmb-secondary'));
                tile.add_child(label(item.value, 'tmb-credit-value'));
                this._tooltip.attach(tile, item.title, item.help);
                tile.x_expand = true;
                row.add_child(tile);
            }
            section.add_child(metrics);
            if (supporting.length) {
                const key = `credit-estimates:${id}`;
                const disclosure = button(
                    `${this._expandedDetails.has(key) ? '▾' : '▸'} Credit estimates and totals`,
                    () => {
                        if (this._expandedDetails.has(key)) this._expandedDetails.delete(key);
                        else this._expandedDetails.add(key);
                        this._renderPage();
                    },
                    'tmb-disclosure',
                );
                disclosure.x_align = Clutter.ActorAlign.START;
                section.add_child(disclosure);
                if (this._expandedDetails.has(key)) {
                    for (const item of supporting) {
                        const row = this._keyValue(item.title, item.value);
                        this._tooltip.attach(row, item.title, item.help);
                        section.add_child(row);
                    }
                }
            }
            if (resets?.details.length) {
                const key = `reset-credits:${id}`;
                const disclosure = button(
                    `${this._expandedDetails.has(key) ? '▾' : '▸'} Reset credit details`,
                    () => {
                        if (this._expandedDetails.has(key)) this._expandedDetails.delete(key);
                        else this._expandedDetails.add(key);
                        this._renderPage();
                    },
                    'tmb-disclosure',
                );
                disclosure.x_align = Clutter.ActorAlign.START;
                section.add_child(disclosure);
                if (this._expandedDetails.has(key)) {
                    for (const entry of resets.details) {
                        const date = (value: string): string =>
                            Number.isNaN(Date.parse(value)) ? value : new Date(value).toLocaleString();
                        const parts = [
                            entry.status,
                            entry.credit_type,
                            entry.description,
                            entry.granted_at && `Granted ${date(entry.granted_at)}`,
                            entry.expires_at && `Expires ${date(entry.expires_at)}`,
                        ].filter(Boolean);
                        const row = this._keyValue(entry.title, parts.join(' · '));
                        this._tooltip.attach(row, entry.title, entry.explanation);
                        section.add_child(row);
                    }
                }
            }
            body.add_child(section);
        }
        if (
            provider.credit_balance !== null &&
            !provider.usage_credits &&
            (provider.spend?.balance === null || provider.spend?.balance === undefined)
        )
            body.add_child(label(`Credits $${provider.credit_balance.toFixed(2)} remaining`, 'tmb-card-meta'));
        if (local) {
            const key = 'local:claude';
            const disclosure = button(
                `${this._expandedDetails.has(key) ? '▾' : '▸'} Local usage details`,
                () => {
                    if (this._expandedDetails.has(key)) this._expandedDetails.delete(key);
                    else this._expandedDetails.add(key);
                    this._renderPage();
                },
                'tmb-disclosure',
            );
            disclosure.x_align = Clutter.ActorAlign.START;
            body.add_child(disclosure);
            if (this._expandedDetails.has(key)) {
                const compact = (value: number): string =>
                    new Intl.NumberFormat(undefined, {notation: 'compact', maximumFractionDigits: 1}).format(value);
                const money = (value: number, digits: number): string =>
                    new Intl.NumberFormat(undefined, {
                        style: 'currency',
                        currency: 'USD',
                        maximumFractionDigits: digits,
                    }).format(value);
                const summary = box(true, 'tmb-local-summary');
                summary.add_child(label('LOCAL SESSION LOGS', 'tmb-eyebrow'));
                for (const metric of state.local_metrics as LocalMetric[]) {
                    const [title, help] = localMetricCopy(metric.kind);
                    let value: string;
                    switch (metric.kind) {
                        case 'window_tokens':
                            value = `${compact(local.window_tokens)} tokens`;
                            break;
                        case 'window_cost':
                            value = money(local.window_cost, metric.money_fraction_digits);
                            break;
                        case 'cost_per_hour':
                            value = `${money(local.cost_per_hour, metric.money_fraction_digits)}/hr`;
                            break;
                        case 'today_tokens':
                            value = `${compact(local.today_tokens)} tokens`;
                            break;
                        case 'today_messages':
                            value = String(local.today_messages);
                            break;
                        case 'today_cost':
                            value = money(local.today_cost, metric.money_fraction_digits);
                            break;
                    }
                    const row = this._keyValue(title, value);
                    this._tooltip.attach(row, title, help);
                    summary.add_child(row);
                }
                body.add_child(summary);
            }
        }
        if (provider.details.length) {
            const disclosure = button(
                `${this._expandedDetails.has(id) ? '▾' : '▸'} Details (${provider.details.length})`,
                () => {
                    if (this._expandedDetails.has(id)) this._expandedDetails.delete(id);
                    else this._expandedDetails.add(id);
                    this._renderPage();
                },
                'tmb-disclosure',
            );
            disclosure.x_align = Clutter.ActorAlign.START;
            body.add_child(disclosure);
            if (this._expandedDetails.has(id)) {
                for (const item of provider.details) {
                    const row = this._keyValue(item.title, item.value);
                    this._tooltip.attach(row, item.title, item.explanation);
                    body.add_child(row);
                }
            }
        }
        if (id === 'codex' && this._reviews !== null) {
            const reviews = box(false, 'tmb-review-summary');
            reviews.add_child(label(COPY.codeReviews, 'tmb-secondary'));
            reviews.add_child(label(this._reviews, 'tmb-review-count'));
            this._tooltip.attach(reviews, COPY.codeReviews, COPY.codeReviewsHelp);
            body.add_child(reviews);
        }
        const problem =
            provider.recovery_status === 'unreadable'
                ? 'Credentials could not be read'
                : provider.recovery_status === 'sign-in'
                  ? 'Sign-in needs renewal'
                  : provider.notice;
        if (problem && problem !== COPY.demoData) {
            const message = label(problem, 'tmb-card-meta');
            if (provider.recovery_status === 'unsupported') {
                message.width = Math.max(190, this._root.width - 56);
                message.clutter_text.set_line_wrap(true);
                message.clutter_text.set_ellipsize(Pango.EllipsizeMode.NONE);
            }
            body.add_child(message);
        }
    },
};

export type ExtraMethods = typeof extraMethods;
