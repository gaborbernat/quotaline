//! Masks account fields and recognizes a personal workspace.

const HIDDEN_EMAIL: &str = "account";
const HIDDEN_ORGANIZATION: &str = "workspace";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentityReplacement {
    pub value: String,
    pub replacement: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentityPresentation {
    pub email: Option<String>,
    pub organization: Option<String>,
    pub show_organization: bool,
    pub replacements: Vec<IdentityReplacement>,
}

pub fn identity_presentation(email: Option<&str>, organization: Option<&str>, hide: bool) -> IdentityPresentation {
    let personal_organization = organization
        .zip(email)
        .is_some_and(|(organization, email)| organization.strip_suffix("'s Organization") == Some(email));
    let shown_email = email.map(|value| if hide { HIDDEN_EMAIL } else { value }.to_owned());
    let shown_organization = organization.map(|value| if hide { HIDDEN_ORGANIZATION } else { value }.to_owned());
    IdentityPresentation {
        email: shown_email,
        organization: shown_organization,
        show_organization: organization.is_some() && !personal_organization,
        replacements: email
            .map(|value| IdentityReplacement {
                value: value.into(),
                replacement: HIDDEN_EMAIL.into(),
            })
            .into_iter()
            .chain(organization.map(|value| IdentityReplacement {
                value: value.into(),
                replacement: HIDDEN_ORGANIZATION.into(),
            }))
            .collect(),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/usage/identity/tests.rs"]
mod tests;
