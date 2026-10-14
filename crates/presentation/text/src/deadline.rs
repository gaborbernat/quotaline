//! Schedules updates for age and reset labels.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum DeadlineKind {
    Age,
    Reset,
}

pub fn next_update(kind: DeadlineKind, target: Option<f64>, now: f64) -> Option<f64> {
    let target = target?;
    match kind {
        DeadlineKind::Age => {
            let seconds = (now - target).max(0.0);
            let interval = if seconds < 3600.0 {
                60.0
            } else if seconds < 86_400.0 {
                3600.0
            } else {
                86_400.0
            };
            Some(target + ((seconds / interval).floor() + 1.0) * interval)
        }
        DeadlineKind::Reset => {
            if target <= now {
                return None;
            }
            let seconds = target - now;
            if seconds < 60.0 {
                return Some(target);
            }
            let interval = if seconds >= 86_400.0 { 3600.0 } else { 60.0 };
            // Rounded countdown text changes just after the exact boundary.
            Some((now + seconds % interval).next_up())
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/deadline/tests.rs"]
mod tests;
