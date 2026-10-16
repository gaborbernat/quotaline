use super::*;

#[test]
fn ffi_identity_presentation_maps_privacy_facts() {
    assert_eq!(
        usage_identity_presentation(Some("dev@example.com".into()), Some("Acme".into()), true),
        IdentityPresentation {
            email: Some("account".into()),
            organization: Some("workspace".into()),
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
fn ffi_identity_presentation_handles_missing_fields() {
    assert_eq!(
        usage_identity_presentation(None, None, false),
        IdentityPresentation {
            email: None,
            organization: None,
            show_organization: false,
            replacements: vec![],
        }
    );
}
