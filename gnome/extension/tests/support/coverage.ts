import fs from 'node:fs';
import path from 'node:path';
import type {CoverageMapData} from 'istanbul-lib-coverage';

export interface MissingBranch {
    id: string;
    outcome: number;
    location?: number[];
}

export interface RustCoverageReport {
    files: {
        file: string;
        lines: {covered: number; total: number; missing: number[]};
        branches: {covered: number; total: number; missing: MissingBranch[]};
    }[];
}

interface LlvmCoverage {
    data: [{files: {filename: string; branches: [number, number, number, number, number, number, ...number[]][]}[]}];
}

const root = path.resolve(import.meta.dirname, '../../../..');
const output = path.join(root, 'target/js-coverage');

function sourceFiles(directory: string): string[] {
    return fs.readdirSync(directory, {withFileTypes: true}).flatMap(entry => {
        const source = path.join(directory, entry.name);
        return entry.isDirectory() ? sourceFiles(source) : entry.name.endsWith('.js') ? [source] : [];
    });
}

if (process.argv[2] === 'prepare') {
    const {default: instrument} = await import('istanbul-lib-instrument');
    const extension = path.join(root, 'target/extension');
    const files = [path.join(extension, 'extension.js'), ...sourceFiles(path.join(extension, 'src'))];
    fs.rmSync(output, {recursive: true, force: true});
    fs.cpSync(extension, path.join(output, 'extension'), {
        recursive: true,
        dereference: true,
        filter: source => source !== path.join(extension, 'tests'),
    });
    fs.mkdirSync(path.join(output, 'runs'));
    const baseline: CoverageMapData = {};
    for (const source of files) {
        const instrumenter = instrument.createInstrumenter({
            esModules: true,
            compact: false,
            coverageGlobalScope: 'globalThis',
            coverageGlobalScopeFunc: false,
        });
        fs.writeFileSync(
            path.join(output, 'extension', path.relative(extension, source)),
            instrumenter.instrumentSync(fs.readFileSync(source, 'utf8'), source),
        );
        baseline[source] = instrumenter.lastFileCoverage();
    }
    fs.writeFileSync(path.join(output, 'baseline.json'), JSON.stringify(baseline));
    process.stdout.write(`Instrumented ${files.length} Shell modules\n`);
} else if (process.argv[2] === 'report') {
    const {default: coverage} = await import('istanbul-lib-coverage');
    const map = coverage.createCoverageMap(
        JSON.parse(fs.readFileSync(path.join(output, 'baseline.json'), 'utf8')) as CoverageMapData,
    );
    const runs = fs.readdirSync(path.join(output, 'runs')).filter(file => file.endsWith('.json'));
    if (runs.length < 2) throw new Error(`Expected demo and local Shell coverage; found ${runs.length} run(s)`);
    for (const run of runs)
        map.merge(JSON.parse(fs.readFileSync(path.join(output, 'runs', run), 'utf8')) as CoverageMapData);
    const summary = map.getCoverageSummary().toJSON();
    for (const file of map.files()) {
        const result = map.fileCoverageFor(file).toSummary().toJSON();
        process.stdout.write(
            `${path.relative(root, file)}: ${result.lines.covered}/${result.lines.total} lines, ` +
                `${result.branches.covered}/${result.branches.total} branches\n`,
        );
    }
    fs.writeFileSync(path.join(output, 'report.json'), JSON.stringify(map.toJSON()));
    process.stdout.write(
        `Shell total: ${summary.lines.covered}/${summary.lines.total} lines, ` +
            `${summary.branches.covered}/${summary.branches.total} branches\n`,
    );
    if (
        summary.lines.total === 0 ||
        summary.branches.total === 0 ||
        summary.lines.covered !== summary.lines.total ||
        summary.branches.covered !== summary.branches.total
    )
        process.exitCode = 1;
} else if (process.argv[2] === 'report-rust') {
    const lines = new Map<string, Map<number, number>>();
    let counts = new Map<number, number>();
    for (const row of fs.readFileSync(path.join(root, 'target/coverage.lcov'), 'utf8').split('\n')) {
        if (row.startsWith('SF:')) {
            const file = row.slice(3);
            counts = lines.get(file) ?? new Map<number, number>();
            lines.set(file, counts);
        } else if (row.startsWith('DA:')) {
            const [line, count] = row.slice(3).split(',').map(Number) as [number, number];
            counts.set(line, (counts.get(line) ?? 0) + count);
        }
    }
    const data = JSON.parse(fs.readFileSync(path.join(root, 'target/coverage-detailed.json'), 'utf8')) as LlvmCoverage;
    const results = [];
    for (const entry of data.data[0].files) {
        const counts = lines.get(entry.filename);
        if (!counts) throw new Error(`LCOV has no lines for ${entry.filename}`);
        const branches = new Map<string, [number, number]>();
        for (const branch of entry.branches) {
            const key = JSON.stringify([...branch.slice(0, 4), ...branch.slice(6)]);
            const taken = branches.get(key) ?? [0, 0];
            taken[0] += branch[4];
            taken[1] += branch[5];
            branches.set(key, taken);
        }
        const missingLines = [...counts]
            .filter(([, count]) => count === 0)
            .map(([line]) => line)
            .sort((a, b) => a - b);
        const missingBranches = [...branches].flatMap(([key, taken]) =>
            taken.flatMap((count, outcome) =>
                count === 0 ? [{id: key, location: (JSON.parse(key) as number[]).slice(0, 2), outcome}] : [],
            ),
        );
        const result = {
            file: path.relative(root, entry.filename),
            lines: {covered: counts.size - missingLines.length, total: counts.size, missing: missingLines},
            branches: {
                covered: branches.size * 2 - missingBranches.length,
                total: branches.size * 2,
                missing: missingBranches,
            },
        };
        results.push(result);
        if (missingLines.length || missingBranches.length)
            process.stdout.write(
                `${result.file}: ${result.lines.covered}/${result.lines.total} lines, ` +
                    `${result.branches.covered}/${result.branches.total} branches\n`,
            );
    }
    const totals = results.reduce(
        (total, result) => {
            total.lines.covered += result.lines.covered;
            total.lines.total += result.lines.total;
            total.branches.covered += result.branches.covered;
            total.branches.total += result.branches.total;
            return total;
        },
        {lines: {covered: 0, total: 0}, branches: {covered: 0, total: 0}},
    );
    if (lines.size !== results.length) throw new Error('Rust JSON and LCOV include different source files');
    fs.writeFileSync(path.join(root, 'target/rust-coverage-report.json'), JSON.stringify({totals, files: results}));
    process.stdout.write(
        `Rust source total: ${totals.lines.covered}/${totals.lines.total} lines, ` +
            `${totals.branches.covered}/${totals.branches.total} branches\n`,
    );
} else if (process.argv[2] === 'merge-rust') {
    const [rustPath, gnomePath, destination] = process.argv.slice(3);
    if (!rustPath || !gnomePath || !destination) throw new Error('Use merge-rust RUST_REPORT GNOME_REPORT OUTPUT');
    const rust = JSON.parse(fs.readFileSync(rustPath, 'utf8')) as RustCoverageReport;
    const gnome = JSON.parse(fs.readFileSync(gnomePath, 'utf8')) as RustCoverageReport;
    const gnomeFiles = new Map(gnome.files.map(entry => [entry.file, entry]));
    if (rust.files.length !== gnomeFiles.size) throw new Error('Rust and GNOME reports include different source files');
    const results = rust.files.map(entry => {
        const peer = gnomeFiles.get(entry.file);
        if (!peer || entry.lines.total !== peer.lines.total || entry.branches.total !== peer.branches.total)
            throw new Error(`Rust and GNOME reports have different coverage targets for ${entry.file}`);
        const peerLines = new Set(peer.lines.missing);
        const peerBranches = new Set(peer.branches.missing.map(branch => `${branch.id}:${branch.outcome}`));
        const missingLines = entry.lines.missing.filter(line => peerLines.has(line));
        const missingBranches = entry.branches.missing.filter(branch =>
            peerBranches.has(`${branch.id}:${branch.outcome}`),
        );
        return {
            file: entry.file,
            lines: {covered: entry.lines.total - missingLines.length, total: entry.lines.total, missing: missingLines},
            branches: {
                covered: entry.branches.total - missingBranches.length,
                total: entry.branches.total,
                missing: missingBranches,
            },
        };
    });
    const totals = results.reduce(
        (total, result) => {
            total.lines.covered += result.lines.covered;
            total.lines.total += result.lines.total;
            total.branches.covered += result.branches.covered;
            total.branches.total += result.branches.total;
            return total;
        },
        {lines: {covered: 0, total: 0}, branches: {covered: 0, total: 0}},
    );
    fs.writeFileSync(destination, JSON.stringify({totals, files: results}));
    process.stdout.write(
        `Rust source total: ${totals.lines.covered}/${totals.lines.total} lines, ` +
            `${totals.branches.covered}/${totals.branches.total} branches\n`,
    );
    if (
        !totals.lines.total ||
        !totals.branches.total ||
        totals.lines.covered !== totals.lines.total ||
        totals.branches.covered !== totals.branches.total
    )
        process.exitCode = 1;
} else {
    throw new Error('Use prepare, report, report-rust, or merge-rust');
}
