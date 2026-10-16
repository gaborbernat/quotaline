export type ResetPrecision = 'instant' | 'day';

export type PaceStatus = 'unknown' | 'on_track' | 'ahead' | 'behind' | 'exhausted';

export interface PaceEstimate {
    status: PaceStatus;
    expected_percent: number | null;
    ratio: number | null;
    projected_exhaustion: number | null;
}

export interface Window {
    id: string;
    label: string;
    used: number;
    active: boolean;
    reset: string | null;
    reset_precision: ResetPrecision;
    group: string;
    scope?: string;
    severity?: string;
    detail: string;
    help?: string;
    duration_seconds: number | null;
    // GetState estimates pace for every window before serializing, so it is never omitted there.
    pace: PaceEstimate;
}

export type CredentialHealth = 'unchecked' | 'missing' | 'valid' | 'expired' | 'unreadable';

export interface AccountSelection {
    count: number;
    label: string;
}

export interface ProviderLocalUsage {
    window_tokens: number;
    window_cost: number;
    cost_per_hour: number;
    today_tokens: number;
    today_cost: number;
    today_messages: number;
}

export interface Detail {
    id: string;
    title: string;
    value: string;
    explanation: string;
}

export interface CreditAllowance {
    used: number | null;
    limit: number | null;
    remaining: number | null;
    source: string | null;
}

export interface UsageCredits {
    unlimited: boolean;
    has_credits: boolean | null;
    overage_limit_reached: boolean;
    local_messages: [number, number] | null;
    cloud_messages: [number, number] | null;
}

export interface ResetCreditDetail {
    id: string;
    title: string;
    status: string | null;
    credit_type: string | null;
    description: string | null;
    granted_at: string | null;
    expires_at: string | null;
    explanation: string;
}

export interface ResetCredits {
    available: number;
    applicable: number;
    total_earned: number | null;
    purchase_eligible: boolean;
    details: ResetCreditDetail[];
}

export interface CloudCredits {
    limit: number;
    remaining: number;
    expires_at: string | null;
    locked_reason: string | null;
}

export interface MoneyAmount {
    amount_minor: number;
    currency: string;
    exponent: number;
}

export interface SpendControl {
    enabled: boolean | null;
    can_toggle: boolean;
    disabled_reason: string | null;
    disabled_reason_code?: string;
    used: number | null;
    limit: number | null;
    used_money?: MoneyAmount;
    limit_money?: MoneyAmount;
    percent: number | null;
    currency: string | null;
    reset: string | null;
    limit_reached: boolean;
    balance: number | null;
    balance_money?: MoneyAmount;
    auto_reload: boolean | null;
    can_purchase_credits: boolean;
    credit_allowance: CreditAllowance | null;
}

export interface CreditExpiry {
    id: string;
    at: number;
}

export type NoticeKind = 'promotion' | 'limitReached' | 'spendControl' | 'info';

export interface Notice {
    kind: NoticeKind;
    text: string;
    windowId?: string;
    resetsAt?: string;
}

export interface Provider {
    id: string;
    name: string;
    status: string;
    credential_usable: boolean;
    credential_health: CredentialHealth;
    recovery_status: string | null;
    plan: string;
    tier?: string;
    identity: string;
    organization: string;
    subscription_active_until?: string;
    source: string;
    account_selection?: AccountSelection;
    windows: Window[];
    local_usage?: ProviderLocalUsage;
    spend: SpendControl | null;
    details: Detail[];
    credit_balance: number | null;
    usage_credits: UsageCredits | null;
    reset_credits: ResetCredits | null;
    cloud_credits: CloudCredits | null;
    reset_credit_expiries: CreditExpiry[];
    notices: Notice[];
    warnings: string[];
    notice: string;
    updated: number | null;
    retry_after: number;
}

export type ProviderAction = 'refresh' | 'retry' | 'sign-in' | 'setup';

export type CreditMetricKind =
    | 'credits'
    | 'overage'
    | 'local_messages'
    | 'cloud_messages'
    | 'limit_resets'
    | 'next_expiry'
    | 'reset_purchase'
    | 'resets_earned';

export type CreditMetric = {
    supporting: boolean;
} & ({kind: 'next_expiry'; epoch: number} | {kind: Exclude<CreditMetricKind, 'next_expiry'>; epoch: null});

export type SpendAmount = 'credits' | 'money' | 'unavailable' | 'unreported_limit';

export type SpendMetricKind = 'remaining' | 'allowance_source' | 'balance' | 'resets' | 'auto_reload' | 'purchase';

export type SpendProjection = {
    percent: number | null;
    metrics: {kind: SpendMetricKind}[];
} & ({used: null; limit: null} | {used: SpendAmount; limit: SpendAmount});

export type CloudCreditStatus = 'active' | 'locked' | 'expired';

export interface CloudCreditProjection {
    status: CloudCreditStatus;
    expiry_epoch: number | null;
    used_percent: number;
}

export interface ResetGroupEdge {
    with_previous: boolean;
    with_next: boolean;
}

export type ProviderView<WindowType extends Window = Window> = Omit<Provider, 'windows' | 'spend' | 'cloud_credits'> & {
    windows: WindowType[];
    action: ProviderAction;
    discovered: boolean;
    show_in_settings: boolean;
    show_in_usage: boolean;
    show_organization: boolean;
    credit_metrics: CreditMetric[];
    reset_group_edges: ResetGroupEdge[];
} & ({spend: null; spend_projection: null} | {spend: SpendControl; spend_projection: SpendProjection}) &
    (
        | {cloud_credits: null; cloud_projection: null}
        | {cloud_credits: CloudCredits; cloud_projection: CloudCreditProjection}
    );

export interface Settings {
    providers: Record<string, boolean>;
    configured_providers: string[];
    provider_order: string[];
    model_order: string[];
    hide_unused_models: boolean;
    show_all_providers: boolean;
    allow_token_refresh: boolean;
    interval_minutes: Record<string, number>;
    format: string;
    order: string;
    show_as: string;
    decimals: number;
    hide_zero: boolean;
    fit_to_space: boolean;
    hide_identity: boolean;
    short_labels: Record<string, string>;
    selected_windows: string[];
    has_custom_selection: boolean | null;
    template: string;
    retention_days: number;
    analytics_interval_minutes: number;
    write_usage_json: boolean;
    widget_size: string;
    last_tab: string;
    history_days: number;
    history_metric: string;
    history_rollup: string;
    history_utc: boolean;
    history_stacked: boolean;
    history_hidden_keys: string[];
    notifications: boolean;
    notify_thresholds: number[];
    notify_resets: boolean;
    notify_sign_in: boolean;
    notify_expiring_credits: boolean;
    notify_pace: boolean;
    notify_sound: boolean;
    detailed_logging: boolean;
    first_run_card_dismissed: boolean;
    workdays: number;
    demo: boolean;
    demo_choice?: boolean;
    known_providers?: string[];
}

export interface ProviderDefinition {
    id: string;
    name: string;
    code: string;
    login: string;
    minimum_minutes: number;
    default_minutes: number;
    supports_analytics: boolean;
}

export interface Visibility {
    analytics: boolean;
    pace: boolean;
    tokenRefresh: boolean;
    expiringCredits: boolean;
}

export interface ModelRow {
    key: string;
    position: number;
    default_label: string;
    label: string;
    selected: boolean;
    can_toggle: boolean;
    overridden: boolean;
    last_used_at: number | null;
}

export interface ModelGroup {
    provider: string;
    rows: ModelRow[];
    selected_count: number;
    total_count: number;
    can_toggle_all: boolean;
}

export interface StatusRestriction {
    label: string;
    reset_at: number | null;
}

export interface StatusEntry {
    provider: string;
    provider_code: string;
    provider_name: string;
    window_id: string;
    window_label: string;
    window_scope: string | null;
    short_label: string;
    used: number;
    reset_at: number | null;
    active: boolean;
    restricted: boolean;
    restrictions: StatusRestriction[];
}

export interface StatusGroup {
    provider: string;
    indices: number[];
    used: number;
    restricted: boolean;
}

export type TemplateToken = 'Newline' | {Text: string; Placeholder?: never} | {Placeholder: string; Text?: never};

export interface MetricDefinition {
    id: string;
    title: string;
    unit: string;
    display_unit: string;
    group: string;
    suppliers: string[];
    mark: string;
    summary: string;
    stackable: boolean;
    parallel: boolean;
    daily_utc: boolean;
    attribution: string;
}

export interface LocalUsage {
    window_tokens: number;
    window_cost: number;
    cost_per_hour: number;
    today_tokens: number;
    today_cost: number;
    today_messages: number;
}

export type LocalMetricKind =
    | 'window_tokens'
    | 'window_cost'
    | 'cost_per_hour'
    | 'today_tokens'
    | 'today_messages'
    | 'today_cost';

export type LocalMetric =
    | {kind: 'window_cost' | 'cost_per_hour' | 'today_cost'; money_fraction_digits: number}
    | {kind: 'window_tokens' | 'today_tokens' | 'today_messages'; money_fraction_digits: null};

export interface ServiceState {
    version: number;
    app_version: string;
    provider_definitions: ProviderDefinition[];
    notification_threshold_choices: number[];
    notification_summary: string;
    maximum_refresh_minutes: number;
    providers: Record<string, ProviderView>;
    settings: Settings;
    settings_context: {visibility: Visibility};
    ordering: {providers: string[]; models: string[]};
    model_groups: ModelGroup[];
    selected_model_count: number;
    first_run_pending: boolean;
    status_entries: StatusEntry[];
    status_provider_indices: number[];
    status_mini_groups: StatusGroup[];
    template_tokens: TemplateToken[];
    snapshot_attention: boolean;
    default_short_labels: Record<string, string>;
    resolved_short_labels: Record<string, string>;
    short_label_limit: number;
    default_selected_windows: string[];
    selected_window_keys: string[];
    history_path: string;
    metric_definitions: MetricDefinition[];
    metric_groups: [string, string[]][];
    claude_local_usage: LocalUsage | null;
    local_metrics: LocalMetric[] | null;
    last_refresh: number | null;
    is_refreshing: boolean;
    refreshing_providers: string[];
    now: number;
}

export interface PaceLabels {
    full: string;
    compact: string;
    brief: string;
}

export interface QuotaPresentation {
    detail: string;
    help: string;
    pace_labels: PaceLabels;
}

export interface StatusCell {
    id: string;
    label: string;
    text: string;
    tooltip: string;
    used: number;
    provider: string;
    restricted: boolean;
    limited: boolean;
    stale: boolean;
}

export interface SnapshotRow {
    id: string;
    provider: string;
    label: string;
    used: number;
    reset: string | null;
    updated: number | null;
    stale: boolean;
}

export interface MiniBarGroup {
    provider: string;
    bars: StatusCell[];
    used: number;
    restricted: boolean;
}

export type StatusTier = 'configured' | 'stacked' | 'providers' | 'icon';

export interface StatusVariant {
    tier: StatusTier;
    cells: StatusCell[];
}

export type StatusIconTone = 'attention' | 'offline' | 'normal';

export interface TimePresentation {
    header_updated: string;
    refresh_age: string;
    header_activity: string;
    provider_statuses: Record<string, string>;
    provider_help: Record<string, string>;
    next_update_ms: number | null;
}

export type PanelWindow = Window & QuotaPresentation;

export type PanelProvider = ProviderView<PanelWindow>;

// The Shell decorates the parsed GetState payload in place, so these fields exist only after `_apply`.
export interface PanelState extends ServiceState {
    providers: Record<string, PanelProvider>;
    cells: StatusCell[];
    snapshot_rows: SnapshotRow[];
    status_tooltip: string;
    status_icon_tone: StatusIconTone;
    empty_usage_title: string;
    mini_bar_groups: MiniBarGroup[];
    status_variants: StatusVariant[];
    time_presentation: TimePresentation;
}

export type HistoryAction =
    | {kind: 'load'}
    | {kind: 'now'}
    | {kind: 'period'; days: number}
    | {kind: 'rollup'; rollup: string}
    | {kind: 'metric'; metric: string}
    | {kind: 'utc'; utc: boolean}
    | {kind: 'custom'; from: string; to: string}
    | {kind: 'page'; forward: boolean};

export type HistoryRollup = 'minute' | 'hour' | 'day';

export type HistoryView = {
    days: number;
    follow_now: boolean;
    start: number;
    end: number;
    metric: string;
    rollup: HistoryRollup;
    utc: boolean;
} & ({custom_from: null; custom_to: null} | {custom_from: string; custom_to: string});

// When, the value, the window's reset, and whether it only carries a reading to the period edge.
export type HistoryPoint = [number, number, number | null, boolean];

export interface HistorySeries {
    id: string;
    provider: string;
    label: string;
    points: HistoryPoint[];
}

export interface History {
    metric: string;
    start: number;
    end: number;
    series: HistorySeries[];
    canonicalTotal?: number;
    project_labels: Record<string, string>;
}

export type HistoryViewResponse =
    | {view: HistoryView; history: History; error: null}
    | {view: HistoryView; history: null; error: string};

export type VisibilityKind = 'toggle' | 'isolate' | 'isolate_undo';

export interface VisibilityAction {
    kind: VisibilityKind;
    metric: string;
    id: string;
    series: string[];
}

export interface VisibilityResponse {
    hidden: string[];
    shown: string[];
}

export interface SpendModel {
    provider: string;
    model: string;
    cost: number;
}

export interface SpendSummary {
    today: number;
    yesterday: number;
    last_window: number;
    models: SpendModel[];
    providers: string[];
}

export interface SpendReport {
    total: SpendSummary;
    by_provider: Record<string, SpendSummary>;
}

export interface ReviewCounts {
    today: number;
    week: number;
}

export interface ShortLabelPreview {
    override_value: string | null;
    conflict_key: string | null;
}

export interface LogEntry {
    timestamp: number;
    level: string;
    category: string;
    message: string;
    line: string;
}
