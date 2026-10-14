use super::*;

#[test]
fn profile_requires_account_or_organization() {
    for (profile, valid) in [
        (json!({"account":{"email":"a@example.com"}}), true),
        (json!({"organization":{}}), true),
        (json!({"account":null}), false),
        (json!({"status":"ok"}), false),
        (json!([]), false),
    ] {
        assert_eq!(profile_valid(&profile), valid);
    }
}
