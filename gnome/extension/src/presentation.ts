import type {Provider, Window} from './service-state.js';
import {COPY} from './shared-copy.js';
import {countdown} from './time-text.js';

export interface ModelWindow {
    id: string;
    row: Window;
    provider: Provider;
}

export function until(stamp: string | null, now = Date.now()): string {
    // Date.parse(null) is NaN, which reads as an unknown reset.
    const date = Date.parse(stamp as string);
    if (Number.isNaN(date)) return 'Reset unknown';
    const seconds = (date - now) / 1000;
    return seconds > 0 ? `Resets in ${countdown(seconds)}` : COPY.resetDue;
}

export function conflictingModelName(windows: Map<string, ModelWindow>, key: string): string {
    const model = windows.get(key);
    return model ? `${model.provider.name} ${model.row.label}` : 'another model';
}
