//! Holds normalized provider and model order drafts behind immutable UniFFI handles.

use std::sync::Arc;

use quotaline_status_bar::OrderDraft;

#[derive(uniffi::Object)]
pub struct RustOrderDraft(OrderDraft);

#[uniffi::export]
impl RustOrderDraft {
    #[uniffi::constructor]
    pub fn new(
        providers: Vec<String>,
        models: Vec<String>,
        available_providers: Vec<String>,
        available_models: Vec<String>,
    ) -> Self {
        Self(OrderDraft::new(
            &providers,
            &models,
            &available_providers,
            &available_models,
        ))
    }

    pub fn providers(&self) -> Vec<String> {
        self.0.providers.clone()
    }

    pub fn models(&self) -> Vec<String> {
        self.0.models.clone()
    }

    pub fn move_provider(&self, source: String, target: String, after: bool) -> Arc<Self> {
        let mut next = self.0.clone();
        next.move_provider(&source, &target, after);
        Arc::new(Self(next))
    }

    pub fn move_model(&self, source: String, target: String, after: bool) -> Arc<Self> {
        let mut next = self.0.clone();
        next.move_model(&source, &target, after);
        Arc::new(Self(next))
    }

    pub fn move_provider_by(&self, source: String, offset: i32) -> Arc<Self> {
        let mut next = self.0.clone();
        next.move_provider_by(&source, offset);
        Arc::new(Self(next))
    }

    pub fn move_model_by(&self, source: String, offset: i32) -> Arc<Self> {
        let mut next = self.0.clone();
        next.move_model_by(&source, offset);
        Arc::new(Self(next))
    }

    pub fn ordered_selection(&self, selected: Vec<String>) -> Vec<String> {
        self.0.ordered_selection(&selected)
    }
}

#[cfg(test)]
#[path = "../tests/unit/order/tests.rs"]
mod tests;
