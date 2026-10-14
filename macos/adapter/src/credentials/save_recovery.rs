//! Exposes shared credential persistence recovery to macOS.

use std::sync::Mutex;

use quotaline_credentials::save_recovery::LoadedProbe;
use quotaline_credentials::save_recovery::Pending;
use quotaline_credentials::save_recovery::ReadProbe;
use quotaline_credentials::save_recovery::Recovery;
use quotaline_credentials::save_recovery::Resolved;
use quotaline_credentials::save_recovery::SaveCheck;
use quotaline_credentials::save_recovery::SaveOutcome;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CredentialRecoverySource {
    pub id: String,
    pub provider: String,
    pub title: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CredentialRecoveryStored {
    pub document: Vec<u8>,
    pub source: CredentialRecoverySource,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CredentialRecoveryPending {
    pub document: Vec<u8>,
    pub replacing: Vec<u8>,
    pub source: CredentialRecoverySource,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CredentialRecoveryResult {
    pub document: Option<Vec<u8>>,
    pub source: Option<CredentialRecoverySource>,
    pub issue: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum CredentialRecoveryReadProbe {
    ReadError,
    Resolved(CredentialRecoveryResult),
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum CredentialRecoveryLoadedProbe {
    Resolved(CredentialRecoveryResult),
    RetrySave(CredentialRecoveryPending),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum CredentialRecoverySave {
    Saved,
    Changed,
    Removed,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum CredentialSaveCheck {
    Write,
    Changed,
    Removed,
}

#[uniffi::export]
pub fn credential_save_check(
    current: Option<Vec<u8>>,
    expected: Vec<u8>,
) -> Result<CredentialSaveCheck, CredentialRecoveryError> {
    let current = current.as_deref().map(parse).transpose()?;
    let expected = parse(&expected)?;
    Ok(
        match quotaline_credentials::save_recovery::check_save(current.as_ref(), &expected) {
            SaveCheck::Write => CredentialSaveCheck::Write,
            SaveCheck::Changed => CredentialSaveCheck::Changed,
            SaveCheck::Removed => CredentialSaveCheck::Removed,
        },
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum CredentialRecoveryError {
    #[error("Invalid credential JSON")]
    InvalidJson,
    #[error("Changed credential is missing")]
    MissingChangedCredential,
}

#[derive(Default, uniffi::Object)]
pub struct CredentialRecoveryLedger(Mutex<Recovery<Value, CredentialRecoverySource>>);

#[uniffi::export]
impl CredentialRecoveryLedger {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pending(&self) -> Option<CredentialRecoveryPending> {
        self.0.lock().unwrap().pending().map(Into::into)
    }

    pub fn probe_loaded(
        &self,
        loaded: Option<CredentialRecoveryStored>,
    ) -> Result<CredentialRecoveryLoadedProbe, CredentialRecoveryError> {
        let loaded = loaded.map(parse_stored).transpose()?;
        Ok(self.0.lock().unwrap().probe_loaded(loaded).into())
    }

    pub fn probe_read_error(&self) -> CredentialRecoveryReadProbe {
        self.0.lock().unwrap().probe_read_error().into()
    }

    pub fn finish(
        &self,
        proposed: CredentialRecoveryPending,
        save: CredentialRecoverySave,
        changed: Option<CredentialRecoveryStored>,
    ) -> Result<CredentialRecoveryResult, CredentialRecoveryError> {
        let proposed = Pending {
            credential: parse(&proposed.document)?,
            replacing: parse(&proposed.replacing)?,
            source: proposed.source,
            detail: proposed.detail,
        };
        let outcome = match save {
            CredentialRecoverySave::Saved => SaveOutcome::Saved,
            CredentialRecoverySave::Changed => {
                let (credential, source) =
                    parse_stored(changed.ok_or(CredentialRecoveryError::MissingChangedCredential)?)?;
                SaveOutcome::Changed(credential, source)
            }
            CredentialRecoverySave::Removed => SaveOutcome::Removed,
            CredentialRecoverySave::Failed => SaveOutcome::Failed,
        };
        Ok(self.0.lock().unwrap().finish(proposed, outcome).into())
    }
}

fn parse(document: &[u8]) -> Result<Value, CredentialRecoveryError> {
    serde_json::from_slice(document).map_err(|_| CredentialRecoveryError::InvalidJson)
}

fn parse_stored(
    stored: CredentialRecoveryStored,
) -> Result<(Value, CredentialRecoverySource), CredentialRecoveryError> {
    Ok((parse(&stored.document)?, stored.source))
}

fn encoded(document: &Value) -> Vec<u8> {
    serde_json::to_vec(document).expect("parsed credential JSON is serializable")
}

impl From<Pending<Value, CredentialRecoverySource>> for CredentialRecoveryPending {
    fn from(pending: Pending<Value, CredentialRecoverySource>) -> Self {
        Self {
            document: encoded(&pending.credential),
            replacing: encoded(&pending.replacing),
            source: pending.source,
            detail: pending.detail,
        }
    }
}

impl From<Resolved<Value, CredentialRecoverySource>> for CredentialRecoveryResult {
    fn from(resolved: Resolved<Value, CredentialRecoverySource>) -> Self {
        Self {
            document: resolved.credential.as_ref().map(encoded),
            source: resolved.source,
            issue: resolved.issue,
        }
    }
}

impl From<ReadProbe<Value, CredentialRecoverySource>> for CredentialRecoveryReadProbe {
    fn from(probe: ReadProbe<Value, CredentialRecoverySource>) -> Self {
        match probe {
            ReadProbe::ReadError => Self::ReadError,
            ReadProbe::Resolved(result) => Self::Resolved(result.into()),
        }
    }
}

impl From<LoadedProbe<Value, CredentialRecoverySource>> for CredentialRecoveryLoadedProbe {
    fn from(probe: LoadedProbe<Value, CredentialRecoverySource>) -> Self {
        match probe {
            LoadedProbe::Resolved(result) => Self::Resolved(result.into()),
            LoadedProbe::Retry(pending) => Self::RetrySave(pending.into()),
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/credentials/save_recovery/tests.rs"]
mod tests;
