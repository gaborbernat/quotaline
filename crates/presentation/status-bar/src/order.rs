//! Normalizes and reorders provider and model selections.

use std::collections::HashSet;

use super::{IDS, Provider, Settings};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderDraft {
    pub providers: Vec<String>,
    pub models: Vec<String>,
}

fn normalized(preferred: &[String], available: &[String]) -> Vec<String> {
    let allowed: HashSet<_> = available.iter().collect();
    let mut seen = HashSet::new();
    preferred
        .iter()
        .chain(available)
        .filter(|value| allowed.contains(value) && seen.insert((*value).clone()))
        .cloned()
        .collect()
}

fn provider(key: &str) -> &str {
    key.split_once(':').map_or(key, |(id, _)| id)
}

impl OrderDraft {
    pub fn from_state(settings: &Settings, providers: &BTreeMap<String, Provider>) -> Self {
        let available_providers = IDS.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>();
        let available_models = providers
            .iter()
            .flat_map(|(id, provider)| provider.windows.iter().map(move |window| format!("{id}:{}", window.id)))
            .collect::<Vec<_>>();
        Self::new(
            &settings.provider_order,
            &settings.model_order,
            &available_providers,
            &available_models,
        )
    }

    pub fn new(
        providers: &[String],
        models: &[String],
        available_providers: &[String],
        available_models: &[String],
    ) -> Self {
        let providers = normalized(providers, available_providers);
        let models = normalized(models, available_models);
        let models = providers
            .iter()
            .flat_map(|id| models.iter().filter(move |key| provider(key) == id).cloned())
            .collect();
        Self { providers, models }
    }

    pub fn move_provider(&mut self, source: &str, target: &str, after: bool) {
        move_to(&mut self.providers, source, target, after);
    }

    pub fn swap_provider(&mut self, source: &str, target: &str) {
        if let Some((first, second)) = self
            .providers
            .iter()
            .position(|id| id == source)
            .zip(self.providers.iter().position(|id| id == target))
        {
            self.providers.swap(first, second);
        }
    }

    pub fn move_model(&mut self, source: &str, target: &str, after: bool) {
        if provider(source) == provider(target) {
            move_to(&mut self.models, source, target, after);
        }
    }

    pub fn move_provider_by(&mut self, source: &str, offset: i32) {
        if let Some(index) = self.providers.iter().position(|id| id == source) {
            let target = (index as i64 + i64::from(offset)).clamp(0, self.providers.len() as i64 - 1) as usize;
            if target != index {
                let neighbor = self.providers[target].clone();
                self.move_provider(source, &neighbor, target > index);
            }
        }
    }

    pub fn move_model_by(&mut self, source: &str, offset: i32) {
        let siblings = self
            .models
            .iter()
            .filter(|key| provider(key) == provider(source))
            .collect::<Vec<_>>();
        if let Some(index) = siblings.iter().position(|key| key.as_str() == source) {
            let target = (index as i64 + i64::from(offset)).clamp(0, siblings.len() as i64 - 1) as usize;
            if target != index {
                let neighbor = siblings[target].clone();
                self.move_model(source, &neighbor, target > index);
            }
        }
    }

    pub fn ordered_selection(&self, selected: &[String]) -> Vec<String> {
        let selected: HashSet<_> = selected.iter().collect();
        let selected = &selected;
        self.providers
            .iter()
            .flat_map(|id| {
                self.models
                    .iter()
                    .filter(move |key| provider(key) == id && selected.contains(key))
                    .cloned()
            })
            .collect()
    }
}

fn move_to(order: &mut Vec<String>, source: &str, target: &str, after: bool) {
    if source == target || !order.iter().any(|key| key == source) || !order.iter().any(|key| key == target) {
        return;
    }
    order.retain(|key| key != source);
    let index = order
        .iter()
        .position(|key| key == target)
        .expect("target remained in order");
    order.insert(index + usize::from(after), source.into());
}

#[cfg(test)]
#[path = "../tests/unit/order/tests.rs"]
mod tests;
