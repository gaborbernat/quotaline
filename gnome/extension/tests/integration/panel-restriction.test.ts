import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Shell from 'gi://Shell';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {failure, findActor, indicator as getIndicator, instance, required} from '../support/actors.js';
import {afterPaint, waitOrFail} from '../support/shell-events.js';

function setSetting(key: string, value: unknown) {
    Gio.DBus.session.call_sync(
        'tech.bernat.Quotaline.Gnome',
        '/tech/bernat/Quotaline/Gnome',
        'tech.bernat.Quotaline.Gnome',
        'SetSetting',
        new GLib.Variant('(ss)', [key, JSON.stringify(value)]),
        new GLib.VariantType('(s)'),
        Gio.DBusCallFlags.NONE,
        2000,
        null,
    );
}

async function capture(name: string) {
    const directory = GLib.getenv('TMB_SCREENSHOT_DIR');
    if (!directory) return;
    GLib.mkdir_with_parents(directory, 0o755);
    const stream = Gio.File.new_for_path(GLib.build_filenamev([directory, `${name}.png`])).replace(
        null,
        false,
        Gio.FileCreateFlags.REPLACE_DESTINATION,
        null,
    );
    await new Promise<void>((resolve, reject) => {
        new Shell.Screenshot().screenshot(false, stream, (source, result) => {
            try {
                required(source).screenshot_finish(result);
                stream.close(null);
                resolve();
            } catch (error) {
                reject(failure(error));
            }
        });
    });
}

export async function run() {
    try {
        Main.overview.hide();
        const indicator = getIndicator();
        if (!indicator) throw new Error('Quotaline indicator is missing');
        const panelCells = required(findActor(indicator, actor => actor.has_style_class_name?.('tmb-panel-cells')));
        await waitOrFail(
            () =>
                panelCells.get_n_children() > 0 &&
                panelCells
                    .get_children()
                    .every(actor => instance(actor, St.Widget).has_style_class_name('tmb-panel-cell-stacked')) &&
                findActor(
                    panelCells,
                    actor => actor.has_style_class_name?.('tmb-panel-value') && actor.text === 'Limit',
                ),
            `Adaptive panel lost the hidden exhausted quota: ${panelCells
                .get_children()
                .map(
                    actor =>
                        `${instance(actor, St.Widget).style_class} ${actor
                            .get_children()
                            .map(child => child.text ?? '')
                            .join(' ')}`,
                )
                .join(' | ')}`,
        );
        await afterPaint();
        await capture('panel-broader-limit-adaptive');
        instance(indicator, St.Widget).set_hover(true);
        await waitOrFail(
            () =>
                findActor(
                    Main.layoutManager.uiGroup,
                    actor =>
                        actor.has_style_class_name?.('tmb-tooltip') &&
                        actor.visible &&
                        findActor(
                            actor,
                            child =>
                                child.text?.includes('Current session: 36%, resets —') &&
                                child.text.includes('Included quota limited by All models') &&
                                child.text.includes('Haiku: 70%') &&
                                child.text.split('Included quota limited by').length === 2,
                        ),
                ),
            'Panel help omitted the selected usage or blocking quota',
        );
        await afterPaint();
        await capture('panel-broader-limit-help');
        instance(indicator, St.Widget).set_hover(false);
        setSetting('fit_to_space', false);
        await waitOrFail(
            () =>
                panelCells.get_n_children() === 7 &&
                panelCells
                    .get_children()
                    .every(actor => !instance(actor, St.Widget).has_style_class_name('tmb-panel-cell-stacked')) &&
                findActor(panelCells, actor => actor.text?.includes('Limit')),
            'Configured panel lost the hidden exhausted quota',
        );
        setSetting('selected_windows', ['claude:session']);
        setSetting('format', 'mini-bars');
        await waitOrFail(
            () =>
                panelCells.get_n_children() === 1 &&
                findActor(panelCells, actor => actor.has_style_class_name?.('tmb-panel-mini-fill'))?.width === 30,
            'A restricted quota did not fill its mini bar',
        );
        setSetting('format', 'exhausted');
        await waitOrFail(
            () =>
                panelCells.get_n_children() === 1 &&
                instance(required(panelCells.get_child_at_index(0)), St.Widget).has_style_class_name(
                    'tmb-panel-cell-stacked',
                ) &&
                findActor(
                    panelCells,
                    actor => actor.has_style_class_name?.('tmb-panel-value') && /^2d\dh$/.test(actor.text ?? ''),
                ),
            'Countdown at 100% did not use a second line or the blocking reset',
        );
        await afterPaint();
        await capture('panel-broader-limit-countdown');
    } finally {
        const directory = GLib.getenv('TMB_JS_COVERAGE_DIR');
        if (directory) {
            GLib.mkdir_with_parents(directory, 0o755);
            GLib.file_set_contents(
                GLib.build_filenamev([directory, `${GLib.uuid_string_random()}.json`]),
                JSON.stringify(globalThis.__coverage__ ?? {}),
            );
        }
    }
}

export function finish() {}
