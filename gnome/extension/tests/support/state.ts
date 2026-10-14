import type {
    History,
    HistorySeries,
    MetricDefinition,
    PanelProvider,
    ProviderDefinition,
    ProviderView,
    ServiceState,
    Settings,
    SpendControl,
    StatusCell,
    Window,
} from '../../src/service-state.js';

export function serviceState<Providers extends Record<string, ProviderView>>(
    providers: Providers,
    overrides: Partial<Omit<ServiceState, 'providers'>> = {},
): ServiceState & {providers: Providers} {
    return {
        version: 1,
        app_version: 'test',
        provider_definitions: [],
        notification_threshold_choices: [],
        notification_summary: '',
        maximum_refresh_minutes: 60,
        providers,
        settings: settings(),
        settings_context: {visibility: {analytics: false, pace: false, tokenRefresh: false, expiringCredits: false}},
        ordering: {providers: [], models: []},
        model_groups: [],
        selected_model_count: 0,
        first_run_pending: false,
        status_entries: [],
        status_provider_indices: [],
        status_mini_groups: [],
        template_tokens: [],
        snapshot_attention: false,
        default_short_labels: {},
        resolved_short_labels: {},
        short_label_limit: 4,
        default_selected_windows: [],
        selected_window_keys: [],
        history_path: '',
        metric_definitions: [],
        metric_groups: [],
        claude_local_usage: null,
        local_metrics: null,
        last_refresh: null,
        is_refreshing: false,
        refreshing_providers: [],
        now: 100,
        ...overrides,
    };
}

export function settings(overrides: Partial<Settings> = {}): Settings {
    return {
        providers: {},
        configured_providers: [],
        provider_order: [],
        model_order: [],
        hide_unused_models: false,
        show_all_providers: false,
        allow_token_refresh: false,
        interval_minutes: {},
        format: 'stacked',
        order: 'provider',
        show_as: 'used',
        decimals: 0,
        hide_zero: false,
        fit_to_space: false,
        hide_identity: false,
        short_labels: {},
        selected_windows: [],
        has_custom_selection: false,
        template: '',
        retention_days: 30,
        analytics_interval_minutes: 5,
        write_usage_json: false,
        widget_size: 'small',
        last_tab: 'Usage',
        history_days: 7,
        history_metric: 'usage',
        history_rollup: 'hour',
        history_utc: false,
        history_stacked: false,
        history_hidden_keys: [],
        notifications: false,
        notify_thresholds: [],
        notify_resets: false,
        notify_sign_in: false,
        notify_expiring_credits: false,
        notify_pace: false,
        notify_sound: false,
        detailed_logging: false,
        first_run_card_dismissed: false,
        workdays: 5,
        demo: false,
        ...overrides,
    };
}

export function provider(
    overrides: Partial<Omit<ProviderView, 'spend' | 'spend_projection' | 'cloud_credits' | 'cloud_projection'>> = {},
): ProviderView {
    return {
        id: 'claude',
        name: 'Claude',
        status: 'connected',
        credential_usable: true,
        credential_health: 'valid',
        recovery_status: null,
        plan: '',
        identity: '',
        organization: '',
        source: '',
        windows: [],
        spend: null,
        spend_projection: null,
        details: [],
        credit_balance: null,
        usage_credits: null,
        reset_credits: null,
        cloud_credits: null,
        cloud_projection: null,
        reset_credit_expiries: [],
        notices: [],
        warnings: [],
        notice: '',
        updated: null,
        retry_after: 0,
        action: 'refresh',
        discovered: true,
        show_in_settings: true,
        show_in_usage: true,
        show_organization: false,
        credit_metrics: [],
        reset_group_edges: [],
        ...overrides,
    };
}

export function quotaWindow(overrides: Partial<Window> = {}): Window {
    return {
        id: 'session',
        label: 'Current session',
        used: 0,
        active: true,
        reset: null,
        reset_precision: 'instant',
        group: 'session',
        detail: '',
        duration_seconds: null,
        pace: {status: 'unknown', expected_percent: null, ratio: null, projected_exhaustion: null},
        ...overrides,
    };
}

export function panelProvider(overrides: Parameters<typeof provider>[0] = {}): PanelProvider {
    const reading = provider(overrides);
    return {
        ...reading,
        windows: reading.windows.map(window => ({
            ...window,
            help: '',
            pace_labels: {full: '', compact: '', brief: ''},
        })),
    };
}

export function spend(overrides: Partial<SpendControl> = {}): SpendControl {
    return {
        enabled: null,
        can_toggle: false,
        disabled_reason: null,
        used: null,
        limit: null,
        percent: null,
        currency: null,
        reset: null,
        limit_reached: false,
        balance: null,
        auto_reload: null,
        can_purchase_credits: false,
        credit_allowance: null,
        ...overrides,
    };
}

export function providerDefinition(overrides: Partial<ProviderDefinition> = {}): ProviderDefinition {
    return {
        id: 'claude',
        name: 'Claude',
        code: 'CC',
        login: '',
        minimum_minutes: 1,
        default_minutes: 5,
        supports_analytics: false,
        ...overrides,
    };
}

export function metricDefinition(overrides: Partial<MetricDefinition> = {}): MetricDefinition {
    return {
        id: 'usage',
        title: 'Usage',
        unit: '%',
        display_unit: '%',
        group: '',
        suppliers: [],
        mark: '',
        summary: '',
        stackable: false,
        parallel: false,
        daily_utc: false,
        attribution: '',
        ...overrides,
    };
}

export function historySeries(overrides: Partial<HistorySeries> = {}): HistorySeries {
    return {id: 'series', provider: 'claude', label: 'Series', points: [], ...overrides};
}

export function history(overrides: Partial<History> = {}): History {
    return {metric: 'usage', start: 0, end: 100, series: [], project_labels: {}, ...overrides};
}

export function statusCell(overrides: Partial<StatusCell> = {}): StatusCell {
    return {
        id: 'claude:session',
        label: 'CC',
        text: 'CC 0%',
        tooltip: 'Claude',
        used: 0,
        provider: 'claude',
        restricted: false,
        limited: false,
        stale: false,
        ...overrides,
    };
}
