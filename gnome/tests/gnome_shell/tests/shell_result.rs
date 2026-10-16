use super::{extension_error, shell_script_succeeded};

#[test]
fn successful_shell_exit_succeeds_without_a_marker() {
    assert!(shell_script_succeeded(true, "other.js", false, ""));
}

#[test]
fn completed_walkthrough_accepts_perf_helper_teardown_failure() {
    assert!(shell_script_succeeded(
        false,
        "extension/tests/integration/shell-smoke.js",
        true,
        "Failed to exit helper: unavailable\nGNOME Shell exited with exit status: 1"
    ));
}

#[test]
fn incomplete_walkthrough_rejects_perf_helper_teardown_failure() {
    assert!(!shell_script_succeeded(
        false,
        "extension/tests/integration/shell-smoke.js",
        false,
        "Failed to exit helper: unavailable\nGNOME Shell exited with exit status: 1"
    ));
}

#[test]
fn unrelated_script_rejects_perf_helper_teardown_failure() {
    assert!(!shell_script_succeeded(
        false,
        "other.js",
        true,
        "Failed to exit helper: unavailable\nGNOME Shell exited with exit status: 1"
    ));
}

#[test]
fn completed_walkthrough_rejects_missing_helper_failure() {
    assert!(!shell_script_succeeded(
        false,
        "extension/tests/integration/shell-smoke.js",
        true,
        "GNOME Shell exited with exit status: 1"
    ));
}

#[test]
fn completed_walkthrough_rejects_missing_exit_status() {
    assert!(!shell_script_succeeded(
        false,
        "extension/tests/integration/shell-smoke.js",
        true,
        "Failed to exit helper: unavailable"
    ));
}

#[test]
fn shell_teardown_errors_outside_the_extension_are_ignored() {
    let logs =
        "JS ERROR: TypeError: this._settings is null\n_sync@resource:///org/gnome/shell/ui/dateMenu.js:796:30\n\nnext";
    assert!(!extension_error(logs));
    assert!(!extension_error("no errors"));
}

#[test]
fn errors_in_the_extension_or_a_test_script_fail_the_run() {
    for frame in [
        "file:///tmp/data/gnome-shell/extensions/quotaline@bernat.tech/src/panel/service.js:5:1",
        "file:///work/gnome/extension/tests/integration/shell-smoke.js:9:2",
    ] {
        assert!(
            extension_error(&format!("JS ERROR: TypeError: boom\n{frame}\n\n")),
            "{frame}"
        );
    }
}
