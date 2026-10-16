import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import St from 'gi://St';
import * as Calendar from 'resource:///org/gnome/shell/ui/calendar.js';
import {until} from '../presentation.js';
import type {ResetPrecision} from '../service-state.js';
import {COPY} from '../shared-copy.js';

const PORTAL = 'org.freedesktop.portal.Desktop';
const PORTAL_PATH = '/org/freedesktop/portal/desktop';
export const GRAPHEMES = new Intl.Segmenter('en', {granularity: 'grapheme'});
export const CHECK_HELP = {
    demo: 'Displays generated provider usage so you can preview the layout without signing in.',
    hide_zero:
        'Removes zero-usage models from the menu bar to save width. Their Usage rows and history remain available.',
    fit_to_space: 'Uses shorter status renderings when panel space runs low.',
    hide_identity:
        'Replaces account, workspace and project names in the interface and exports. Stored history stays unchanged.',
    hide_unused_models: 'Shows windows with current quota use or retained activity.',
    show_all_providers: 'Reveals providers without credentials or cached data so you can set them up.',
    allow_token_refresh:
        'Refreshes expired provider tokens when possible. Claude, Codex and Gemini may save rotated credentials; Antigravity keeps its token in memory.',
    write_usage_json: 'Writes a local quota snapshot for shell prompts and dashboards without extra provider requests.',
    notifications: 'Enables usage thresholds, window reset and sign-in alerts. Turning this off silences all notices.',
    notify_resets: 'Sends a notice when a tracked usage window resets.',
    notify_sign_in: 'Sends a notice when a provider needs authentication.',
    notify_expiring_credits: 'Sends a notice when a Codex reset credit expires within 24 hours.',
    notify_pace: 'Sends one notice per reset period when a quota is on course to run out or pulls ahead of pace.',
    notify_sound: 'Plays the default alert sound with a notification.',
    detailed_logging: 'Records refresh timing and panel layout decisions in the local log.',
} satisfies Record<string, string>;
export type CheckKey = keyof typeof CHECK_HELP;
export const SEGMENT_HELP = {
    order: [
        COPY.modelOrder,
        'Stable follows the provider and model order below. Usage sorts current percentages from highest to lowest.',
    ],
    format: [
        'Format',
        'Stacked uses two lines; Inline uses one; Mini bars use gauges. Countdown shows reset time. Custom uses the template.',
    ],
    decimals: ['Decimals', 'Sets decimal places in menu bar percentages.'],
    show_as: [
        COPY.showUsageAs,
        'Used shows the consumed percentage. Left shows the remaining percentage; meters still show usage.',
    ],
    widget_size: ['Snapshot size', 'Chooses the small, medium or large cached Quick Settings view.'],
} satisfies Record<string, [string, string]>;
export type SegmentKey = keyof typeof SEGMENT_HELP;

export const PickerEventSource = GObject.registerClass(
    class PickerEventSource extends Calendar.EventSourceBase {
        override requestRange(): void {}

        override hasEvents(): boolean {
            return false;
        }
    },
);

export function resetText(stamp: string | null, precision: ResetPrecision): string {
    // Date.parse(null) is NaN, which reads as an unknown reset.
    const date = Date.parse(stamp as string);
    if (date <= Date.now()) return 'Reset due\nAwaiting refresh';
    const clock = Number.isFinite(date)
        ? precision === 'day'
            ? `\n${new Date(date).toLocaleDateString([], {timeZone: 'UTC', month: 'short', day: 'numeric'})}`
            : `\n${new Date(date).toLocaleTimeString([], {hour: 'numeric', minute: '2-digit'})}`
        : '';
    return `${until(stamp)}${clock}`;
}

interface MeterOptions extends Partial<St.Widget.ConstructorProps> {
    height?: number;
    track?: string;
    fill?: string;
}

export function meter(
    width: number,
    used: number,
    color: string,
    {height = 6, track = 'tmb-meter', fill = 'tmb-meter-fill', ...props}: MeterOptions = {},
): St.Widget {
    const bar = new St.Widget({layout_manager: new Clutter.FixedLayout(), style_class: track, width, height, ...props});
    bar.add_child(
        new St.Widget({
            style_class: fill,
            height,
            style: `background-color: ${color}; background-gradient-start: ${color}; background-gradient-end: ${color};`,
            width: Math.max(2, Math.round((width * Math.clamp(used, 0, 100)) / 100)),
        }),
    );
    return bar;
}

export function shortLabel(value: string, limit: number): string {
    let result = '';
    let count = 0;
    for (const {segment} of GRAPHEMES.segment(value)) {
        if (count === limit) break;
        result += segment;
        count++;
    }
    return result;
}

export function percent(used: number): string {
    return `${Math.round(used)}%`;
}

export function chooseExportPath(
    setCancel: (cancel: (() => void) | null) => void,
    filename: string,
): Promise<string | null> {
    const bus = Gio.DBus.session;
    const token = `tmb_${GLib.uuid_string_random().replaceAll('-', '')}`;
    const responses = new Map<string, GLib.Variant>();
    let handle: string | null = null;
    let subscription = 0;
    let settled = false;
    return new Promise((resolve, reject) => {
        const finish = (error: Error | null, path: string | null = null) => {
            if (settled) return;
            settled = true;
            bus.signal_unsubscribe(subscription);
            setCancel(null);
            if (error) reject(error);
            else resolve(path);
        };
        const close = () => {
            if (handle)
                bus.call(
                    PORTAL,
                    handle,
                    'org.freedesktop.portal.Request',
                    'Close',
                    null,
                    null,
                    Gio.DBusCallFlags.NONE,
                    -1,
                    null,
                    null,
                );
            finish(null);
        };
        const respond = (parameters: GLib.Variant) => {
            const [code, results] = parameters.deep_unpack<[number, {uris?: GLib.Variant}]>();
            if (code === 1) {
                finish(null);
                return;
            }
            if (code !== 0) {
                finish(new Error('Save dialog closed without a file'));
                return;
            }
            const uri = results.uris?.deep_unpack<string[]>()[0];
            const path = uri && Gio.File.new_for_uri(uri).get_path();
            if (!path) finish(new Error('Save dialog did not return a local file'));
            else finish(null, path);
        };
        subscription = bus.signal_subscribe(
            PORTAL,
            'org.freedesktop.portal.Request',
            'Response',
            null,
            null,
            Gio.DBusSignalFlags.NONE,
            (_connection, _sender, objectPath, _interface, _signal, parameters) => {
                if (handle === objectPath) respond(parameters);
                else if (!handle) responses.set(objectPath, parameters);
            },
        );
        setCancel(close);
        bus.call(
            PORTAL,
            PORTAL_PATH,
            'org.freedesktop.portal.FileChooser',
            'SaveFile',
            new GLib.Variant('(ssa{sv})', [
                '',
                COPY.exportHistory,
                {
                    handle_token: new GLib.Variant('s', token),
                    current_name: new GLib.Variant('s', filename),
                },
            ]),
            new GLib.VariantType('(o)'),
            Gio.DBusCallFlags.NONE,
            30_000,
            null,
            (_connection, result) => {
                try {
                    handle = bus.call_finish(result).deep_unpack<[string]>()[0];
                    if (settled) close();
                    else {
                        const response = responses.get(handle);
                        if (response) respond(response);
                    }
                } catch (error) {
                    finish(error as Error);
                }
            },
        );
    });
}

export function clear(container: Clutter.Actor): void {
    for (const child of container.get_children()) child.destroy();
}

export function checkButton(
    text: string,
    checked: boolean,
    clicked: () => void,
    styleClass: string,
    mixed = false,
): St.Button {
    const content = box(false, 'tmb-check-content');
    content.add_child(
        label(checked ? '✓' : mixed ? '−' : '', `tmb-check-mark${checked || mixed ? ' tmb-check-mark-on' : ''}`),
    );
    if (text) content.add_child(label(text, 'tmb-check-text'));
    const control = new St.Button({
        child: content,
        style_class: styleClass,
        can_focus: true,
        reactive: true,
        track_hover: true,
        toggle_mode: true,
        checked,
    });
    control.connect('clicked', clicked);
    return control;
}

export function button(value: string, clicked: () => void, styleClass = 'tmb-control'): St.Button {
    const control = new St.Button({
        label: value,
        style_class: styleClass,
        can_focus: true,
        reactive: true,
        track_hover: true,
    });
    control.connect('clicked', clicked);
    return control;
}

export function coloredLabel(value: string, styleClass: string, color: string): St.Label {
    const text = label(value, styleClass);
    text.set_style(`color: ${color};`);
    return text;
}

export function label(value: unknown, styleClass = ''): St.Label {
    return new St.Label({text: String(value), style_class: styleClass, y_align: Clutter.ActorAlign.CENTER});
}

export function box(vertical = false, styleClass = ''): St.BoxLayout {
    return new St.BoxLayout({
        orientation: vertical ? Clutter.Orientation.VERTICAL : Clutter.Orientation.HORIZONTAL,
        style_class: styleClass,
    });
}
