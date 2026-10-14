//! Checks checkpoint migration, aggregation, and local usage through the state API.

use super::*;

#[path = "tests/aggregate.rs"]
mod aggregate;
#[path = "tests/checkpoint.rs"]
mod checkpoint;
#[path = "tests/offsets.rs"]
mod offsets;
#[path = "tests/usage.rs"]
mod usage;

fn record(id: &str, timestamp: &str, model: &str) -> Record {
    super::super::parse(
        serde_json::json!({
            "type": "assistant", "timestamp": timestamp,
            "message": {"id": id, "model": model, "usage": {"input_tokens": 2, "output_tokens": 3}}
        })
        .to_string()
        .as_bytes(),
    )
    .unwrap()
}
