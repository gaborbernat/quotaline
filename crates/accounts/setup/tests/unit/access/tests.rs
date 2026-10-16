use super::{ResourceAccess, first_missing_resource};

fn resource(group: Option<&str>, required: bool, granted: bool) -> ResourceAccess {
    ResourceAccess {
        alternatives_group: group.map(str::to_owned),
        required,
        granted,
    }
}

#[test]
fn selects_the_first_unmet_required_resource() {
    for (case, resources, expected) in [
        ("empty", vec![], None),
        ("standalone needed", vec![resource(None, true, false)], Some(0)),
        ("standalone granted", vec![resource(None, true, true)], None),
        ("optional ignored", vec![resource(None, false, false)], None),
        (
            "first alternative",
            vec![
                resource(Some("cursor"), true, false),
                resource(Some("cursor"), true, false),
            ],
            Some(0),
        ),
        (
            "alternative granted",
            vec![
                resource(Some("cursor"), true, false),
                resource(Some("cursor"), true, true),
            ],
            None,
        ),
        (
            "unrelated group",
            vec![
                resource(Some("cursor"), true, true),
                resource(Some("copilot"), true, false),
            ],
            Some(1),
        ),
        (
            "optional grant does not satisfy group",
            vec![
                resource(Some("cursor"), false, true),
                resource(Some("cursor"), true, false),
            ],
            Some(1),
        ),
    ] {
        assert_eq!(first_missing_resource(&resources), expected, "{case}");
    }
}
