import GioUnix from 'gi://GioUnix';
import St from 'gi://St';
import type {TokenPanel} from '../panel.js';
import type {MiniBarGroup, PanelState, Settings, StatusCell, Visibility} from '../service-state.js';
import {COPY} from '../shared-copy.js';
import {box, button, label} from './support.js';

export interface SettingsContext {
    settings: Settings;
    visibility: Visibility;
    narrow: boolean;
    veryNarrow: boolean;
    fullLabels: boolean;
    providerOrder: string[];
}

// Preview items are either status cells or mini-bar groups; each lacks the other's identifying fields.
type PreviewCell =
    | (StatusCell & {bars?: undefined})
    | (MiniBarGroup & {id?: undefined; label?: undefined; text?: undefined});

export const settingsOverviewMethods = {
    _renderSettingsOverview(
        this: TokenPanel,
        state: PanelState,
        {settings, narrow, fullLabels}: SettingsContext,
    ): void {
        this._section('About');
        const version = box(false, 'tmb-setting-controls');
        version.add_child(label(`${state.app_version} · local build`, 'tmb-secondary'));
        this._settingRow('Version', version);
        const legal = box(false, 'tmb-setting-controls');
        for (const [title, file] of [
            [COPY.privacyPolicy, 'privacy.html'],
            ['Acknowledgements', 'notices.html'],
        ] as const) {
            const document = button(
                title,
                () => {
                    this._openDocument(file);
                },
                'tmb-small-control',
            );
            this._tooltip.attach(document, title, `Opens the local ${title.toLowerCase()} in your default browser.`);
            legal.add_child(document);
        }
        this._settingRow('Legal', legal);
        const extensions = button(
            'Open Extensions',
            () => {
                // Gio returns null for a missing desktop file; its GIT annotation omits that case.
                const app = GioUnix.DesktopAppInfo.new(
                    'org.gnome.Shell.Extensions.desktop',
                ) as GioUnix.DesktopAppInfo | null;
                if (!app) {
                    this._notice = 'GNOME Extensions is not installed';
                    this._renderPage();
                    return;
                }
                try {
                    app.launch([], null);
                } catch (error) {
                    this._notice = (error as Error).message;
                    this._renderPage();
                }
            },
            'tmb-small-control',
        );
        this._tooltip.attach(
            extensions,
            'Startup',
            'Opens GNOME Extensions, where enabled extensions start when you sign in.',
        );
        this._settingRow('Startup', extensions);
        this._settingNote('GNOME starts enabled extensions when you sign in. Manage this panel item in Extensions.');
        this._settingRow('Preview', this._check(state, COPY.demoData, 'demo'));

        this._section(COPY.menuBar);
        const orderChoice = this._segments(state, 'order', [
            ['stable', 'Stable'],
            ['usage', 'Usage'],
        ]);
        const formats: [string, string][] = [
            ['stacked', 'Stacked'],
            ['inline', 'Inline'],
            ['mini-bars', COPY.miniBars],
            ['countdown', 'Countdown'],
            ['exhausted', fullLabels ? COPY.countdownAt100 : 'At 100%'],
            ['custom', 'Custom'],
        ];
        const remainingLabel = fullLabels ? 'Remaining' : 'Left';
        if (narrow) {
            this._settingRow('Order', orderChoice);
            this._settingRow('Format', this._segments(state, 'format', formats.slice(0, 3)));
            this._settingRow('', this._segments(state, 'format', formats.slice(3)));
            this._settingRow(
                'Decimals',
                this._segments(state, 'decimals', [
                    [0, '0'],
                    [1, '1'],
                    [2, '2'],
                ]),
            );
            const visibility = box(false, 'tmb-setting-controls');
            visibility.add_child(this._check(state, 'Hide 0%', 'hide_zero'));
            visibility.add_child(this._check(state, COPY.fitToSpace, 'fit_to_space'));
            this._settingRow('Visibility', visibility);
            this._settingRow(
                'Show as',
                this._segments(state, 'show_as', [
                    ['used', 'Used'],
                    ['remaining', remainingLabel],
                ]),
            );
        } else {
            const order = box(false, 'tmb-setting-controls');
            order.add_child(orderChoice);
            order.add_child(label('Format', 'tmb-secondary'));
            order.add_child(this._segments(state, 'format', formats));
            this._settingRow('Order', order);
            const display = box(false, 'tmb-setting-controls');
            display.add_child(label('Decimals', 'tmb-secondary'));
            display.add_child(
                this._segments(state, 'decimals', [
                    [0, '0'],
                    [1, '1'],
                    [2, '2'],
                ]),
            );
            display.add_child(this._check(state, 'Hide 0%', 'hide_zero'));
            display.add_child(this._check(state, COPY.fitToSpace, 'fit_to_space'));
            display.add_child(label(fullLabels ? COPY.showUsageAs : 'Show as', 'tmb-secondary'));
            display.add_child(
                this._segments(state, 'show_as', [
                    ['used', 'Used'],
                    ['remaining', remainingLabel],
                ]),
            );
            this._settingRow('Display', display);
        }
        this._settingRow('Privacy', this._check(state, COPY.hideAccountAndProjectDetails, 'hide_identity'));
        this._settingRow(
            'Snapshot',
            this._segments(state, 'widget_size', [
                ['small', 'Small'],
                ['medium', 'Medium'],
                ['large', 'Large'],
            ]),
        );
        if (settings.format === 'custom') this._entry('Template', 'template', settings.template);
        const preview = box(true, 'tmb-status-preview');
        const previewBlock = box(true, 'tmb-preview-block');
        previewBlock.add_child(label('Preview', 'tmb-setting-name'));
        previewBlock.add_child(preview);
        this._settingsSection.add_child(previewBlock);
        let previewRow = box(false, 'tmb-preview-row');
        preview.add_child(previewRow);
        let previewWidth = 0;
        let previewCount = 0;
        const previewLimit = this._root.width - 52;
        const previewItems = (settings.format === 'mini-bars' ? state.mini_bar_groups : state.cells).map(
            (cell: PreviewCell) => {
                const key = cell.id ?? (cell.bars[0] as StatusCell).id;
                const previewName = cell.label ?? this._providerNames[cell.provider];
                const item = new St.Button({
                    child: this._panelCell(cell),
                    style_class: 'tmb-preview-cell',
                    can_focus: true,
                    reactive: true,
                    track_hover: true,
                });
                item.accessible_name = `Show model for ${previewName}`;
                this._tooltip.attach(
                    item,
                    `Preview ${previewName}`,
                    'Select to scroll to and focus the matching model label.',
                );
                item.connect('clicked', () => {
                    this._revealedModelKey = key;
                    this._modelFilter = '';
                    this._renderPage();
                    this._scrollToSettingsTarget(() => this._labelEntries.get(key), true);
                });
                previewRow.add_child(item);
                const width = Math.max(
                    item.get_preferred_width(-1)[1],
                    settings.format === 'inline' ? 88 : 43,
                    cell.bars ? 100 : Math.max(...cell.text.split('\n').map(line => line.length)) * 6 + 10,
                );
                item.width = width;
                return {item, width};
            },
        );
        let previewRows = 1;
        let packedWidth = 0;
        for (const {width} of previewItems) {
            if (packedWidth && packedWidth + width + 3 > previewLimit) {
                previewRows++;
                packedWidth = 0;
            }
            packedWidth += width + 3;
        }
        let rowTarget = Math.ceil(previewItems.length / previewRows);
        for (const [index, {item, width}] of previewItems.entries()) {
            if (previewWidth && (previewCount >= rowTarget || previewWidth + width + 3 > previewLimit)) {
                previewRow = box(false, 'tmb-preview-row');
                preview.add_child(previewRow);
                previewWidth = 0;
                previewCount = 0;
                rowTarget = Math.ceil(
                    (previewItems.length - index) / Math.max(1, previewRows - preview.get_n_children() + 1),
                );
            }
            const parent = item.get_parent();
            if (parent !== previewRow) {
                parent?.remove_child(item);
                previewRow.add_child(item);
            }
            previewWidth += width + 3;
            previewCount++;
        }
        if (!state.cells.length) previewRow.add_child(label('No selected limits', 'tmb-secondary'));
        preview.height =
            preview
                .get_children()
                .reduce((height: number, row) => height + Math.max(37, row.get_preferred_height(-1)[1]), 8) +
            Math.max(0, preview.get_n_children() - 1) * 4;
    },
};

export type SettingsOverviewMethods = typeof settingsOverviewMethods;
