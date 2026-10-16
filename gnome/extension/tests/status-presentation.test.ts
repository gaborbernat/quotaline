import assert from 'node:assert/strict';
import {test} from 'node:test';
import {moduleUrl, required} from './support/node.ts';
import * as fixtures from './support/state.ts';
import {provider} from './support/state.ts';

const {emptyUsageTitle, miniBarGroups, statusIconTone, statusTooltip, statusVariants} = (await import(
    moduleUrl('status-presentation.js')
)) as typeof import('../src/status-presentation.js');
const cell = (id: string, provider: string, used: number, restricted = false, limited = false) =>
    fixtures.statusCell({
        id,
        provider,
        used,
        restricted,
        limited,
        label: id,
        text: 'configured',
    });
await test('status variants keep all display tiers for an empty selection', () => {
    const variants = statusVariants([], fixtures.settings({show_as: 'used', decimals: 0}), []);
    assert.deepEqual(
        variants.map(variant => variant.tier),
        ['configured', 'stacked', 'providers', 'icon'],
    );
    assert.ok(variants.every(variant => variant.cells.length === 0));
});
await test('stacked variants format used, remaining and limited readings', () => {
    const rows = [cell('Used', 'claude', 25), cell('Limited', 'codex', 100, true, true)];
    assert.deepEqual(
        required(statusVariants(rows, fixtures.settings({show_as: 'used', decimals: 0}), [0, 1])[1]).cells.map(
            row => row.text,
        ),
        ['Used\n25%', 'Limited\nLimit'],
    );
    assert.equal(
        required(
            required(
                statusVariants(
                    [cell('Remaining', 'claude', 27.5)],
                    fixtures.settings({show_as: 'remaining', decimals: 1}),
                    [0],
                )[1],
            ).cells[0],
        ).text,
        'Remaining\n72.5%',
    );
    assert.deepEqual(
        required(statusVariants(rows, fixtures.settings({show_as: 'used', decimals: 0}), [0, 1])[0]).cells,
        rows,
    );
});
await test('provider tier renders selected Rust indices in order', () => {
    const rows = [
        cell('first', 'claude', 25),
        cell('other', 'codex', 60),
        cell('higher', 'claude', 70),
        cell('lower', 'codex', 20),
        cell('restricted', 'claude', 40, true),
        cell('equal', 'claude', 40, true),
    ];
    assert.deepEqual(
        required(statusVariants(rows, fixtures.settings({show_as: 'used', decimals: 0}), [4, 1])[2]).cells.map(
            row => row.id,
        ),
        ['restricted', 'other'],
    );
});
await test('mini bars group quotas in stable provider and window order', () => {
    const rows = [cell('first', 'claude', 25), cell('other', 'codex', 60), cell('higher', 'claude', 70, true)];
    const groups = miniBarGroups(rows, [
        {provider: 'claude', indices: [0, 2], used: 70, restricted: true},
        {provider: 'codex', indices: [1], used: 60, restricted: false},
    ]);
    assert.deepEqual(
        groups.map(group => group.provider),
        ['claude', 'codex'],
    );
    assert.deepEqual(
        required(groups[0]).bars.map(row => row.id),
        ['first', 'higher'],
    );
    assert.equal(required(groups[0]).used, 70);
    assert.equal(required(groups[0]).restricted, true);
    assert.equal(required(groups[1]).restricted, false);
    assert.deepEqual(miniBarGroups([], []), []);
});
await test('status tooltip shares one repeated restriction', () => {
    assert.equal(statusTooltip([]), 'No usage yet');
    assert.equal(
        statusTooltip([fixtures.statusCell({tooltip: 'A: 20%'}), fixtures.statusCell({tooltip: 'B: 80%'})]),
        'A: 20%\nB: 80%',
    );
    assert.equal(
        statusTooltip([
            fixtures.statusCell({tooltip: 'A: 20%\nBlocked'}),
            fixtures.statusCell({tooltip: 'B: 80%\nBlocked'}),
        ]),
        'A: 20%\nB: 80%\nBlocked',
    );
    assert.equal(
        statusTooltip([
            fixtures.statusCell({tooltip: 'A: 20%\nBlocked A'}),
            fixtures.statusCell({tooltip: 'B: 80%\nBlocked B'}),
        ]),
        'A: 20%\nBlocked A\nB: 80%\nBlocked B',
    );
});
await test('status icon tone follows quota and enabled provider state', () => {
    const providers = {
        claude: provider(fixtures.provider({status: 'connected'})),
        codex: provider(fixtures.provider({status: 'offline'})),
    };
    const enabled = fixtures.settings({providers: {claude: true, codex: true}});
    assert.equal(statusIconTone([cell('A', 'claude', 100, true)], providers, enabled), 'attention');
    assert.equal(
        statusIconTone([], {...providers, claude: fixtures.provider({status: 'sign-in'})}, enabled),
        'attention',
    );
    assert.equal(statusIconTone([], providers, enabled), 'offline');
    assert.equal(statusIconTone([], providers, fixtures.settings({providers: {claude: true, codex: false}})), 'normal');
    assert.equal(
        statusIconTone(
            [],
            {...providers, claude: fixtures.provider({status: 'offline', recovery_status: 'connected'})},
            fixtures.settings({providers: {claude: true, codex: false}}),
        ),
        'normal',
    );
});
await test('empty usage title distinguishes an enabled provider from an empty selection', () => {
    assert.equal(emptyUsageTitle(fixtures.settings({providers: {claude: true}})), 'No usage available');
    assert.equal(emptyUsageTitle(fixtures.settings({providers: {claude: false}})), 'No providers enabled');
});
