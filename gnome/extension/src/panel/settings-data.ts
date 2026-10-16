import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Pango from 'gi://Pango';
import St from 'gi://St';
import type {TokenPanel} from '../panel.js';
import type {PanelState} from '../service-state.js';
import {COPY} from '../shared-copy.js';
import type {SettingsContext} from './settings-overview.js';
import type {CheckKey} from './support.js';
import {box, button, checkButton, label} from './support.js';

export const settingsDataMethods = {
    _renderSettingsData(
        this: TokenPanel,
        state: PanelState,
        {settings, visibility, narrow, veryNarrow}: SettingsContext,
    ): void {
        this._section('Data');
        this._settingRow(
            'Retention',
            this._stepper(
                state,
                'retention_days',
                'history retention',
                'days',
                7,
                365,
                1,
                'Keeps usage samples for 7 to 365 days. Longer retention uses more disk space.',
            ),
        );
        const collection = box(narrow, 'tmb-setting-controls');
        if (visibility.analytics) {
            const analytics = box(false, 'tmb-setting-controls');
            analytics.add_child(label('Analytics every', 'tmb-secondary'));
            analytics.add_child(
                this._stepper(
                    state,
                    'analytics_interval_minutes',
                    'analytics interval',
                    'min',
                    5,
                    120,
                    5,
                    'Sets the interval for transcript and provider analytics. Shorter intervals use more disk, network and CPU.',
                ),
            );
            collection.add_child(analytics);
        }
        if (visibility.pace) {
            const pace = box(false, 'tmb-setting-controls');
            pace.add_child(label('Pace days', 'tmb-secondary'));
            pace.add_child(
                this._stepper(
                    state,
                    'workdays',
                    'pace workdays',
                    '/ week',
                    4,
                    7,
                    1,
                    'Sets the expected usage days in a week for pace estimates. Seven uses an even pace.',
                ),
            );
            collection.add_child(pace);
        }
        const usageFile = this._check(state, COPY.writeUsageJson, 'write_usage_json');
        usageFile.x_align = Clutter.ActorAlign.START;
        collection.add_child(usageFile);
        const collectionRow = this._settingRow('Collection', collection);
        if (narrow && !veryNarrow) collectionRow.height = collection.get_preferred_height(-1)[1] + 4;
        const exportHistory = button(
            'Export all history CSV…',
            () => {
                void this._exportHistory('ExportAllHistory', '(s)', [], 'quotaline-all-history.csv');
            },
            'tmb-small-control',
        );
        exportHistory.accessible_name = 'Export all history CSV…';
        this._tooltip.attach(
            exportHistory,
            COPY.exportHistory,
            'Exports stored history without changing it. Privacy settings apply to project names in the CSV.',
        );
        this._settingRow('Export', exportHistory);
        const history = box(veryNarrow, 'tmb-setting-controls');
        const historyFiles = veryNarrow ? box(false, 'tmb-setting-controls') : history;
        if (veryNarrow) history.add_child(historyFiles);
        const historyPath = label(state.history_path, 'tmb-history-path');
        historyPath.width = narrow ? 150 : this._compact ? 245 : 450;
        historyPath.clutter_text.set_ellipsize(Pango.EllipsizeMode.MIDDLE);
        const copyHistoryPath = button(
            '',
            () => {
                St.Clipboard.get_default().set_text(St.ClipboardType.CLIPBOARD, state.history_path);
            },
            'tmb-history-path-copy',
        );
        copyHistoryPath.set_child(historyPath);
        copyHistoryPath.accessible_name = 'Copy history path';
        this._tooltip.attach(copyHistoryPath, COPY.historyFile, 'Copies the full local history database path.');
        historyFiles.add_child(copyHistoryPath);
        const openHistory = button(
            'Open folder',
            () =>
                Gio.AppInfo.launch_default_for_uri(
                    Gio.File.new_for_path(GLib.path_get_dirname(state.history_path)).get_uri(),
                    null,
                ),
            'tmb-small-control',
        );
        openHistory.accessible_name = 'Open history folder';
        this._tooltip.attach(
            openHistory,
            COPY.openHistory,
            'Opens the history database folder without stopping collection or changing the file.',
        );
        historyFiles.add_child(openHistory);
        let clearPending = false;
        const clearButton = button(
            'Clear history…',
            () => {
                if (!clearPending) {
                    clearPending = true;
                    clearButton.set_label('Confirm clear history');
                } else {
                    this._call('ClearHistory', '()', [], () => {
                        this._history = null;
                        this._modelGroups = null;
                        this._modelGroupsState = null;
                        this._notice = 'History cleared';
                        this._renderPage();
                    });
                }
            },
            'tmb-small-control',
        );
        clearButton.accessible_name = 'Clear history';
        this._tooltip.attach(clearButton, COPY.clearHistory, COPY.clearHistoryHelp);
        history.add_child(clearButton);
        this._settingRow('History', history);

        this._section('Notifications');
        const stackedAlerts = this._root.width < 360;
        const alerts = box(stackedAlerts, 'tmb-setting-controls');
        const notifications = this._check(state, 'Notifications', 'notifications');
        if (stackedAlerts) notifications.x_align = Clutter.ActorAlign.START;
        alerts.add_child(notifications);
        const thresholds = stackedAlerts ? box(false, 'tmb-setting-controls') : alerts;
        if (stackedAlerts) alerts.add_child(thresholds);
        if (settings.notifications) {
            for (const threshold of state.notification_threshold_choices) {
                const selected = settings.notify_thresholds.includes(threshold);
                const control = checkButton(
                    `${threshold}%`,
                    selected,
                    () => {
                        this._call('ToggleNotificationThreshold', '(i)', [threshold], raw => {
                            this._apply(raw);
                        });
                    },
                    `tmb-setting-check${selected ? ' tmb-setting-check-on' : ''}`,
                );
                control.accessible_name = `Notify at ${threshold}%`;
                this._tooltip.attach(
                    control,
                    control.accessible_name,
                    `Sends one notice when a usage window reaches ${threshold} percent.`,
                );
                thresholds.add_child(control);
            }
        }
        this._settingRow('Alerts', alerts);
        this._settingRow('', label(state.notification_summary, 'tmb-secondary'));
        if (settings.notifications) {
            const options: [string, CheckKey][] = [
                [COPY.windowResets, 'notify_resets'],
                [COPY.signInNeeded, 'notify_sign_in'],
            ];
            if (visibility.expiringCredits) options.push(['Credits expiring', 'notify_expiring_credits']);
            options.push(['Pace', 'notify_pace'], ['Sound', 'notify_sound']);
            const events = box(false, 'tmb-setting-controls');
            for (const [title, key] of options.slice(0, veryNarrow ? 2 : narrow ? 3 : options.length))
                events.add_child(this._check(state, title, key));
            this._settingRow('Events', events);
            if (narrow) {
                const moreEvents = box(false, 'tmb-setting-controls');
                for (const [title, key] of options.slice(veryNarrow ? 2 : 3, veryNarrow ? 4 : undefined))
                    moreEvents.add_child(this._check(state, title, key));
                this._settingRow('', moreEvents);
                if (veryNarrow && visibility.expiringCredits)
                    this._settingRow('', this._check(state, 'Sound', 'notify_sound'));
            }
        }
        this._section('Log');
        this._renderLog(state);
        let resetPending = false;
        const resetButton = button('Reset all settings…', () => {
            if (!resetPending) {
                resetPending = true;
                resetButton.set_label('Confirm reset all settings');
            } else {
                this._call('ResetSettings', '()', [], raw => {
                    this._preferencesLoaded = false;
                    this._apply(raw);
                    this._history = null;
                    this._historyRange = null;
                    this._shown.clear();
                    this._tab = 'Settings';
                    this._render();
                });
            }
        });
        resetButton.accessible_name = 'Reset all settings…';
        this._tooltip.attach(
            resetButton,
            'Reset all settings',
            'Restores default preferences after confirmation. Stored usage history remains available.',
        );
        this._settingRow('Preferences', resetButton);
    },
};

export type SettingsDataMethods = typeof settingsDataMethods;
