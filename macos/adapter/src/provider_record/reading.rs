//! Pairs a provider record with when and how it was read, the unit Swift holds and the snapshot cache stores.

use serde::{Deserialize, Serialize};

use super::ProviderRecord;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "camelCase")]
pub enum ReadingSource {
    Network,
    LocalLog,
    Cache,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct ProviderReadingRecord {
    pub provider: ProviderRecord,
    pub fetched_at: f64,
    pub source: ReadingSource,
}
