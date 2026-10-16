use super::*;

#[test]
fn cards_require_an_enabled_provider_with_usage_evidence() {
    let cases = [
        ("empty", CardFacts::default(), false),
        (
            "enabled_empty",
            CardFacts {
                enabled: true,
                ..CardFacts::default()
            },
            false,
        ),
        (
            "disabled_snapshot",
            CardFacts {
                snapshot: true,
                ..CardFacts::default()
            },
            false,
        ),
        (
            "snapshot",
            CardFacts {
                enabled: true,
                snapshot: true,
                ..CardFacts::default()
            },
            true,
        ),
        (
            "analytics",
            CardFacts {
                enabled: true,
                analytics: true,
                ..CardFacts::default()
            },
            true,
        ),
        (
            "credential",
            CardFacts {
                enabled: true,
                credential_usable: true,
                ..CardFacts::default()
            },
            true,
        ),
        (
            "supplemental",
            CardFacts {
                enabled: true,
                supplemental: true,
                ..CardFacts::default()
            },
            true,
        ),
    ];
    for (case, facts, expected) in cases {
        assert_eq!(card_visible(facts), expected, "{case}");
    }
}

#[test]
fn provider_card_uses_snapshot_credentials_and_supporting_data() {
    let mut provider = Provider::empty("claude", "sign-in");
    assert!(!provider_card_visible(&provider, true, false));
    assert!(provider_card_visible(&provider, true, true));
    provider.credential_usable = true;
    assert!(provider_card_visible(&provider, true, false));
    provider.credential_usable = false;
    provider.source = "local credential".into();
    assert!(provider_card_visible(&provider, true, false));
    assert!(!provider_card_visible(&provider, false, true));
    provider.source.clear();
    provider.updated = Some(1);
    assert!(!provider_card_visible(&provider, true, false));
    provider
        .windows
        .push(crate::Window::new("session", "Session", 25.0, None, "session"));
    assert!(provider_card_visible(&provider, true, false));
}

#[test]
fn each_provider_supplement_can_reveal_its_card() {
    let cases = [
        ("source", serde_json::json!("Local credential")),
        (
            "spend",
            serde_json::json!({"enabled":true,"used":2.0,"limit":10.0,"percent":20.0,"currency":"USD","reset":null,
                "limit_reached":false}),
        ),
        ("credit_balance", serde_json::json!(1.0)),
        (
            "usage_credits",
            serde_json::json!({"unlimited":false,"has_credits":true,"overage_limit_reached":false,
                "local_messages":null,"cloud_messages":null}),
        ),
        (
            "reset_credits",
            serde_json::json!({"available":1,"applicable":1,"total_earned":null,"purchase_eligible":false,
                "details":[]}),
        ),
        (
            "cloud_credits",
            serde_json::json!({"limit":10.0,"remaining":4.0,"expires_at":null,"locked_reason":null}),
        ),
        ("details", serde_json::json!([{"title":"Tier","value":"Pro"}])),
        ("notices", serde_json::json!([{"kind":"promotion","text":"Available"}])),
        ("warnings", serde_json::json!(["Check connection"])),
    ];
    for (field, value) in cases {
        let mut document = serde_json::to_value(Provider::empty("claude", "sign-in")).unwrap();
        document[field] = value;
        let provider = serde_json::from_value(document).unwrap();
        assert!(provider_card_visible(&provider, true, false), "{field}");
    }
}
