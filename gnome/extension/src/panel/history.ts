import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import St from 'gi://St';
import * as Calendar from 'resource:///org/gnome/shell/ui/calendar.js';
import type {PresentedHistory} from '../history-presentation.js';
import {presentHistory} from '../history-presentation.js';
import type {TokenPanel} from '../panel.js';
import type {
    HistoryAction,
    HistoryViewResponse,
    PanelState,
    VisibilityKind,
    VisibilityResponse,
} from '../service-state.js';
import {COPY} from '../shared-copy.js';
import type {HistoryMetricChoice} from './history-content.js';
import {historyContentMethods} from './history-content.js';
import {box, button, label, PickerEventSource} from './support.js';

const methods = {
    _loadHistory(this: TokenPanel, action: HistoryAction = {kind: 'load'}): void {
        const state = this._state as PanelState;
        this._historyStepRequest++;
        this._historyStepDirections = null;
        this._historyStepBase = null;
        const request = ++this._historyRequest;
        this._historyError = null;
        const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
        const offset = -new Date().getTimezoneOffset() * 60;
        this._call(
            'GetHistoryView',
            '(ssi)',
            [JSON.stringify(action), zone, offset],
            (raw: string) => {
                if (request !== this._historyRequest) return;
                const {view, history, error} = JSON.parse(raw) as HistoryViewResponse;
                this._days = view.days;
                this._metric = view.metric;
                this._rollup = view.rollup;
                this._utc = view.utc;
                this._historyRange = view.follow_now ? null : [view.start, view.end];
                if (view.custom_from) {
                    this._customFrom = view.custom_from;
                    this._customTo = view.custom_to;
                }
                if (action.kind === 'custom' && !error) this._customOpen = false;
                this._historyError = error?.replace(/^GDBus\.Error:[^:]+:\s*/, '') ?? null;
                if (history) {
                    this._history = presentHistory(history, state.metric_definitions, state.provider_definitions);
                    this._shown = new Set(history.series.map(series => series.id).filter(id => !this._hidden.has(id)));
                    if (
                        this._selectedHistoryAt !== null &&
                        (this._selectedHistoryAt < history.start || this._selectedHistoryAt > history.end)
                    )
                        this._selectedHistoryAt = null;
                }
                if (this._tab === 'History') this._renderPage();
            },
            (error: Error): false => {
                if (request === this._historyRequest) {
                    const message = error.message.replace(/^GDBus\.Error:[^:]+:\s*/, '');
                    if (action.kind === 'custom') this._error = message;
                    else this._historyError = message;
                    this._renderPage();
                }
                return false;
            },
        );
    },

    _renderHistory(this: TokenPanel, state: PanelState): void {
        const periodButton = (text: string, clicked: () => void, style: string): St.Button => {
            const control = button(text, clicked, style);
            this._tooltip.attach(control, COPY.historyPeriod, COPY.nowHelp);
            this._historyFocus.set(`period:${text}`, control);
            return control;
        };
        const metrics = state.metric_definitions.map(definition => ({
            id: definition.id === 'windowUsagePercent' ? 'usage' : definition.id,
            title: definition.title,
            unit: definition.display_unit,
            summary: definition.summary,
            stackable: definition.stackable,
        })) as [HistoryMetricChoice, ...HistoryMetricChoice[]];
        const index = Math.max(
            0,
            metrics.findIndex(item => item.id === this._metric),
        );
        const current = metrics[index] as HistoryMetricChoice;
        const metricRow = box(false, 'tmb-row');
        metricRow.add_child(label('Metric', 'tmb-history-label'));
        const metricPicker = button(
            `${current.title}  ▾`,
            () => {
                this._metricOpen = !this._metricOpen;
                this._renderPage();
            },
            'tmb-control tmb-metric-picker',
        );
        metricPicker.accessible_name = 'Metric';
        this._tooltip.attach(metricPicker, COPY.historyMetric, COPY.historyMetricHelp);
        this._historyFocus.set('metric', metricPicker);
        metricRow.add_child(metricPicker);
        const layoutWidth = this._layoutWidth;
        const veryNarrow = layoutWidth < 440;
        const controls = box(veryNarrow, 'tmb-row');
        let periodRow = veryNarrow ? box(false, 'tmb-row') : controls;
        if (veryNarrow) controls.add_child(periodRow);
        const rollupControls = layoutWidth < 650 ? box(veryNarrow, 'tmb-row') : controls;
        periodRow.add_child(
            periodButton(
                'Now',
                () => {
                    this._customOpen = false;
                    this._selectedHistoryAt = null;
                    this._loadHistory({kind: 'now'});
                },
                `tmb-control${!this._historyRange ? ' tmb-tab-selected' : ''}`,
            ),
        );
        for (const days of [1, 7, 30, 60]) {
            if (veryNarrow && days === 30) {
                periodRow = box(false, 'tmb-row');
                controls.add_child(periodRow);
            }
            periodRow.add_child(
                periodButton(
                    days === 1 ? 'Today' : `${days}d`,
                    () => {
                        this._customOpen = false;
                        this._selectedHistoryAt = null;
                        this._loadHistory({kind: 'period', days});
                    },
                    `tmb-control${this._days === days && this._historyRange ? ' tmb-tab-selected' : ''}`,
                ),
            );
        }
        periodRow.add_child(
            periodButton(
                'Custom',
                () => {
                    this._customOpen = !this._customOpen;
                    if (this._customOpen && !this._customFrom) {
                        const today = new Date();
                        const stamp = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, '0')}-${String(today.getDate()).padStart(2, '0')}`;
                        this._customFrom = stamp;
                        this._customTo = stamp;
                    }
                    this._renderPage();
                },
                `tmb-control${this._days === 0 && this._historyRange ? ' tmb-tab-selected' : ''}`,
            ),
        );
        const rollupRow = veryNarrow ? box(false, 'tmb-row') : rollupControls;
        if (veryNarrow) rollupControls.add_child(rollupRow);
        if (this._metric === 'usage') {
            rollupRow.add_child(label('Rollup', 'tmb-history-label'));
            for (const rollup of ['minute', 'hour', 'day']) {
                const control = button(
                    rollup.charAt(0).toUpperCase() + rollup.slice(1),
                    () => {
                        this._loadHistory({kind: 'rollup', rollup});
                    },
                    `tmb-control${this._rollup === rollup ? ' tmb-tab-selected' : ''}`,
                );
                this._tooltip.attach(control, 'Rollup', COPY.historyResolutionHelp);
                this._historyFocus.set(`rollup:${rollup}`, control);
                rollupRow.add_child(control);
            }
        }
        if (!veryNarrow) {
            const controlSpacer = box();
            controlSpacer.x_expand = true;
            rollupControls.add_child(controlSpacer);
        }
        const history = this._history;
        const metric = this._metric;
        const exportControl = button(COPY.exportCsv, () => {});
        if (history)
            exportControl.connect('clicked', () => {
                void this._exportHistory(
                    'ExportMetricHistory',
                    '(sxxs)',
                    [metric, history.start, history.end],
                    'quotaline-history.csv',
                );
            });
        this._tooltip.attach(
            exportControl,
            COPY.exportSelectedPeriod,
            'Writes the selected metric and period to the file you choose.',
        );
        exportControl.reactive = Boolean(this._history);
        exportControl.can_focus = Boolean(this._history);
        this._historyFocus.set('export', exportControl);
        if (veryNarrow) {
            const exportRow = box(false, 'tmb-row');
            exportRow.add_child(exportControl);
            rollupControls.add_child(exportRow);
        } else {
            rollupControls.add_child(exportControl);
        }
        this._page.add_child(controls);
        if (rollupControls !== controls) this._page.add_child(rollupControls);
        const navigation = box(false, 'tmb-row');
        const previous = button('←', () => {
            this._pageHistory(-1);
        });
        this._tooltip.attach(previous, COPY.previousPeriod, COPY.previousPeriodHelp);
        this._historyFocus.set('previous', previous);
        navigation.add_child(previous);
        const next = button('→', () => {
            this._pageHistory(1);
        });
        this._tooltip.attach(next, COPY.nextPeriod, COPY.nextPeriodHelp);
        this._historyFocus.set('next', next);
        navigation.add_child(next);
        if (this._history) {
            const format = new Intl.DateTimeFormat(undefined, {
                month: 'short',
                day: 'numeric',
                year: 'numeric',
                timeZone: this._metric !== 'usage' || this._utc ? 'UTC' : undefined,
            });
            const first = format.format(new Date(this._history.start * 1000));
            const last = format.format(new Date((this._history.end - 1) * 1000));
            navigation.add_child(label(first === last ? first : `${first} – ${last}`, 'tmb-history-path'));
        }
        this._page.add_child(navigation);
        this._page.add_child(metricRow);
        if (this._metricOpen) {
            const menu = new St.ScrollView({
                style_class: 'tmb-metric-options',
                width: Math.min(360, layoutWidth - 28),
                height: 240,
                x_align: Clutter.ActorAlign.START,
                x_expand: false,
                overlay_scrollbars: true,
            });
            const options = box(true, 'tmb-metric-options-content');
            for (const [group, ids] of state.metric_groups) {
                const choices = metrics.filter(metric => ids.includes(metric.id));
                if (!choices.length) continue;
                options.add_child(label(group, 'tmb-metric-group'));
                for (const {id, title} of choices) {
                    const selected = id === this._metric;
                    const item = new St.Button({
                        style_class: `tmb-metric-option${selected ? ' tmb-metric-option-selected' : ''}`,
                        can_focus: true,
                        reactive: true,
                        track_hover: true,
                        x_expand: true,
                    });
                    const content = box(false, 'tmb-metric-option-content');
                    content.width = Math.min(330, layoutWidth - 58);
                    content.add_child(label(selected ? '✓' : '', 'tmb-metric-option-mark'));
                    content.add_child(label(title));
                    item.set_child(content);
                    item.accessible_name = `Choose metric: ${id}`;
                    item.connect('clicked', () => {
                        this._metricOpen = false;
                        this._selectedHistoryAt = null;
                        this._history = null;
                        this._historyError = null;
                        this._renderPage();
                        this._loadHistory({kind: 'metric', metric: id});
                    });
                    options.add_child(item);
                }
            }
            menu.set_child(options);
            this._page.add_child(menu);
        }
        if (this._customOpen) {
            const dates = box(layoutWidth < 440, 'tmb-row');
            const from = new St.Entry({
                text: this._customFrom,
                hint_text: 'From YYYY-MM-DD',
                style_class: 'tmb-entry',
                can_focus: true,
            });
            const to = new St.Entry({
                text: this._customTo,
                hint_text: 'To YYYY-MM-DD',
                style_class: 'tmb-entry',
                can_focus: true,
            });
            this._tooltip.attach(
                from,
                COPY.startDate,
                'Sets the first day, switches the period to Custom, and stops following Now.',
            );
            this._tooltip.attach(
                to,
                COPY.endDate,
                'Sets the last day, switches the period to Custom, and stops following Now.',
            );
            const calendar = new Calendar.Calendar();
            calendar.setEventSource(new PickerEventSource());
            calendar.add_style_class_name('tmb-date-calendar');
            calendar.visible = false;
            calendar.x_align = Clutter.ActorAlign.START;
            let activeDate = from;
            let selectedDate = '';
            const useDate = button(
                'Use selected day',
                () => {
                    activeDate.set_text(selectedDate);
                    calendar.visible = false;
                    useDate.visible = false;
                },
                'tmb-small-control',
            );
            useDate.visible = false;
            const picker = (entry: St.Entry, title: string): St.Button => {
                const control = button(
                    '▦',
                    () => {
                        const wasOpen = calendar.visible && activeDate === entry;
                        calendar.visible = false;
                        useDate.visible = false;
                        activeDate = entry;
                        if (wasOpen) return;
                        const date = new Date(`${entry.get_text()}T12:00:00`);
                        const shownDate = Number.isNaN(date.getTime()) ? new Date() : date;
                        selectedDate = calendarDate(
                            GLib.DateTime.new_from_unix_local(Math.floor(shownDate.getTime() / 1000)),
                        );
                        calendar.setDate(shownDate);
                        calendar.visible = true;
                        useDate.visible = true;
                    },
                    'tmb-small-control tmb-date-picker',
                );
                control.accessible_name = `Choose ${title} date`;
                this._tooltip.attach(control, `Choose ${title} date`, 'Pick a day from the calendar.');
                return control;
            };
            calendar.connect('selected-date-changed', (_calendar: Calendar.Calendar, date: GLib.DateTime) => {
                selectedDate = calendarDate(date);
                if (calendar.visible && calendar._shouldDateGrabFocus) useDate.emit('clicked', 1);
            });
            dates.add_child(from);
            dates.add_child(picker(from, 'start'));
            dates.add_child(to);
            dates.add_child(picker(to, 'end'));
            const applyDates = button('Apply dates', () => {
                const first = from.get_text();
                const last = to.get_text();
                this._error = null;
                this._loadHistory({kind: 'custom', from: first, to: last});
            });
            this._tooltip.attach(applyDates, 'Apply dates', 'Loads the selected date range and stops following Now.');
            dates.add_child(applyDates);
            this._page.add_child(dates);
            this._page.add_child(calendar);
            this._page.add_child(useDate);
        }
        this._renderHistoryContent(state, current, layoutWidth, veryNarrow);
    },

    _isolateHistorySeries(this: TokenPanel, id: string): void {
        this._historyClick = null;
        this._updateHistoryVisibility('isolate', id);
    },

    _updateHistoryVisibility(this: TokenPanel, kind: VisibilityKind, id: string): void {
        const history = this._history as PresentedHistory;
        this._call(
            'UpdateHistoryVisibility',
            '(s)',
            [
                JSON.stringify({
                    kind,
                    metric: this._metric,
                    id,
                    series: history.series.map(series => series.id),
                }),
            ],
            (raw: string) => {
                const {hidden, shown} = JSON.parse(raw) as VisibilityResponse;
                this._hidden = new Set(hidden);
                this._shown = new Set(shown);
                this._renderPage();
            },
        );
    },

    _pageHistory(this: TokenPanel, direction: number): void {
        if (!this._history) return;
        this._loadHistory({kind: 'page', forward: direction > 0});
    },

    _stepHistoryReading(this: TokenPanel, readings: number[], direction: number): void {
        if (!readings.length) return;
        this._historyStepBase ??= (readings as (number | null)[]).indexOf(this._selectedHistoryAt);
        this._historyStepDirections ??= [];
        this._historyStepDirections.push(direction);
        const request = ++this._historyStepRequest;
        this._call(
            'StepHistoryReading',
            '(iiai)',
            [readings.length, this._historyStepBase, this._historyStepDirections],
            (index: number) => {
                if (request !== this._historyStepRequest) return;
                this._historyStepDirections = null;
                this._historyStepBase = null;
                this._selectedHistoryAt = readings[index] as number;
                this._renderPage();
            },
            (): false | undefined => {
                if (request !== this._historyStepRequest) return false;
                this._historyStepDirections = null;
                this._historyStepBase = null;
                return undefined;
            },
        );
    },
};

export const historyMethods = {...methods, ...historyContentMethods};

function calendarDate(date: GLib.DateTime): string {
    return `${date.get_year()}-${String(date.get_month()).padStart(2, '0')}-${String(date.get_day_of_month()).padStart(2, '0')}`;
}
