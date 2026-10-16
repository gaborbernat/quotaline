import Clutter from 'gi://Clutter';
import Pango from 'gi://Pango';
import St from 'gi://St';
import {usageColor} from '../color.js';
import type {TokenPanel} from '../panel.js';
import type {PanelProvider, PanelWindow, ResetGroupEdge, Settings} from '../service-state.js';
import {box, label, meter, percent, resetText} from './support.js';

export const quotaMethods = {
    _renderQuotaRows(
        this: TokenPanel,
        id: string,
        provider: PanelProvider,
        windows: PanelWindow[],
        settings: Settings,
        body: St.BoxLayout,
    ): void {
        const inline = this._root.width >= 600;
        const medium = inline && this._compact;
        for (const [index, row] of windows.entries()) {
            // GetState emits one reset edge per displayed window.
            const {with_previous: sharesPrevious, with_next: sharesNext} = provider.reset_group_edges[
                index
            ] as ResetGroupEdge;
            const grouped = sharesPrevious || sharesNext;
            const used = settings.show_as === 'remaining' ? 100 - row.used : row.used;
            const block = box(true, 'tmb-window');
            const primary = box(false, `tmb-quota-row${inline ? '' : ' tmb-quota-row-compact'}`);
            const title = box(true, 'tmb-quota-title-group');
            const heading = box(false, 'tmb-quota-heading');
            heading.add_child(label(row.label, 'tmb-quota-title'));
            title.add_child(heading);
            const context = label(
                [row.detail, !row.active ? 'inactive' : ''].filter(Boolean).join(' · '),
                'tmb-quota-detail tmb-quota-identifier',
            );
            context.width = inline ? (medium ? 160 : 230) : Math.max(100, this._root.width - 112);
            context.clutter_text.set_ellipsize(Pango.EllipsizeMode.END);
            title.add_child(context);
            if (!inline) title.x_expand = true;
            else title.width = medium ? 160 : 230;
            primary.add_child(title);
            const meterWidth = inline ? (medium ? Math.max(160, this._root.width - 425) : 360) : this._root.width - 82;
            const track = meter(
                meterWidth,
                row.used,
                provider.status === 'stale' ? '#808080' : usageColor(row.used, row.pace.status),
                {y_align: Clutter.ActorAlign.CENTER},
            );
            if (row.pace.expected_percent !== null) {
                const marker = new St.Widget({style_class: 'tmb-meter-expected', width: 2, height: 6});
                marker.x = Math.min(
                    meterWidth - 2,
                    Math.max(0, Math.floor((meterWidth * row.pace.expected_percent) / 100)),
                );
                track.add_child(marker);
            }
            if (!inline) {
                primary.add_child(label(percent(used), 'tmb-quota-value'));
                block.add_child(primary);
                block.add_child(track);
                if (row.duration_seconds) block.add_child(label(row.pace_labels.compact, 'tmb-quota-pace'));
                if (!grouped)
                    block.add_child(
                        label(resetText(row.reset, row.reset_precision), 'tmb-quota-reset tmb-quota-reset-compact'),
                    );
            } else {
                const meterColumn = box(true, 'tmb-meter-column');
                meterColumn.width = meterWidth;
                meterColumn.add_child(track);
                if (row.duration_seconds) {
                    const pace = label(medium ? row.pace_labels.brief : row.pace_labels.full, 'tmb-quota-pace');
                    pace.width = meterWidth;
                    meterColumn.add_child(pace);
                }
                primary.add_child(meterColumn);
                primary.add_child(label(percent(used), 'tmb-quota-value'));
                if (grouped) primary.add_child(new St.Widget({width: 120}));
                else {
                    const reset = label(resetText(row.reset, row.reset_precision), 'tmb-quota-reset');
                    reset.width = 120;
                    reset.clutter_text.set_line_wrap(true);
                    reset.clutter_text.set_ellipsize(Pango.EllipsizeMode.NONE);
                    primary.add_child(reset);
                }
                block.add_child(primary);
            }
            this._tooltip.attach(block, row.label, row.help);
            block.accessible_name = `${provider.name}, ${row.label}, ${percent(row.used)} used`;
            this._quotaRows.set(`${id}:${row.id}`, block);
            body.add_child(block);
            if (sharesPrevious && !sharesNext) {
                const reset = label(
                    resetText(row.reset, row.reset_precision),
                    `tmb-quota-group-reset${!inline ? ' tmb-quota-group-reset-compact' : ''}`,
                );
                reset.x_align = inline ? Clutter.ActorAlign.END : Clutter.ActorAlign.START;
                body.add_child(reset);
            }
        }
    },
};

export type QuotaMethods = typeof quotaMethods;
