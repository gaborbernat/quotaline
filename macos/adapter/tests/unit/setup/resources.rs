use super::*;

#[test]
fn bridge_preserves_resource_descriptors() {
    let resources = sandbox_resource_descriptors();
    assert_eq!(resources.len(), 7);
    assert_eq!(resources[0].need, SandboxResourceNeed::Optional);
    assert_eq!(resources[1].need, SandboxResourceNeed::Required);
    assert_eq!(resources[3].need, SandboxResourceNeed::OneOf { group: "cursor".into() });
    assert_eq!(
        resources[6].need,
        SandboxResourceNeed::OneOf {
            group: "copilot".into()
        }
    );
    assert!(resources.iter().all(|resource| !resource.explanation.is_empty()));
}

#[test]
fn bridge_resolves_resource_path() {
    assert_eq!(
        configured_sandbox_resource_path(
            "copilot.config".into(),
            HashMap::from([("XDG_CONFIG_HOME".into(), "/xdg".into())]),
            "/home/user".into(),
        ),
        "/xdg/github-copilot"
    );
}

#[test]
fn bridge_selects_grants_from_environment() {
    assert_eq!(sandbox_resources_requiring_access(HashMap::new()).len(), 7);
    assert_eq!(
        sandbox_resources_requiring_access(HashMap::from([("COPILOT_GITHUB_TOKEN".into(), "token".into())])),
        [
            "claude.home",
            "codex.home",
            "gemini.home",
            "cursor.support",
            "cursor.home"
        ]
    );
    assert!(!provider_needs_sandbox_resources(
        "copilot".into(),
        Some("copilot.environment".into())
    ));
    assert!(provider_needs_sandbox_resources("cursor".into(), None));
}

#[test]
fn bridge_resolves_provider_settings_and_missing_grants() {
    assert_eq!(
        configured_sandbox_providers(vec!["claude".into()], vec!["codex".into()], vec!["gemini.home".into()],),
        ["claude", "codex", "gemini"]
    );
    assert_eq!(
        missing_sandbox_resource_ids("cursor".into(), vec!["cursor.home".into()]),
        ["cursor.support"]
    );
}
