import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {test} from 'node:test';
import {moduleUrl} from './support/node.ts';

const {compactCountdown, countdown, relativeAge} = (await import(
    moduleUrl('time-text.js')
)) as typeof import('../src/time-text.js');
const cases = JSON.parse(
    fs.readFileSync(
        path.resolve(import.meta.dirname, '../../../crates/presentation/text/tests/fixtures/time-text.json'),
        'utf8',
    ),
) as Record<'countdown' | 'compact_countdown' | 'relative_age', [number, string][]>;
await test('durations match the cases Rust words', () => {
    for (const [name, format] of [
        ['countdown', countdown],
        ['compact_countdown', compactCountdown],
        ['relative_age', relativeAge],
    ] as const) {
        for (const [seconds, expected] of cases[name]) assert.equal(format(seconds), expected, `${name} ${seconds}`);
    }
});
