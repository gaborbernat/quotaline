//! Builds Claude history metrics from daily model, session, and project totals.

use super::Point;

#[derive(Clone, Debug, PartialEq)]
pub struct ModelTotal {
    pub day: String,
    pub model: String,
    pub input: i64,
    pub cache_read: i64,
    pub output: i64,
    pub cache_write: i64,
    pub cost: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DayTotal {
    pub day: String,
    pub messages: i64,
    pub sessions: i64,
    pub tool_calls: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectTotal {
    pub day: String,
    pub project: String,
    pub messages: i64,
    pub cost: Option<f64>,
}

pub fn claude_points(models: &[ModelTotal], days: &[DayTotal], projects: &[ProjectTotal]) -> Vec<Point> {
    let mut points = Vec::with_capacity(models.len() * 5 + days.len() * 3 + projects.len() * 2);
    let mut push = |day: &str, metric: &str, series: &str, value: f64| {
        points.push(Point {
            day: day.into(),
            provider: "claude".into(),
            metric: metric.into(),
            series: series.into(),
            value,
        });
    };
    for model in models {
        for (metric, value) in [
            ("inputTokens", model.input),
            ("cachedInputTokens", model.cache_read),
            ("outputTokens", model.output),
            ("cacheWriteTokens", model.cache_write),
        ] {
            push(&model.day, metric, &model.model, value as f64);
        }
        if let Some(cost) = model.cost {
            push(&model.day, "costUSD", &model.model, cost);
        }
    }
    for day in days {
        for (metric, series, value) in [
            ("messages", "messages", day.messages),
            ("sessions", "sessions", day.sessions),
            ("toolCalls", "tool calls", day.tool_calls),
        ] {
            push(&day.day, metric, series, value as f64);
        }
    }
    for project in projects {
        push(
            &project.day,
            "projectMessages",
            &project.project,
            project.messages as f64,
        );
        if let Some(cost) = project.cost {
            push(&project.day, "projectCost", &project.project, cost);
        }
    }
    points.sort_by(|left, right| {
        (&left.day, &left.metric, &left.series).cmp(&(&right.day, &right.metric, &right.series))
    });
    points
}

#[cfg(test)]
#[path = "../tests/unit/claude/tests.rs"]
mod tests;
