//! Keeps the last provider readings so macOS can show them at launch.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::provider_record::{ProviderReadingRecord, ReadingSource};
use crate::storage::SnapshotCacheError;

const VERSION: u32 = 2;

#[derive(Serialize, Deserialize)]
struct Document {
    version: u32,
    readings: Vec<ProviderReadingRecord>,
}

#[uniffi::export]
pub fn read_reading_cache(path: String) -> Result<Vec<ProviderReadingRecord>, SnapshotCacheError> {
    let data = quotaline_storage::read_optional_file(Path::new(&path)).map_err(|error| SnapshotCacheError::Io {
        message: error.to_string(),
    })?;
    let Some(data) = data else { return Ok(Vec::new()) };
    let readings = serde_json::from_slice::<Document>(&data)
        .map(|document| document.readings)
        .ok()
        .ok_or(SnapshotCacheError::Malformed)?;
    Ok(readings
        .into_iter()
        .map(|reading| ProviderReadingRecord {
            source: ReadingSource::Cache,
            ..reading
        })
        .collect())
}

#[uniffi::export]
pub fn encode_reading_cache(readings: Vec<ProviderReadingRecord>) -> Vec<u8> {
    serde_json::to_vec(&Document {
        version: VERSION,
        readings,
    })
    .expect("provider readings serialize")
}

#[cfg(test)]
#[path = "../tests/unit/reading_cache/tests.rs"]
mod tests;
