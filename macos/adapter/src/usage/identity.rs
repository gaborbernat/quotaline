//! Exposes shared account masking decisions to Swift.

use quotaline_quota::usage;
use quotaline_quota::usage::IdentityPresentation as CorePresentation;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct IdentityReplacement {
    pub value: String,
    pub replacement: String,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct IdentityPresentation {
    pub email: Option<String>,
    pub organization: Option<String>,
    pub show_organization: bool,
    pub replacements: Vec<IdentityReplacement>,
}

impl From<CorePresentation> for IdentityPresentation {
    fn from(value: CorePresentation) -> Self {
        Self {
            email: value.email,
            organization: value.organization,
            show_organization: value.show_organization,
            replacements: value
                .replacements
                .into_iter()
                .map(|replacement| IdentityReplacement {
                    value: replacement.value,
                    replacement: replacement.replacement,
                })
                .collect(),
        }
    }
}

#[uniffi::export]
pub fn usage_identity_presentation(
    email: Option<String>,
    organization: Option<String>,
    hide: bool,
) -> IdentityPresentation {
    usage::identity_presentation(email.as_deref(), organization.as_deref(), hide).into()
}

#[cfg(test)]
#[path = "../../tests/unit/usage/identity/tests.rs"]
mod tests;
