use super::*;

#[test]
fn visible_identity_keeps_account_fields_and_privacy_replacements() {
    assert_eq!(
        identity_presentation(Some("dev@example.com"), Some("Acme"), false),
        IdentityPresentation {
            email: Some("dev@example.com".into()),
            organization: Some("Acme".into()),
            show_organization: true,
            replacements: vec![
                IdentityReplacement {
                    value: "dev@example.com".into(),
                    replacement: "account".into()
                },
                IdentityReplacement {
                    value: "Acme".into(),
                    replacement: "workspace".into()
                },
            ],
        }
    );
}

#[test]
fn hidden_identity_masks_account_and_workspace() {
    let identity = identity_presentation(Some("dev@example.com"), Some("Acme"), true);
    assert_eq!(identity.email.as_deref(), Some("account"));
    assert_eq!(identity.organization.as_deref(), Some("workspace"));
    assert!(identity.show_organization);
}

#[test]
fn personal_workspace_is_suppressed_even_when_masked() {
    for hide in [false, true] {
        let identity = identity_presentation(Some("dev@example.com"), Some("dev@example.com's Organization"), hide);
        assert!(!identity.show_organization);
        assert_eq!(identity.replacements.len(), 2);
    }
}

#[test]
fn missing_and_standalone_workspace_keep_available_fields() {
    assert_eq!(
        identity_presentation(None, None, true),
        IdentityPresentation {
            email: None,
            organization: None,
            show_organization: false,
            replacements: vec![],
        }
    );
    let workspace = identity_presentation(None, Some("Acme"), true);
    assert_eq!(workspace.email, None);
    assert_eq!(workspace.organization.as_deref(), Some("workspace"));
    assert!(workspace.show_organization);
}
