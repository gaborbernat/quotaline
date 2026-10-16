import assert from 'node:assert/strict';
import {test} from 'node:test';
import {moduleUrl} from './support/node.ts';
import * as fixtures from './support/state.ts';
import {historySeries} from './support/state.ts';

const {historyAttribution, historySeriesLabel, presentHistory} = (await import(
    moduleUrl('history-presentation.js')
)) as typeof import('../src/history-presentation.js');
const definitions = [
    fixtures.metricDefinition({
        id: 'windowUsagePercent',
        suppliers: ['claude', 'codex'],
        attribution: 'Every model · step line',
    }),
    fixtures.metricDefinition({
        id: 'inputTokens',
        suppliers: ['claude', 'codex'],
        attribution: 'Claude + Codex · by model · daily UTC',
    }),
    fixtures.metricDefinition({
        id: 'turns',
        suppliers: ['codex'],
        attribution: 'Codex · by model and surface · daily UTC',
    }),
    fixtures.metricDefinition({id: 'costUSD', suppliers: ['claude'], attribution: 'Claude · by model · daily UTC'}),
];
const providers = [
    fixtures.providerDefinition({id: 'claude', name: 'Claude'}),
    fixtures.providerDefinition({id: 'codex', name: 'Codex'}),
];
await test('history attribution follows the metric suppliers and contributing series', () => {
    assert.equal(historyAttribution('unknown', ['claude'], definitions, providers), null);
    assert.equal(historyAttribution('turns', [], definitions, providers), 'No enabled provider data in this period');
    assert.equal(
        historyAttribution('windowUsagePercent', ['claude'], definitions, providers),
        'Claude · enabled models · step line · selected time zone',
    );
    assert.equal(
        historyAttribution('inputTokens', ['claude', 'codex'], definitions, providers),
        'Claude + Codex · Claude by model, Codex total · daily UTC',
    );
    assert.equal(
        historyAttribution('inputTokens', ['claude'], definitions, providers),
        'Claude · by model · daily UTC',
    );
    assert.equal(historyAttribution('inputTokens', ['codex'], definitions, providers), 'Codex · total · daily UTC');
    assert.equal(
        historyAttribution('turns', ['claude', 'codex'], definitions, providers),
        'Codex · by model and surface · daily UTC',
    );
});
await test('history attribution retains an unrecognized supplier identifier', () => {
    assert.equal(historyAttribution('turns', ['codex'], definitions, []), 'codex · by model and surface · daily UTC');
});
await test('history series labels use the native provider and breakdown names', () => {
    assert.equal(historySeriesLabel('usage', 'claude', 'session', definitions, providers), 'Claude session');
    assert.equal(
        historySeriesLabel('windowUsagePercent', 'codex', 'model:gpt', definitions, providers),
        'Codex Model · gpt',
    );
    assert.equal(
        historySeriesLabel('inputTokens', 'claude', 'model:sonnet', definitions, providers),
        'Claude · Model · sonnet',
    );
    assert.equal(
        historySeriesLabel('inputTokens', 'codex', 'surface:cli', definitions, providers),
        'Codex · Surface · cli',
    );
    assert.equal(historySeriesLabel('costUSD', 'claude', 'repo', definitions, providers), 'repo');
    assert.equal(historySeriesLabel('costUSD', 'future', 'surface:web', definitions, providers), 'Surface · web');
});
await test('history projection keeps private project labels and contributing providers', () => {
    const history = fixtures.history({
        metric: 'usage',
        project_labels: {'/private/work': 'Private project'},
        series: [
            historySeries({
                id: 'claude:work',
                provider: 'claude',
                label: '/private/work',
                points: [[100, 20, null, false]],
            }),
            historySeries({id: 'codex:empty', provider: 'codex', label: 'weekly', points: []}),
        ],
    });
    const shown = presentHistory(history, definitions, providers);
    assert.equal(shown.display_labels['claude:work'], 'Claude Private project');
    assert.equal(shown.display_labels['codex:empty'], 'Codex weekly');
    assert.equal(shown.attribution, 'Claude · enabled models · step line · selected time zone');
});
