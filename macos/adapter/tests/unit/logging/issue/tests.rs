use super::*;

#[test]
fn bridge_builds_bounded_issue_draft() {
    assert_eq!(repository_url(), "https://github.com/quotaline/quotaline");
    let draft = diagnostic_issue_url(
        "https://github.com/quotaline/quotaline".into(),
        "Issue report".into(),
        "Bearer secret".into(),
        String::new(),
    )
    .unwrap();
    assert!(draft.len() <= diagnostic_issue_url_limit() as usize);
    assert!(draft.contains("Bearer+%3Credacted%3E"));
}

#[test]
fn bridge_rejects_invalid_repository() {
    assert_eq!(
        diagnostic_issue_url("invalid".into(), "Issue".into(), "body".into(), String::new()),
        None
    );
}
