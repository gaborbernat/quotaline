import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import {pathToFileURL} from 'node:url';

export function moduleUrl(name: string): string {
    const {TMB_EXTENSION_DIR} = process.env;
    return pathToFileURL(
        path.join(TMB_EXTENSION_DIR ?? path.resolve(import.meta.dirname, '../../../../target/extension'), 'src', name),
    ).href;
}

export function required<Value>(value: Value | null | undefined): NonNullable<Value> {
    assert.ok(value !== null && value !== undefined, 'Expected a test value');
    return value;
}

process.once('exit', () => {
    const {TMB_JS_COVERAGE_DIR} = process.env;
    if (!TMB_JS_COVERAGE_DIR) return;
    fs.mkdirSync(TMB_JS_COVERAGE_DIR, {recursive: true});
    fs.writeFileSync(
        path.join(TMB_JS_COVERAGE_DIR, `${randomUUID()}.json`),
        JSON.stringify(globalThis.__coverage__ ?? {}),
    );
});
