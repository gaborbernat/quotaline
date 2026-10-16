use super::{ResourceAccessRecord, first_missing_resource};

#[test]
fn bridge_selects_the_original_resource_index() {
    for (case, required, expected) in [("required", true, Some(1)), ("optional", false, None)] {
        assert_eq!(
            first_missing_resource(vec![
                ResourceAccessRecord {
                    alternatives_group: None,
                    required: false,
                    granted: false,
                },
                ResourceAccessRecord {
                    alternatives_group: Some("cursor".into()),
                    required,
                    granted: false,
                },
            ]),
            expected,
            "{case}"
        );
    }
}
