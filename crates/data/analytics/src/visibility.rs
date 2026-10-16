//! Owns History series visibility and isolation across UI events.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum VisibilityKind {
    Toggle,
    Isolate,
    IsolateUndo,
}

#[derive(Deserialize)]
pub struct VisibilityAction {
    kind: VisibilityKind,
    metric: String,
    id: String,
    series: Vec<String>,
}

impl VisibilityAction {
    pub fn apply(self, visibility: &mut HistoryVisibility, scope: &str) -> (bool, Vec<String>) {
        let changed = match self.kind {
            VisibilityKind::Toggle => visibility.toggle(scope, &self.metric, &self.id, &self.series),
            VisibilityKind::Isolate => visibility.isolate(scope, &self.metric, &self.id, &self.series, false),
            VisibilityKind::IsolateUndo => visibility.isolate(scope, &self.metric, &self.id, &self.series, true),
        };
        (changed, visibility.shown(scope, &self.series))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Isolation {
    id: String,
    hidden: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Toggle {
    id: String,
    hidden: BTreeSet<String>,
    isolation: Option<Isolation>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HistoryVisibility {
    hidden: BTreeMap<String, BTreeSet<String>>,
    isolation: BTreeMap<String, Isolation>,
    last_toggle: BTreeMap<String, Toggle>,
}

impl HistoryVisibility {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn set_hidden(&mut self, scope: &str, ids: impl IntoIterator<Item = String>) {
        let hidden = ids.into_iter().collect::<BTreeSet<_>>();
        if self.hidden.get(scope) == Some(&hidden) {
            return;
        }
        self.hidden.insert(scope.into(), hidden);
        self.isolation.clear();
        self.last_toggle.clear();
    }

    pub fn hidden(&self, scope: &str) -> Vec<String> {
        self.hidden
            .get(scope)
            .map(|hidden| hidden.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn shown(&self, scope: &str, series: &[String]) -> Vec<String> {
        series.iter().filter(|id| self.visible(scope, id)).cloned().collect()
    }

    pub fn visible(&self, scope: &str, id: &str) -> bool {
        self.hidden.get(scope).is_none_or(|hidden| !hidden.contains(id))
    }

    pub fn toggle(&mut self, scope: &str, group: &str, id: &str, series: &[String]) -> bool {
        if !series.iter().any(|series_id| series_id == id) {
            return false;
        }
        let hidden = self.hidden.entry(scope.into()).or_default();
        if !hidden.contains(id) && series.iter().filter(|series_id| !hidden.contains(*series_id)).count() == 1 {
            self.last_toggle.remove(group);
            return false;
        }
        self.last_toggle.insert(
            group.into(),
            Toggle {
                id: id.into(),
                hidden: hidden.clone(),
                isolation: self.isolation.remove(group),
            },
        );
        if !hidden.insert(id.into()) {
            hidden.remove(id);
        }
        true
    }

    pub fn isolate(&mut self, scope: &str, group: &str, id: &str, series: &[String], undo_last_toggle: bool) -> bool {
        if !series.iter().any(|series_id| series_id == id) {
            return false;
        }
        if undo_last_toggle && let Some(toggle) = self.last_toggle.get(group).filter(|toggle| toggle.id == id).cloned()
        {
            self.hidden.insert(scope.into(), toggle.hidden);
            if let Some(isolation) = toggle.isolation {
                self.isolation.insert(group.into(), isolation);
            } else {
                self.isolation.remove(group);
            }
        }
        self.last_toggle.remove(group);
        if let Some(isolation) = self
            .isolation
            .get(group)
            .filter(|isolation| isolation.id == id)
            .cloned()
        {
            self.hidden.insert(scope.into(), isolation.hidden);
            self.isolation.remove(group);
            return true;
        }
        let original = self
            .isolation
            .get(group)
            .map(|isolation| isolation.hidden.clone())
            .unwrap_or_else(|| self.hidden.get(scope).cloned().unwrap_or_default());
        let mut hidden = original.clone();
        hidden.extend(series.iter().filter(|series_id| *series_id != id).cloned());
        hidden.remove(id);
        self.hidden.insert(scope.into(), hidden);
        self.isolation.insert(
            group.into(),
            Isolation {
                id: id.into(),
                hidden: original,
            },
        );
        true
    }
}

#[cfg(test)]
#[path = "../tests/unit/visibility/tests.rs"]
mod tests;
