use super::*;

#[test]
fn endpoints_cover_dashboard_profile_and_period_fallback() {
    assert_eq!(SUMMARY_URL, "https://cursor.com/api/usage-summary");
    assert_eq!(PROFILE_URL, "https://cursor.com/api/auth/me");
    assert_eq!(
        PERIOD_URL,
        "https://api2.cursor.sh/aiserver.v1.DashboardService/GetCurrentPeriodUsage"
    );
}

#[test]
fn cookie_and_bearer_requests_use_distinct_authentication() {
    assert_eq!(
        cookie_headers("WorkosCursorSessionToken=secret"),
        [
            ("Cookie", "WorkosCursorSessionToken=secret".into()),
            ("Origin", "https://cursor.com".into()),
            ("User-Agent", "quotaline".into()),
        ]
    );
    assert_eq!(
        bearer_headers("secret"),
        [
            ("Authorization", "Bearer secret".into()),
            ("Connect-Protocol-Version", "1".into()),
            ("User-Agent", "quotaline".into()),
        ]
    );
}
