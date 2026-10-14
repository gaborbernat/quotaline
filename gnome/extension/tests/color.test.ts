import assert from 'node:assert/strict';
import {test} from 'node:test';
import {moduleUrl} from './support/node.ts';

const {usageColor, usageTextColor} = (await import(moduleUrl('color.js'))) as typeof import('../src/color.js');
function luminance(color: string): number {
    const linear = (offset: number) => {
        const value = parseInt(color.slice(offset, offset + 2), 16) / 255;
        return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
    };
    return 0.2126 * linear(1) + 0.7152 * linear(3) + 0.0722 * linear(5);
}
await test('usage meter palette follows percentage and pace', () => {
    for (const [percent, color] of [
        [0, '#258540'],
        [30, '#419f2e'],
        [60, '#f09b4d'],
        [100, '#e6a993'],
    ] as const)
        assert.equal(usageColor(percent), color);
    assert.equal(usageColor(-5), usageColor(0));
    assert.equal(usageColor(105), usageColor(100));
    assert.equal(usageColor(20, 'ahead'), usageColor(60));
    assert.equal(usageColor(20, 'exhausted'), usageColor(100));
});
await test('panel text palette follows percentage and pace', () => {
    for (const [percent, color] of [
        [0, '#9ff5b7'],
        [60, '#ffd7b3'],
        [100, '#ffd8d1'],
    ] as const)
        assert.equal(usageTextColor(percent), color);
    assert.equal(usageTextColor(20, 'ahead'), usageTextColor(60));
    assert.equal(usageTextColor(20, 'exhausted'), usageTextColor(100));
});
await test('high usage meters meet dark tile contrast', () => {
    const darkTile = luminance('#3b3b3e');
    for (let percent = 60; percent <= 100; percent++)
        assert.ok((luminance(usageColor(percent)) + 0.05) / (darkTile + 0.05) >= 4.5, `${percent}%`);
});
await test('panel text meets dark and light contrast', () => {
    const activePanel = luminance('#5e5e61');
    const lightPanel = luminance('#bdbdbf');
    for (let percent = 0; percent <= 100; percent++) {
        assert.ok((luminance(usageTextColor(percent)) + 0.05) / (activePanel + 0.05) >= 4.5, `${percent}% dark`);
        assert.ok(
            (lightPanel + 0.05) / (luminance(usageTextColor(percent, null, true)) + 0.05) >= 4.5,
            `${percent}% light`,
        );
    }
});
