//! Selects the most recent credential for the first available Claude account.

use super::jwt_claims;

pub struct ClaudeCredentialCandidate<'a> {
    pub access_token: &'a str,
    pub refresh_token: Option<&'a str>,
    pub expires_at: Option<f64>,
}

pub fn select_claude_credential(candidates: &[ClaudeCredentialCandidate<'_>]) -> Option<usize> {
    if candidates.is_empty() {
        return None;
    }
    let mut selected = 0;
    for (index, candidate) in candidates.iter().enumerate().skip(1) {
        let current = &candidates[selected];
        let current_subject =
            jwt_claims(current.access_token).and_then(|claims| claims["sub"].as_str().map(str::to_owned));
        let candidate_subject =
            jwt_claims(candidate.access_token).and_then(|claims| claims["sub"].as_str().map(str::to_owned));
        let same_account = current_subject.is_some() && current_subject == candidate_subject
            || current.refresh_token.is_some() && current.refresh_token == candidate.refresh_token
            || current.access_token == candidate.access_token;
        if same_account && candidate.expires_at.unwrap_or(f64::INFINITY) > current.expires_at.unwrap_or(f64::INFINITY) {
            selected = index;
        }
    }
    Some(selected)
}

#[cfg(test)]
#[path = "../tests/unit/selection/tests.rs"]
mod tests;
