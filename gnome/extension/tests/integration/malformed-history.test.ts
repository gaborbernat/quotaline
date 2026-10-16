import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {buttonLabel, containsText, findActor, indicator as getIndicator, required} from '../support/actors.js';
import {waitOrFail} from '../support/shell-events.js';

function waitFor(check: () => unknown) {
    return waitOrFail(check, 'Malformed provider response did not reach the menu');
}

export async function run() {
    try {
        Main.overview.hide();
        const indicator = getIndicator();
        if (!indicator) throw new Error('Quotaline indicator is missing');
        indicator.menu.open(false);
        await waitFor(() => containsText(indicator.menu.box, 'Cost history error'));
        if (!containsText(indicator.menu.box, 'Cost history unavailable'))
            throw new Error('Provider card did not explain that cost history is unavailable');
        required(findActor(indicator.menu.box, actor => actor.accessible_name === 'History')).emit('clicked', 1);
        await waitFor(() => containsText(indicator.menu.box, 'History unavailable'));
        if (
            !containsText(indicator.menu.box, 'History error') ||
            !findActor(indicator.menu.box, actor => buttonLabel(actor) === 'Retry')
        )
            throw new Error('Failed History load did not offer a retry');
        required(findActor(indicator.menu.box, actor => actor.accessible_name === 'Settings')).emit('clicked', 1);
        await waitFor(() =>
            findActor(indicator.menu.box, actor => actor.accessible_name === 'Connection details for Claude'),
        );
        required(
            findActor(indicator.menu.box, actor => actor.accessible_name === 'Connection details for Claude'),
        ).emit('clicked', 1);
        const launch = Gio.AppInfo.launch_default_for_uri.bind(Gio.AppInfo);
        try {
            const uri = await new Promise<string>(resolve => {
                Gio.AppInfo.launch_default_for_uri = value => {
                    resolve(value);
                    return true;
                };
                required(findActor(indicator.menu.box, actor => actor.accessible_name === 'Sign in to Claude')).emit(
                    'clicked',
                    1,
                );
            });
            if (!uri.endsWith('/provider-setup.html#claude'))
                throw new Error(`Failed sign-in opened the wrong guide: ${uri}`);
        } finally {
            Gio.AppInfo.launch_default_for_uri = launch;
        }
        indicator.menu.close(false);
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
