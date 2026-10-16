use super::*;

#[test]
fn endpoints_cover_usage_reset_credits_events_and_oauth() {
    assert_eq!(endpoint(USAGE_PATH), "https://chatgpt.com/backend-api/wham/usage");
    assert_eq!(
        endpoint(RESET_CREDITS_PATH),
        "https://chatgpt.com/backend-api/wham/rate-limit-reset-credits"
    );
    assert_eq!(
        endpoint(CREDIT_EVENTS_PATH),
        "https://chatgpt.com/backend-api/wham/usage/credit-usage-events"
    );
    assert_eq!(TOKEN_URL, "https://auth.openai.com/oauth/token");
}
