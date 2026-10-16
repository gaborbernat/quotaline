use super::*;

#[test]
fn cloud_credit_availability_crosses_swift_boundary() {
    let projection = usage_cloud_credit_projection(
        CloudCreditFactsInput {
            limit: 250.0,
            remaining: 83.0,
            expires_at: Some(200),
            locked: false,
        },
        100,
    );
    assert_eq!(projection.status, CloudCreditStatus::Active);
    assert_eq!(projection.expiry_epoch, Some(200));
    assert!((projection.used_percent - 66.8).abs() < 1e-9);

    let locked = usage_cloud_credit_projection(
        CloudCreditFactsInput {
            locked: true,
            ..CloudCreditFactsInput {
                limit: 250.0,
                remaining: 83.0,
                expires_at: Some(200),
                locked: false,
            }
        },
        100,
    );
    assert_eq!(locked.status, CloudCreditStatus::Locked);
    assert_eq!(locked.expiry_epoch, None);

    let expired = usage_cloud_credit_projection(
        CloudCreditFactsInput {
            limit: 250.0,
            remaining: 83.0,
            expires_at: Some(100),
            locked: false,
        },
        100,
    );
    assert_eq!(expired.status, CloudCreditStatus::Expired);
    assert_eq!(expired.expiry_epoch, None);
}
