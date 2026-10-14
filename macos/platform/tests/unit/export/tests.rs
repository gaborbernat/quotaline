use super::*;

#[test]
fn command_flags_require_a_directory_and_follow_command_order() {
    for (arguments, expected) in [
        (
            vec!["app", "--export-icon", "/icons"],
            Some((Command::Icons, "/icons".into())),
        ),
        (
            vec!["--export-menubar", "/bar"],
            Some((Command::MenuBar, "/bar".into())),
        ),
        (
            vec!["--export-popover", "/panel"],
            Some((Command::Popover, "/panel".into())),
        ),
        (
            vec!["--export-popover", "/panel", "--export-icon", "/icons"],
            Some((Command::Icons, "/icons".into())),
        ),
        (vec!["app", "--export-icon"], None),
        (vec!["app"], None),
        (vec![], None),
    ] {
        assert_eq!(
            parse_command(&arguments.into_iter().map(str::to_owned).collect::<Vec<_>>()),
            expected
        );
    }
}

#[test]
fn usage_json_takes_precedence_over_file_exports() {
    assert_eq!(
        parse_invocation(&["--usage-json".into(), "--export-icon".into(), "/icons".into()]),
        Some(Invocation::UsageJson)
    );
    assert_eq!(
        parse_invocation(&["--export-popover".into(), "/panel".into()]),
        Some(Invocation::Files {
            command: Command::Popover,
            directory: "/panel".into()
        })
    );
    assert_eq!(parse_invocation(&["--export-icon".into()]), None);
}

#[test]
fn failure_messages_name_the_export() {
    assert_eq!(Command::Icons.failure_message(), "icon export failed");
    assert_eq!(Command::MenuBar.failure_message(), "menu bar export failed");
    assert_eq!(Command::Popover.failure_message(), "popover export failed");
}
