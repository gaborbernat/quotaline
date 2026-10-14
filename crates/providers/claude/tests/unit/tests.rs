//! Exercises Claude response projection without platform credential or network code.

use super::*;
use serde_json::json;

#[path = "tests/limits.rs"]
mod limits;
#[path = "tests/plan.rs"]
mod plan;
#[path = "tests/profile.rs"]
mod profile;
#[path = "tests/spend.rs"]
mod spend;

fn projection(data: Value, auth: Value, profile: Option<Value>) -> Result<Provider, ProjectionError> {
    project(Input {
        data: &data,
        auth: &auth,
        profile: profile.as_ref(),
        source: "test".into(),
    })
}

#[test]
fn numeric_reset_time_is_projected() {
    let provider = projection(
        json!({"five_hour": {"utilization": 42, "resets_at": 1_788_000_000}}),
        Value::Null,
        None,
    )
    .unwrap();
    assert_eq!(provider.windows[0].reset.as_deref(), Some("2026-08-29T10:40:00+00:00"));
}
