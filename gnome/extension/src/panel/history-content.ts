import Clutter from 'gi://Clutter';
import Pango from 'gi://Pango';
import St from 'gi://St';
import {createChart, createLegendSwatch} from '../chart.js';
import {historyValue, valueAt} from '../chart-data.js';
import type {TokenPanel} from '../panel.js';
import type {HistoryPoint, PanelState} from '../service-state.js';
import {COPY} from '../shared-copy.js';
import {box, button, checkButton, label} from './support.js';

export interface HistoryMetricChoice {
    id: string;
    title: string;
    unit: string;
    summary: string;
    stackable: boolean;
}

export const historyContentMethods = {
    _renderHistoryContent(
        this: TokenPanel,
        state: PanelState,
        current: HistoryMetricChoice,
        layoutWidth: number,
        veryNarrow: boolean,
    ): void {
        if (!this._history) {
            if (this._historyError) {
                this._page.add_child(label(COPY.historyUnavailable, 'tmb-history-error-title'));
                this._page.add_child(label(`History error: ${this._historyError}`, 'tmb-secondary'));
                this._page.add_child(
                    button(
                        'Retry',
                        () => {
                            this._loadHistory();
                        },
                        'tmb-control',
                    ),
                );
            } else {
                this._page.add_child(label(COPY.loadingHistory));
            }
            return;
        }
        if (this._historyError) {
            const failure = box(false, 'tmb-history-failure');
            failure.add_child(label(`Update failed: ${this._historyError}`, 'tmb-secondary'));
            failure.add_child(
                button(
                    'Retry',
                    () => {
                        this._loadHistory();
                    },
                    'tmb-control',
                ),
            );
            this._page.add_child(failure);
        }
        if (!this._history.series.length) {
            const noSelection =
                this._metric === 'usage' &&
                state.settings.has_custom_selection &&
                !state.settings.selected_windows.length;
            this._page.add_child(
                label(
                    noSelection
                        ? 'No models selected. Choose models in Settings to see history.'
                        : 'No recorded data for this metric and period.',
                ),
            );
            return;
        }
        const providers = [
            ...new Set(this._history.series.filter(series => series.points.length).map(series => series.provider)),
        ].sort((left, right) => this._usageOrder.indexOf(left) - this._usageOrder.indexOf(right));
        const attribution = box(veryNarrow, 'tmb-history-attribution');
        const iconRow = veryNarrow ? box(false) : attribution;
        for (const id of providers) {
            const icon = this._providerIcon(id);
            icon.icon_size = 16;
            icon.add_style_class_name('tmb-history-provider-icon');
            iconRow.add_child(icon);
        }
        if (veryNarrow) attribution.add_child(iconRow);
        const caption = label(this._history.attribution, 'tmb-history-attribution-label');
        caption.width = veryNarrow
            ? layoutWidth - 32
            : this._compact
              ? Math.min(430, Math.max(180, layoutWidth - 185))
              : Math.min(760, layoutWidth - 200);
        caption.clutter_text.set_line_wrap(true);
        caption.clutter_text.set_ellipsize(Pango.EllipsizeMode.NONE);
        attribution.add_child(caption);
        this._page.add_child(attribution);
        const visible = this._history.series.filter(series => this._shown.has(series.id));
        const total =
            this._history.canonicalTotal ??
            visible.flatMap(series => series.points).reduce((sum, point) => sum + point[1], 0);
        const latestAt = visible
            .flatMap(series => series.points.map(point => point[0]))
            .reduce((latest, at) => Math.max(latest, at), 0);
        const summary =
            this._metric === 'usage'
                ? `${this._history.series.length} model${this._history.series.length === 1 ? '' : 's'}`
                : current.summary === 'latest'
                  ? `${visible.length} series · latest ${new Date(latestAt * 1000).toLocaleDateString(undefined, {
                        month: 'short',
                        day: 'numeric',
                        timeZone: 'UTC',
                    })}`
                  : `${historyValue(total, current.unit)}${current.unit === 'count' || current.unit === 'USD' ? '' : ` ${current.unit}`} total`;
        const canStack = current.stackable && visible.length > 1;
        if (canStack) {
            const stack = checkButton(
                'Stacked',
                this._stacked,
                () => {
                    this._stacked = !this._stacked;
                    this._set('history_stacked', this._stacked);
                    this._renderPage();
                },
                'tmb-control',
            );
            stack.accessible_name = 'Stacked';
            this._tooltip.attach(stack, COPY.stackSeries, COPY.stackSeriesHelp);
            this._page.add_child(stack);
        }
        this._page.add_child(label(summary, 'tmb-history-intro'));
        const highlighted: {id: string | null} = {id: null};
        const chart = createChart(
            this._history.series,
            this._history.start,
            this._history.end,
            this._shown,
            this._metric,
            canStack && this._stacked,
            this._selectedHistoryAt,
            this._metric !== 'usage' || this._utc,
            (timestamp: number | null) => {
                this._selectedHistoryAt = timestamp;
                this._renderPage();
            },
            highlighted,
            current.unit,
        );
        this._chart = chart;
        chart.can_focus = true;
        chart.accessible_name += '; Left and Right move between readings, Backspace clears the selection';
        this._tooltip.attach(
            chart,
            COPY.historyChart,
            'Move the pointer to inspect a date. Use Left and Right to move between visible dates, and Backspace to clear the selection.',
        );
        // Points carried to the period edge draw the line but are not readings to step to.
        const readings = [
            ...new Set(visible.flatMap(series => series.points.filter(point => !point[3]).map(point => point[0]))),
        ].sort((left, right) => left - right);
        const keys = new Clutter.KeyController();
        keys.connect('key-press', controller => {
            const symbol = controller.get_key()[1];
            if (symbol === Clutter.KEY_Left || symbol === Clutter.KEY_Right) {
                this._stepHistoryReading(readings, symbol === Clutter.KEY_Left ? -1 : 1);
                return Clutter.EVENT_STOP;
            }
            if (symbol === Clutter.KEY_BackSpace) {
                this._selectedHistoryAt = null;
                this._renderPage();
                return Clutter.EVENT_STOP;
            }
            return Clutter.EVENT_PROPAGATE;
        });
        chart.add_action_full('tmb-history-arrows', Clutter.EventPhase.CAPTURE, keys);
        const sideLegend = layoutWidth - 36 - 12 - 240 >= 600;
        chart.width = Math.max(210, layoutWidth - (sideLegend ? 288 : veryNarrow ? 44 : 36));
        chart.x_align = Clutter.ActorAlign.START;
        chart.height = Math.max(280, Math.min(480, this._panelMonitor().height - 350));
        const visual = box(!sideLegend, 'tmb-history-visual');
        visual.x_align = Clutter.ActorAlign.START;
        const plot = box(true, 'tmb-history-plot');
        plot.x_align = Clutter.ActorAlign.START;
        plot.add_child(chart);
        visual.add_child(plot);
        const legend = box(true, 'tmb-history-legend');
        legend.width = sideLegend ? 240 : chart.width;
        legend.x_align = Clutter.ActorAlign.START;
        visual.add_child(legend);
        this._page.add_child(visual);
        if (readings.length) {
            const inspection = box(false, 'tmb-row');
            inspection.add_child(
                button('Previous reading', () => {
                    this._stepHistoryReading(readings, -1);
                }),
            );
            inspection.add_child(
                button('Next reading', () => {
                    this._stepHistoryReading(readings, 1);
                }),
            );
            if (this._selectedHistoryAt !== null)
                inspection.add_child(
                    button('Clear selection', () => {
                        this._selectedHistoryAt = null;
                        this._renderPage();
                    }),
                );
            this._page.add_child(inspection);
            if (this._selectedHistoryAt !== null) {
                this._page.add_child(
                    label(
                        `Selected ${new Date(this._selectedHistoryAt * 1000).toLocaleString(
                            undefined,
                            this._metric !== 'usage' || this._utc ? {timeZone: 'UTC'} : {},
                        )}`,
                        'tmb-section',
                    ),
                );
            }
        }
        const details = button(
            `${this._historyDetailsOpen ? '▾' : '▸'} Data details`,
            () => {
                this._historyDetailsOpen = !this._historyDetailsOpen;
                this._renderPage();
            },
            'tmb-disclosure',
        );
        details.accessible_name = COPY.dataDetails;
        details.x_align = Clutter.ActorAlign.START;
        this._page.add_child(details);
        if (this._historyDetailsOpen) {
            const count = this._history.series.reduce((sum, series) => sum + series.points.length, 0);
            this._page.add_child(
                label(`${count} plotted points · ${this._history.series.length} series`, 'tmb-secondary'),
            );
        }
        const legendHeader = box(false, 'tmb-history-legend-header');
        legendHeader.add_child(
            label(
                this._selectedHistoryAt === null
                    ? COPY.rangeValues
                    : new Date(this._selectedHistoryAt * 1000).toLocaleString(
                          undefined,
                          this._metric !== 'usage' || this._utc ? {timeZone: 'UTC'} : {},
                      ),
                'tmb-eyebrow',
            ),
        );
        const legendSpacer = box();
        legendSpacer.x_expand = true;
        legendHeader.add_child(legendSpacer);
        if (this._metric === 'usage') {
            const utc = checkButton(
                'UTC',
                this._utc,
                () => {
                    this._loadHistory({kind: 'utc', utc: !this._utc});
                },
                'tmb-setting-check',
            );
            utc.accessible_name = 'UTC';
            this._tooltip.attach(
                utc,
                COPY.utcBoundaries,
                'Uses UTC boundaries for window buckets. Provider analytics already arrives as UTC days.',
            );
            legendHeader.add_child(utc);
        }
        legend.add_child(legendHeader);
        const firstColumn = sideLegend ? legend : box(true, 'tmb-history-legend-column');
        const secondColumn = sideLegend || layoutWidth < 440 ? firstColumn : box(true, 'tmb-history-legend-column');
        const legendColumns = [firstColumn, secondColumn] as const;
        if (!sideLegend) {
            const columns = box(false, 'tmb-history-legend-columns');
            for (const column of new Set(legendColumns)) {
                column.width = layoutWidth < 440 ? chart.width - 24 : Math.floor((chart.width - 24) / 2);
                columns.add_child(column);
            }
            legend.add_child(columns);
        }
        for (const [index, series] of this._history.series.entries()) {
            const seriesName = this._history.display_labels[series.id] as string;
            const latest = series.points.at(-1) as HistoryPoint;
            const inspected =
                this._selectedHistoryAt === null
                    ? latest[1]
                    : valueAt(series.points, this._selectedHistoryAt, this._metric);
            const reading = inspected === null ? '—' : historyValue(inspected, current.unit);
            const timestamp = this._selectedHistoryAt ?? latest[0];
            const date = new Date(timestamp * 1000).toLocaleString(
                undefined,
                this._metric !== 'usage' || this._utc ? {timeZone: 'UTC'} : {},
            );
            const sum = series.points.reduce((amount, point) => amount + point[1], 0);
            const measure =
                this._selectedHistoryAt !== null ||
                this._metric === 'usage' ||
                current.unit === '%' ||
                this._metric === 'creditBalance'
                    ? reading
                    : historyValue(sum, current.unit);
            const value = label(measure, 'tmb-legend-value');
            const columnWidth = sideLegend
                ? legend.width - 14
                : layoutWidth < 440
                  ? chart.width - 24
                  : Math.floor((chart.width - 24) / 2);
            value.width = Math.min(Math.max(38, measure.length * 8), Math.max(38, columnWidth - 137));
            value.clutter_text.set_line_wrap(true);
            value.clutter_text.set_ellipsize(Pango.EllipsizeMode.NONE);
            const row = box(false, 'tmb-legend-row');
            row.add_child(createLegendSwatch(index, this._metric));
            const name = label(seriesName, 'tmb-legend-name');
            name.width = Math.max(80, columnWidth - value.width - 50);
            name.clutter_text.set_line_wrap(true);
            name.clutter_text.set_ellipsize(Pango.EllipsizeMode.NONE);
            row.add_child(name);
            row.add_child(value);
            row.add_child(
                label(
                    this._shown.has(series.id) ? '✓' : '',
                    `tmb-legend-check${this._shown.has(series.id) ? ' tmb-legend-check-on' : ''}`,
                ),
            );
            const toggle = new St.Button({
                child: row,
                style_class: `tmb-legend-toggle${this._shown.has(series.id) ? '' : ' tmb-legend-off'}`,
                can_focus: true,
                reactive: true,
                track_hover: true,
                toggle_mode: true,
                checked: this._shown.has(series.id),
            });
            toggle.accessible_name = `${this._shown.has(series.id) ? '☑' : '☐'} ${seriesName} · ${measure} · ${series.points.length} points · ${date}`;
            this._tooltip.attach(
                toggle,
                seriesName,
                'Shows the value at the selected date. Double-click or press I to isolate this series; repeat to restore the others.',
            );
            this._historyFocus.set(`series:${series.id}`, toggle);
            toggle.connect('notify::hover', () => {
                if (toggle.hover && this._shown.has(series.id)) {
                    highlighted.id = series.id;
                    toggle.add_style_class_name('tmb-legend-highlighted');
                } else {
                    if (highlighted.id === series.id) highlighted.id = null;
                    toggle.remove_style_class_name('tmb-legend-highlighted');
                }
                chart.queue_repaint();
            });
            const isolateKey = new Clutter.KeyController();
            isolateKey.connect('key-press', controller => {
                const symbol = controller.get_key()[1];
                if (symbol !== Clutter.KEY_i && symbol !== Clutter.KEY_I) return Clutter.EVENT_PROPAGATE;
                this._isolateHistorySeries(series.id);
                return Clutter.EVENT_STOP;
            });
            toggle.add_action_full('tmb-legend-isolate', Clutter.EventPhase.CAPTURE, isolateKey);
            toggle.connect('clicked', () => {
                // Programmatic clicks have no current event; Clutter's GIT omits nullable.
                const event = Clutter.get_current_event() as Clutter.Event | null;
                const pointer =
                    event?.type() === Clutter.EventType.BUTTON_RELEASE || event?.type() === Clutter.EventType.TOUCH_END;
                const [x, y] = event?.get_coords() ?? [0, 0];
                const previous = this._historyClick;
                const settings = global.stage.context.get_settings();
                this._historyClick = pointer ? {id: series.id, time: event.get_time(), x, y} : null;
                if (
                    pointer &&
                    previous?.id === series.id &&
                    event.get_time() - previous.time >= 0 &&
                    event.get_time() - previous.time <= settings.double_click_time &&
                    Math.hypot(x - previous.x, y - previous.y) <= settings.double_click_distance
                ) {
                    this._historyClick = null;
                    this._updateHistoryVisibility('isolate_undo', series.id);
                    return;
                }
                toggle.checked = this._shown.has(series.id);
                this._updateHistoryVisibility('toggle', series.id);
            });
            (sideLegend
                ? legend
                : legendColumns[layoutWidth < 440 ? 0 : index < Math.ceil(this._history.series.length / 2) ? 0 : 1]
            ).add_child(toggle);
        }
        const help = label(COPY.seriesToggleHelp, 'tmb-history-legend-help');
        help.width = legend.width - 14;
        help.clutter_text.set_line_wrap(true);
        help.clutter_text.set_ellipsize(Pango.EllipsizeMode.NONE);
        legend.add_child(help);
    },
};

export type HistoryContentMethods = typeof historyContentMethods;
