//! Validates and normalizes the settings document both desktops store.

use quotaline_catalog::{IDS, REFRESH_POLICY};
use quotaline_text::text::normalized_label;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use unicode_segmentation::UnicodeSegmentation;

pub mod context;
mod effects;
mod history;
mod thresholds;

pub use effects::SettingsUpdateEffects;
pub use history::{history_session_from_settings, sync_history_settings};
pub use thresholds::{DEFAULT_THRESHOLDS, normalize_thresholds, set_threshold};

pub const DEFAULT_HISTORY_RETENTION_DAYS: u32 = 60;
pub const DEFAULT_ANALYTICS_INTERVAL_MINUTES: u32 = 15;
pub const DEFAULT_STATUS_TEMPLATE: &str = "{provider} {window}\\n{pct}";

pub fn normalized_percent_decimals(value: i64) -> u8 {
    value.clamp(0, 2) as u8
}

pub fn normalized_pace_workdays(value: i64) -> u8 {
    value.clamp(
        i64::from(quotaline_quota::pace::MIN_WORKDAYS),
        i64::from(quotaline_quota::pace::MAX_WORKDAYS),
    ) as u8
}

pub fn normalized_polling_seconds(provider: &str, requested: i64) -> Option<u32> {
    let definition = quotaline_catalog::definition(provider)?;
    let minimum = definition.minimum_minutes * 60;
    Some((requested.max(0) as u64).clamp(
        u64::from(minimum),
        u64::from(quotaline_catalog::MAXIMUM_REFRESH_MINUTES * 60),
    ) as u32)
}

pub fn step_provider_interval(
    intervals: &BTreeMap<String, u32>,
    provider: &str,
    delta: i32,
) -> Option<BTreeMap<String, u32>> {
    let definition = quotaline_catalog::definition(provider)?;
    let current = intervals.get(provider).copied().unwrap_or(definition.default_minutes);
    let minutes = (i64::from(current) + i64::from(delta)).clamp(
        i64::from(definition.minimum_minutes),
        i64::from(quotaline_catalog::MAXIMUM_REFRESH_MINUTES),
    );
    let mut intervals = intervals.clone();
    intervals.insert(provider.into(), minutes as u32);
    Some(intervals)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderPreferences {
    pub added: Vec<String>,
    pub enabled: Vec<String>,
    pub configured: Vec<String>,
    pub order: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderOverride {
    pub enabled: Vec<String>,
    pub configured: Vec<String>,
}

pub fn set_provider_override(
    enabled: &[String],
    configured: &[String],
    provider: &str,
    active: bool,
) -> Option<ProviderOverride> {
    if !IDS.contains(&provider) {
        return None;
    }
    let mut enabled = enabled.iter().cloned().collect::<BTreeSet<_>>();
    let mut configured = configured.iter().cloned().collect::<BTreeSet<_>>();
    configured.insert(provider.into());
    if active {
        enabled.insert(provider.into());
    } else {
        enabled.remove(provider);
    }
    Some(ProviderOverride {
        enabled: enabled.into_iter().collect(),
        configured: configured.into_iter().collect(),
    })
}

pub fn migrate_provider_preferences(
    known: Option<&[String]>,
    stored_enabled: Option<&[String]>,
    stored_configured: Option<&[String]>,
    stored_order: Option<&[String]>,
) -> ProviderPreferences {
    let current = IDS.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>();
    let added = known.map_or_else(Vec::new, |known| {
        current.iter().filter(|id| !known.contains(id)).cloned().collect()
    });
    let mut enabled = stored_enabled.unwrap_or(&current).to_vec();
    for id in &added {
        if !enabled.contains(id) {
            enabled.push(id.clone());
        }
    }
    let configured = stored_configured.map_or_else(
        || stored_enabled.map_or_else(Vec::new, |_| current.clone()),
        ToOwned::to_owned,
    );
    let mut order = stored_order.unwrap_or(&current).to_vec();
    for id in &added {
        if !order.contains(id) {
            order.push(id.clone());
        }
    }
    ProviderPreferences {
        added,
        enabled,
        configured,
        order,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub providers: BTreeMap<String, bool>,
    pub configured_providers: BTreeSet<String>,
    pub provider_order: Vec<String>,
    pub model_order: Vec<String>,
    pub hide_unused_models: bool,
    pub show_all_providers: bool,
    pub allow_token_refresh: bool,
    pub interval_minutes: BTreeMap<String, u32>,
    pub format: String,
    pub order: String,
    pub show_as: String,
    pub decimals: u8,
    pub hide_zero: bool,
    pub fit_to_space: bool,
    pub hide_identity: bool,
    pub short_labels: BTreeMap<String, String>,
    pub selected_windows: Vec<String>,
    #[serde(default)]
    pub has_custom_selection: Option<bool>,
    pub template: String,
    pub retention_days: u32,
    pub analytics_interval_minutes: u32,
    pub write_usage_json: bool,
    pub widget_size: String,
    pub last_tab: String,
    pub history_days: u32,
    pub history_metric: String,
    pub history_rollup: String,
    pub history_utc: bool,
    pub history_stacked: bool,
    pub history_hidden_keys: Vec<String>,
    pub notifications: bool,
    pub notify_thresholds: Vec<u8>,
    pub notify_resets: bool,
    pub notify_sign_in: bool,
    pub notify_expiring_credits: bool,
    pub notify_pace: bool,
    pub notify_sound: bool,
    pub detailed_logging: bool,
    pub first_run_card_dismissed: bool,
    pub workdays: u8,
    pub demo: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demo_choice: Option<bool>,
    /// Providers this document has seen. A provider added in a later release is enabled once; a document written
    /// before this field existed treats every current provider as seen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub known_providers: Option<Vec<String>>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            providers: IDS.iter().map(|id| ((*id).into(), true)).collect(),
            configured_providers: BTreeSet::new(),
            provider_order: IDS.iter().map(|id| (*id).into()).collect(),
            model_order: Vec::new(),
            hide_unused_models: false,
            show_all_providers: false,
            allow_token_refresh: false,
            interval_minutes: REFRESH_POLICY
                .iter()
                .map(|(id, _, default)| ((*id).into(), *default))
                .collect(),
            format: "stacked".into(),
            order: "stable".into(),
            show_as: "used".into(),
            decimals: 0,
            hide_zero: true,
            fit_to_space: true,
            hide_identity: false,
            short_labels: BTreeMap::new(),
            selected_windows: Vec::new(),
            has_custom_selection: Some(false),
            template: DEFAULT_STATUS_TEMPLATE.into(),
            retention_days: DEFAULT_HISTORY_RETENTION_DAYS,
            analytics_interval_minutes: DEFAULT_ANALYTICS_INTERVAL_MINUTES,
            write_usage_json: false,
            widget_size: "small".into(),
            last_tab: "Usage".into(),
            history_days: 1,
            history_metric: "usage".into(),
            history_rollup: "minute".into(),
            history_utc: false,
            history_stacked: false,
            history_hidden_keys: Vec::new(),
            notifications: true,
            notify_thresholds: crate::DEFAULT_THRESHOLDS.to_vec(),
            notify_resets: true,
            notify_sign_in: true,
            notify_expiring_credits: true,
            notify_pace: true,
            notify_sound: true,
            detailed_logging: false,
            first_run_card_dismissed: false,
            workdays: quotaline_quota::pace::MAX_WORKDAYS,
            demo: false,
            demo_choice: None,
            known_providers: None,
        }
    }
}

impl Settings {
    pub fn request_demo(&mut self, requested: bool) {
        self.demo = self.demo_choice.unwrap_or(requested);
    }

    pub fn stored(&self) -> Self {
        let mut settings = self.clone();
        settings.demo = self.demo_choice.unwrap_or(false);
        settings
    }

    pub fn normalize(&mut self) {
        let legacy_empty = self.selected_windows.as_slice() == ["-"];
        if legacy_empty {
            self.selected_windows.clear();
        }
        self.has_custom_selection = Some(
            self.has_custom_selection
                .unwrap_or(legacy_empty || !self.selected_windows.is_empty()),
        );
        self.providers.retain(|id, _| IDS.contains(&id.as_str()));
        let known = self.known_providers.replace(IDS.map(String::from).to_vec());
        for id in IDS {
            let added = known.as_ref().is_some_and(|known| !known.iter().any(|seen| seen == id));
            if added && !self.provider_order.iter().any(|ordered| ordered == id) {
                self.provider_order.push(id.into());
            }
            let enabled = self.providers.entry(id.into()).or_insert(false);
            *enabled |= added;
        }
        self.configured_providers.retain(|id| IDS.contains(&id.as_str()));
        self.interval_minutes.retain(|id, _| IDS.contains(&id.as_str()));
        for (id, minimum, default) in REFRESH_POLICY {
            let minutes = self.interval_minutes.entry(id.into()).or_insert(default);
            *minutes = (*minutes).clamp(minimum, quotaline_catalog::MAXIMUM_REFRESH_MINUTES);
        }
    }

    pub fn set(&mut self, key: &str, value: serde_json::Value) -> Result<(), String> {
        if key == "demo_choice" {
            return Err("Demo preference is managed by the demo switch".into());
        }
        if let Some(id) = key.strip_prefix("provider:") {
            if !IDS.contains(&id) {
                return Err("Unknown provider".into());
            }
            let enabled = value.as_bool().ok_or("Invalid provider state")?;
            self.providers.insert(id.into(), enabled);
            self.configured_providers.insert(id.into());
            return Ok(());
        }
        let mut document = serde_json::to_value(&*self).expect("settings contain only JSON values");
        let keys = document.as_object_mut().expect("settings serialize as an object");
        if !keys.contains_key(key) {
            return Err("Unknown setting".into());
        }
        if key == "has_custom_selection" && !value.is_boolean() {
            return Err("Invalid selection mode".into());
        }
        keys.insert(key.into(), value);
        let mut changed: Settings = serde_json::from_value(document).map_err(|error| error.to_string())?;
        if key == "demo" {
            changed.demo_choice = Some(changed.demo);
        }
        if key == "providers" {
            for (id, enabled) in &changed.providers {
                if self.providers.get(id) != Some(enabled) {
                    changed.configured_providers.insert(id.clone());
                }
            }
        }
        if key == "selected_windows" {
            changed.has_custom_selection = Some(true);
        }
        self.replace(changed)
    }

    /// Validates and normalizes a whole edited document, as a frontend that holds every field sends it.
    pub fn replace(&mut self, mut changed: Settings) -> Result<(), String> {
        if !["stacked", "inline", "mini-bars", "countdown", "exhausted", "custom"].contains(&changed.format.as_str()) {
            return Err("Invalid format".into());
        }
        if !["stable", "usage"].contains(&changed.order.as_str()) {
            return Err("Invalid order".into());
        }
        let providers: std::collections::BTreeSet<_> = changed.provider_order.iter().collect();
        if providers.len() != changed.provider_order.len()
            || changed.provider_order.iter().any(|id| !IDS.contains(&id.as_str()))
        {
            return Err("Invalid provider order".into());
        }
        let models: std::collections::BTreeSet<_> = changed.model_order.iter().collect();
        if models.len() != changed.model_order.len() {
            return Err("Invalid model order".into());
        }
        if !["used", "remaining"].contains(&changed.show_as.as_str()) {
            return Err("Invalid display mode".into());
        }
        if !["small", "medium", "large"].contains(&changed.widget_size.as_str()) {
            return Err("Invalid widget size".into());
        }
        if !["Usage", "History", "Settings"].contains(&changed.last_tab.as_str()) {
            return Err("Invalid tab".into());
        }
        if ![1, 7, 30, 60].contains(&changed.history_days) {
            return Err("Invalid history period".into());
        }
        if changed.history_metric != "usage"
            && !quotaline_analytics::METRICS
                .iter()
                .any(|(metric, _, _, _)| *metric == changed.history_metric)
        {
            return Err("Invalid history metric".into());
        }
        if !["minute", "hour", "day"].contains(&changed.history_rollup.as_str()) {
            return Err("Invalid history rollup".into());
        }
        changed.history_hidden_keys.sort();
        changed.history_hidden_keys.dedup();
        changed.decimals = normalized_percent_decimals(i64::from(changed.decimals));
        changed.retention_days = normalize_history_retention_days(changed.retention_days);
        changed.analytics_interval_minutes = normalize_analytics_interval_minutes(changed.analytics_interval_minutes);
        changed.workdays = normalized_pace_workdays(i64::from(changed.workdays));
        changed.notify_thresholds = crate::normalize_thresholds(changed.notify_thresholds);
        changed.normalize();
        if changed
            .short_labels
            .iter()
            .any(|(key, label)| self.short_labels.get(key) != Some(label) && label.graphemes(true).count() > 6)
        {
            return Err("Short labels are limited to six characters".into());
        }
        if changed.short_labels.iter().any(|(key, label)| {
            self.short_labels.get(key) != Some(label)
                && changed
                    .short_labels
                    .iter()
                    .any(|(other, value)| other != key && normalized_label(value) == normalized_label(label))
        }) {
            return Err("Short labels must be unique".into());
        }
        *self = changed;
        Ok(())
    }
}

pub fn normalize_history_retention_days(days: u32) -> u32 {
    days.clamp(7, 365)
}

pub fn normalize_analytics_interval_minutes(minutes: u32) -> u32 {
    minutes.clamp(5, 120).div_ceil(5) * 5
}

#[cfg(test)]
#[path = "../tests/unit/controls/tests.rs"]
mod controls_tests;
#[cfg(test)]
#[path = "../tests/unit/settings/tests.rs"]
mod settings_tests;
#[cfg(test)]
#[path = "../tests/unit/preferences/tests.rs"]
mod tests;
