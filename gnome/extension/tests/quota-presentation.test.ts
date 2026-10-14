import assert from 'node:assert/strict';
import {test} from 'node:test';
import {moduleUrl} from './support/node.ts';
import {quotaWindow} from './support/state.ts';

const {paceLabels, quotaPresentation} = (await import(
    moduleUrl('quota-presentation.js')
)) as typeof import('../src/quota-presentation.js');
function window() {
    return quotaWindow({
        id: 'session',
        detail: '',
        duration_seconds: null,
        used: 42,
        pace: {status: 'on_track', expected_percent: 40, ratio: 1.05, projected_exhaustion: null},
    });
}
await test('pace labels use native titles and optional pace facts', () => {
    for (const [status, full, compact] of [
        ['unknown', 'Learning pace', 'Learning pace'],
        ['on_track', 'On pace', 'On pace'],
        ['ahead', 'Ahead of pace', 'Ahead'],
        ['behind', 'Under pace', 'Under pace'],
        ['exhausted', 'Limit reached', 'Limit reached'],
    ] as const) {
        const pace = {status, expected_percent: null, ratio: null, projected_exhaustion: null};
        assert.deepEqual(paceLabels(pace, 100), {full, compact, brief: compact}, status);
    }
});
await test('pace labels show expected usage and exhaustion time', () => {
    const pace = window().pace;
    assert.deepEqual(paceLabels(pace, 100), {
        full: 'On pace · expected 40% · 1.1× · lasts to reset',
        compact: 'On pace · exp 40% · 1.1× · lasts to reset',
        brief: 'On pace · exp 40%',
    });
    pace.projected_exhaustion = 7300;
    assert.equal(paceLabels(pace, 100).compact, 'On pace · exp 40% · 1.1× · hits 100% in 2h 0m');
    pace.projected_exhaustion = 90100;
    assert.equal(paceLabels(pace, 100).compact, 'On pace · exp 40% · 1.1× · hits 100% in 1d 1h');
    pace.projected_exhaustion = 99;
    assert.ok(paceLabels(pace, 100).full.endsWith('hits 100% in 0h 0m'));
});
await test('quota copy uses provider detail, duration, pace and severity', () => {
    const row = {...window(), duration_seconds: 18000, severity: 'warning'};
    assert.deepEqual(quotaPresentation(row, 100), {
        detail: 'window · 5h',
        help:
            'Used 42.0%; 58.0% remains. Window duration: 5h. Even pace would be 40% by now. ' +
            'Pace ratio: 1.05×. On pace · expected 40% · 1.1× · lasts to reset. Severity: warning.',
        pace_labels: {
            full: 'On pace · expected 40% · 1.1× · lasts to reset',
            compact: 'On pace · exp 40% · 1.1× · lasts to reset',
            brief: 'On pace · exp 40%',
        },
    });
    row.detail = 'Provider detail';
    assert.equal(quotaPresentation(row, 100).detail, 'Provider detail');
});
await test('quota copy names custom windows and whole day durations', () => {
    for (const [id, duration, detail] of [
        ['weekly', null, 'window'],
        ['model-1', null, 'model-1'],
        ['session', 900, 'window · 15m'],
        ['session', 86400, 'window · 24h'],
        ['session', 172800, 'window · 2d'],
        ['session', 90000, 'window · 25h'],
    ] as const)
        assert.equal(quotaPresentation({...window(), id, duration_seconds: duration}, 100).detail, detail);
});
await test('quota copy clamps inconsistent percentages and skips absent facts', () => {
    const row = window();
    row.used = 150;
    row.pace.expected_percent = null;
    row.pace.ratio = null;
    assert.equal(quotaPresentation(row, 100).help, 'Used 100.0%; 0.0% remains. On pace.');
    row.pace.expected_percent = 150;
    assert.ok(quotaPresentation(row, 100).help.includes('Even pace would be 100% by now.'));
    row.duration_seconds = 0;
    assert.equal(quotaPresentation(row, 100).detail, 'window');
});
