//! Adapts the shared SQLite history store to Swift value types.

use std::{path::Path, sync::Arc};

use quotaline_history as history;
use quotaline_history::chart::{AnalyticsQuery, HistoryChartSession};

use quotaline_analytics::{AnalyticsChart, WindowChart, WindowChartRequest};

use super::StoredAnalyticsChartQuery;
use crate::provider_record::{ProviderReadingRecord, ReadingSource};

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum HistoryStoreError {
    #[error("Unsupported history schema version {version}")]
    Schema { version: i32 },
    #[error("History database error: {message}")]
    Database { message: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Record)]
pub struct HistoryStoreMetadata {
    pub schema_version: i32,
    pub default_retention_days: u32,
    pub demo_days: u32,
}

#[uniffi::export]
pub fn history_store_metadata() -> HistoryStoreMetadata {
    HistoryStoreMetadata {
        schema_version: history::SCHEMA_VERSION,
        default_retention_days: history::DEFAULT_RETENTION_DAYS,
        demo_days: history::DEMO_DAYS,
    }
}

impl From<history::HistoryError> for HistoryStoreError {
    fn from(error: history::HistoryError) -> Self {
        match error {
            history::HistoryError::Schema(version) => Self::Schema { version },
            other => Self::Database {
                message: other.to_string(),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct StoredSampleQuery {
    pub keys: Option<Vec<String>>,
    pub start: f64,
    pub end: f64,
    pub rollup: Option<f64>,
    pub time_zone: String,
    pub fallback_offset: i32,
    pub includes_end: bool,
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct StoredHistoryUsageDate {
    pub key: String,
    pub at: f64,
}

#[derive(uniffi::Object)]
pub struct RustHistoryStore {
    store: Arc<history::Store>,
    charts: HistoryChartSession,
}

#[uniffi::export]
impl RustHistoryStore {
    #[uniffi::constructor]
    pub fn new(path: Option<String>, retention_days: u32) -> Result<Arc<Self>, HistoryStoreError> {
        let store = Arc::new(history::Store::open(path.as_deref().map(Path::new), retention_days)?);
        Ok(Arc::new(Self {
            charts: HistoryChartSession::new(store.clone(), 24_000),
            store,
        }))
    }

    pub fn clear_chart_cache(&self) {
        self.charts.clear();
    }

    pub fn cached_window_chart(&self, request: WindowChartRequest) -> Result<WindowChart, HistoryStoreError> {
        self.charts.window(request).map_err(Into::into)
    }

    pub fn cached_analytics_chart(
        &self,
        request: StoredAnalyticsChartQuery,
    ) -> Result<Option<AnalyticsChart>, HistoryStoreError> {
        let StoredAnalyticsChartQuery {
            metric,
            providers,
            start_day,
            before_day,
            hidden,
            stacked,
        } = request;
        self.charts
            .analytics(
                AnalyticsQuery {
                    metric,
                    providers,
                    start_day,
                    before_day,
                },
                &hidden
                    .into_iter()
                    .filter_map(|id| {
                        id.strip_prefix("analytics:")
                            .and_then(|key| key.split_once(':'))
                            .map(|(provider, series)| (provider.into(), series.into()))
                    })
                    .collect::<Vec<_>>(),
                stacked,
            )
            .map_err(Into::into)
    }

    pub fn interrupt(&self) {
        self.store.interrupt();
    }

    pub fn retention_days(&self) -> u32 {
        self.store.retention_days()
    }

    pub fn set_retention_days(&self, days: u32) {
        self.store.set_retention_days(days);
    }

    pub fn apply_retention_days(&self, days: u32, now: f64) -> Result<history::PruneResult, HistoryStoreError> {
        Ok(self.store.apply_retention_days(days, now)?)
    }

    pub fn record_reading(&self, reading: ProviderReadingRecord, now: f64) -> Result<u64, HistoryStoreError> {
        let network = reading.source == ReadingSource::Network;
        Ok(self.store.record_provider(&reading.into(), now, network)?)
    }

    pub fn seed(&self, rows: Vec<history::SampleWrite>) -> Result<(), HistoryStoreError> {
        self.store.seed(&rows)?;
        Ok(())
    }

    pub fn seed_demo(&self, providers: Vec<String>, now: f64, long_text: bool) -> Result<(), HistoryStoreError> {
        self.store.seed_demo(&providers, now, long_text)?;
        Ok(())
    }

    pub fn record_analytics(
        &self,
        provider: String,
        fingerprint: Option<String>,
        fetched_at: f64,
        points: Vec<history::AnalyticsPoint>,
        scopes: Vec<history::CoverageScope>,
    ) -> Result<u64, HistoryStoreError> {
        self.store
            .record_analytics(&provider, fingerprint.as_deref(), fetched_at, &points, &scopes)
            .map_err(Into::into)
    }

    pub fn activate_account(&self, provider: String, fingerprint: String) -> Result<(), HistoryStoreError> {
        self.store.activate_account(&provider, &fingerprint)?;
        Ok(())
    }

    pub fn clear(&self) -> Result<u64, HistoryStoreError> {
        Ok(self.store.clear()?)
    }

    pub fn samples(&self, query: StoredSampleQuery) -> Result<Vec<history::Sample>, HistoryStoreError> {
        Ok(self.store.samples(history::SampleQuery {
            keys: query.keys.as_deref(),
            start: query.start,
            end: query.end,
            rollup: query.rollup,
            time_zone: &query.time_zone,
            fallback_offset: query.fallback_offset,
            includes_end: query.includes_end,
        })?)
    }

    pub fn last_usage_dates(
        &self,
        keys: Vec<String>,
        start: f64,
        end: f64,
    ) -> Result<Vec<StoredHistoryUsageDate>, HistoryStoreError> {
        Ok(self
            .store
            .last_usage_dates(Some(&keys), start, end)?
            .into_iter()
            .map(|(key, at)| StoredHistoryUsageDate { key, at })
            .collect())
    }

    pub fn analytics(
        &self,
        provider: String,
        start: String,
        end: String,
    ) -> Result<Vec<history::AnalyticsPoint>, HistoryStoreError> {
        Ok(self.store.analytics(&provider, &start, &end)?)
    }

    pub fn analytics_rows(
        &self,
        metric: String,
        providers: Vec<String>,
        start: String,
        end: String,
        includes_end: bool,
    ) -> Result<Vec<history::AnalyticsPoint>, HistoryStoreError> {
        Ok(self
            .store
            .analytics_rows(&metric, &providers, &start, &end, includes_end)?)
    }

    pub fn summaries(&self) -> Result<Vec<history::WindowSummary>, HistoryStoreError> {
        Ok(self.store.summaries()?)
    }

    pub fn earliest_sample(&self, keys: Option<Vec<String>>) -> Result<Option<f64>, HistoryStoreError> {
        Ok(self.store.earliest_sample(keys.as_deref())?)
    }

    pub fn earliest_analytics(&self, metric: String, providers: Vec<String>) -> Result<Option<f64>, HistoryStoreError> {
        Ok(self.store.earliest_analytics(&metric, &providers)?)
    }

    pub fn stats(&self) -> Result<history::Stats, HistoryStoreError> {
        Ok(self.store.stats()?)
    }

    pub fn export_all(&self, path: String, hide_identity: bool) -> Result<(), HistoryStoreError> {
        self.store.export_all(Path::new(&path), hide_identity)?;
        Ok(())
    }

    pub fn export_samples(
        &self,
        path: String,
        start: f64,
        end: f64,
        keys: Option<Vec<String>>,
        includes_end: bool,
    ) -> Result<(), HistoryStoreError> {
        self.store
            .export_samples(Path::new(&path), start, end, keys.as_deref(), includes_end)?;
        Ok(())
    }

    pub fn export_analytics(
        &self,
        path: String,
        metric: String,
        providers: Vec<String>,
        start: f64,
        end: f64,
        hide_identity: bool,
    ) -> Result<(), HistoryStoreError> {
        self.store
            .export_analytics(Path::new(&path), &metric, &providers, start, end, hide_identity)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "../../tests/unit/history/store/tests.rs"]
mod tests;
