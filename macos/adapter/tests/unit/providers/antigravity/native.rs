//! Checks the Antigravity process scanner bridge.

use super::*;
use mockall::mock;
use quotaline_macos_platform::process::{ProcessCalls, Scanner};
use std::io::{BufRead, Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};

mock! {
    Calls {}
    impl ProcessCalls for Calls {
        fn pids(&self) -> Vec<i32>;
        fn path(&self, pid: i32) -> Option<String>;
        fn argument_bytes(&self, pid: i32) -> Vec<u8>;
        fn ports(&self, pid: i32) -> Vec<u16>;
    }
}

#[test]
fn native_scanner_discovers_servers() {
    let mut calls = MockCalls::new();
    calls.expect_pids().return_const(vec![7]);
    calls.expect_path().return_const(Some("/opt/bin/agy".into()));
    calls
        .expect_argument_bytes()
        .return_const([1, 0, 0, 0, b'/', 0, 0, b'a', 0].to_vec());
    calls.expect_ports().return_const(vec![9090]);
    let scanner = AntigravityNativeScanner::with_scanner(Scanner::with_calls(calls));
    assert_eq!(
        scanner.discover_servers(),
        [AntigravityLocalServer {
            pid: 7,
            port: 9090,
            csrf_token: None,
        }]
    );
}

#[test]
fn native_scanner_constructs_system_adapter() {
    if std::env::var_os("TMB_ANTIGRAVITY_SCAN_CHILD").is_some() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        println!("TMB_PORT={}", listener.local_addr().unwrap().port());
        std::io::stdout().flush().unwrap();
        let _ = std::io::stdin().read(&mut [0]);
        return;
    }
    let current_exe = std::env::current_exe().unwrap();
    let directory = tempfile::tempdir_in(current_exe.parent().unwrap()).unwrap();
    let executable = directory.path().join("agy");
    std::fs::hard_link(current_exe, &executable).unwrap();
    let mut child = Command::new(executable)
        .args(["--nocapture", "native_scanner_constructs_system_adapter"])
        .env("TMB_ANTIGRAVITY_SCAN_CHILD", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut output = std::io::BufReader::new(child.stdout.take().unwrap());
    let port = (&mut output)
        .lines()
        .map(Result::unwrap)
        .find_map(|line| {
            line.split_once("TMB_PORT=")
                .map(|(_, value)| value.parse::<u16>().unwrap())
        })
        .unwrap();
    let found = AntigravityNativeScanner::default()
        .discover_servers()
        .contains(&AntigravityLocalServer {
            pid: child.id() as i32,
            port,
            csrf_token: None,
        });
    drop(child.stdin.take());
    assert!(child.wait().unwrap().success());
    assert!(found);
}
