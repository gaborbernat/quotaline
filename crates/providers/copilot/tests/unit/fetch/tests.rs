//! Drives the shared Copilot request through a scripted transport.

use futures_executor::block_on as run;

use super::*;
use quotaline_http::test_transport::{Scripted, unavailable};

#[test]
fn user_document_is_read_with_the_token_headers() {
    let transport = Scripted::new(vec![Ok(br#"{"login":"ada"}"#.to_vec())]);
    let document = run(fetch_user(
        &transport,
        "https://api.github.com/copilot_internal/user",
        "secret",
    ))
    .unwrap();
    assert_eq!(document["login"], "ada");
    let sent = transport.sent.lock().unwrap();
    assert_eq!(
        (sent[0].operation.as_str(), &sent[0].headers[0]),
        ("copilot.user", &("Authorization".into(), "token secret".into()))
    );
}

#[test]
fn failures_and_unreadable_documents_are_errors() {
    for (reply, expected) in [
        (Err(unavailable()), unavailable()),
        (
            Ok(b"<html>".to_vec()),
            HttpError::Decoding("copilot.user: invalid payload".into()),
        ),
    ] {
        let transport = Scripted::new(vec![reply]);
        assert_eq!(
            run(fetch_user(&transport, "https://example.test", "secret")).unwrap_err(),
            expected
        );
    }
}
