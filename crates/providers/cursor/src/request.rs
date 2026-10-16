//! Defines Cursor usage endpoints and authentication headers.

pub const SUMMARY_URL: &str = "https://cursor.com/api/usage-summary";
pub const PROFILE_URL: &str = "https://cursor.com/api/auth/me";
pub const PERIOD_URL: &str = "https://api2.cursor.sh/aiserver.v1.DashboardService/GetCurrentPeriodUsage";

pub fn cookie_headers(cookie: &str) -> [(&'static str, String); 3] {
    [
        ("Cookie", cookie.into()),
        ("Origin", "https://cursor.com".into()),
        ("User-Agent", "quotaline".into()),
    ]
}

pub fn bearer_headers(token: &str) -> [(&'static str, String); 3] {
    [
        ("Authorization", format!("Bearer {token}")),
        ("Connect-Protocol-Version", "1".into()),
        ("User-Agent", "quotaline".into()),
    ]
}

#[cfg(test)]
#[path = "../tests/unit/request/tests.rs"]
mod tests;
