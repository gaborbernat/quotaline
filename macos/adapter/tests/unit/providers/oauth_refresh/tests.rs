use super::*;

use crate::mock_http::MockTransport;

fn input(provider: RefreshProvider) -> OAuthRefreshInput {
    OAuthRefreshInput {
        provider,
        credential: br#"{"accessToken":"old","refreshToken":"refresh"}"#.to_vec(),
        document: match provider {
            RefreshProvider::Claude => br#"{"claudeAiOauth":{"accessToken":"old"}}"#.to_vec(),
            RefreshProvider::Codex => br#"{"tokens":{"access_token":"old"}}"#.to_vec(),
            RefreshProvider::Gemini => br#"{"access_token":"old"}"#.to_vec(),
            RefreshProvider::Antigravity => vec![],
        },
        access_token: "old".into(),
        refresh_token: Some("refresh".into()),
        client: Some(OAuthClientRecord {
            id: "client-id".into(),
            secret: "client-secret".into(),
        }),
        now_millis: 1000,
        refreshed_at: "now".into(),
    }
}

fn json_transport(provider: RefreshProvider, response: Result<Vec<u8>, HttpErrorRecord>) -> Arc<MockTransport> {
    let mut transport = MockTransport::new();
    transport
        .expect_start_post_json()
        .once()
        .return_once(move |request, completion| {
            assert_eq!(request.timeout_millis, None);
            if provider == RefreshProvider::Claude {
                assert_eq!(request.url, claude::request::TOKEN_URL);
                assert_eq!(request.headers["User-Agent"], CLAUDE_USER_AGENT);
                assert_eq!(request.operation, "claude.refresh");
            } else {
                assert_eq!(request.url, codex::request::TOKEN_URL);
                assert!(request.headers.is_empty());
                assert_eq!(request.operation, "codex.refresh");
            }
            assert_eq!(
                serde_json::from_slice::<Value>(&request.body).unwrap()["refresh_token"],
                "refresh"
            );
            match response {
                Ok(data) => completion.succeed(data),
                Err(error) => completion.fail(error),
            }
        });
    Arc::new(transport)
}

fn form_transport(provider: RefreshProvider, response: Result<Vec<u8>, HttpErrorRecord>) -> Arc<MockTransport> {
    let mut transport = MockTransport::new();
    transport
        .expect_start_post_form()
        .once()
        .return_once(move |request, completion| {
            assert_eq!(request.url, gemini::request::TOKEN_URL);
            assert!(request.headers.is_empty());
            assert_eq!(request.fields["client_id"], "client-id");
            assert_eq!(request.fields["client_secret"], "client-secret");
            assert_eq!(request.fields["refresh_token"], "refresh");
            assert_eq!(
                request.operation,
                if provider == RefreshProvider::Gemini {
                    "gemini.refresh"
                } else {
                    "antigravity.refresh"
                }
            );
            match response {
                Ok(data) => completion.succeed(data),
                Err(error) => completion.fail(error),
            }
        });
    Arc::new(transport)
}

fn fetch(
    session: &OAuthRefreshSession,
    input: OAuthRefreshInput,
    transport: Arc<MockTransport>,
) -> OAuthRefreshOutcome {
    futures_executor::block_on(session.refresh(input, transport))
}

#[test]
fn completes_refresh_for_each_provider() {
    for provider in [
        RefreshProvider::Claude,
        RefreshProvider::Codex,
        RefreshProvider::Gemini,
        RefreshProvider::Antigravity,
    ] {
        let response = Ok(br#"{"access_token":"new","expires_in":60}"#.to_vec());
        let transport = match provider {
            RefreshProvider::Claude | RefreshProvider::Codex => json_transport(provider, response),
            _ => form_transport(provider, response),
        };
        let expected = match provider {
            RefreshProvider::Claude => OAuthRefreshOutcome::Credential {
                document: serde_json::to_vec(&serde_json::json!({
                    "claudeAiOauth":{"accessToken":"new","expiresAt":61000}
                }))
                .unwrap(),
            },
            RefreshProvider::Codex => OAuthRefreshOutcome::Credential {
                document: serde_json::to_vec(&serde_json::json!({
                    "tokens":{"access_token":"new"},"last_refresh":"now"
                }))
                .unwrap(),
            },
            RefreshProvider::Gemini => OAuthRefreshOutcome::Credential {
                document: serde_json::to_vec(&serde_json::json!({
                    "access_token":"new","expiry_date":61000
                }))
                .unwrap(),
            },
            RefreshProvider::Antigravity => OAuthRefreshOutcome::Antigravity {
                access_token: "new".into(),
                expires_at: 61.0,
            },
        };
        assert_eq!(fetch(&OAuthRefreshSession::new(), input(provider), transport), expected);
    }
}

#[test]
fn missing_refresh_token_and_oauth_client_skip_network() {
    let mut missing_token = input(RefreshProvider::Gemini);
    missing_token.refresh_token = None;
    assert_eq!(
        fetch(
            &OAuthRefreshSession::new(),
            missing_token,
            Arc::new(MockTransport::new())
        ),
        OAuthRefreshOutcome::MissingRefreshToken
    );
    for provider in [RefreshProvider::Gemini, RefreshProvider::Antigravity] {
        let mut missing_client = input(provider);
        missing_client.client = None;
        assert_eq!(
            fetch(
                &OAuthRefreshSession::new(),
                missing_client,
                Arc::new(MockTransport::new())
            ),
            OAuthRefreshOutcome::OAuthClientUnavailable
        );
    }
}

#[test]
fn invalid_stored_credential_skips_network() {
    let mut invalid = input(RefreshProvider::Claude);
    invalid.credential = b"invalid".to_vec();
    assert_eq!(
        fetch(&OAuthRefreshSession::new(), invalid, Arc::new(MockTransport::new())),
        OAuthRefreshOutcome::InvalidResponse {
            detail: "stored credential is not JSON".into()
        }
    );
}

#[test]
fn terminal_rejection_is_reused_until_credential_changes() {
    let session = OAuthRefreshSession::new();
    let rejected = HttpErrorRecord::Response {
        status: 400,
        body: r#"{"error":"invalid_grant"}"#.into(),
        retry_after: None,
    };
    assert_eq!(
        fetch(
            &session,
            input(RefreshProvider::Claude),
            json_transport(RefreshProvider::Claude, Err(rejected))
        ),
        OAuthRefreshOutcome::SignInRequired {
            code: "invalid_grant".into()
        }
    );
    assert_eq!(
        fetch(&session, input(RefreshProvider::Claude), Arc::new(MockTransport::new())),
        OAuthRefreshOutcome::SignInRequired {
            code: "invalid_grant".into()
        }
    );
    let mut changed = input(RefreshProvider::Claude);
    changed.credential = br#"{"accessToken":"different"}"#.to_vec();
    assert_eq!(
        fetch(
            &session,
            changed,
            json_transport(
                RefreshProvider::Claude,
                Ok(br#"{"access_token":"new","expires_in":60}"#.to_vec()),
            ),
        ),
        OAuthRefreshOutcome::Credential {
            document: serde_json::to_vec(&serde_json::json!({
                "claudeAiOauth":{"accessToken":"new","expiresAt":61000}
            }))
            .unwrap(),
        }
    );
}

#[test]
fn nonterminal_http_failures_reach_the_caller() {
    for error in [
        HttpErrorRecord::Network {
            detail: "offline".into(),
        },
        HttpErrorRecord::Response {
            status: 500,
            body: r#"{"error":"invalid_grant"}"#.into(),
            retry_after: None,
        },
        HttpErrorRecord::Response {
            status: 400,
            body: "not json".into(),
            retry_after: None,
        },
        HttpErrorRecord::Response {
            status: 400,
            body: r#"{"error":"other"}"#.into(),
            retry_after: None,
        },
    ] {
        assert_eq!(
            fetch(
                &OAuthRefreshSession::new(),
                input(RefreshProvider::Codex),
                json_transport(RefreshProvider::Codex, Err(error.clone())),
            ),
            OAuthRefreshOutcome::Http { error }
        );
    }
    let error = HttpErrorRecord::Network {
        detail: "offline".into(),
    };
    assert_eq!(
        fetch(
            &OAuthRefreshSession::new(),
            input(RefreshProvider::Gemini),
            form_transport(RefreshProvider::Gemini, Err(error.clone())),
        ),
        OAuthRefreshOutcome::Http { error }
    );
}

#[test]
fn refresh_completion_errors_preserve_provider_policy() {
    for (provider, response, expected) in [
        (
            RefreshProvider::Claude,
            b"invalid".to_vec(),
            OAuthRefreshOutcome::Http {
                error: HttpErrorRecord::Decoding {
                    detail: "claude.refresh: invalid payload".into(),
                },
            },
        ),
        (
            RefreshProvider::Claude,
            b"{}".to_vec(),
            OAuthRefreshOutcome::Http {
                error: HttpErrorRecord::Decoding {
                    detail: "claude.refresh: invalid payload".into(),
                },
            },
        ),
        (
            RefreshProvider::Codex,
            b"invalid".to_vec(),
            OAuthRefreshOutcome::Http {
                error: HttpErrorRecord::Decoding {
                    detail: "codex.refresh: invalid payload".into(),
                },
            },
        ),
        (
            RefreshProvider::Codex,
            b"{}".to_vec(),
            OAuthRefreshOutcome::InvalidResponse {
                detail: "refresh returned no access token".into(),
            },
        ),
        (
            RefreshProvider::Gemini,
            br#"{"access_token":"new","expires_in":0}"#.to_vec(),
            OAuthRefreshOutcome::Http {
                error: HttpErrorRecord::Decoding {
                    detail: "gemini.refresh: invalid payload".into(),
                },
            },
        ),
        (
            RefreshProvider::Antigravity,
            b"{}".to_vec(),
            OAuthRefreshOutcome::InvalidResponse {
                detail: "refresh returned no access token".into(),
            },
        ),
        (
            RefreshProvider::Antigravity,
            b"invalid".to_vec(),
            OAuthRefreshOutcome::Http {
                error: HttpErrorRecord::Decoding {
                    detail: "antigravity.refresh: invalid payload".into(),
                },
            },
        ),
    ] {
        let transport = match provider {
            RefreshProvider::Claude | RefreshProvider::Codex => json_transport(provider, Ok(response)),
            _ => form_transport(provider, Ok(response)),
        };
        assert_eq!(fetch(&OAuthRefreshSession::new(), input(provider), transport), expected);
    }
}

#[test]
fn invalid_document_reports_completion_failure() {
    for (provider, detail) in [
        (RefreshProvider::Claude, "Claude credentials cannot be updated"),
        (RefreshProvider::Codex, "Codex credentials cannot be updated"),
    ] {
        let mut invalid = input(provider);
        invalid.document = b"[]".to_vec();
        assert_eq!(
            fetch(
                &OAuthRefreshSession::new(),
                invalid,
                json_transport(provider, Ok(br#"{"access_token":"new","expires_in":60}"#.to_vec())),
            ),
            OAuthRefreshOutcome::InvalidResponse { detail: detail.into() }
        );
    }
}

#[test]
fn refresh_failure_wording_is_shared() {
    assert_eq!(
        refresh_failed_message("Claude".into(), refresh_rejected_message("invalid_grant".into())),
        quotaline_credentials::refresh_failed_message(
            "Claude",
            &quotaline_credentials::refresh_rejected_message("invalid_grant")
        )
    );
}
