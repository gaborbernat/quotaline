import Pango from 'gi://Pango';
import {credentialHealthLabel, credentialSourceCopy} from '../credential-copy.js';
import type {TokenPanel} from '../panel.js';
import type {PanelState, ProviderDefinition} from '../service-state.js';
import {COPY} from '../shared-copy.js';
import type {SettingsContext} from './settings-overview.js';
import {box, button, checkButton, label} from './support.js';

export const settingsProviderMethods = {
    _renderSettingsProviders(
        this: TokenPanel,
        state: PanelState,
        {settings, providerOrder, visibility, narrow, veryNarrow}: SettingsContext,
    ): void {
        this._section('Providers');
        this._settingRow(COPY.showAllProviders, this._check(state, COPY.showAllProviders, 'show_all_providers'));
        let visibleProviders = 0;
        for (const [id, provider] of Object.entries(state.providers).sort(
            ([left], [right]) => providerOrder.indexOf(left) - providerOrder.indexOf(right),
        )) {
            const discovered = provider.discovered;
            if (!provider.show_in_settings && !this._providerDetails.has(id)) continue;
            visibleProviders++;
            const providerRow = box(veryNarrow, 'tmb-setting-controls');
            const providerHeader = veryNarrow ? box(false, 'tmb-setting-controls') : providerRow;
            const providerActions = veryNarrow ? box(false, 'tmb-setting-controls') : providerRow;
            if (veryNarrow) {
                providerRow.add_child(providerHeader);
                providerRow.add_child(providerActions);
            }
            const enabled =
                settings.providers[id] === true && (discovered || settings.configured_providers.includes(id));
            const toggle = checkButton(
                '',
                enabled,
                () => {
                    this._set(`provider:${id}`, !enabled);
                },
                `tmb-setting-check${enabled ? ' tmb-setting-check-on' : ''}`,
            );
            toggle.accessible_name = `Enable ${provider.name}`;
            this._tooltip.attach(
                toggle,
                `${provider.name} provider`,
                'Discovered providers turn on automatically. Changing this checkbox saves an explicit polling choice. Turning it off keeps stored history, account details and provider settings.',
            );
            providerHeader.add_child(toggle);
            providerHeader.add_child(this._providerIcon(id));
            const availability: Record<string, string> = {
                'sign-in': 'Sign in needed',
                'rate-limited': COPY.rateLimited,
                stale: 'Last reading',
            };
            const source = credentialSourceCopy(provider);
            const status = label(
                !settings.providers[id]
                    ? 'Off'
                    : !discovered
                      ? COPY.notConfigured
                      : provider.status === 'connected'
                        ? `${provider.plan || 'Connected'} · ${
                              source === COPY.demoData
                                  ? 'Demo'
                                  : source.startsWith('/')
                                    ? 'Local'
                                    : source || 'Signed in'
                          }`
                        : availability[provider.status] || provider.status,
                `tmb-provider-setting-status${
                    !settings.providers[id] || !discovered || provider.status === 'connected'
                        ? ''
                        : ' tmb-provider-setting-warning'
                }`,
            );
            status.width = veryNarrow ? 150 : narrow ? 170 : this._compact ? 180 : 260;
            status.clutter_text.set_ellipsize(Pango.EllipsizeMode.END);
            providerHeader.add_child(status);
            if (settings.providers[id] && discovered) {
                const minimum = (
                    state.provider_definitions.find(definition => definition.id === id) as ProviderDefinition
                ).minimum_minutes;
                const minutes = settings.interval_minutes[id] as number;
                const interval = box(false, 'tmb-interval-stepper');
                const change = (delta: number) => {
                    this._call('StepProviderInterval', '(si)', [id, delta], raw => {
                        this._apply(raw);
                    });
                };
                const decrease = button(
                    '−',
                    () => {
                        change(-1);
                    },
                    'tmb-small-control tmb-interval-button',
                );
                decrease.accessible_name = `Decrease ${provider.name} refresh interval`;
                this._tooltip.attach(
                    decrease,
                    `${provider.name} refresh interval`,
                    'Sets background usage polling. Shorter intervals use more network and CPU. An open menu uses the provider minimum.',
                );
                decrease.reactive = minutes > minimum;
                decrease.can_focus = decrease.reactive;
                decrease.opacity = decrease.reactive ? 255 : 102;
                interval.add_child(decrease);
                interval.add_child(label(`Every ${minutes} min`, 'tmb-secondary'));
                const increase = button(
                    '+',
                    () => {
                        change(1);
                    },
                    'tmb-small-control tmb-interval-button',
                );
                increase.accessible_name = `Increase ${provider.name} refresh interval`;
                this._tooltip.attach(
                    increase,
                    `${provider.name} refresh interval`,
                    'Sets background usage polling. Shorter intervals use more network and CPU. An open menu uses the provider minimum.',
                );
                increase.reactive = minutes < state.maximum_refresh_minutes;
                increase.can_focus = increase.reactive;
                increase.opacity = increase.reactive ? 255 : 102;
                interval.add_child(increase);
                providerActions.add_child(interval);
                const refresh = button(
                    '↻',
                    () => {
                        this._refresh(id);
                    },
                    'tmb-small-control',
                );
                refresh.accessible_name = `Check ${provider.name} again`;
                this._tooltip.attach(
                    refresh,
                    refresh.accessible_name,
                    `Checks only ${provider.name}. Stored usage and the last successful reading remain available.`,
                );
                providerActions.add_child(refresh);
            }
            const details = box(true, 'tmb-provider-setting-details');
            details.style = `margin-left: ${veryNarrow ? 8 : this._compact ? 107 : 124}px;`;
            details.visible = this._providerDetails.has(id);
            details.add_child(
                label(`Authentication · ${source || credentialHealthLabel(provider)}`, 'tmb-provider-detail'),
            );
            details.add_child(
                label(
                    `Status · ${provider.status}${
                        provider.updated ? ` · last success ${new Date(provider.updated * 1000).toLocaleString()}` : ''
                    }`,
                    'tmb-provider-detail',
                ),
            );
            if (provider.notice) {
                const issue = label(`Issue · ${provider.notice}`, 'tmb-provider-detail');
                issue.width = Math.max(180, this._root.width - (veryNarrow ? 54 : this._compact ? 150 : 174));
                issue.clutter_text.set_line_wrap(true);
                issue.clutter_text.set_ellipsize(Pango.EllipsizeMode.NONE);
                details.add_child(issue);
            }
            if (provider.identity) details.add_child(label(`Account · ${provider.identity}`, 'tmb-provider-detail'));
            const detailActions = box(false, 'tmb-provider-detail-actions');
            const guide = button(
                COPY.setupGuide,
                () => {
                    this._openDocument('provider-setup.html', id);
                },
                'tmb-small-control',
            );
            guide.accessible_name = `Setup guide for ${provider.name}`;
            this._tooltip.attach(
                guide,
                guide.accessible_name,
                `Opens the ${provider.name} setup guide in your default browser.`,
            );
            detailActions.add_child(guide);
            const signIn = button(
                'Sign in',
                () => {
                    this._startSignIn(id);
                },
                'tmb-small-control',
            );
            signIn.accessible_name = `Sign in to ${provider.name}`;
            this._tooltip.attach(
                signIn,
                signIn.accessible_name,
                `Starts the ${provider.name} client login when supported. Otherwise opens its setup guide.`,
            );
            detailActions.add_child(signIn);
            details.add_child(detailActions);
            const disclosure = button(
                details.visible ? '▾' : '▸',
                () => {
                    details.visible = !details.visible;
                    disclosure.set_label(details.visible ? '▾' : '▸');
                    if (details.visible) this._providerDetails.add(id);
                    else this._providerDetails.delete(id);
                },
                'tmb-small-control',
            );
            disclosure.accessible_name = `Connection details for ${provider.name}`;
            this._tooltip.attach(
                disclosure,
                disclosure.accessible_name,
                `Shows the ${provider.name} authentication source, status, account and recovery actions. Tokens stay hidden.`,
            );
            providerHeader.add_child(disclosure);
            this._providerRows.set(id, this._settingRow(provider.name, providerRow));
            this._settingsSection.add_child(details);
        }
        if (!visibleProviders) this._settingNote('No providers discovered. Select Show all providers to set one up.');
        if (visibility.tokenRefresh) {
            this._settingRow(
                COPY.tokenRefresh,
                this._check(
                    state,
                    veryNarrow
                        ? 'Refresh expired tokens'
                        : 'Refresh expired Claude, Codex, Gemini and Antigravity tokens',
                    'allow_token_refresh',
                ),
            );
            this._settingNote(
                'Claude, Codex and Gemini save refreshed tokens. Antigravity keeps its token in memory. A concurrent CLI change stops a credential save.',
            );
        }
    },
};

export type SettingsProviderMethods = typeof settingsProviderMethods;
