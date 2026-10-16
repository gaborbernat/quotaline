//! Reads Claude transcripts without exposing file state to a desktop frontend.

mod reader;
pub use reader::MAX_INDEXED_FILES;
pub use reader::{Config, Reader, Snapshot, Workload};
