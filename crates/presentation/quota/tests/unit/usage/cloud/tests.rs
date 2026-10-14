use super::*;

fn facts() -> CloudCreditFacts {
    CloudCreditFacts {
        limit: 250.0,
        remaining: 83.0,
        expires_at: Some(200),
        locked: false,
    }
}

#[test]
fn active_cloud_credit_projects_future_expiry_and_meter() {
    let projection = cloud_credit_projection(facts(), 100);
    assert_eq!(projection.status, CloudCreditStatus::Active);
    assert_eq!(projection.expiry_epoch, Some(200));
    assert!((projection.used_percent - 66.8).abs() < 1e-9);
}

#[test]
fn locked_cloud_credit_hides_expiry() {
    let projection = cloud_credit_projection(
        CloudCreditFacts {
            locked: true,
            ..facts()
        },
        100,
    );
    assert_eq!(projection.status, CloudCreditStatus::Locked);
    assert_eq!(projection.expiry_epoch, None);
}

#[test]
fn expired_cloud_credit_hides_expiry() {
    let projection = cloud_credit_projection(facts(), 200);
    assert_eq!(projection.status, CloudCreditStatus::Expired);
    assert_eq!(projection.expiry_epoch, None);
}

#[test]
fn cloud_credit_without_expiry_has_no_expiry_epoch() {
    let projection = cloud_credit_projection(
        CloudCreditFacts {
            expires_at: None,
            ..facts()
        },
        100,
    );
    assert_eq!(projection.status, CloudCreditStatus::Active);
    assert_eq!(projection.expiry_epoch, None);
}

#[test]
fn cloud_credit_meter_clamps_invalid_cached_amounts() {
    assert_eq!(
        cloud_credit_projection(
            CloudCreditFacts {
                remaining: -10.0,
                ..facts()
            },
            100
        )
        .used_percent,
        100.0
    );
    assert_eq!(
        cloud_credit_projection(
            CloudCreditFacts {
                remaining: 300.0,
                ..facts()
            },
            100
        )
        .used_percent,
        0.0
    );
    assert_eq!(
        cloud_credit_projection(CloudCreditFacts { limit: 0.0, ..facts() }, 100).used_percent,
        0.0
    );
}

#[test]
fn cloud_credit_facts_parse_provider_expiry() {
    let credits = CloudCredits {
        limit: 250.0,
        remaining: 83.0,
        expires_at: Some("2026-11-04T23:59:00Z".into()),
        locked_reason: Some("provider_locked".into()),
    };
    let facts = CloudCreditFacts::from_credits(&credits);
    assert_eq!(facts.limit, 250.0);
    assert_eq!(facts.remaining, 83.0);
    assert_eq!(facts.expires_at, Some(1_793_836_740));
    assert!(facts.locked);

    let absent = CloudCredits {
        expires_at: None,
        locked_reason: None,
        ..credits
    };
    let facts = CloudCreditFacts::from_credits(&absent);
    assert_eq!(facts.expires_at, None);
    assert!(!facts.locked);
}
