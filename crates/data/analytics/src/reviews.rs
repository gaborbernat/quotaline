//! Counts daily code reviews for provider cards.

use chrono::{DateTime, Days, Utc};

#[derive(Clone, Debug, PartialEq)]
pub struct ReviewRow {
    pub day: String,
    pub value: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ReviewCounts {
    pub today: i64,
    pub week: i64,
}

pub fn review_counts(rows: &[ReviewRow], now: DateTime<Utc>) -> Option<ReviewCounts> {
    if rows.is_empty() {
        return None;
    }
    let today = now.date_naive();
    let week = today.checked_sub_days(Days::new(6)).expect("valid UTC day");
    let (today_count, week_count) = rows.iter().fold((0.0, 0.0), |(today_count, week_count), row| {
        let Ok(day) = chrono::NaiveDate::parse_from_str(&row.day, "%Y-%m-%d") else {
            return (today_count, week_count);
        };
        (
            today_count + if day == today { row.value } else { 0.0 },
            week_count + if day >= week && day <= today { row.value } else { 0.0 },
        )
    });
    Some(ReviewCounts {
        today: today_count as i64,
        week: week_count as i64,
    })
}

#[cfg(test)]
#[path = "../tests/unit/reviews/tests.rs"]
mod tests;
