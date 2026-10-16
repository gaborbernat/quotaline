//! Describes the selected account when several credential sources are present.

use std::path::Path;

pub fn codex_account_label(email: Option<&str>, account_home: &Path, user_home: &Path) -> String {
    if let Some(email) = email.filter(|email| !email.is_empty()) {
        return email.into();
    }
    match account_home.strip_prefix(user_home) {
        Ok(relative) if relative.as_os_str().is_empty() => "~".into(),
        Ok(relative) => format!("~/{}", relative.display()),
        Err(_) => account_home.display().to_string(),
    }
}

#[cfg(test)]
#[path = "../tests/unit/account/tests.rs"]
mod tests;
