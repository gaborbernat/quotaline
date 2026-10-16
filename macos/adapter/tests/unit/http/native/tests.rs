use super::*;

use crate::mock_http::MockTransport;

fn request() -> NativeHttpGet {
    NativeHttpGet {
        url: "https://example.test/usage".into(),
        headers: std::collections::HashMap::new(),
        operation: "usage".into(),
    }
}

#[test]
fn completion_delivers_the_first_success() {
    let mut transport = MockTransport::new();
    transport
        .expect_start_get()
        .withf(|request, _| request.url == "https://example.test/usage")
        .returning(|_, completion| {
            completion.succeed(vec![1, 2, 3]);
            completion.succeed(vec![4]);
        });
    assert_eq!(
        futures_executor::block_on(get(Arc::new(transport), request())),
        Ok(vec![1, 2, 3])
    );
}

#[test]
fn completion_delivers_the_first_failure() {
    let mut transport = MockTransport::new();
    transport.expect_start_get().returning(|_, completion| {
        completion.fail(HttpErrorRecord::Decoding {
            detail: "invalid".into(),
        });
        completion.fail(HttpErrorRecord::Network { detail: "later".into() });
    });
    assert_eq!(
        futures_executor::block_on(get(Arc::new(transport), request())),
        Err(HttpErrorRecord::Decoding {
            detail: "invalid".into()
        })
    );
}

#[test]
fn abandoned_completion_reports_cancellation() {
    let mut transport = MockTransport::new();
    transport.expect_start_get().returning(|_, _| {});
    assert_eq!(
        futures_executor::block_on(get(Arc::new(transport), request())),
        Err(HttpErrorRecord::Network {
            detail: "request canceled".into()
        })
    );
}

#[test]
fn post_json_delivers_the_response() {
    let mut transport = MockTransport::new();
    transport.expect_start_post_json().return_once(|request, completion| {
        assert_eq!(request.url, "https://example.test/usage");
        assert_eq!(request.body, b"{}".to_vec());
        completion.succeed(vec![4, 5]);
    });
    assert_eq!(
        futures_executor::block_on(post_json(
            Arc::new(transport),
            NativeHttpPostJson {
                url: "https://example.test/usage".into(),
                headers: std::collections::HashMap::new(),
                operation: "usage".into(),
                body: b"{}".to_vec(),
                timeout_millis: None,
            },
        )),
        Ok(vec![4, 5])
    );
}

#[test]
fn post_form_delivers_the_response() {
    let mut transport = MockTransport::new();
    transport.expect_start_post_form().return_once(|request, completion| {
        assert_eq!(request.url, "https://example.test/token");
        assert_eq!(request.fields["grant_type"], "refresh_token");
        completion.succeed(vec![4, 5]);
    });
    assert_eq!(
        futures_executor::block_on(post_form(
            Arc::new(transport),
            NativeHttpPostForm {
                url: "https://example.test/token".into(),
                headers: std::collections::HashMap::new(),
                operation: "token".into(),
                fields: std::collections::HashMap::from([("grant_type".into(), "refresh_token".into())]),
            },
        )),
        Ok(vec![4, 5])
    );
}

#[test]
fn native_transport_posts_shared_pipeline_forms() {
    let mut transport = MockTransport::new();
    transport
        .expect_start_post_form()
        .withf(|request, _| {
            (request.operation.as_str(), request.fields["grant_type"].as_str()) == ("test.form", "refresh")
        })
        .returning(|_, completion| completion.succeed(b"ok".to_vec()));
    let request = Request {
        body: Body::Form(vec![("grant_type".into(), "refresh".into())]),
        ..Request::get("https://example.test/token", Vec::new(), "test.form")
    };
    assert_eq!(
        futures_executor::block_on(NativeTransport(Arc::new(transport)).send(request)),
        Ok(b"ok".to_vec())
    );
}
