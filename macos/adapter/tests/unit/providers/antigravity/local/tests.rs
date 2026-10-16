use super::*;
use crate::http::HttpErrorRecord;
use crate::mock_http::MockTransport;
use crate::mock_scanner::MockServerScanner;

fn server(port: u16) -> AntigravityLocalServer {
    AntigravityLocalServer {
        pid: i32::from(port),
        port,
        csrf_token: Some("csrf".into()),
    }
}

fn input() -> AntigravityLocalInput {
    AntigravityLocalInput {
        port: 4321,
        csrf_token: Some("csrf".into()),
    }
}

fn quota() -> Vec<u8> {
    br#"{"groups":[{"buckets":[{"bucketId":"gemini-5h","remainingFraction":0.25}]}]}"#.to_vec()
}

fn probe(transport: MockTransport) -> AntigravityLocalResult {
    futures_executor::block_on(antigravity_local_probe(input(), Arc::new(transport)))
}

#[test]
fn local_probe_retries_http_and_uses_the_matching_status_endpoint() {
    let mut transport = MockTransport::new();
    transport
        .expect_start_post_json()
        .once()
        .return_once(|request, completion| {
            assert!(request.url.starts_with("https://127.0.0.1:4321/"));
            assert_eq!(request.timeout_millis, Some(500));
            assert_eq!(request.headers.get("X-Codeium-Csrf-Token"), Some(&"csrf".into()));
            assert_eq!(request.body, br#"{"forceRefresh":true}"#);
            completion.fail(HttpErrorRecord::Network { detail: "TLS".into() });
        });
    transport
        .expect_start_post_json()
        .once()
        .return_once(|request, completion| {
            assert!(request.url.starts_with("http://127.0.0.1:4321/"));
            assert_eq!(request.operation, "antigravity.local-quota");
            completion.succeed(quota());
        });
    transport
        .expect_start_post_json()
        .once()
        .return_once(|request, completion| {
            assert!(request.url.starts_with("http://127.0.0.1:4321/"));
            assert_eq!(request.operation, "antigravity.local-status");
            assert_eq!(request.timeout_millis, Some(500));
            completion.succeed(br#"{"userStatus":{"userTier":{"name":"Ultra"},"email":"a@b"}}"#.to_vec());
        });
    let result = probe(transport);
    assert_eq!(
        result.debug_messages,
        ["antigravity: language server port=4321 scheme=https Network error: TLS"]
    );
    assert_eq!(
        result.snapshot,
        Some(Box::new(AntigravityLocalSnapshot {
            provider: AntigravityQuotaResponse::new(quota()).unwrap().provider(
                super::super::antigravity_identity(
                    br#"{"userStatus":{"userTier":{"name":"Ultra"},"email":"a@b"}}"#.to_vec()
                )
                .unwrap()
            ),
        }))
    );
}

#[test]
fn local_probe_uses_default_identity_when_status_fails() {
    for status in [
        Err(HttpErrorRecord::Response {
            status: 500,
            body: String::new(),
            retry_after: None,
        }),
        Ok(b"{".to_vec()),
    ] {
        let mut transport = MockTransport::new();
        transport
            .expect_start_post_json()
            .once()
            .return_once(|_, completion| completion.succeed(quota()));
        transport
            .expect_start_post_json()
            .once()
            .return_once(move |_, completion| match status {
                Ok(payload) => completion.succeed(payload),
                Err(error) => completion.fail(error),
            });
        let result = probe(transport);
        assert_eq!(
            result.snapshot.unwrap().provider.plan,
            super::super::antigravity_default_identity().plan_name
        );
        assert_eq!(result.debug_messages.len(), 1);
    }
}

#[test]
fn local_probe_retries_malformed_quota_on_the_second_scheme() {
    let mut transport = MockTransport::new();
    transport
        .expect_start_post_json()
        .once()
        .return_once(|_, completion| completion.succeed(b"{".to_vec()));
    transport
        .expect_start_post_json()
        .once()
        .return_once(|_, completion| completion.succeed(quota()));
    transport
        .expect_start_post_json()
        .once()
        .return_once(|_, completion| completion.succeed(b"{}".to_vec()));
    let result = probe(transport);
    assert_eq!(
        result.debug_messages,
        ["antigravity: invalid local quota response port=4321"]
    );
    assert_eq!(result.snapshot.unwrap().provider.windows[0].id, "gemini:session");
}

#[test]
fn local_probe_rejects_rpc_errors_without_retrying_the_same_server() {
    let mut transport = MockTransport::new();
    transport.expect_start_post_json().once().return_once(|_, completion| {
        completion.succeed(br#"{"code":13,"message":"internal"}"#.to_vec());
    });
    let result = probe(transport);
    assert_eq!(result.snapshot, None);
    assert_eq!(
        result.debug_messages,
        ["antigravity: local quota response rejected port=4321"]
    );
}

#[test]
fn local_probe_returns_no_snapshot_after_both_schemes_fail() {
    let mut transport = MockTransport::new();
    transport.expect_start_post_json().times(2).returning(|_, completion| {
        completion.fail(HttpErrorRecord::Network {
            detail: "offline".into(),
        });
    });
    let result = probe(transport);
    assert_eq!(result.snapshot, None);
    assert_eq!(result.debug_messages.len(), 2);
}

#[test]
fn local_session_reuses_recent_snapshot_and_preferred_server() {
    let mut scanner = MockServerScanner::new();
    scanner
        .expect_discover_servers()
        .once()
        .return_once(|| vec![server(4321)]);
    let mut transport = MockTransport::new();
    transport
        .expect_start_post_json()
        .times(4)
        .returning(|request, completion| {
            assert_eq!(url::Url::parse(&request.url).unwrap().port(), Some(4321));
            if request.operation == "antigravity.local-quota" {
                completion.succeed(quota());
            } else {
                completion.succeed(b"{}".to_vec());
            }
        });
    let session = AntigravityLocalSession::new();
    let scanner = Arc::new(scanner);
    let transport = Arc::new(transport);
    let first = futures_executor::block_on(session.fetch(100.0, scanner.clone(), transport.clone()));
    assert_eq!(first.fetched_at, Some(100.0));
    assert_eq!(
        first.snapshot.as_ref().unwrap().provider.windows[0].id,
        "gemini:session"
    );
    let cached = futures_executor::block_on(session.fetch(104.0, scanner.clone(), transport.clone()));
    assert_eq!(cached.fetched_at, Some(100.0));
    assert_eq!(cached.snapshot, first.snapshot);
    let expired = futures_executor::block_on(session.fetch(105.0, scanner, transport));
    assert_eq!(expired.fetched_at, Some(105.0));
}

#[test]
fn local_session_rediscovers_after_the_preferred_server_fails() {
    let mut scanner = MockServerScanner::new();
    scanner
        .expect_discover_servers()
        .times(2)
        .returning(|| vec![server(4321), server(5353)]);
    let mut transport = MockTransport::new();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = calls.clone();
    transport
        .expect_start_post_json()
        .times(6)
        .returning(move |request, completion| {
            let port = url::Url::parse(&request.url).unwrap().port();
            if request.operation == "antigravity.local-quota" && port == Some(4321) {
                if observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                    completion.succeed(quota());
                } else {
                    completion.fail(HttpErrorRecord::Network {
                        detail: "offline".into(),
                    });
                }
            } else if request.operation == "antigravity.local-status" {
                completion.succeed(b"{}".to_vec());
            } else {
                completion.succeed(quota());
            }
        });
    let session = AntigravityLocalSession::new();
    let scanner = Arc::new(scanner);
    let transport = Arc::new(transport);
    assert_eq!(
        futures_executor::block_on(session.fetch(100.0, scanner.clone(), transport.clone())).fetched_at,
        Some(100.0)
    );
    let second = futures_executor::block_on(session.fetch(106.0, scanner, transport));
    assert_eq!(second.fetched_at, Some(106.0));
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 3);
}

#[test]
fn local_session_tries_the_next_discovered_server() {
    let mut scanner = MockServerScanner::new();
    scanner
        .expect_discover_servers()
        .once()
        .return_once(|| vec![server(4321), server(5353)]);
    let mut transport = MockTransport::new();
    transport
        .expect_start_post_json()
        .times(4)
        .returning(|request, completion| {
            let port = url::Url::parse(&request.url).unwrap().port();
            if port == Some(4321) {
                completion.fail(HttpErrorRecord::Network {
                    detail: "offline".into(),
                });
            } else if request.operation == "antigravity.local-quota" {
                completion.succeed(quota());
            } else {
                completion.succeed(b"{}".to_vec());
            }
        });
    let result =
        futures_executor::block_on(AntigravityLocalSession::new().fetch(100.0, Arc::new(scanner), Arc::new(transport)));
    assert_eq!(result.fetched_at, Some(100.0));
    assert_eq!(result.debug_messages.len(), 2);
    assert_eq!(result.snapshot.unwrap().provider.windows[0].id, "gemini:session");
}

#[test]
fn local_session_reports_failure_after_all_discovered_servers_fail() {
    let mut scanner = MockServerScanner::new();
    scanner
        .expect_discover_servers()
        .once()
        .return_once(|| vec![server(4321), server(5353)]);
    let mut transport = MockTransport::new();
    transport.expect_start_post_json().times(4).returning(|_, completion| {
        completion.fail(HttpErrorRecord::Network {
            detail: "offline".into(),
        });
    });
    let result =
        futures_executor::block_on(AntigravityLocalSession::new().fetch(100.0, Arc::new(scanner), Arc::new(transport)));
    assert_eq!(result.snapshot, None);
    assert_eq!(result.fetched_at, None);
    assert_eq!(result.debug_messages.len(), 4);
}

#[test]
fn local_session_returns_none_when_no_server_exists() {
    let mut scanner = MockServerScanner::new();
    scanner.expect_discover_servers().once().return_once(Vec::new);
    let mut transport = MockTransport::new();
    transport.expect_start_post_json().never();
    assert_eq!(
        futures_executor::block_on(AntigravityLocalSession::new().fetch(100.0, Arc::new(scanner), Arc::new(transport))),
        AntigravityLocalReading {
            snapshot: None,
            fetched_at: None,
            debug_messages: vec![],
        }
    );
}
