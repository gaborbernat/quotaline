//! Exposes shared usage exports and macOS file persistence.

mod command;
pub use command::*;

use std::path::Path;

use std::collections::BTreeMap;

use quotaline_export as export;
use quotaline_quota::Provider;

use crate::provider_record::ProviderReadingRecord;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum UsageExportFailure {
    #[error("usage export write failed: {message}")]
    Io { message: String },
}

fn encode(readings: Vec<ProviderReadingRecord>) -> Vec<u8> {
    let providers = readings
        .into_iter()
        .map(|reading| {
            let provider = Provider::from(reading);
            (provider.id.clone(), provider)
        })
        .collect::<BTreeMap<_, _>>();
    export::UsageExport::from(&providers).json()
}

#[uniffi::export]
pub fn usage_export_json(readings: Vec<ProviderReadingRecord>) -> Vec<u8> {
    encode(readings)
}

#[uniffi::export]
pub fn usage_export_write(path: String, readings: Vec<ProviderReadingRecord>) -> Result<u64, UsageExportFailure> {
    quotaline_storage::write_private_file(Path::new(&path), &encode(readings))
        .map(|count| count as u64)
        .map_err(|error| UsageExportFailure::Io {
            message: error.to_string(),
        })
}

#[cfg(test)]
#[path = "../tests/unit/export/tests.rs"]
mod tests;
