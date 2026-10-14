//! Persists shared History visibility after Shell events.

use super::*;
use quotaline_analytics::VisibilityAction;

impl Bus {
    pub(super) async fn history_visibility(&self, raw_action: &str) -> fdo::Result<String> {
        let action: VisibilityAction =
            serde_json::from_str(raw_action).map_err(|error| fdo::Error::InvalidArgs(error.to_string()))?;
        let (hidden, shown, changed) = {
            let mut inner = self.0.inner.lock().await;
            let mut visibility = inner.history_visibility.clone();
            let (changed, shown) = action.apply(&mut visibility, "global");
            let hidden = visibility.hidden("global");
            if changed {
                let mut settings = inner.settings.clone();
                settings.history_hidden_keys = hidden.clone();
                store::atomic_json(&store::config(), &settings.stored()).map_err(failed)?;
                inner.settings = settings;
                inner.history_visibility = visibility;
            }
            (hidden, shown, changed)
        };
        if changed {
            self.0.emit().await;
        }
        Ok(json!({"hidden": hidden, "shown": shown}).to_string())
    }
}
