import assert from 'node:assert/strict';
import {test} from 'node:test';
import {moduleUrl} from './support/node.ts';

const {menuWidth} = (await import(moduleUrl('menu-geometry.js'))) as typeof import('../src/menu-geometry.js');
await test('menu keeps its full width near a monitor edge', () => {
    assert.equal(menuWidth(1920, 880), 880);
});
await test('menu uses the native width when the status item has room on both sides', () => {
    assert.equal(menuWidth(1920, 860), 860);
    assert.equal(menuWidth(1920, 890), 890);
});
await test('menu fits within the monitor inset', () => {
    assert.equal(menuWidth(900, 880), 868);
    assert.equal(menuWidth(300, 880), 268);
});
