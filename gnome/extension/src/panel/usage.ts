import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';
import type {TokenPanel} from '../panel.js';
import type {PanelState} from '../service-state.js';
import {COPY} from '../shared-copy.js';
import {identityChips} from '../usage-copy.js';
import {box, button, label} from './support.js';
import {extraMethods} from './usage-extras.js';
import {quotaMethods} from './usage-quotas.js';

const methods = {
    _renderUsage(this: TokenPanel, state: PanelState): void {
        const providers = state.providers;
        const settings = state.settings;
        if (state.first_run_pending) {
            const detected = this._providerIds.filter(id => providers[id]?.credential_usable);
            const welcome = box(false, 'tmb-first-run');
            welcome.add_child(
                new St.Icon({icon_name: 'starred-symbolic', icon_size: 22, style_class: 'tmb-first-run-icon'}),
            );
            const content = box(true, 'tmb-first-run-content');
            content.x_expand = true;
            const heading = label(
                detected.length
                    ? `Detected: ${detected.map(id => this._providerNames[id]).join(', ')}`
                    : COPY.noProvidersDetectedYet,
                'tmb-first-run-title',
            );
            heading.clutter_text.set_line_wrap(true);
            content.add_child(heading);
            const description = label(detected.length ? COPY.reviewProvidersHelp : COPY.firstRunHelp, 'tmb-secondary');
            description.clutter_text.set_line_wrap(true);
            content.add_child(description);
            const actions = box(false, 'tmb-first-run-actions');
            const review = button(
                COPY.reviewProviders,
                () => {
                    this._selectTab('Settings');
                },
                'tmb-control',
            );
            this._tooltip.attach(review, COPY.reviewProviders, 'Opens provider setup in Settings.');
            actions.add_child(review);
            const dismiss = button(
                'Got it',
                () => {
                    this._set('first_run_card_dismissed', true);
                },
                'tmb-control',
            );
            this._tooltip.attach(dismiss, 'Got it', COPY.dismissCardHelp);
            actions.add_child(dismiss);
            content.add_child(actions);
            welcome.add_child(content);
            this._page.add_child(welcome);
        }
        const copyChip = (value: string, kind: string): St.Button => {
            const chip = button(
                value,
                () => {
                    St.Clipboard.get_default().set_text(St.ClipboardType.CLIPBOARD, value);
                },
                'tmb-chip tmb-chip-action',
            );
            chip.accessible_name = `Copy ${kind}: ${value}`;
            this._tooltip.attach(chip, `Copy ${kind}`, `Copies the displayed ${kind} value.`);
            return chip;
        };
        this._quotaRows = new Map();
        const selected = new Set(state.selected_window_keys);
        let count = 0;
        for (const id of this._usageOrder) {
            const provider = providers[id];
            const local = id === 'claude' ? state.claude_local_usage : null;
            if (!provider?.show_in_usage) continue;
            const windows = provider.windows.filter(row => selected.has(`${id}:${row.id}`));
            count++;
            const card = box(true, 'tmb-card');
            const heading = box(true, 'tmb-card-header');
            const top = box(false, 'tmb-card-header-top');
            top.add_child(this._providerIcon(id));
            top.add_child(label(provider.name, 'tmb-card-title'));
            const badges = box(false, 'tmb-card-header-badges tmb-card-header-badges-inline');
            const extraBadges = box(false, 'tmb-card-header-badges');
            for (const chip of identityChips(provider)) {
                const value =
                    chip.kind === 'renewal'
                        ? `Renews ${new Intl.DateTimeFormat(undefined, {
                              month: 'short',
                              day: 'numeric',
                              year: 'numeric',
                          }).format(new Date(chip.value))}`
                        : chip.value;
                const container = chip.kind === 'workspace' || chip.kind === 'renewal' ? extraBadges : badges;
                container.add_child(
                    copyChip(value, chip.kind === 'renewal' ? 'renewal date' : chip.kind.replace('_', ' ')),
                );
            }
            const spacer = box();
            spacer.x_expand = true;
            top.add_child(spacer);
            const updated = label(state.time_presentation.provider_statuses[id], 'tmb-card-updated');
            updated.accessible_name = `${provider.name} status`;
            this._tooltip.attach(
                updated,
                `${provider.name} status`,
                () => state.time_presentation.provider_help[id] as string,
            );
            this._providerStatusLabels.push({actor: updated, id});
            if (this._root.width >= 440) top.add_child(updated);
            const signIn = provider.action === 'sign-in';
            const setup = provider.action === 'setup';
            const retry = provider.action === 'retry';
            const action = button(
                setup ? 'Set up' : signIn ? 'Sign in' : '',
                () => {
                    if (setup) this._showProviderSettings(id);
                    else if (signIn) this._showProviderSignIn(id);
                    else this._refresh(id);
                },
                setup || signIn ? 'tmb-small-control' : 'tmb-icon-button',
            );
            if (!setup && !signIn) action.set_child(new St.Icon({icon_name: 'view-refresh-symbolic', icon_size: 16}));
            action.accessible_name = `${
                setup ? 'Set up' : signIn ? 'Sign in to' : retry ? 'Retry' : 'Refresh'
            } ${provider.name}`;
            this._tooltip.attach(
                action,
                action.accessible_name,
                setup
                    ? 'Opens provider access settings and setup guidance for this issue.'
                    : signIn
                      ? 'Starts the provider login when supported, otherwise opens setup instructions. Cached values stay muted until fresh usage arrives.'
                      : retry
                        ? COPY.retryHelp
                        : COPY.refreshHelp,
            );
            top.add_child(action);
            if (signIn) {
                const settingsAction = new St.Button({
                    child: new St.Icon({icon_name: 'preferences-system-symbolic', icon_size: 16}),
                    style_class: 'tmb-icon-button',
                    can_focus: true,
                    reactive: true,
                    track_hover: true,
                });
                settingsAction.accessible_name = `Set up ${provider.name}`;
                settingsAction.connect('clicked', () => this._showProviderSettings(id));
                this._tooltip.attach(
                    settingsAction,
                    settingsAction.accessible_name,
                    `Opens setup and recovery for ${provider.name}. Last-known usage remains visible while access is repaired.`,
                );
                top.add_child(settingsAction);
            }
            const inline = badges.get_n_children() > 0 && this._root.width >= 560;
            if (inline) top.insert_child_at_index(badges, 2);
            else {
                badges.remove_style_class_name('tmb-card-header-badges-inline');
                if (
                    badges.get_preferred_width(-1)[1] + extraBadges.get_preferred_width(-1)[1] + 7 <=
                    this._root.width - 50
                ) {
                    for (const chip of extraBadges.get_children()) {
                        extraBadges.remove_child(chip);
                        badges.add_child(chip);
                    }
                }
            }
            heading.add_child(top);
            if (!inline && badges.get_n_children()) heading.add_child(badges);
            if (extraBadges.get_n_children()) heading.add_child(extraBadges);
            if (updated.get_parent() !== top) {
                updated.x_align = Clutter.ActorAlign.END;
                heading.add_child(updated);
            }
            card.add_child(heading);
            const body = box(true, 'tmb-card-body');
            for (const warning of provider.warnings) {
                const callout = box(false, 'tmb-provider-notice tmb-provider-notice-warning');
                callout.accessible_name = `Warning: ${warning}`;
                callout.add_child(new St.Icon({icon_name: 'dialog-warning-symbolic', icon_size: 14}));
                const message = label(warning, 'tmb-provider-notice-text');
                message.width = Math.max(210, this._root.width - 105);
                message.clutter_text.set_line_wrap(true);
                callout.add_child(message);
                body.add_child(callout);
            }
            for (const notice of provider.notices) {
                const warning = notice.kind === 'limitReached' || notice.kind === 'spendControl';
                const callout = box(false, `tmb-provider-notice${warning ? ' tmb-provider-notice-warning' : ''}`);
                callout.add_child(
                    new St.Icon({
                        icon_name: warning ? 'dialog-warning-symbolic' : 'dialog-information-symbolic',
                        icon_size: 14,
                    }),
                );
                const message = label(notice.text, 'tmb-provider-notice-text');
                message.width = Math.max(210, this._root.width - 105);
                message.clutter_text.set_line_wrap(true);
                callout.add_child(message);
                body.add_child(callout);
            }
            this._renderQuotaRows(id, provider, windows, settings, body);
            this._renderUsageExtras(state, id, provider, local, body);
            card.add_child(body);
            this._page.add_child(card);
        }
        if (!count) {
            this._root.width = Math.min(this._root.width, 620);
            const narrow = this._root.width < 440;
            const empty = box(narrow, 'tmb-usage-empty');
            const summary = narrow ? box(false, 'tmb-usage-empty-summary') : empty;
            summary.add_child(
                new St.Icon({
                    icon_name: 'preferences-system-symbolic',
                    icon_size: 26,
                    style_class: 'tmb-usage-empty-icon',
                }),
            );
            const copy = box(true, 'tmb-usage-empty-copy');
            copy.x_expand = true;
            copy.add_child(label(state.empty_usage_title, 'tmb-first-run-title'));
            const description = label(COPY.setUpProvidersHint, 'tmb-secondary');
            description.clutter_text.set_line_wrap(true);
            copy.add_child(description);
            summary.add_child(copy);
            if (narrow) empty.add_child(summary);
            const review = button(
                COPY.openProviders,
                () => {
                    this._selectTab('Settings');
                },
                'tmb-control',
            );
            if (narrow) review.x_align = Clutter.ActorAlign.END;
            else review.y_align = Clutter.ActorAlign.CENTER;
            empty.add_child(review);
            this._page.add_child(empty);
        }
    },

    _providerIcon(this: TokenPanel, id: string): St.Icon {
        const file = id === 'claude' || id === 'gemini' ? `${id}.svg` : `${id}-symbolic.svg`;
        const path = GLib.build_filenamev([this.path, 'icons', file]);
        return new St.Icon({
            gicon: new Gio.FileIcon({file: Gio.File.new_for_path(path)}),
            icon_size: 21,
            style_class: 'tmb-provider-icon',
        });
    },

    _keyValue(this: TokenPanel, title: string, value: string): St.BoxLayout {
        const row = box(false, 'tmb-key-value');
        const name = label(title, 'tmb-secondary');
        name.x_expand = true;
        row.add_child(name);
        row.add_child(label(value, 'tmb-key-value-number'));
        return row;
    },
};

export const usageMethods = {...methods, ...quotaMethods, ...extraMethods};
