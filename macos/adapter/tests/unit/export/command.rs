use super::*;

#[test]
fn export_bridge_maps_file_and_usage_commands() {
    assert_eq!(
        parse_export_command(vec!["--export-icon".into(), "/icons".into()]),
        Some(ParsedExportCommand {
            command: ExportCommandRecord::Icons,
            directory: "/icons".into(),
        })
    );
    assert_eq!(
        parse_export_invocation(vec!["--usage-json".into()]),
        Some(ParsedExportInvocation::UsageJson)
    );
    assert_eq!(
        parse_export_invocation(vec!["--export-menubar".into(), "/bar".into()]),
        Some(ParsedExportInvocation::Files {
            command: ExportCommandRecord::MenuBar,
            directory: "/bar".into(),
        })
    );
    assert_eq!(parse_export_command(vec![]), None);
    assert_eq!(parse_export_invocation(vec![]), None);
}

#[test]
fn export_bridge_preserves_failure_messages() {
    for (command, message) in [
        (Some(ExportCommandRecord::Icons), "icon export failed"),
        (Some(ExportCommandRecord::MenuBar), "menu bar export failed"),
        (Some(ExportCommandRecord::Popover), "popover export failed"),
        (None, "usage export failed"),
    ] {
        assert_eq!(export_failure_message(command), message);
    }
}

#[test]
fn export_command_record_maps_all_variants() {
    for (record, command) in [
        (ExportCommandRecord::Icons, Command::Icons),
        (ExportCommandRecord::MenuBar, Command::MenuBar),
        (ExportCommandRecord::Popover, Command::Popover),
    ] {
        assert_eq!(ExportCommandRecord::from(command), record);
        assert_eq!(Command::from(record), command);
    }
}
