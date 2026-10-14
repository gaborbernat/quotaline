use super::{LocalRole, candidate, role};

#[test]
fn local_candidate_matches_only_supported_executables() {
    for (path, expected) in [
        ("/opt/bin/agy", true),
        ("/opt/bin/antigravity-cli", true),
        ("/opt/bin/language_server_macos_arm", true),
        ("/opt/bin/language-server", true),
        ("/opt/bin/other", false),
        ("/opt/bin/agy-helper", false),
        ("", false),
    ] {
        assert_eq!(candidate(path), expected, "{path}");
    }
}

#[test]
fn local_role_accepts_cli_and_both_server_flag_forms() {
    for (path, arguments, expected) in [
        ("/opt/bin/agy", vec![], Some(LocalRole::Cli)),
        ("antigravity-cli", vec!["quota"], Some(LocalRole::Cli)),
        (
            "language_server_macos_arm",
            vec!["--csrf_token", "secret", "--app_data_dir", "antigravity"],
            Some(LocalRole::LanguageServer {
                csrf_token: "secret".into(),
            }),
        ),
        (
            "/Applications/Antigravity/language-server",
            vec!["--app_data_dir=/Users/user/ANTIGRAVITY-IDE/", "--csrf_token=other"],
            Some(LocalRole::LanguageServer {
                csrf_token: "other".into(),
            }),
        ),
    ] {
        assert_eq!(role(path, &arguments), expected, "{path}: {arguments:?}");
    }
}

#[test]
fn local_role_rejects_other_apps_and_incomplete_flags() {
    for (path, arguments) in [
        ("/bin/zsh", vec![]),
        ("", vec![]),
        ("language_server", vec!["--csrf_token", "secret"]),
        (
            "language_server",
            vec!["--app_data_dir", "windsurf", "--csrf_token", "secret"],
        ),
        ("language_server", vec!["--app_data_dir", "antigravity", "--csrf_token"]),
        ("language_server", vec!["--app_data_dir"]),
    ] {
        assert_eq!(role(path, &arguments), None, "{path}: {arguments:?}");
    }
}
