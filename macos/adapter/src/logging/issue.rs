//! Exposes shared issue-draft URLs to Swift.

use quotaline_logging::issue;

#[uniffi::export]
pub fn diagnostic_issue_url_limit() -> u32 {
    issue::MAX_ISSUE_URL_LENGTH as u32
}

#[uniffi::export]
pub fn repository_url() -> String {
    issue::REPOSITORY_URL.into()
}

#[uniffi::export]
pub fn diagnostic_issue_url(repository: String, title: String, report: String, home: String) -> Option<String> {
    issue::url(&repository, &title, &report, Some(&home), issue::MAX_ISSUE_URL_LENGTH).ok()
}

#[cfg(test)]
#[path = "../../tests/unit/logging/issue/tests.rs"]
mod tests;
