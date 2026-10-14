//! Serves Antigravity fixture responses for integration tests.

use std::{
    fs::OpenOptions,
    io::{self, BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::Path,
};

fn handle(
    mut stream: TcpStream,
    expects_csrf: bool,
    status_unavailable: bool,
    quota_mode: &str,
    requests: &Path,
) -> bool {
    let mut first = [0];
    stream.peek(&mut first).unwrap();
    // The client tries HTTPS first; closing on its TLS handshake record sends it straight to plain HTTP.
    if first[0] == 0x16 {
        return false;
    }
    let mut request = String::new();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    reader.read_line(&mut request).unwrap();
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(requests)
        .unwrap()
        .write_all(request.as_bytes())
        .unwrap();
    let mut csrf = None;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).unwrap();
        if header.trim().is_empty() {
            break;
        }
        if header.to_ascii_lowercase().starts_with("x-codeium-csrf-token:") {
            csrf = header.split_once(':').map(|(_, value)| value.trim().to_owned());
        }
    }
    let authorized = request.contains("/Shutdown")
        || if expects_csrf {
            csrf.as_deref() == Some("fixture-csrf")
        } else {
            csrf.is_none()
        };
    let status_failed = status_unavailable && request.contains("/GetUserStatus");
    let body = if !authorized {
        r#"{"code":16,"message":"unexpected CSRF header"}"#
    } else if status_failed {
        r#"{"message":"status unavailable"}"#
    } else if request.contains("/GetUserStatus") {
        r#"{"userStatus":{"email":"fixture@example.com","userTier":{"name":"Fixture Pro"}}}"#
    } else if quota_mode == "--quota-code-number" {
        r#"{"code":16,"response":{"groups":[{"buckets":[{"bucketId":"gemini-5h","remainingFraction":0.25}]}]}}"#
    } else if quota_mode == "--quota-code-string" {
        r#"{"code":"failed","response":{"groups":[{"buckets":[{"bucketId":"gemini-5h","remainingFraction":0.25}]}]}}"#
    } else if quota_mode == "--quota-empty" {
        r#"{"code":0,"response":{}}"#
    } else if quota_mode == "--quota-code-ok" {
        r#"{"code":"OK","response":{"groups":[{"buckets":[{"bucketId":"gemini-5h","remainingFraction":0.25}]}]}}"#
    } else if quota_mode == "--quota-summary-groups" {
        r#"{"summary":{"groups":[{"buckets":[{"bucketId":"gemini-5h","remainingFraction":0.25}]}]}}"#
    } else if quota_mode == "--quota-root-groups" {
        r#"{"groups":[{"buckets":[{"bucketId":"gemini-5h","remainingFraction":0.25}]}]}"#
    } else if quota_mode == "--quota-invalid-json" {
        "{"
    } else {
        r#"{"response":{"groups":[{"buckets":[{"bucketId":"gemini-5h","remainingFraction":0.25}]}]}}"#
    };
    write!(
        stream,
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        if !authorized {
            "403 Forbidden"
        } else if status_failed {
            "500 Internal Server Error"
        } else {
            "200 OK"
        },
        body.len()
    )
    .unwrap();
    request.contains("/Shutdown")
}

fn main() {
    let mut args = std::env::args();
    let program = args.next().unwrap();
    let expects_csrf = program.contains("language_server") || program.contains("language-server");
    let arguments: Vec<_> = args.collect();
    let status_unavailable = arguments.iter().any(|arg| arg == "--status-unavailable");
    let quota_mode = arguments
        .iter()
        .find(|arg| arg.starts_with("--quota-"))
        .map_or("", String::as_str);
    let requests = std::env::var_os("TMB_FIXTURE_REQUEST_LOG").unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    println!("{}", listener.local_addr().unwrap().port());
    io::stdout().flush().unwrap();
    for stream in listener.incoming() {
        if handle(
            stream.unwrap(),
            expects_csrf,
            status_unavailable,
            quota_mode,
            Path::new(&requests),
        ) {
            break;
        }
    }
}
