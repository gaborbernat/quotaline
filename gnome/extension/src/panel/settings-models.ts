import Clutter from 'gi://Clutter';
import Pango from 'gi://Pango';
import St from 'gi://St';
import {usageColor} from '../color.js';
import type {TokenPanel} from '../panel.js';
import type {ModelWindow} from '../presentation.js';
import {conflictingModelName} from '../presentation.js';
import type {PanelProvider, PanelState, ServiceState, ShortLabelPreview} from '../service-state.js';
import {COPY} from '../shared-copy.js';
import type {SettingsContext} from './settings-overview.js';
import {box, button, checkButton, label, meter, shortLabel} from './support.js';

export const settingsModelMethods = {
    _renderSettingsModels(
        this: TokenPanel,
        state: PanelState,
        labelDraft: [string, string] | null | undefined,
        {settings, providerOrder, veryNarrow}: SettingsContext,
    ): void {
        this._loadModelGroups();
        const visibleProviderOrder = Object.entries(state.providers)
            .filter(([, provider]) => provider.windows.length)
            .map(([id]) => id)
            .sort((left, right) => providerOrder.indexOf(left) - providerOrder.indexOf(right));
        const allWindows = new Map(
            Object.entries(state.providers).flatMap(([id, provider]) =>
                provider.windows.map((row): [string, ModelWindow] => [`${id}:${row.id}`, {id, row, provider}]),
            ),
        );
        const modelControls = box(this._root.width < 740, 'tmb-setting-controls');
        modelControls.x_align = Clutter.ActorAlign.START;
        const modelFilter = new St.Entry({
            hint_text: COPY.filterModelsPlaceholder,
            text: this._modelFilter,
            style_class: 'tmb-model-filter',
            can_focus: true,
        });
        this._modelFilterEntry = modelFilter;
        modelFilter.width = Math.min(425, this._root.width - (veryNarrow ? 36 : this._compact ? 140 : 160));
        modelFilter.x_align = Clutter.ActorAlign.START;
        this._tooltip.attach(
            modelFilter,
            COPY.filterModels,
            'Matches provider names, model names, identifiers and short labels without changing selection.',
        );
        modelControls.add_child(modelFilter);
        const hideUnused = this._check(state, COPY.hideUnusedInRange, 'hide_unused_models');
        hideUnused.x_align = Clutter.ActorAlign.START;
        modelControls.add_child(hideUnused);
        const modelRow = this._settingRow('Models', modelControls);
        if (this._root.width < 740 && !veryNarrow)
            modelRow.height = modelFilter.get_preferred_height(-1)[1] + hideUnused.get_preferred_height(-1)[1] + 7;
        this._settingNote(
            'Checked limits appear in the menu bar, Usage and History. Activity uses retained samples and the current quota window.',
        );
        modelFilter.clutter_text.connect('text-changed', () => {
            this._modelFilter = modelFilter.get_text();
            this._revealedModelKey = null;
            this._loadModelGroups();
        });
        if (this._modelGroupsError) {
            this._settingNote(`Model list unavailable: ${this._modelGroupsError}`);
            this._settingsSection.add_child(
                button('Retry model list', () => {
                    this._loadModelGroups(true);
                }),
            );
        } else if (!this._modelGroups) {
            this._settingNote('Loading models…');
        }
        for (const projected of this._modelGroups ?? []) {
            const id = projected.provider;
            const provider = state.providers[id] as PanelProvider;
            const windows = provider.windows;
            const visibleWindows = projected.rows.flatMap(entry =>
                windows
                    .filter(row => `${id}:${row.id}` === entry.key)
                    .map(row => [entry.position, row, entry] as const),
            );
            const group = box(true, 'tmb-model-group-card');
            const heading = box(veryNarrow, 'tmb-model-heading');
            const headingMain = veryNarrow ? box(false, 'tmb-setting-controls') : heading;
            if (veryNarrow) heading.add_child(headingMain);
            const icon = this._providerIcon(id);
            icon.icon_size = 19;
            headingMain.add_child(icon);
            headingMain.add_child(label(provider.name, 'tmb-model-heading-name'));
            const selectedCount = projected.selected_count;
            headingMain.add_child(label(`${selectedCount} of ${projected.total_count} shown`, 'tmb-secondary'));
            const spacer = box();
            spacer.x_expand = true;
            headingMain.add_child(spacer);
            const all = checkButton(
                'All',
                selectedCount === projected.total_count,
                () => {
                    this._call('ToggleProviderModels', '(s)', [id], raw => {
                        this._apply(raw);
                    });
                },
                'tmb-setting-check',
                selectedCount > 0 && selectedCount < windows.length,
            );
            all.accessible_name = `Show all ${provider.name} models`;
            this._tooltip.attach(
                all,
                all.accessible_name,
                'Selects or clears this provider’s models as a group. At least one model stays selected.',
            );
            all.reactive = projected.can_toggle_all;
            all.can_focus = all.reactive;
            all.opacity = all.reactive ? 255 : 102;
            headingMain.add_child(all);
            if (settings.order === 'stable') {
                headingMain.add_child(this._dragHandle('provider', id, provider.name));
                this._reorderTarget(heading, 'provider', id, (source, target, after) => {
                    this._moveProviderTo(source, target, after);
                });
                const reorder = box(false, 'tmb-reorder-controls');
                const position = visibleProviderOrder.indexOf(id);
                reorder.add_child(
                    this._reorderButton(`Move ${provider.name} up`, 'up', position > 0, () => {
                        this._moveProvider(state, id, -1);
                    }),
                );
                reorder.add_child(
                    this._reorderButton(
                        `Move ${provider.name} down`,
                        'down',
                        position < visibleProviderOrder.length - 1,
                        () => {
                            this._moveProvider(state, id, 1);
                        },
                    ),
                );
                if (veryNarrow) reorder.x_align = Clutter.ActorAlign.END;
                heading.add_child(reorder);
            }
            group.add_child(heading);
            this._settingsSection.add_child(group);
            for (const [position, row, projectedRow] of visibleWindows) {
                const key = `${id}:${row.id}`;
                const chosen = projectedRow.selected;
                const item = box(veryNarrow, 'tmb-model-row');
                const identity = veryNarrow ? box(false, 'tmb-model-row') : item;
                if (veryNarrow) item.add_child(identity);
                const select = checkButton(
                    '',
                    chosen,
                    () => {
                        this._call('ToggleModel', '(s)', [key], raw => {
                            this._apply(raw);
                        });
                    },
                    `tmb-model-check${chosen ? ' tmb-model-check-on' : ''}`,
                );
                select.accessible_name = `Show ${provider.name} ${row.label}`;
                this._tooltip.attach(
                    select,
                    COPY.modelSelection,
                    `Shows ${row.label} in the menu bar, Usage and History. Unchecking keeps stored history.`,
                );
                select.reactive = projectedRow.can_toggle;
                select.can_focus = select.reactive;
                select.opacity = select.reactive ? 255 : 102;
                identity.add_child(select);
                const title = box(true, 'tmb-model-title');
                title.width = veryNarrow ? 118 : this._compact ? 170 : 235;
                const name = label(row.label, 'tmb-model-name');
                name.clutter_text.set_ellipsize(Pango.EllipsizeMode.END);
                title.add_child(name);
                const modelId = label(row.id, 'tmb-model-id');
                modelId.clutter_text.set_ellipsize(Pango.EllipsizeMode.MIDDLE);
                title.add_child(modelId);
                identity.add_child(title);
                const track = meter(veryNarrow ? this._root.width - 60 : 92, row.used, usageColor(row.used), {
                    height: 4,
                    track: 'tmb-model-meter',
                });
                const last = projectedRow.last_used_at;
                const recency = last
                    ? new Date(last * 1000).toDateString() === new Date().toDateString()
                        ? 'today'
                        : `last ${new Date(last * 1000).toLocaleDateString([], {month: 'short', day: 'numeric'})}`
                    : COPY.noUsageRecorded;
                track.accessible_name = `${provider.name} ${row.label} usage, ${row.used.toFixed(2)}%, ${recency}`;
                this._tooltip.attach(
                    track,
                    COPY.usageAndRecency,
                    `Shows the latest ${row.label} percentage and when usage was last observed.`,
                );
                const usage = veryNarrow
                    ? box(true, 'tmb-model-usage')
                    : label(`${row.used.toFixed(2)}% ${recency}`, 'tmb-model-percent');
                if (veryNarrow) {
                    const precise = label(`${row.used.toFixed(2)}%`, 'tmb-model-percent');
                    precise.width = 74;
                    precise.x_align = Clutter.ActorAlign.END;
                    usage.add_child(precise);
                    const recent = label(recency, 'tmb-model-recency');
                    recent.width = 74;
                    recent.clutter_text.set_ellipsize(Pango.EllipsizeMode.END);
                    recent.x_align = Clutter.ActorAlign.END;
                    usage.add_child(recent);
                }
                usage.width = veryNarrow ? 74 : 105;
                if (veryNarrow) {
                    identity.add_child(usage);
                    item.add_child(track);
                } else {
                    const gap = box();
                    gap.x_expand = true;
                    item.add_child(gap);
                    const usageBlock = box(true, 'tmb-model-usage');
                    usageBlock.width = 105;
                    usageBlock.add_child(usage);
                    track.x_align = Clutter.ActorAlign.END;
                    usageBlock.add_child(track);
                    item.add_child(usageBlock);
                }
                const actions = veryNarrow ? box(false, 'tmb-model-row') : item;
                if (veryNarrow) item.add_child(actions);
                const originalLabel = projectedRow.default_label;
                const savedLabel = projectedRow.label;
                const input = new St.Entry({
                    text: labelDraft?.[0] === key ? labelDraft[1] : (this._labelDrafts.get(key) ?? savedLabel),
                    style_class: 'tmb-short-entry',
                    can_focus: true,
                });
                this._labelEntries.set(key, input);
                input.width = 64;
                input.accessible_name = `${provider.name} ${row.label} short label, ${state.short_label_limit} characters maximum`;
                this._tooltip.attach(
                    input,
                    COPY.shortLabel,
                    `Sets this model’s menu bar label, up to ${state.short_label_limit} characters. Clearing the field restores its derived label.`,
                );
                const conflictIcon = label('⚠', 'tmb-label-conflict');
                conflictIcon.visible = false;
                this._labelConflictIcons.set(key, conflictIcon);
                let conflictHelp = '';
                this._tooltip.attach(conflictIcon, COPY.duplicateShortLabel, () => conflictHelp);
                let validationRequest = 0;
                input.connect('destroy', () => {
                    validationRequest++;
                });
                const showConflict = (conflictKey: string | null) => {
                    conflictIcon.visible = Boolean(conflictKey);
                    if (conflictKey) {
                        input.add_style_class_name('tmb-short-entry-conflict');
                        conflictHelp = `Already used by ${conflictingModelName(allWindows, conflictKey)}; saved label remains ${savedLabel}.`;
                    } else {
                        this._tooltip.hide(conflictIcon);
                        input.remove_style_class_name('tmb-short-entry-conflict');
                    }
                    input.accessible_name = `${provider.name} ${row.label} short label, ${state.short_label_limit} characters maximum${
                        conflictKey ? `; already used by ${conflictingModelName(allWindows, conflictKey)}` : ''
                    }`;
                };
                const updateConflict = () => {
                    const request = ++validationRequest;
                    if (input.get_text() === savedLabel) {
                        showConflict(null);
                        return;
                    }
                    this._call('PreviewShortLabel', '(ss)', [key, input.get_text()], raw => {
                        if (request !== validationRequest || !input.get_parent()) return;
                        const preview = JSON.parse(raw) as ShortLabelPreview;
                        showConflict(preview.conflict_key);
                    });
                };
                input.clutter_text.connect('text-changed', () => {
                    const trimmed = shortLabel(input.get_text(), state.short_label_limit);
                    if (trimmed !== input.get_text()) {
                        input.set_text(trimmed);
                        return;
                    }
                    if (input.get_text() === savedLabel) this._labelDrafts.delete(key);
                    else this._labelDrafts.set(key, input.get_text());
                    updateConflict();
                });
                updateConflict();
                let submitted = false;
                const save = () => {
                    if (this._suppressedLabelSaves.has(input) || submitted || input.get_text().trim() === savedLabel)
                        return;
                    submitted = true;
                    const draft = input.get_text();
                    this._call('SetSetting', '(ss)', [`short_label:${key}`, JSON.stringify(draft)], raw => {
                        submitted = false;
                        const saved = (JSON.parse(raw) as ServiceState).settings.short_labels[key];
                        if (saved !== settings.short_labels[key] && this._labelDrafts.get(key) === draft)
                            this._labelDrafts.delete(key);
                        this._apply(raw);
                    });
                };
                input.clutter_text.connect('activate', save);
                input.clutter_text.connect('key-focus-out', save);
                actions.add_child(input);
                actions.add_child(conflictIcon);
                if (settings.short_labels[key] && savedLabel !== originalLabel) {
                    const revert = button(
                        '',
                        () => {
                            this._suppressedLabelSaves.add(input);
                            this._labelDrafts.delete(key);
                            this._set(
                                'short_labels',
                                Object.fromEntries(
                                    Object.entries(settings.short_labels).filter(([labelKey]) => labelKey !== key),
                                ),
                            );
                        },
                        'tmb-model-revert',
                    );
                    revert.set_child(new St.Icon({icon_name: 'edit-undo-symbolic', icon_size: 12}));
                    revert.accessible_name = `Revert label for ${provider.name} ${row.label}`;
                    this._tooltip.attach(revert, COPY.revertShortLabel, `Restores ${originalLabel}.`);
                    actions.add_child(revert);
                }
                if (settings.order === 'stable') {
                    actions.add_child(this._dragHandle('model', key, `${provider.name} ${row.label}`));
                    this._reorderTarget(item, 'model', key, (source, target, after) => {
                        this._moveModelTo(source, target, after);
                    });
                    const space = box();
                    if (veryNarrow) space.x_expand = true;
                    else space.width = this._compact ? 0 : 64;
                    actions.add_child(space);
                    actions.add_child(
                        this._reorderButton(`Move ${provider.name} ${row.label} up`, 'up', position > 0, () => {
                            this._moveModel(state, id, key, -1);
                        }),
                    );
                    actions.add_child(
                        this._reorderButton(
                            `Move ${provider.name} ${row.label} down`,
                            'down',
                            position < windows.length - 1,
                            () => {
                                this._moveModel(state, id, key, 1);
                            },
                        ),
                    );
                }
                group.add_child(item);
                if (veryNarrow)
                    item.height =
                        identity.get_preferred_height(-1)[1] + actions.get_preferred_height(-1)[1] + track.height + 14;
            }
        }
        if (state.selected_model_count === 1) this._settingNote(COPY.lastModelSelected);
    },
};

export type SettingsModelMethods = typeof settingsModelMethods;
