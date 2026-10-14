import type {TokenPanel} from '../panel.js';
import type {PanelState} from '../service-state.js';
import {settingsDataMethods} from './settings-data.js';
import {settingsModelMethods} from './settings-models.js';
import type {SettingsContext} from './settings-overview.js';
import {settingsOverviewMethods} from './settings-overview.js';
import {settingsProviderMethods} from './settings-providers.js';

export const settingsMethods = {
    _renderSettings(this: TokenPanel, state: PanelState, labelDraft: [string, string] | null): void {
        this._settingFocus = new Map();
        if (this._fullLog) {
            this._section('Full log');
            this._renderLog(state);
            return;
        }
        const settings = state.settings;
        const context: SettingsContext = {
            settings,
            visibility: state.settings_context.visibility,
            narrow: this._root.width < 650,
            veryNarrow: this._root.width < 520,
            fullLabels: this._root.width >= 800,
            providerOrder: state.ordering.providers,
        };
        this._renderSettingsOverview(state, context);
        this._renderSettingsModels(state, labelDraft, context);
        this._renderSettingsProviders(state, context);
        this._renderSettingsData(state, context);
    },
    ...settingsOverviewMethods,
    ...settingsModelMethods,
    ...settingsProviderMethods,
    ...settingsDataMethods,
};
