import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import {fileURLToPath} from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

test('site builds with versioned links and platform reference pages', context => {
    const {destination, result} = build(context);
    assert.equal(result.status, 0, result.stderr);
    const start = fs.readFileSync(path.join(destination, 'start/index.html'), 'utf8');
    assert.match(start, /\/en\/preview\/start\/connect\//);
    const menuBar = fs.readFileSync(path.join(destination, 'reference/menu-bar/index.html'), 'utf8');
    assert.match(menuBar, /\/en\/preview\/reference\/settings\/menu-bar\/#custom-templates/);
    const home = fs.readFileSync(path.join(destination, 'index.html'), 'utf8');
    assert.match(home, />Get started<\/a>/);
    assert.doesNotMatch(home, /Build from source<\/a>/);
    for (const provider of ['claude', 'codex', 'gemini', 'antigravity', 'cursor', 'copilot']) {
        assert.match(home, new RegExp(`/en/preview/reference/providers/${provider}/`));
        assert.ok(fs.statSync(path.join(destination, `reference/providers/${provider}/index.md`)).size > 0);
    }
    for (const file of ['llms.txt', 'reference/providers/index.md', 'gnome/index.html']) {
        assert.ok(fs.statSync(path.join(destination, file)).size > 0);
    }
});

test('site switches screenshots and steps with the stored desktop choice', context => {
    const {destination, result} = build(context);
    assert.equal(result.status, 0, result.stderr);
    const usage = fs.readFileSync(path.join(destination, 'reference/usage/index.html'), 'utf8');
    assert.match(usage, /class="?shot only-light only-macos"?/);
    assert.match(usage, /class="?shot only-gnome"?/);
    assert.match(usage, /<figcaption class="?only-gnome"?>/);
    const start = fs.readFileSync(path.join(destination, 'start/index.html'), 'utf8');
    assert.match(start, /<div class="?only-macos"?>[\s\S]*brew install --cask/);
    assert.match(start, /<div class="?only-gnome"?>[\s\S]*just install/);
    assert.match(start, /localStorage\.getItem\("tmb-platform"\)/);
    assert.match(start, /data-platform="?gnome"?/);
    const gnome = fs.readFileSync(path.join(destination, 'gnome/index.html'), 'utf8');
    assert.doesNotMatch(gnome, /only-macos[^-]/);
});

test('site rejects an unknown desktop', context => {
    const {result} = build(context, '{{< platform windows >}}Text{{< /platform >}}');
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /platform shortcode takes macos or gnome/);
});

test('site rejects missing GNOME screenshots', context => {
    const {result} = build(context, '{{< shot gnome="missing-gnome-image" alt="Missing" >}}');
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /Missing screenshot images\/missing-gnome-image/);
});

test('site rejects missing internal pages', context => {
    const {result} = build(context, '[Missing page](/missing-doc-page/)');
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /Unresolved documentation link/);
});

test('site rejects missing screenshots', context => {
    const {result} = build(context, '{{< shot name="missing-doc-image" alt="Missing" >}}');
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /Missing screenshot images\/missing-doc-image/);
});

function build(context, addition) {
    const temporary = path.join(root, 'target/tmp');
    fs.mkdirSync(temporary, {recursive: true});
    const scratch = fs.mkdtempSync(path.join(temporary, 'site-test-'));
    context.after(() => fs.rmSync(scratch, {recursive: true, force: true}));
    const source = path.join(scratch, 'site');
    fs.mkdirSync(source);
    for (const item of ['hugo.toml', 'content', 'layouts', 'assets', 'static', 'resources']) {
        fs.cpSync(path.join(root, 'website', item), path.join(source, item), {recursive: true});
    }
    if (addition) {
        fs.writeFileSync(path.join(source, 'content/link-check.md'), `---\ntitle: Link check\n---\n${addition}\n`);
    }
    const destination = path.join(scratch, 'output');
    const result = spawnSync('hugo', ['--source', source, '--destination', destination,
        '--baseURL', 'https://example.invalid/en/preview/', '--minify'],
    {encoding: 'utf8', env: {...process.env, TMPDIR: temporary}});
    return {destination, result};
}
