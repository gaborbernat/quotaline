import assert from 'node:assert/strict';
import {test} from 'node:test';
import {moduleUrl, required} from './support/node.ts';
import * as fixtures from './support/state.ts';
import {panelProvider, quotaWindow} from './support/state.ts';

const {snapshotRows, snapshotStale} = (await import(
    moduleUrl('widget-data.js')
)) as typeof import('../src/widget-data.js');
await test('an empty snapshot has no stale quota', () => {
    assert.equal(snapshotStale([], 4000), false);
});
await test('a current quota keeps the snapshot fresh', () => {
    assert.equal(
        snapshotStale(
            [
                {
                    ...{id: 'claude:session', provider: 'Claude', label: 'Session', used: 0, reset: null},
                    updated: 1,
                    stale: true,
                },
                {
                    ...{id: 'claude:session', provider: 'Claude', label: 'Session', used: 0, reset: null},
                    updated: 4000,
                    stale: false,
                },
            ],
            4000,
        ),
        false,
    );
});
await test('a quota older than one hour is stale', () => {
    assert.equal(
        snapshotStale(
            [
                {
                    ...{id: 'claude:session', provider: 'Claude', label: 'Session', used: 0, reset: null},
                    updated: 1,
                    stale: false,
                },
            ],
            4000,
        ),
        true,
    );
});
await test('a quota with no reading is stale', () => {
    assert.equal(
        snapshotStale(
            [
                {
                    ...{id: 'claude:session', provider: 'Claude', label: 'Session', used: 0, reset: null},
                    updated: null,
                    stale: false,
                },
            ],
            4000,
        ),
        true,
    );
});
await test('a provider marked stale overrides its recent timestamp', () => {
    assert.equal(
        snapshotStale(
            [
                {
                    ...{id: 'claude:session', provider: 'Claude', label: 'Session', used: 0, reset: null},
                    updated: 4000,
                    stale: true,
                },
            ],
            4000,
        ),
        true,
    );
});
await test('snapshot rows keep raw provider quota values', () => {
    const cells = [
        fixtures.statusCell({id: 'claude:session', provider: 'claude', used: 100}),
        fixtures.statusCell({id: 'codex:missing', provider: 'codex'}),
    ];
    const providers = {
        claude: panelProvider({
            name: 'Claude',
            status: 'stale',
            updated: 50,
            windows: [quotaWindow({id: 'session', label: 'Current session', used: 25, reset: null})],
        }),
    };
    assert.deepEqual(snapshotRows(cells, providers), [
        {
            id: 'claude:session',
            provider: 'Claude',
            label: 'Current session',
            used: 25,
            reset: null,
            updated: 50,
            stale: true,
        },
    ]);
    providers.claude.status = 'connected';
    assert.equal(required(snapshotRows(cells, providers)[0]).stale, false);
});
