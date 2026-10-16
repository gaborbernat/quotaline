//! Caches history chart queries and projects their charts.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::{self as history, SampleQuery, Store};
use quotaline_analytics as analytics;
use quotaline_analytics::AnalyticsChart;
use quotaline_analytics::Point;
use quotaline_analytics::WindowChart;
use quotaline_analytics::WindowChartRequest;
use quotaline_analytics::WindowRollup;
use quotaline_analytics::WindowSample;

#[derive(Clone, Debug, PartialEq, Eq)]
enum QueryKey {
    Window {
        keys: Vec<String>,
        start: u64,
        end: u64,
        rollup: WindowRollup,
        time_zone: String,
        fallback_offset: i32,
        includes_end: bool,
    },
    Analytics(AnalyticsQuery),
}

#[derive(Clone)]
enum Rows {
    Window(Arc<Vec<WindowSample>>),
    Analytics(Arc<Vec<Point>>),
}

impl Rows {
    fn len(&self) -> usize {
        match self {
            Self::Window(rows) => rows.len(),
            Self::Analytics(rows) => rows.len(),
        }
    }
}

struct Cache {
    entries: VecDeque<(QueryKey, Rows)>,
    max_rows: usize,
}

impl Cache {
    fn get(&self, key: &QueryKey) -> Option<Rows> {
        self.entries
            .iter()
            .find(|(stored, _)| stored == key)
            .map(|(_, rows)| rows.clone())
    }

    fn insert(&mut self, key: QueryKey, rows: Rows) {
        self.entries.retain(|(stored, _)| stored != &key);
        self.entries.push_back((key, rows));
        while self.entries.len() > 2 || self.entries.iter().map(|(_, rows)| rows.len()).sum::<usize>() > self.max_rows {
            self.entries.pop_front();
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalyticsQuery {
    pub metric: String,
    pub providers: Vec<String>,
    pub start_day: String,
    pub before_day: String,
}

pub struct HistoryChartSession {
    store: Arc<Store>,
    cache: Mutex<Cache>,
}

impl HistoryChartSession {
    pub fn new(store: Arc<Store>, max_cached_rows: usize) -> Self {
        Self {
            store,
            cache: Mutex::new(Cache {
                entries: VecDeque::new(),
                max_rows: max_cached_rows,
            }),
        }
    }

    pub fn clear(&self) {
        self.cache.lock().unwrap().entries.clear();
    }

    pub fn window(&self, request: WindowChartRequest) -> history::Result<WindowChart> {
        let key = QueryKey::Window {
            keys: request.keys.clone(),
            start: request.start.to_bits(),
            end: request.end.to_bits(),
            rollup: request.rollup,
            time_zone: request.time_zone.clone(),
            fallback_offset: request.fallback_offset,
            includes_end: request.includes_end,
        };
        let cached = self.cache.lock().unwrap().get(&key);
        let rows = match cached {
            Some(Rows::Window(rows)) => rows,
            _ => {
                let rollup = analytics::history_query_rollup(
                    request.start,
                    request.end,
                    request.rollup.seconds(),
                    analytics::SAMPLE_INTERVAL,
                    analytics::HISTORY_MAX_QUERY_POINTS,
                );
                let rows = Arc::new(self.store.samples(SampleQuery {
                    keys: Some(&request.keys),
                    start: request.start - rollup.max(analytics::SAMPLE_INTERVAL) * 1.5,
                    end: request.end,
                    rollup: Some(rollup),
                    time_zone: &request.time_zone,
                    fallback_offset: request.fallback_offset,
                    includes_end: request.includes_end,
                })?);
                self.cache.lock().unwrap().insert(key, Rows::Window(rows.clone()));
                rows
            }
        };
        Ok(analytics::window_chart(&rows, &request))
    }

    pub fn analytics(
        &self,
        query: AnalyticsQuery,
        hidden: &[(String, String)],
        stacked: bool,
    ) -> history::Result<Option<AnalyticsChart>> {
        let key = QueryKey::Analytics(query.clone());
        let cached = self.cache.lock().unwrap().get(&key);
        let rows = match cached {
            Some(Rows::Analytics(rows)) => rows,
            _ => {
                let rows = Arc::new(self.store.analytics_rows(
                    &query.metric,
                    &query.providers,
                    &query.start_day,
                    &query.before_day,
                    false,
                )?);
                self.cache.lock().unwrap().insert(key, Rows::Analytics(rows.clone()));
                rows
            }
        };
        Ok(analytics::analytics_chart(&rows, &query.metric, hidden, stacked))
    }
}

#[cfg(test)]
#[path = "../tests/unit/chart/tests.rs"]
mod tests;
