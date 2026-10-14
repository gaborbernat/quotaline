import assert from 'node:assert/strict';
import {test} from 'node:test';
import {moduleUrl} from './support/node.ts';
import {provider, providerDefinition, quotaWindow, serviceState, settings} from './support/state.ts';

const {availabilityTitle, nextAgeUpdateMs, relativeAge, timePresentation} = (await import(
    moduleUrl('time-presentation.js')
)) as typeof import('../src/time-presentation.js');
function state() {
    return serviceState(
        {
            claude: provider({
                status: 'connected',
                recovery_status: null,
                updated: 100,
                windows: [quotaWindow({id: 'session'})],
            }),
            codex: provider({status: 'offline', recovery_status: null, updated: null, windows: []}),
        },
        {
            provider_definitions: [providerDefinition({id: 'claude'}), providerDefinition({id: 'codex'})],
            settings: settings({
                demo: false,
                has_custom_selection: false,
                selected_windows: [],
                providers: {claude: true},
            }),
            selected_window_keys: ['claude:session'],
            refreshing_providers: [],
            last_refresh: 100,
        },
    );
}
await test('availability titles follow effective service state', () => {
    for (const [status, expected] of [
        ['connected', 'Up to date'],
        ['stale', 'Showing last known values'],
        ['checking', 'Loading'],
        ['sign-in', 'Sign-in required'],
        ['offline', 'Offline'],
        ['rate-limited', 'Rate limited'],
        ['disabled', 'Disabled'],
        ['unreadable', 'Unavailable'],
    ] as const)
        assert.equal(availabilityTitle(status), expected);
});
await test('relative age uses the visible threshold units', () => {
    for (const [timestamp, now, expected] of [
        [null, 160, 'never'],
        [undefined, 160, 'never'],
        [161, 160, 'just now'],
        [157, 160, 'just now'],
        [130, 160, '30s ago'],
        [100, 160, '1 min ago'],
        [0, 7200, '2 hr ago'],
        [0, 172800, '2 d ago'],
    ] as const)
        assert.equal(relativeAge(timestamp, now), expected);
});
await test('next age update follows the next visible boundary', () => {
    for (const [timestamp, now, expected] of [
        [null, 160, null],
        [undefined, 160, null],
        [160, 160, 5001],
        [100, 104.5, 501],
        [150, 160, 1001],
        [100, 159.5, 501],
        [100, 160, 60001],
        [0, 7200, 3600001],
        [0, 172800, 86400001],
        [110, 100, 15001],
    ] as const)
        assert.equal(nextAgeUpdateMs(timestamp, now), expected);
});
await test('time presentation shows selected readings and their refresh deadline', () => {
    const result = timePresentation(state(), 160);
    assert.equal(result.header_updated, 'Updated 1 min ago');
    assert.equal(result.refresh_age, '1 min ago');
    assert.equal(result.header_activity, '');
    assert.partialDeepStrictEqual(result.provider_statuses, {claude: 'fetched 1 min ago'});
    assert.partialDeepStrictEqual(result.provider_statuses, {codex: 'Offline'});
    assert.partialDeepStrictEqual(result.provider_help, {claude: 'Values as of the last successful fetch.'});
    assert.partialDeepStrictEqual(result.provider_help, {codex: 'No successful quota reading yet.'});
    assert.equal(result.next_update_ms, 60001);
});
await test('time presentation handles missing providers and first refresh', () => {
    const input = {...state(), providers: {}};
    input.last_refresh = null;
    input.refreshing_providers = ['claude'];
    const result = timePresentation(input, 160);
    assert.equal(result.header_updated, 'Updated never');
    assert.equal(result.header_activity, 'Refreshing…');
    assert.partialDeepStrictEqual(result.provider_statuses, {claude: 'Refreshing…'});
    assert.partialDeepStrictEqual(result.provider_statuses, {codex: 'Sign-in required'});
    assert.partialDeepStrictEqual(result.provider_help, {claude: 'Fetching the first values.'});
    assert.equal(result.next_update_ms, null);
});
await test('time presentation shows recovery while keeping selected readings', () => {
    const input = state();
    input.providers.claude.status = 'stale';
    input.providers.claude.recovery_status = 'offline';
    input.refreshing_providers = ['claude'];
    let result = timePresentation(input, 160);
    assert.partialDeepStrictEqual(result.provider_statuses, {claude: 'Offline · refreshing… · showing 1 min ago'});
    assert.partialDeepStrictEqual(result.provider_help, {
        claude: 'Fetching new values; the ones below are from the last successful fetch.',
    });
    input.providers.claude.recovery_status = null;
    result = timePresentation(input, 160);
    assert.partialDeepStrictEqual(result.provider_statuses, {claude: 'refreshing… · showing 1 min ago'});
    input.refreshing_providers = [];
    result = timePresentation(input, 160);
    assert.partialDeepStrictEqual(result.provider_statuses, {claude: 'Showing last known values · 1 min ago'});
});
await test('time presentation uses projected selection and demo activity', () => {
    const input = state();
    input.selected_window_keys = [];
    input.settings.demo = true;
    input.last_refresh = null;
    const result = timePresentation(input, 160);
    assert.equal(result.header_activity, 'Demo data');
    assert.partialDeepStrictEqual(result.provider_statuses, {claude: 'Up to date'});
    assert.equal(result.next_update_ms, null);
    input.selected_window_keys = ['claude:session'];
    assert.equal(timePresentation(input, 160).next_update_ms, 60001);
    input.providers.claude.updated = null;
    assert.partialDeepStrictEqual(timePresentation(input, 160).provider_statuses, {claude: 'fetched never'});
});
