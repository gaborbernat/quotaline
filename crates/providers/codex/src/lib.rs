//! Projects Codex account usage into shared models.

use quotaline_text::text::slug;

pub mod codex_reset;
pub mod codex_usage;
pub mod fetch;
mod presentation;
pub mod request;

#[derive(Clone, Debug, PartialEq)]
pub struct LimitWindow {
    pub used_percent: f64,
    pub duration_seconds: Option<f64>,
    pub reset_at: Option<f64>,
    pub reset_after_seconds: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LimitKind {
    Usage,
    CodeReview,
    Additional(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct LimitGroup {
    pub kind: LimitKind,
    pub limit_reached: bool,
    pub primary: Option<LimitWindow>,
    pub secondary: Option<LimitWindow>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DisplayWindow {
    pub id: String,
    pub label: String,
    pub group: &'static str,
    pub used_percent: f64,
    pub reset_at: Option<f64>,
    pub duration_seconds: f64,
    pub severity: &'static str,
    pub scope: Option<String>,
}

pub fn windows(groups: &[LimitGroup], now: f64) -> Vec<DisplayWindow> {
    groups
        .iter()
        .flat_map(|limit| {
            let (prefix, label_prefix, scope) = match &limit.kind {
                LimitKind::Usage => (String::new(), String::new(), None),
                LimitKind::CodeReview => ("code_review:".into(), "Code review ".into(), None),
                LimitKind::Additional(name) => (
                    format!("additional:{}:", slug(name)),
                    format!("{name} "),
                    Some(name.clone()),
                ),
            };
            [limit.primary.as_ref(), limit.secondary.as_ref()]
                .into_iter()
                .flatten()
                .map(move |window| {
                    let seconds = window.duration_seconds.unwrap_or(18_000.0);
                    let (key, label, group) = window_name(seconds);
                    DisplayWindow {
                        id: format!("{prefix}{key}"),
                        label: format!("{label_prefix}{label}"),
                        group,
                        used_percent: window.used_percent.clamp(0.0, 100.0),
                        reset_at: window
                            .reset_at
                            .or_else(|| window.reset_after_seconds.map(|delta| now + delta)),
                        duration_seconds: seconds,
                        severity: if limit.limit_reached || window.used_percent >= 90.0 {
                            "critical"
                        } else if window.used_percent >= 75.0 {
                            "warning"
                        } else {
                            "normal"
                        },
                        scope: scope.clone(),
                    }
                })
        })
        .collect()
}

fn window_name(seconds: f64) -> (String, String, &'static str) {
    match seconds {
        18_000.0 => ("session".into(), "5-hour".into(), "session"),
        604_800.0 => ("weekly".into(), "Weekly".into(), "weekly"),
        2_592_000.0 | 2_678_400.0 => ("monthly".into(), "Monthly".into(), "monthly"),
        _ => {
            let hours = (seconds / 3600.0) as i64;
            let label = if seconds == 86_400.0 {
                "Daily".into()
            } else if hours >= 24 && hours % 24 == 0 {
                format!("{}d", hours / 24)
            } else if hours > 0 {
                format!("{hours}h")
            } else {
                format!("{}m", (seconds / 60.0) as i64)
            };
            (format!("window-{}", seconds as i64), label, "other")
        }
    }
}

use quotaline_http::ProviderFailureKind;

/// Whether locally logged usage may stand in after the usage API failed. A failure or rate limit the API reported
/// stays visible instead, because older local readings would hide it.
pub fn local_fallback(kind: ProviderFailureKind) -> bool {
    !matches!(kind, ProviderFailureKind::Failed | ProviderFailureKind::RateLimited)
}

#[cfg(test)]
#[path = "../tests/unit/tests.rs"]
mod tests;
