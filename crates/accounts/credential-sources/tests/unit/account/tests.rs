//! Checks descriptions for single and selected accounts.

use super::*;

#[test]
fn codex_label_prefers_the_account_email() {
    assert_eq!(
        codex_account_label(
            Some("work@example.com"),
            Path::new("/home/user/.codex"),
            Path::new("/home/user")
        ),
        "work@example.com"
    );
}

#[test]
fn codex_label_abbreviates_a_home_directory() {
    assert_eq!(
        codex_account_label(None, Path::new("/home/user"), Path::new("/home/user")),
        "~"
    );
    assert_eq!(
        codex_account_label(Some(""), Path::new("/home/user/.codex"), Path::new("/home/user")),
        "~/.codex"
    );
}

#[test]
fn codex_label_preserves_a_path_outside_the_home_directory() {
    assert_eq!(
        codex_account_label(None, Path::new("/opt/codex"), Path::new("/home/user")),
        "/opt/codex"
    );
}
