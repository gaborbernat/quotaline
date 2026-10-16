//! Maps macOS quota snapshots into the shared notification planner.

use std::sync::Arc;

use quotaline_notifications as notifications;
use quotaline_notifications::Kind;
use quotaline_notifications::Moments;
use quotaline_quota::Provider;
use quotaline_settings::Settings;

use crate::provider_record::ProviderReadingRecord;
use crate::time::{Clock, ClockFormatter};

mod delivery;
pub use delivery::{DeliveryCredentialRecord, DeliveryEventRecord, NotificationDeliveryLedger};

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct NotificationOptionsRecord {
    pub enabled: bool,
    pub thresholds: Vec<u8>,
    pub resets: bool,
    pub sign_in: bool,
    pub expiring_credits: bool,
    pub pace: bool,
    pub workdays: u8,
}

#[uniffi::export]
pub fn notification_thresholds(thresholds: Vec<i32>) -> Vec<u8> {
    quotaline_settings::normalize_thresholds(thresholds)
}

#[uniffi::export]
pub fn notification_set_threshold(thresholds: Vec<i32>, target: i32, enabled: bool) -> Vec<u8> {
    quotaline_settings::set_threshold(thresholds, target, enabled)
}

#[uniffi::export]
pub fn notification_threshold_choices() -> Vec<u8> {
    notifications::THRESHOLD_CHOICES.to_vec()
}

#[uniffi::export]
pub fn notification_default_thresholds() -> Vec<u8> {
    quotaline_settings::DEFAULT_THRESHOLDS.to_vec()
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct NotificationSampleRecord {
    pub window_id: String,
    pub at: f64,
    pub used: f64,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct NotificationRequestRecord {
    pub provider_id: String,
    pub previous: Option<ProviderReadingRecord>,
    pub current: Option<ProviderReadingRecord>,
    pub previous_sign_in_required: bool,
    pub current_sign_in_required: bool,
    pub credential_missing: bool,
    pub options: NotificationOptionsRecord,
    pub samples: Vec<NotificationSampleRecord>,
    pub now: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum NotificationKindRecord {
    Threshold,
    Reset,
    Authentication,
    Credits,
    ExpiringCredit,
    WillRunOut,
    AheadOfPace,
}

impl From<Kind> for NotificationKindRecord {
    fn from(value: Kind) -> Self {
        match value {
            Kind::Threshold => Self::Threshold,
            Kind::Reset => Self::Reset,
            Kind::Authentication => Self::Authentication,
            Kind::Credits => Self::Credits,
            Kind::ExpiringCredit => Self::ExpiringCredit,
            Kind::WillRunOut => Self::WillRunOut,
            Kind::AheadOfPace => Self::AheadOfPace,
        }
    }
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct NotificationEventRecord {
    pub id: String,
    pub kind: NotificationKindRecord,
    pub window_id: Option<String>,
    pub title: String,
    pub body: String,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct NotificationWordingRecord {
    pub provider_name: String,
    pub login_hint: String,
    pub fetched_at: f64,
}

#[uniffi::export]
pub fn notification_summary(options: NotificationOptionsRecord) -> String {
    notifications::summary(&notifications::Selection {
        enabled: options.enabled,
        thresholds: &options.thresholds,
        resets: options.resets,
        sign_in: options.sign_in,
        pace: options.pace,
        expiring_credits: options.expiring_credits,
    })
}

#[uniffi::export]
pub fn plan_notifications(
    request: NotificationRequestRecord,
    wording: NotificationWordingRecord,
    clock: Arc<dyn ClockFormatter>,
) -> Vec<NotificationEventRecord> {
    let settings = Settings {
        notifications: request.options.enabled,
        notify_thresholds: request.options.thresholds,
        notify_resets: request.options.resets,
        notify_sign_in: request.options.sign_in,
        notify_expiring_credits: request.options.expiring_credits,
        notify_pace: request.options.pace,
        workdays: request.options.workdays,
        ..Settings::default()
    };
    let previous = request.previous.map(Provider::from);
    let current = request.current.map(Provider::from);
    let mut events = notifications::auth_transition(
        &request.provider_id,
        request.previous_sign_in_required,
        request.current_sign_in_required,
        request.credential_missing,
        &settings,
    )
    .into_iter()
    .collect::<Vec<_>>();
    if let Some(current) = current {
        let samples = request
            .samples
            .into_iter()
            .map(|sample| (sample.window_id, sample.at as i64, sample.used))
            .collect::<Vec<_>>();
        events.extend(notifications::plan(
            previous.as_ref(),
            &current,
            &settings,
            &samples,
            request.now as i64,
        ));
    }
    let moments = Moments {
        now: request.now as i64,
        fetched_at: wording.fetched_at as i64,
    };
    let clock = Clock(clock);
    events
        .into_iter()
        .map(|event| {
            let copy = notifications::copy(
                &event.facts,
                &wording.provider_name,
                &wording.login_hint,
                moments,
                &clock,
            );
            NotificationEventRecord {
                id: event.id,
                kind: event.kind.into(),
                window_id: event.window_id,
                title: copy.title,
                body: copy.body,
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "../tests/unit/notifications/tests.rs"]
mod tests;
