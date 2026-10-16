//! Exposes macOS persistence policy through UniFFI.

use quotaline_macos_platform::persistence::{
    PersistenceFailure as PlatformFailure, PersistenceWorkload as PlatformWorkload,
    SnapshotWriteLedger as PlatformLedger,
};

use quotaline_status_bar::WidgetContent;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, uniffi::Record)]
pub struct PersistenceWorkload {
    pub cache_loads: u64,
    pub cache_submissions: u64,
    pub cache_writes: u64,
    pub coalesced_cache_submissions: u64,
    pub usage_file_writes: u64,
    pub widget_submissions: u64,
    pub widget_writes: u64,
    pub coalesced_widget_submissions: u64,
    pub widget_reloads: u64,
}

impl From<PlatformWorkload> for PersistenceWorkload {
    fn from(value: PlatformWorkload) -> Self {
        Self {
            cache_loads: value.cache_loads,
            cache_submissions: value.cache_submissions,
            cache_writes: value.cache_writes,
            coalesced_cache_submissions: value.coalesced_cache_submissions,
            usage_file_writes: value.usage_file_writes,
            widget_submissions: value.widget_submissions,
            widget_writes: value.widget_writes,
            coalesced_widget_submissions: value.coalesced_widget_submissions,
            widget_reloads: value.widget_reloads,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum PersistenceFailure {
    CacheWrite { detail: String },
    UsageFileWrite { detail: String },
    WidgetWrite { detail: String },
}

impl From<PlatformFailure> for PersistenceFailure {
    fn from(value: PlatformFailure) -> Self {
        match value {
            PlatformFailure::CacheWrite { detail } => Self::CacheWrite { detail },
            PlatformFailure::UsageFileWrite { detail } => Self::UsageFileWrite { detail },
            PlatformFailure::WidgetWrite { detail } => Self::WidgetWrite { detail },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct WidgetDrainResult {
    pub failures: Vec<PersistenceFailure>,
    pub reloads: u64,
}

#[derive(uniffi::Object)]
pub struct SnapshotWriteLedger(PlatformLedger);

#[uniffi::export]
impl SnapshotWriteLedger {
    #[uniffi::constructor]
    pub fn new(cache_path: Option<String>, usage_path: Option<String>, widget_path: Option<String>) -> Self {
        Self(PlatformLedger::new(cache_path, usage_path, widget_path))
    }

    pub fn workload(&self) -> PersistenceWorkload {
        self.0.workload().into()
    }

    pub fn record_cache_load(&self) {
        self.0.record_cache_load();
    }

    pub fn submit_snapshots(&self, cache: Vec<u8>, usage: Option<Vec<u8>>) {
        self.0.submit_snapshots(cache, usage);
    }

    pub fn has_pending_snapshots(&self) -> bool {
        self.0.has_pending_snapshots()
    }

    pub fn drain_snapshots(&self) -> Vec<PersistenceFailure> {
        self.0.drain_snapshots().into_iter().map(Into::into).collect()
    }

    pub fn submit_widget(&self, data: Vec<u8>, content: WidgetContent) {
        self.0.submit_widget(data, content);
    }

    pub fn has_pending_widget(&self) -> bool {
        self.0.has_pending_widget()
    }

    pub fn drain_widgets(&self) -> WidgetDrainResult {
        let result = self.0.drain_widgets();
        WidgetDrainResult {
            failures: result.failures.into_iter().map(Into::into).collect(),
            reloads: result.reloads,
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/persistence/tests.rs"]
mod tests;
