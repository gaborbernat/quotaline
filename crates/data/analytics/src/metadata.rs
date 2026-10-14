//! Defines history metric labels and chart behavior for every frontend.

use super::{METRIC_GROUPS, METRICS};
use quotaline_catalog as providers;
use std::sync::OnceLock;

static DEFINITIONS: OnceLock<Vec<MetricDefinition>> = OnceLock::new();

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct MetricDefinition {
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

fn definitions() -> &'static [MetricDefinition] {
    DEFINITIONS.get_or_init(build_definitions)
}

pub(super) fn definition(id: &str) -> Option<&'static MetricDefinition> {
    definitions().iter().find(|item| item.id == id)
}

pub fn metric_definitions() -> Vec<MetricDefinition> {
    definitions().to_vec()
}

fn build_definitions() -> Vec<MetricDefinition> {
    let mut definitions = vec![MetricDefinition {
        id: "windowUsagePercent".into(),
        title: "Usage %".into(),
        unit: "percentage".into(),
        display_unit: "%".into(),
        group: "Windows".into(),
        suppliers: {
            let mut ids = providers::ALL
                .iter()
                .map(|provider| provider.id.to_owned())
                .collect::<Vec<_>>();
            ids.sort();
            ids
        },
        mark: "stepLine".into(),
        summary: "latest".into(),
        stackable: false,
        parallel: false,
        daily_utc: false,
        attribution: "Every model · step line · selected time zone".into(),
    }];
    definitions.extend(METRICS.iter().map(|(id, title, display_unit, attribution)| {
        let group = METRIC_GROUPS
            .iter()
            .find(|(_, ids)| ids.contains(id))
            .map(|(group, _)| *group)
            .expect("every analytics metric belongs to a group");
        let suppliers = match group {
            "Claude and Codex" => vec!["claude".into(), "codex".into()],
            "Claude" | "Projects" => vec!["claude".into()],
            _ => vec!["codex".into()],
        };
        let unit = match *display_unit {
            "%" => "percentage",
            "USD" => "usd",
            other => other,
        };
        let parallel = matches!(*id, "turns" | "threads" | "credits");
        let mark = if matches!(*id, "surfaceUsagePercent" | "creditBalance") {
            "line"
        } else {
            "bars"
        };
        MetricDefinition {
            id: (*id).into(),
            title: (*title).into(),
            unit: unit.into(),
            display_unit: (*display_unit).into(),
            group: group.into(),
            suppliers,
            mark: mark.into(),
            summary: if unit == "percentage" || *id == "creditBalance" {
                "latest"
            } else {
                "sum"
            }
            .into(),
            stackable: mark == "bars" && !parallel,
            parallel,
            daily_utc: true,
            attribution: (*attribution).into(),
        }
    }));
    definitions
}

#[cfg(test)]
#[path = "../tests/unit/metadata/tests.rs"]
mod tests;
