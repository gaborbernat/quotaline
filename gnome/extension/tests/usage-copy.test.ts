import assert from 'node:assert/strict';
import {test} from 'node:test';
import type {CreditMetric} from '../src/service-state.js';
import {moduleUrl} from './support/node.ts';
import * as fixtures from './support/state.ts';

const {
    autoReloadText,
    cloudCreditStatus,
    creditMetricCopy,
    humanizeIdentifier,
    identityChips,
    localMetricCopy,
    spendMetricCopy,
    reviewCountText,
    spendStatus,
} = (await import(moduleUrl('usage-copy.js'))) as typeof import('../src/usage-copy.js');
await test('local metrics keep native titles and explanations', () => {
    assert.deepEqual(
        (
            ['window_tokens', 'window_cost', 'cost_per_hour', 'today_tokens', 'today_messages', 'today_cost'] as const
        ).map(localMetricCopy),
        [
            ['5-hour block', 'Tokens the CLI logged since the session window started, including cache reads.'],
            ['Block cost', 'API list-price equivalent for the same traffic; subscriptions are not billed per token.'],
            ['Burn rate', 'API-equivalent spend per hour over the current block.'],
            ['Today', 'Tokens logged today, including cache reads.'],
            ['Messages today', 'Assistant messages logged today.'],
            ['Today cost', "API-equivalent cost of today's traffic."],
        ],
    );
});
await test('cloud credit status distinguishes active, expired and locked credit', () => {
    assert.equal(cloudCreditStatus({used_percent: 0, status: 'active', expiry_epoch: null}), '');
    assert.equal(
        cloudCreditStatus({used_percent: 0, status: 'active', expiry_epoch: 200}),
        `Expires ${new Date(200000).toLocaleString()}`,
    );
    assert.equal(cloudCreditStatus({used_percent: 0, status: 'expired', expiry_epoch: null}), 'Expired');
    assert.equal(cloudCreditStatus({used_percent: 0, status: 'locked', expiry_epoch: null}), 'Unavailable');
});
const provider = fixtures.provider({
    credit_balance: 12.5,
    usage_credits: {
        has_credits: true,
        overage_limit_reached: true,
        unlimited: false,
        local_messages: [2, 4],
        cloud_messages: [6, 8],
    },
    reset_credits: {available: 3, applicable: 2, total_earned: 5, purchase_eligible: true, details: []},
});
for (const [kind, value] of [
    ['overage', 'Limit reached'],
    ['local_messages', '~2–4'],
    ['cloud_messages', '~6–8'],
    ['limit_resets', '3 available'],
    ['reset_purchase', 'Eligible'],
    ['resets_earned', '5'],
] as const) {
    await test(`credit ${kind} uses native text`, () => {
        const copy = creditMetricCopy({kind, supporting: false, epoch: null}, provider, false);
        assert.equal(copy.value, value);
        assert.ok(copy.title.length > 0);
        assert.ok(copy.help.length > 0);
    });
}
await test('credit balance distinguishes reported, unlimited and absent values', () => {
    const metric: CreditMetric = {kind: 'credits', supporting: false, epoch: null};
    assert.equal(
        creditMetricCopy(metric, provider, false).value,
        new Intl.NumberFormat(undefined, {style: 'currency', currency: 'USD'}).format(12.5),
    );
    assert.equal(creditMetricCopy(metric, {...provider, credit_balance: null}, false).value, 'Balance not reported');
    assert.equal(
        creditMetricCopy(
            metric,
            {
                ...provider,
                usage_credits: {
                    unlimited: true,
                    has_credits: true,
                    overage_limit_reached: false,
                    local_messages: null,
                    cloud_messages: null,
                },
            },
            false,
        ).value,
        'Unlimited',
    );
    assert.ok(creditMetricCopy(metric, provider, true).help.includes('ChatGPT Work and Codex'));
});
await test('reset-credit expiry uses the projected earliest deadline', () => {
    assert.equal(
        creditMetricCopy({kind: 'next_expiry', epoch: 200, supporting: false}, provider, false).value,
        new Date(200000).toLocaleString(),
    );
});
await test('spend copy keeps enablement and provider reason', () => {
    assert.equal(spendStatus(fixtures.spend({enabled: null})), 'Enablement not reported');
    assert.equal(spendStatus(fixtures.spend({enabled: true})), null);
    assert.equal(spendStatus(fixtures.spend({enabled: false, disabled_reason: null})), 'Off');
    assert.equal(
        spendStatus(fixtures.spend({enabled: false, disabled_reason: 'org_level-disabled'})),
        'Off (Org Level Disabled)',
    );
    assert.equal(humanizeIdentifier('workspace_owner'), 'Workspace Owner');
});
await test('spend detail kinds keep native titles and explanations', () => {
    const kinds = ['remaining', 'allowance_source', 'balance', 'resets', 'auto_reload', 'purchase'] as const;
    assert.deepEqual(
        kinds.map(kind => spendMetricCopy(kind)[0]),
        ['Remaining', 'Allowance source', 'Balance', 'Resets', 'Auto-reload', 'Purchase'],
    );
    assert.ok(kinds.every(kind => spendMetricCopy(kind)[1].length > 0));
});
await test('auto reload text follows its current setting', () => {
    assert.equal(autoReloadText(true), 'On');
    assert.equal(autoReloadText(false), 'Off');
});
await test('identity chips use masked account fields and shared workspace visibility', () => {
    const provider = fixtures.provider({
        plan: 'Pro',
        identity: 'account',
        organization: 'workspace',
        show_organization: true,
        subscription_active_until: '2026-11-04',
    });
    assert.deepEqual(identityChips(provider), [
        {kind: 'plan', value: 'Pro'},
        {kind: 'account', value: 'account'},
        {kind: 'workspace', value: 'workspace'},
        {kind: 'renewal', value: '2026-11-04'},
    ]);
    assert.deepEqual(
        identityChips({...provider, show_organization: false}).map(chip => chip.kind),
        ['plan', 'account', 'renewal'],
    );
    assert.deepEqual(identityChips(fixtures.provider()), []);
});
await test('review counts become provider card copy', () => {
    assert.equal(reviewCountText({today: 5, week: 12}), '5 today · 12 this week');
});
