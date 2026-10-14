import GLib from 'gi://GLib';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {disableHelperAutoExit} from 'resource:///org/gnome/shell/ui/scripting.js';
import {accessibleName, indicator as getIndicator, instance, required} from '../support/actors.js';
import {afterPaint, waitOrFail} from '../support/shell-events.js';

const UUID = 'quotaline@bernat.tech';

export async function run() {
    await disableHelperAutoExit();
    const cases = JSON.parse(required(GLib.getenv('TMB_SHELL_CASES'))) as {
        script: string;
        variables?: Record<string, string>;
    }[];
    const keys = [...new Set(cases.flatMap(item => Object.keys(item.variables ?? {})))];
    const resetBefore = Number(GLib.getenv('TMB_SHELL_RESET_BEFORE'));
    const readyText = GLib.getenv('TMB_SHELL_READY_TEXT');
    try {
        for (const [index, {script, variables = {}}] of cases.entries()) {
            for (const key of keys) GLib.unsetenv(key);
            for (const [key, value] of Object.entries(variables)) GLib.setenv(key, value, true);
            if (index === resetBefore) {
                if (getIndicator() && !Main.extensionManager.disableExtension(UUID))
                    throw new Error(`Could not disable the extension before ${script}`);
                if (!Main.extensionManager.enableExtension(UUID))
                    throw new Error(`Could not enable the extension before ${script}`);
            }
            const indicator = getIndicator();
            if (!indicator) throw new Error(`Missing extension before ${script}`);
            if (readyText)
                await waitOrFail(
                    () => accessibleName(indicator)?.includes(readyText),
                    `Extension status did not reach ${readyText} before ${script}`,
                );
            indicator.menu.close(false);
            instance(indicator, St.Widget).set_hover(false);
            global.stage.set_key_focus(null);
            await afterPaint();
            try {
                const scenario = (await import(`./${script}`)) as {run(): Promise<void>};
                await scenario.run();
            } catch (error) {
                throw new Error(
                    `${script} (${JSON.stringify(variables)}): ${error instanceof Error ? error.stack : String(error)}`,
                    {
                        cause: error,
                    },
                );
            }
        }
    } finally {
        const indicator = getIndicator();
        indicator?.menu.close(false);
        indicator?.set_hover(false);
        global.stage.set_key_focus(null);
        await afterPaint();
        for (const key of keys) GLib.unsetenv(key);
    }
}

export function finish() {}
