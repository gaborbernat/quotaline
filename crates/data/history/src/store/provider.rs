//! Records one provider reading, mapped the same way on both desktops.

use quotaline_quota::Provider;

use super::{Result, Sample, SampleWrite, SnapshotWrite, Store};

impl Store {
    /// `network` is false for readings restored from a local log, which must not move credit history.
    pub fn record_provider(&self, provider: &Provider, now: f64, network: bool) -> Result<u64> {
        let windows = provider
            .windows
            .iter()
            .map(|row| SampleWrite {
                sample: Sample {
                    at: 0.0,
                    key: format!("{}:{}", provider.id, row.id),
                    used: row.used,
                    reset_at: row.reset.as_deref().and_then(quotaline_text::time::parse_iso_epoch),
                },
                label: row.label.clone(),
            })
            .collect::<Vec<_>>();
        let fingerprint = provider.account_fingerprint();
        self.record_snapshot(SnapshotWrite {
            provider: &provider.id,
            fingerprint: fingerprint.as_deref(),
            now,
            fetched_at: provider.updated.map_or(now, |updated| updated as f64),
            network,
            credit_balance: provider.credit_balance.filter(|value| value.is_finite()),
            credit_unlimited: provider.usage_credits.as_ref().is_some_and(|credits| credits.unlimited),
            windows: &windows,
        })
    }
}
