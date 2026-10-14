import assert from 'node:assert/strict';
import {test} from 'node:test';
import {moduleUrl} from './support/node.ts';
import * as fixtures from './support/state.ts';
import {provider} from './support/state.ts';

const {credentialHealthLabel, credentialSourceCopy} = (await import(
    moduleUrl('credential-copy.js')
)) as typeof import('../src/credential-copy.js');
await test('credential health copy names every state', () => {
    for (const [health, label] of [
        ['unchecked', 'Not checked yet'],
        ['missing', 'Not found'],
        ['valid', 'Credential found'],
        ['expired', 'Expired'],
        ['unreadable', 'Unreadable'],
    ] as const)
        assert.equal(credentialHealthLabel({credential_health: health, credential_usable: false}), label);
});
await test('credential health copy falls back to credential usability', () => {
    assert.equal(credentialHealthLabel({credential_usable: true}), 'Credential found');
    assert.equal(credentialHealthLabel({credential_usable: false}), 'Not checked yet');
});
await test('credential source copy names selected accounts', () => {
    assert.equal(credentialSourceCopy(provider(fixtures.provider({source: 'Keychain item'}))), 'Keychain item');
    assert.equal(
        credentialSourceCopy(
            provider(
                fixtures.provider({source: 'Keychain item', account_selection: {count: 2, label: 'work@example.com'}}),
            ),
        ),
        '2 accounts found; showing work@example.com',
    );
});
