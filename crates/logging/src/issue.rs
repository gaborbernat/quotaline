//! Builds bounded issue drafts from redacted diagnostics.

use super::redact;
use url::Url;

pub const MAX_ISSUE_URL_LENGTH: usize = 8000;
pub const REPOSITORY_URL: &str = "https://github.com/quotaline/quotaline";

#[derive(Debug, thiserror::Error)]
pub enum IssueUrlError {
    #[error("Invalid issue repository: {0}")]
    Repository(#[from] url::ParseError),
    #[error("Issue URL exceeds its length limit")]
    TooLong,
}

pub fn url(
    repository: &str,
    title: &str,
    report: &str,
    home: Option<&str>,
    maximum_length: usize,
) -> Result<String, IssueUrlError> {
    let mut base = Url::parse(repository)?;
    base.path_segments_mut()
        .map_err(|_| IssueUrlError::TooLong)?
        .extend(["issues", "new"]);
    let safe = redact(report, home);
    let lines = safe.split('\n').collect::<Vec<_>>();
    let candidate = |count| {
        let mut draft = base.clone();
        draft
            .query_pairs_mut()
            .append_pair("title", title)
            .append_pair("body", &format!("```\n{}\n```", lines[..count].join("\n")));
        draft.to_string()
    };
    if candidate(0).len() > maximum_length {
        return Err(IssueUrlError::TooLong);
    }
    let (mut low, mut high) = (0, lines.len());
    while low < high {
        let middle = (low + high).div_ceil(2);
        if candidate(middle).len() <= maximum_length {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    Ok(candidate(low))
}

#[cfg(test)]
#[path = "../tests/unit/issue/tests.rs"]
mod tests;
