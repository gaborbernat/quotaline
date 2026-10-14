//! Exposes shared private file access to macOS.

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum StorageFileError {
    #[error("File error: {message}")]
    Io { message: String },
    #[error("Credential document is not JSON")]
    Malformed,
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum SnapshotCacheError {
    #[error("Snapshot cache file error: {message}")]
    Io { message: String },
    #[error("Snapshot cache is not valid JSON")]
    Malformed,
}

#[uniffi::export]
pub fn write_snapshot_cache(path: String, data: Vec<u8>) -> Result<u64, SnapshotCacheError> {
    quotaline_storage::write_private_file(std::path::Path::new(&path), &data)
        .map(|count| count as u64)
        .map_err(|error| SnapshotCacheError::Io {
            message: error.to_string(),
        })
}

#[uniffi::export]
pub fn write_secret_file(path: String, data: Vec<u8>) -> Result<(), StorageFileError> {
    quotaline_storage::write_secret_file(std::path::Path::new(&path), &data).map_err(|error| StorageFileError::Io {
        message: error.to_string(),
    })
}

#[uniffi::export]
pub fn read_optional_file(path: String) -> Result<Option<Vec<u8>>, StorageFileError> {
    quotaline_storage::read_optional_file(std::path::Path::new(&path)).map_err(|error| StorageFileError::Io {
        message: error.to_string(),
    })
}

#[uniffi::export]
pub fn write_private_file(path: String, data: Vec<u8>) -> Result<u64, StorageFileError> {
    quotaline_storage::write_private_file(std::path::Path::new(&path), &data)
        .map(|count| count as u64)
        .map_err(|error| StorageFileError::Io {
            message: error.to_string(),
        })
}

#[cfg(test)]
#[path = "../tests/unit/storage/tests.rs"]
mod tests;
