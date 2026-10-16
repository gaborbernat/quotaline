import assert from 'node:assert/strict';
import {test} from 'node:test';
import type {HistoryPoint} from '../src/service-state.js';
import {moduleUrl} from './support/node.ts';
import {historySeries} from './support/state.ts';

const {historyValue, readingAt, valueAt} = (await import(
    moduleUrl('chart-data.js')
)) as typeof import('../src/chart-data.js');
await test('history values use the axis unit and compact count suffix', () => {
    assert.equal(historyValue(12.6, '%'), '13%');
    assert.equal(
        historyValue(12.5, 'USD'),
        new Intl.NumberFormat(undefined, {
            style: 'currency',
            currency: 'USD',
            minimumFractionDigits: 2,
            maximumFractionDigits: 2,
        }).format(12.5),
    );
    assert.equal(historyValue(999.4, 'count'), '999');
    assert.equal(historyValue(9.45, 'count'), '9');
    assert.equal(historyValue(1250, 'count'), '1.3K');
    assert.equal(historyValue(12500, 'count'), '13K');
    assert.equal(historyValue(1250000, 'count'), '1.3M');
    assert.equal(historyValue(1250000000, 'count'), '1.3B');
    assert.equal(historyValue(-1250, 'count'), '-1.3K');
});
await test('chart reading selection follows visible series and pointer position', () => {
    const series = [
        historySeries({
            id: 'shown',
            points: [
                [10, 20, null, false],
                [60, 40, null, false],
                [95, 80, null, false],
            ],
        }),
        historySeries({id: 'hidden', points: [[50, 90, null, false]]}),
    ];
    const shown = new Set(['shown']);
    assert.equal(readingAt(series, shown, 0, 100, 200, 12), 10);
    assert.equal(readingAt(series, shown, 0, 100, 200, 162), 95);
    assert.equal(readingAt(series, shown, 0, 100, 200, 87), 60);
    assert.equal(readingAt(series, new Set(['hidden']), 0, 100, 200, 87), 50);
    assert.equal(readingAt(series, new Set(), 0, 100, 200, 87), null);
    assert.equal(readingAt(series, shown, 0, 100, 20, 1000), 95);
    assert.equal(readingAt(series, shown, 0, 100, 200, -1000), 10);
});
await test('history inspection steps usage and interpolates continuous balances', () => {
    const points: HistoryPoint[] = [
        [10, 20, null, false],
        [20, 80, null, false],
    ];
    assert.equal(valueAt(points, 10, 'usage'), 20);
    assert.equal(valueAt(points, 15, 'usage'), 20);
    assert.equal(valueAt(points, 15, 'surfaceUsagePercent'), 50);
    assert.equal(valueAt(points, 15, 'creditBalance'), 50);
    assert.equal(valueAt(points, 15, 'modelCredits'), null);
    assert.equal(valueAt(points, 0, 'usage'), null);
    assert.equal(valueAt(points, 30, 'usage'), null);
});
