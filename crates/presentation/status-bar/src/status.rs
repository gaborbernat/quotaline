//! Selects status labels and quota restrictions.

use super::*;

pub(super) fn default_label(code: &str, id: &str, label: &str, scope: Option<&str>) -> String {
    if scope.is_none() {
        match id {
            "session" => return format!("{code} 5h"),
            "weekly" => return format!("{code} 7d"),
            "monthly" => return format!("{code} 1mo"),
            _ => {}
        }
    }
    let source = scope.unwrap_or(label);
    let words: Vec<_> = source
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let normalized = words.join(" ").to_lowercase();
    let digits: String = words
        .iter()
        .flat_map(|word| word.chars().filter(char::is_ascii_digit))
        .collect();
    if normalized.contains("opus") {
        return digits
            .chars()
            .next()
            .map_or_else(|| "OP".into(), |digit| format!("OP{digit}"));
    }
    for (needle, short) in [("sonnet", "SO"), ("haiku", "HA"), ("fable", "FAB"), ("spark", "SPK")] {
        if normalized.contains(needle) {
            return short.into();
        }
    }
    if normalized.contains("flash") {
        return if code == "GM" { "GFL" } else { "FLA" }.into();
    }
    if code == "GM" && normalized.contains("pro") && !digits.is_empty() {
        return format!("G{}", digits.chars().take(2).collect::<String>());
    }
    for (needle, short) in [("code review", "CR"), ("on demand", "OND"), ("premium", "PRE")] {
        if normalized.contains(needle) {
            return short.into();
        }
    }
    if code == "CP" && normalized.contains("completion") {
        return "GHX".into();
    }
    if code == "CP" && normalized.contains("chat") {
        return "GHC".into();
    }
    let meaningful: Vec<_> = words
        .into_iter()
        .filter(|word| {
            ![
                "claude", "codex", "gemini", "github", "copilot", "model", "models", "window",
            ]
            .contains(&word.to_lowercase().as_str())
        })
        .collect();
    if meaningful.len() == 1 {
        return meaningful[0].chars().take(3).collect::<String>().to_uppercase();
    }
    let initials: String = meaningful
        .iter()
        .filter_map(|word| word.chars().next())
        .take(3)
        .collect();
    if initials.chars().count() >= 2 {
        return initials.to_uppercase();
    }
    source.chars().take(3).collect::<String>().to_uppercase()
}

pub fn default_selected_windows(providers: &BTreeMap<String, Provider>) -> Vec<String> {
    IDS.into_iter()
        .filter_map(|id| providers.get(id).map(|provider| (id, provider)))
        .flat_map(|(id, provider)| {
            default_window_ids(provider.windows.iter().map(|row| (row.id.as_str(), row.active)))
                .into_iter()
                .map(move |window| format!("{id}:{window}"))
        })
        .collect()
}

pub(super) fn short_label_override(value: &str, default: &str) -> Option<String> {
    let value = short_label_draft(value);
    let value = value.trim();
    (!value.is_empty() && normalized_label(value) != normalized_label(default)).then(|| value.to_string())
}

pub fn derived_short_labels(providers: &BTreeMap<String, Provider>) -> BTreeMap<String, String> {
    let mut windows = BTreeMap::new();
    for (id, provider) in providers {
        if let Some(position) = IDS.iter().position(|candidate| *candidate == id) {
            for row in &provider.windows {
                windows.insert(
                    format!("{id}:{}", row.id),
                    default_label(CODES[position], &row.id, &row.label, row.scope.as_deref()),
                );
            }
        }
    }
    unique_short_labels(windows)
}

pub(super) fn unique_short_labels(windows: BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    let mut used = std::collections::BTreeSet::new();
    for (key, base) in windows {
        let base = base.graphemes(true).take(SHORT_LABEL_LIMIT).collect::<String>();
        let mut label = base.clone();
        let mut ordinal = 2;
        while !used.insert(normalized_label(&label)) {
            let mut number = ordinal;
            let mut suffix = String::new();
            while number > 0 {
                suffix.insert(0, char::from_digit(number % 36, 36).unwrap().to_ascii_uppercase());
                number /= 36;
            }
            label = format!(
                "{}{}",
                base.graphemes(true)
                    .take(SHORT_LABEL_LIMIT.saturating_sub(suffix.len()))
                    .collect::<String>(),
                suffix
            );
            ordinal += 1;
        }
        labels.insert(key, label);
    }
    labels
}

pub fn short_label_maps(
    providers: &BTreeMap<String, Provider>,
    overrides: &BTreeMap<String, String>,
) -> (BTreeMap<String, String>, BTreeMap<String, String>) {
    let defaults = derived_short_labels(providers);
    let resolved = resolve_short_labels(&defaults, overrides);
    (defaults, resolved)
}

pub(super) fn resolve_short_labels(
    defaults: &BTreeMap<String, String>,
    overrides: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut candidates = BTreeMap::new();
    let mut used = std::collections::BTreeSet::new();
    for (key, fallback) in defaults {
        if let Some(value) = overrides
            .get(key)
            .and_then(|value| short_label_override(value, fallback))
            && used.insert(normalized_label(&value))
        {
            candidates.insert(key.clone(), value);
        }
    }
    loop {
        let fallbacks: std::collections::BTreeSet<_> = defaults
            .iter()
            .filter(|(key, _)| !candidates.contains_key(*key))
            .map(|(_, value)| normalized_label(value))
            .collect();
        let rejected: Vec<_> = candidates
            .iter()
            .filter(|(_, value)| fallbacks.contains(&normalized_label(value)))
            .map(|(key, _)| key.clone())
            .collect();
        if rejected.is_empty() {
            break;
        }
        for key in rejected {
            candidates.remove(&key);
        }
    }
    defaults
        .iter()
        .map(|(key, fallback)| {
            let label = candidates.remove(key).unwrap_or_else(|| fallback.clone());
            (key.clone(), label)
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ShortLabelPreview {
    pub override_value: Option<String>,
    pub conflict_key: Option<String>,
}

pub fn preview_short_label(
    providers: &BTreeMap<String, Provider>,
    overrides: &BTreeMap<String, String>,
    key: &str,
    draft: &str,
) -> Option<ShortLabelPreview> {
    let defaults = derived_short_labels(providers);
    preview_from_defaults(&defaults, overrides, key, draft)
}

pub(super) fn preview_from_defaults(
    defaults: &BTreeMap<String, String>,
    overrides: &BTreeMap<String, String>,
    key: &str,
    draft: &str,
) -> Option<ShortLabelPreview> {
    let fallback = defaults.get(key)?;
    let override_value = short_label_override(draft, fallback);
    let conflict_key = override_value.as_ref().and_then(|candidate| {
        let mut other_overrides = overrides.clone();
        other_overrides.remove(key);
        let resolved = resolve_short_labels(defaults, &other_overrides);
        let normalized = normalized_label(candidate);
        defaults
            .keys()
            .find(|other| {
                other.as_str() != key
                    && (resolved
                        .get(*other)
                        .is_some_and(|label| normalized_label(label) == normalized)
                        || other_overrides.get(*other).is_some_and(|value| {
                            defaults.get(*other).is_some_and(|fallback| {
                                short_label_override(value, fallback)
                                    .is_some_and(|label| normalized_label(&label) == normalized)
                            })
                        }))
            })
            .cloned()
    });
    Some(ShortLabelPreview {
        override_value,
        conflict_key,
    })
}

pub fn short_label_conflicts(
    providers: &BTreeMap<String, Provider>,
    previous: &BTreeMap<String, String>,
    overrides: &BTreeMap<String, String>,
) -> bool {
    let defaults = derived_short_labels(providers);
    let resolved = resolve_short_labels(&defaults, overrides);
    overrides.iter().any(|(key, value)| {
        previous.get(key) != Some(value)
            && defaults.get(key).is_some_and(|fallback| {
                short_label_override(value, fallback).is_some_and(|candidate| {
                    let normalized = normalized_label(&candidate);
                    resolved.get(key) != Some(&candidate)
                        || overrides.iter().any(|(other, label)| {
                            other != key
                                && defaults.get(other).is_some_and(|other_default| {
                                    short_label_override(label, other_default)
                                        .is_some_and(|other_label| normalized_label(&other_label) == normalized)
                                })
                        })
                })
            })
    })
}
