//! Exposes shared notification delivery state to Swift.

use std::{collections::BTreeSet, sync::Mutex};

use quotaline_notifications::CredentialKey;
use quotaline_notifications::DeliveryEvent;
use quotaline_notifications::DeliveryLedger;
use quotaline_notifications::Kind;

use super::NotificationKindRecord;

#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum DeliveryCredentialRecord {
    Missing { reason: String },
    Expired { at: f64 },
    Valid { expires_at: Option<f64> },
}

impl From<DeliveryCredentialRecord> for CredentialKey {
    fn from(value: DeliveryCredentialRecord) -> Self {
        match value {
            DeliveryCredentialRecord::Missing { reason } => Self::Missing(reason),
            DeliveryCredentialRecord::Expired { at } => Self::Expired(at),
            DeliveryCredentialRecord::Valid { expires_at } => Self::Valid(expires_at),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct DeliveryEventRecord {
    pub id: String,
    pub kind: NotificationKindRecord,
    pub window_id: Option<String>,
}

impl From<NotificationKindRecord> for Kind {
    fn from(value: NotificationKindRecord) -> Self {
        match value {
            NotificationKindRecord::Threshold => Self::Threshold,
            NotificationKindRecord::Reset => Self::Reset,
            NotificationKindRecord::Authentication => Self::Authentication,
            NotificationKindRecord::Credits => Self::Credits,
            NotificationKindRecord::ExpiringCredit => Self::ExpiringCredit,
            NotificationKindRecord::WillRunOut => Self::WillRunOut,
            NotificationKindRecord::AheadOfPace => Self::AheadOfPace,
        }
    }
}

#[derive(Default, uniffi::Object)]
pub struct NotificationDeliveryLedger(Mutex<DeliveryLedger>);

#[uniffi::export]
impl NotificationDeliveryLedger {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn filter(
        &self,
        provider: String,
        credential: Option<DeliveryCredentialRecord>,
        active_windows: Option<Vec<String>>,
        events: Vec<DeliveryEventRecord>,
    ) -> Vec<bool> {
        let credential = credential.map(Into::into);
        let active_windows = active_windows.map(|windows| windows.into_iter().collect::<BTreeSet<_>>());
        let events = events
            .into_iter()
            .map(|event| DeliveryEvent {
                id: event.id,
                kind: event.kind.into(),
                window_id: event.window_id,
            })
            .collect::<Vec<_>>();
        self.0
            .lock()
            .unwrap()
            .filter(&provider, credential.as_ref(), active_windows.as_ref(), &events)
    }

    pub fn clear_pace(&self) {
        self.0.lock().unwrap().clear_pace();
    }

    pub fn forget_pace(&self, provider: String) {
        self.0.lock().unwrap().forget_pace(&provider);
    }

    pub fn forget_sign_in(&self, provider: String) {
        self.0.lock().unwrap().forget_sign_in(&provider);
    }

    pub fn retain_providers(&self, providers: Vec<String>) {
        self.0
            .lock()
            .unwrap()
            .retain_providers(&providers.into_iter().collect::<BTreeSet<_>>());
    }
}

#[cfg(test)]
#[path = "../../tests/unit/notifications/delivery/tests.rs"]
mod tests;
