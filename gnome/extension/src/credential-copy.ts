import type {Provider} from './service-state.js';
import {COPY} from './shared-copy.js';
export function credentialHealthLabel(
    provider: Pick<Provider, 'credential_usable'> & Partial<Pick<Provider, 'credential_health'>>,
): string {
    const health = provider.credential_health ?? (provider.credential_usable ? 'valid' : 'unchecked');
    return {
        unchecked: COPY.notCheckedYet,
        missing: COPY.notFound,
        valid: 'Credential found',
        expired: 'Expired',
        unreadable: 'Unreadable',
    }[health];
}

export function credentialSourceCopy(provider: Provider): string {
    const selection = provider.account_selection;
    return selection ? `${selection.count} accounts found; showing ${selection.label}` : provider.source;
}
