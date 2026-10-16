import assert from 'node:assert/strict';
import {test} from 'node:test';
import {moduleUrl} from './support/node.ts';
import {provider, quotaWindow} from './support/state.ts';

const {conflictingModelName, until} = (await import(
    moduleUrl('presentation.js')
)) as typeof import('../src/presentation.js');
await test('reset countdown handles missing, past and longer deadlines', () => {
    const now = Date.UTC(2026, 9, 12, 12);
    for (const [stamp, expected] of [
        [null, 'Reset unknown'],
        ['invalid', 'Reset unknown'],
        [new Date(now - 1000).toISOString(), 'Reset due'],
        [new Date(now + 90 * 60000).toISOString(), 'Resets in 1 hr 30 min'],
        [new Date(now + 26 * 3600000).toISOString(), 'Resets in 1d 2h'],
    ] as const)
        assert.equal(until(stamp, now), expected);
});
await test('short label conflicts retain a useful name when the model list changes', () => {
    const windows = new Map([
        [
            'claude:session',
            {id: 'claude:session', provider: provider({name: 'Claude'}), row: quotaWindow({label: 'Session'})},
        ],
    ]);
    assert.equal(conflictingModelName(windows, 'claude:session'), 'Claude Session');
    assert.equal(conflictingModelName(windows, 'removed:model'), 'another model');
});
