//! Owns the retained Claude transcript aggregates and checkpoint format.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::Record;

mod aggregate;
pub use aggregate::TranscriptAnalytics;
mod usage;
pub use usage::LocalTranscriptUsage;

#[derive(Clone, Copy, Debug, PartialEq)]
struct SwiftDate(f64);

impl Serialize for SwiftDate {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(self.0 - 978_307_200.0)
    }
}

impl<'de> Deserialize<'de> for SwiftDate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        f64::deserialize(deserializer).map(|seconds| Self(seconds + 978_307_200.0))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TranscriptOffset {
    pub bytes: u64,
    #[serde(rename = "scanGeneration")]
    pub scan_generation: u64,
}

impl<'de> Deserialize<'de> for TranscriptOffset {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Stored {
            Legacy(u64),
            Current {
                bytes: u64,
                #[serde(rename = "scanGeneration")]
                scan_generation: u64,
            },
        }
        Ok(match Stored::deserialize(deserializer)? {
            Stored::Legacy(bytes) => Self {
                bytes,
                scan_generation: 0,
            },
            Stored::Current { bytes, scan_generation } => Self { bytes, scan_generation },
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct TokenUsage {
    input: i64,
    output: i64,
    #[serde(rename = "cacheWrite")]
    cache_write: i64,
    #[serde(rename = "cacheRead")]
    cache_read: i64,
    #[serde(rename = "cacheWriteOneHour", default, skip_serializing_if = "Option::is_none")]
    cache_write_one_hour: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct ProjectAggregate {
    cost: f64,
    messages: i64,
    #[serde(default)]
    priced: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct DayAggregate {
    models: BTreeMap<String, TokenUsage>,
    #[serde(default)]
    costs: BTreeMap<String, f64>,
    messages: i64,
    sessions: BTreeSet<String>,
    #[serde(rename = "toolCalls")]
    tool_calls: i64,
    #[serde(default)]
    projects: BTreeMap<String, ProjectAggregate>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct RecentEvent {
    timestamp: SwiftDate,
    tokens: i64,
    cost: f64,
    messages: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct RecentAggregate {
    timestamp: SwiftDate,
    tokens: i64,
    cost: f64,
    messages: i64,
    #[serde(rename = "firstTimestamp", default, skip_serializing_if = "Option::is_none")]
    first_timestamp: Option<SwiftDate>,
    #[serde(rename = "lastTimestamp", default, skip_serializing_if = "Option::is_none")]
    last_timestamp: Option<SwiftDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    events: Option<Vec<RecentEvent>>,
}

impl RecentAggregate {
    fn new(at: f64) -> Self {
        Self {
            timestamp: SwiftDate(at),
            tokens: 0,
            cost: 0.0,
            messages: 0,
            first_timestamp: None,
            last_timestamp: None,
            events: None,
        }
    }

    fn append(&mut self, event: RecentEvent) {
        self.tokens += event.tokens;
        self.cost += event.cost;
        self.messages += event.messages;
        self.first_timestamp = Some(SwiftDate(
            self.first_timestamp
                .map_or(event.timestamp.0, |first| first.0.min(event.timestamp.0)),
        ));
        self.last_timestamp = Some(SwiftDate(
            self.last_timestamp
                .map_or(event.timestamp.0, |last| last.0.max(event.timestamp.0)),
        ));
        self.events.get_or_insert_with(Vec::new).push(event);
    }

    fn merge(&mut self, other: Self) {
        self.tokens += other.tokens;
        self.cost += other.cost;
        self.messages += other.messages;
        let first = other.first_timestamp.unwrap_or(other.timestamp).0;
        let last = other.last_timestamp.unwrap_or(other.timestamp).0;
        self.first_timestamp = Some(SwiftDate(
            self.first_timestamp.map_or(first, |current| current.0.min(first)),
        ));
        self.last_timestamp = Some(SwiftDate(
            self.last_timestamp.map_or(last, |current| current.0.max(last)),
        ));
        self.events
            .get_or_insert_with(Vec::new)
            .extend(other.events.unwrap_or_else(|| {
                vec![RecentEvent {
                    timestamp: other.timestamp,
                    tokens: other.tokens,
                    cost: other.cost,
                    messages: other.messages,
                }]
            }));
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TranscriptState {
    offsets: BTreeMap<String, TranscriptOffset>,
    #[serde(rename = "seenByDay")]
    seen_by_day: BTreeMap<String, BTreeSet<String>>,
    #[serde(rename = "projectSeenByDay", default)]
    project_seen_by_day: BTreeMap<String, BTreeSet<String>>,
    days: BTreeMap<String, DayAggregate>,
    recent: BTreeMap<String, RecentAggregate>,
    #[serde(rename = "activeRetentionDays", default = "default_retention")]
    active_retention_days: u32,
    #[serde(rename = "unpricedModels", default)]
    unpriced_models: BTreeSet<String>,
    #[serde(rename = "projectIdentityVersion", default = "default_project_version")]
    project_identity_version: u8,
    #[serde(skip)]
    requires_checkpoint: bool,
}

fn default_retention() -> u32 {
    quotaline_settings::DEFAULT_HISTORY_RETENTION_DAYS
}
fn default_project_version() -> u8 {
    0
}

impl Default for TranscriptState {
    fn default() -> Self {
        Self {
            offsets: BTreeMap::new(),
            seen_by_day: BTreeMap::new(),
            project_seen_by_day: BTreeMap::new(),
            days: BTreeMap::new(),
            recent: BTreeMap::new(),
            active_retention_days: quotaline_settings::DEFAULT_HISTORY_RETENTION_DAYS,
            unpriced_models: BTreeSet::new(),
            project_identity_version: 2,
            requires_checkpoint: false,
        }
    }
}

impl TranscriptState {
    pub fn requires_checkpoint(&self) -> bool {
        self.requires_checkpoint
    }

    pub fn configure_retention(&mut self, days: u32) -> bool {
        if days == self.active_retention_days {
            return false;
        }
        if days > self.active_retention_days {
            for offset in self.offsets.values_mut() {
                offset.bytes = 0;
            }
        }
        self.active_retention_days = days;
        true
    }

    pub fn offset(&self, path: &str) -> Option<&TranscriptOffset> {
        self.offsets.get(path)
    }

    pub fn set_offset(&mut self, path: String, bytes: u64, scan_generation: u64) {
        self.offsets.insert(path, TranscriptOffset { bytes, scan_generation });
    }

    pub fn touch_offset(&mut self, path: &str, scan_generation: u64) {
        if let Some(offset) = self.offsets.get_mut(path) {
            offset.scan_generation = scan_generation;
        }
    }

    pub fn retain_offsets(&mut self, scan_generation: u64) -> bool {
        let before = self.offsets.len();
        self.offsets
            .retain(|_, offset| offset.scan_generation == scan_generation);
        self.offsets.len() != before
    }

    pub fn maximum_scan_generation(&self) -> u64 {
        self.offsets
            .values()
            .map(|offset| offset.scan_generation)
            .max()
            .unwrap_or_default()
    }

    pub fn from_checkpoint(data: &[u8]) -> serde_json::Result<Self> {
        let value: serde_json::Value = serde_json::from_slice(data)?;
        let has_project_seen = value.get("projectSeenByDay").is_some();
        let mut state: Self = serde_json::from_value(value)?;
        if state.project_identity_version != 2 {
            state.requires_checkpoint = true;
            for offset in state.offsets.values_mut() {
                offset.bytes = 0;
            }
            for day in state.days.values_mut() {
                day.projects = std::mem::take(&mut day.projects)
                    .into_iter()
                    .map(|(project, total)| (format!("legacy:{project}"), total))
                    .collect();
            }
            state.project_seen_by_day.clear();
        } else if !has_project_seen {
            state.project_seen_by_day = state.seen_by_day.clone();
        }
        state.project_identity_version = 2;
        Ok(state)
    }

    pub fn checkpoint(&self) -> serde_json::Result<Vec<u8>> {
        serde_json::to_vec(self)
    }

    pub fn message_count(&self) -> i64 {
        self.days.values().map(|day| day.messages).sum()
    }

    pub fn unpriced_models(&self) -> Vec<String> {
        self.unpriced_models.iter().cloned().collect()
    }
}

#[cfg(test)]
#[path = "../../tests/unit/claude_transcripts/state/tests.rs"]
mod tests;
