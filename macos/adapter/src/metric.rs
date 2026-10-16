//! Exposes shared metric metadata to the macOS chart views.

use quotaline_analytics as analytics;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct MetricCatalogEntry {
    pub id: String,
    pub title: String,
    pub unit: String,
    pub display_unit: String,
    pub group: String,
    pub suppliers: Vec<String>,
    pub mark: String,
    pub summary: String,
    pub stackable: bool,
    pub parallel: bool,
    pub daily_utc: bool,
    pub attribution: String,
}

#[uniffi::export]
pub fn history_metric_definitions() -> Vec<MetricCatalogEntry> {
    analytics::metric_definitions()
        .into_iter()
        .map(|definition| MetricCatalogEntry {
            id: definition.id,
            title: definition.title,
            unit: definition.unit,
            display_unit: definition.display_unit,
            group: definition.group,
            suppliers: definition.suppliers,
            mark: definition.mark,
            summary: definition.summary,
            stackable: definition.stackable,
            parallel: definition.parallel,
            daily_utc: definition.daily_utc,
            attribution: definition.attribution,
        })
        .collect()
}

#[cfg(test)]
#[path = "../tests/unit/metric/tests.rs"]
mod tests;
