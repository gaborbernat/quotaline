//! Identifies unreadable Claude fields while retaining usable quota data.

use serde_json::{Map, Value};

use quotaline_text::json::{number, text};

pub(super) fn inspect(fields: &Map<String, Value>) -> (Vec<String>, usize) {
    let dropped_limits = fields.get("limits").and_then(Value::as_array).map_or(0, |limits| {
        limits
            .iter()
            .filter(|limit| text(&limit["kind"]).is_none() || number(&limit["percent"]).is_none())
            .count()
    });
    let mut malformed = fields
        .iter()
        .filter_map(|(name, value)| {
            let invalid = match name.as_str() {
                "limits" => !value.is_null() && !value.is_array(),
                "spend" | "extra_usage" => !value.is_null() && !value.is_object(),
                "five_hour"
                | "seven_day"
                | "seven_day_opus"
                | "seven_day_sonnet"
                | "seven_day_oauth_apps"
                | "seven_day_cowork" => !value.is_null() && number(&value["utilization"]).is_none(),
                "member_dashboard_available" | "iguana_necktie" => false,
                _ => value.get("utilization").is_some() && number(&value["utilization"]).is_none(),
            };
            invalid.then(|| name.clone())
        })
        .collect::<Vec<_>>();
    malformed.sort();
    (malformed, dropped_limits)
}
