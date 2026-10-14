//! Words planned notifications and the notification settings summary for every desktop.

use super::Facts;
use quotaline_text::time::{ClockFormat, ResetPrecision, countdown, remaining, reset_clock};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Copy {
    pub title: String,
    pub body: String,
}

/// The moments a notification is worded against: `fetched_at` anchors the reset clock to the reading that planned
/// it, and `now` measures countdowns from when it is shown.
#[derive(Clone, Copy, Debug)]
pub struct Moments {
    pub now: i64,
    pub fetched_at: i64,
}

fn percent(value: f64) -> i64 {
    value.round_ties_even() as i64
}

pub fn copy(facts: &Facts, provider: &str, login_hint: &str, moments: Moments, clock: &dyn ClockFormat) -> Copy {
    let Moments { now, fetched_at } = moments;
    let (title, body) = match facts {
        Facts::Authentication { detail } => (
            format!("{provider} sign-in needed"),
            detail.as_deref().unwrap_or(login_hint).into(),
        ),
        Facts::Reset { label, used } => (
            format!("{provider} {label} reset"),
            format!("Usage is back to {}%.", percent(*used)),
        ),
        Facts::ThresholdGroup { crossings, lowest } => (
            format!("{provider}: {} limits crossed {lowest}%", crossings.len()),
            crossings
                .iter()
                .map(|crossing| format!("{} at {}%", crossing.label, percent(crossing.used)))
                .collect::<Vec<_>>()
                .join(", ")
                + ".",
        ),
        Facts::Threshold {
            label,
            used,
            threshold,
            reset_at,
            precision,
        } => {
            let body = if *threshold >= 100 {
                "Limit reached.".into()
            } else {
                format!("Crossed {threshold}% of the {} limit.", label.to_lowercase())
            };
            let resets = reset_at.map_or_else(String::new, |reset_at| {
                let clock = reset_clock(reset_at, *precision, fetched_at, clock);
                if *threshold < 100 || *precision == ResetPrecision::Day {
                    format!(" Resets {clock}.")
                } else if reset_at > now {
                    format!(" Resets in {} · {clock}.", countdown((reset_at - now) as f64))
                } else {
                    " Reset due · Awaiting provider refresh.".into()
                }
            });
            (format!("{provider} {label} at {}%", percent(*used)), body + &resets)
        }
        Facts::Credits => (
            format!("{provider} credits depleted"),
            "Usage credits ran out; plan limits now apply.".into(),
        ),
        Facts::ExpiringCredit { expires_at } => (
            format!("{provider} reset credit expires in {}", remaining(expires_at - now)),
            "Use it before it lapses.".into(),
        ),
        Facts::WillRunOut {
            label,
            exhaustion_at,
            reset_at,
        } => (
            format!("{provider} {label} will run out in {}", remaining(exhaustion_at - now)),
            format!(
                "At this pace it hits 100% before the reset in {}.",
                remaining(reset_at - now)
            ),
        ),
        Facts::AheadOfPace { label, used, reset_at } => (
            format!("{provider} {label} is ahead of pace"),
            format!(
                "{}% used with {} left.",
                percent(*used),
                countdown((reset_at - now) as f64)
            ),
        ),
    };
    Copy { title, body }
}

/// The alerts a person chose, as the settings row summarizes them.
#[derive(Clone, Debug)]
pub struct Selection<'a> {
    pub enabled: bool,
    pub thresholds: &'a [u8],
    pub resets: bool,
    pub sign_in: bool,
    pub pace: bool,
    pub expiring_credits: bool,
}

pub fn summary(selection: &Selection<'_>) -> String {
    if !selection.enabled {
        return "Off".into();
    }
    let mut alerts = selection
        .thresholds
        .iter()
        .map(|threshold| format!("{threshold}%"))
        .collect::<Vec<_>>();
    for (selected, name) in [
        (selection.resets, "resets"),
        (selection.sign_in, "sign-in"),
        (selection.pace, "pace"),
        (selection.expiring_credits, "credit expiry"),
    ] {
        if selected {
            alerts.push(name.into());
        }
    }
    if alerts.is_empty() {
        "On · No alerts selected".into()
    } else {
        format!("On · {}", alerts.join(", "))
    }
}

#[cfg(test)]
#[path = "../tests/unit/copy/tests.rs"]
mod tests;
