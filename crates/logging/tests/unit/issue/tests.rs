use super::*;

const REPOSITORY: &str = "https://github.com/quotaline/quotaline";

fn body(draft: &str) -> String {
    Url::parse(draft)
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == "body")
        .unwrap()
        .1
        .into_owned()
}

#[test]
fn includes_all_lines_when_they_fit() {
    let draft = url(REPOSITORY, "Issue report", "first\nsecond", None, 8000).unwrap();
    assert!(draft.starts_with("https://github.com/quotaline/quotaline/issues/new?"));
    assert_eq!(body(&draft), "```\nfirst\nsecond\n```");
    assert_eq!(
        Url::parse(&draft).unwrap().query_pairs().next().unwrap().1,
        "Issue report"
    );
}

#[test]
fn removes_complete_lines_to_fit_the_limit() {
    let report = (0..400)
        .map(|index| format!("line {index} {}", "x".repeat(40)))
        .collect::<Vec<_>>()
        .join("\n");
    let draft = url(REPOSITORY, "Issue report", &report, None, 8000).unwrap();
    assert!(draft.len() <= 8000);
    assert!(body(&draft).contains("line 0"));
    assert!(!body(&draft).contains("line 399"));
}

#[test]
fn excludes_a_single_line_that_exceeds_the_limit() {
    let draft = url(REPOSITORY, "Issue report", &"x".repeat(9000), None, 8000).unwrap();
    assert_eq!(body(&draft), "```\n\n```");
}

#[test]
fn redacts_report_secrets_before_encoding() {
    let draft = url(
        REPOSITORY,
        "Issue report",
        "Bearer secret /home/test/private",
        Some("/home/test"),
        8000,
    )
    .unwrap();
    assert_eq!(body(&draft), "```\nBearer <redacted> ~/private\n```");
}

#[test]
fn rejects_invalid_repositories_and_limits() {
    assert!(matches!(
        url("invalid", "Issue", "body", None, 8000),
        Err(IssueUrlError::Repository(_))
    ));
    assert!(matches!(
        url("mailto:test@example.com", "Issue", "body", None, 8000),
        Err(IssueUrlError::TooLong)
    ));
    assert!(matches!(
        url(REPOSITORY, "Issue", "body", None, 1),
        Err(IssueUrlError::TooLong)
    ));
}
