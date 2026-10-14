import assert from 'node:assert/strict';
import {test} from 'node:test';
import {moduleUrl} from './support/node.ts';

const {menuWidth} = (await import(moduleUrl('menu-geometry.js'))) as typeof import('../src/menu-geometry.js');
await test('menu stays centered on a status item near the right edge', () => {
    const monitor = {x: 0, width: 1920};
    const indicatorX = 1290;
    const indicatorWidth = 578;
    const width = menuWidth(indicatorX, indicatorWidth, monitor, 860);
    assert.equal(width, 650);
    assert.equal(indicatorX + indicatorWidth / 2 + width / 2, monitor.width - 16);
});
await test('menu uses the native width when the status item has room on both sides', () => {
    assert.equal(menuWidth(760, 80, {x: 0, width: 1920}, 860), 860);
    assert.equal(menuWidth(760, 80, {x: 0, width: 1920}, 890), 890);
});
await test('menu respects a monitor origin and its usable width', () => {
    assert.equal(menuWidth(1060, 80, {x: 500, width: 900}, 860), 620);
    assert.equal(menuWidth(100, 40, {x: 0, width: 300}, 860), 268);
});
