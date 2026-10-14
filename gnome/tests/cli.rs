//! Checks command-line snapshot and export behavior.

use std::process::Command;

#[test]
fn help_exits_before_connecting_to_the_session_bus() {
    let output = Command::new(env!("CARGO_BIN_EXE_quotaline-gnome"))
        .arg("--help")
        .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/dev/null")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        output.stdout,
        b"Usage: quotaline-gnome [--demo | --usage-json | --help]\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn unsupported_argument_exits_before_connecting_to_the_session_bus() {
    let output = Command::new(env!("CARGO_BIN_EXE_quotaline-gnome"))
        .arg("--invalid")
        .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/dev/null")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        output.stderr,
        b"Unsupported argument. Run quotaline-gnome --help for usage.\n"
    );
}

#[test]
fn combined_arguments_do_not_start_the_service() {
    let output = Command::new(env!("CARGO_BIN_EXE_quotaline-gnome"))
        .args(["--usage-json", "--help"])
        .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/dev/null")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}
