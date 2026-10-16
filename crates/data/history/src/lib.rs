//! Owns SQLite history storage and export behavior shared by desktop frontends.

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

pub mod chart;
pub mod claude;

mod account;
mod store;

pub use store::{
    AnalyticsPoint, CoverageScope, DEFAULT_RETENTION_DAYS, DEMO_DAYS, HistoryError, PruneResult, Result,
    SCHEMA_VERSION, Sample, SampleQuery, SampleWrite, SnapshotWrite, Stats, Store, WindowSummary,
};
