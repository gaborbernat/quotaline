import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import test, {type TestContext} from 'node:test';
import type {MissingBranch, RustCoverageReport} from './coverage.ts';
import {required} from './node.ts';

await test('merge-rust combines complementary line and branch hits', context => {
    const {result, output} = merge(context, report([2], [{id: 'a', outcome: 0}]), report([1], [{id: 'a', outcome: 1}]));
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(JSON.parse(fs.readFileSync(output, 'utf8')) as unknown, {
        totals: {lines: {covered: 2, total: 2}, branches: {covered: 2, total: 2}},
        files: report([], []).files,
    });
});

await test('merge-rust rejects a shared uncovered branch', context => {
    const {result} = merge(context, report([], [{id: 'a', outcome: 0}]), report([], [{id: 'a', outcome: 0}]));
    assert.equal(result.status, 1);
    assert.match(result.stdout, /1\/2 branches/);
});

await test('merge-rust rejects reports with different source files', context => {
    const gnome = report([], []);
    required(gnome.files[0]).file = 'src/other.rs';
    const {result} = merge(context, report([], []), gnome);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /different coverage targets/);
});

await test('merge-rust rejects reports with different branch totals', context => {
    const gnome = report([], []);
    required(gnome.files[0]).branches.total = 4;
    const {result} = merge(context, report([], []), gnome);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /different coverage targets/);
});

function merge(context: TestContext, rust: RustCoverageReport, gnome: RustCoverageReport) {
    const root = path.resolve(import.meta.dirname, '../../../..');
    fs.mkdirSync(path.join(root, 'target/tmp'), {recursive: true});
    const directory = fs.mkdtempSync(path.join(root, 'target/tmp/coverage-merge-'));
    context.after(() => {
        fs.rmSync(directory, {recursive: true, force: true});
    });
    const rustPath = path.join(directory, 'rust.json');
    const gnomePath = path.join(directory, 'gnome.json');
    const output = path.join(directory, 'merged.json');
    fs.writeFileSync(rustPath, JSON.stringify(rust));
    fs.writeFileSync(gnomePath, JSON.stringify(gnome));
    return {
        result: spawnSync(
            process.execPath,
            [path.join(root, 'gnome/extension/tests/support/coverage.ts'), 'merge-rust', rustPath, gnomePath, output],
            {encoding: 'utf8'},
        ),
        output,
    };
}

function report(missingLines: number[], missingBranches: MissingBranch[]) {
    return {
        files: [
            {
                file: 'src/example.rs',
                lines: {covered: 2 - missingLines.length, total: 2, missing: missingLines},
                branches: {covered: 2 - missingBranches.length, total: 2, missing: missingBranches},
            },
        ],
    };
}
