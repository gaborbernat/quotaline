//! Checks macOS process discovery, argument parsing, and socket inspection.

use super::*;
use std::net::{TcpListener, TcpStream};
use std::os::unix::net::UnixListener;
use std::path::Path;

fn fixture_arguments(arguments: &[&str]) -> Vec<u8> {
    let mut bytes = (arguments.len() as i32).to_ne_bytes().to_vec();
    bytes.extend(b"/opt/bin/agy\0\0\0");
    for argument in arguments {
        bytes.extend(argument.as_bytes());
        bytes.push(0);
    }
    bytes.extend(b"HOME=/unused\0");
    bytes
}

fn fixture_scanner(bytes: Vec<u8>) -> Scanner {
    let mut calls = MockProcessCalls::new();
    calls.expect_pids().return_const(vec![1]);
    calls
        .expect_path()
        .with(mockall::predicate::eq(1))
        .return_const(Some("/opt/bin/agy".into()));
    calls
        .expect_argument_bytes()
        .with(mockall::predicate::eq(1))
        .return_const(bytes);
    Scanner::with_calls(calls)
}

#[test]
fn scanner_returns_only_candidate_processes() {
    let mut calls = MockProcessCalls::new();
    calls.expect_pids().return_const(vec![1, 2, 3]);
    calls.expect_path().returning(|pid| match pid {
        1 => None,
        2 => Some("/bin/zsh".into()),
        _ => Some("/opt/bin/agy".into()),
    });
    calls
        .expect_argument_bytes()
        .with(mockall::predicate::eq(3))
        .return_const(fixture_arguments(&["agy", "quota"]));
    assert_eq!(
        Scanner::with_calls(calls).processes(),
        [ProcessEntry {
            pid: 3,
            path: "/opt/bin/agy".into(),
            arguments: vec!["agy".into(), "quota".into()],
        }]
    );
}

#[test]
fn scanner_passes_through_listening_ports() {
    let mut calls = MockProcessCalls::new();
    calls
        .expect_ports()
        .with(mockall::predicate::eq(7))
        .return_const(vec![8080]);
    assert_eq!(Scanner::with_calls(calls).listening_ports(7), [8080]);
}

#[test]
fn server_discovery_selects_roles_and_orders_ports() {
    let mut queried = Vec::new();
    let servers = discover_servers(
        [
            ProcessEntry {
                pid: 1,
                path: "/opt/bin/language_server".into(),
                arguments: vec!["--app_data_dir=windsurf".into(), "--csrf_token=other".into()],
            },
            ProcessEntry {
                pid: 2,
                path: "/opt/bin/language_server".into(),
                arguments: vec!["--app_data_dir=antigravity".into(), "--csrf_token=secret".into()],
            },
            ProcessEntry {
                pid: 3,
                path: "/opt/bin/agy".into(),
                arguments: vec![],
            },
            ProcessEntry {
                pid: 4,
                path: "/opt/bin/agy".into(),
                arguments: vec![],
            },
        ],
        |pid| {
            queried.push(pid);
            match pid {
                2 => vec![4321, 1234],
                3 => vec![8080],
                _ => vec![],
            }
        },
    );
    assert_eq!(queried, [2, 3, 4]);
    assert_eq!(
        servers,
        [
            LocalServer {
                pid: 2,
                port: 1234,
                csrf_token: Some("secret".into()),
            },
            LocalServer {
                pid: 2,
                port: 4321,
                csrf_token: Some("secret".into()),
            },
            LocalServer {
                pid: 3,
                port: 8080,
                csrf_token: None,
            },
        ]
    );
}

#[test]
fn scanner_discovers_servers_through_process_calls() {
    let mut calls = MockProcessCalls::new();
    calls.expect_pids().return_const(vec![7]);
    calls.expect_path().return_const(Some("/opt/bin/agy".into()));
    calls.expect_argument_bytes().return_const(fixture_arguments(&["agy"]));
    calls.expect_ports().return_const(vec![9090]);
    assert_eq!(
        Scanner::with_calls(calls).discover_servers(),
        [LocalServer {
            pid: 7,
            port: 9090,
            csrf_token: None,
        }]
    );
}

#[test]
fn scanner_rejects_incomplete_argument_buffers() {
    for bytes in [
        vec![],
        vec![1, 0, 0, 0, b'/', b'b', b'i', b'n'],
        0i32.to_ne_bytes().to_vec(),
        (-1i32).to_ne_bytes().to_vec(),
    ] {
        assert!(fixture_scanner(bytes).processes()[0].arguments.is_empty());
    }
}

#[test]
fn scanner_keeps_empty_arguments_and_stops_before_environment() {
    assert_eq!(
        fixture_scanner(fixture_arguments(&["agy", "", "quota"])).processes()[0].arguments,
        ["agy", "", "quota"]
    );
}

#[test]
fn system_process_calls_read_own_process() {
    let calls = SystemCalls::default();
    let pid = std::process::id() as i32;
    assert!(calls.pids().contains(&pid));
    assert!(calls.path(pid).is_some());
    assert!(!calls.argument_bytes(pid).is_empty());
}

#[test]
fn system_process_calls_ignore_missing_process() {
    let calls = SystemCalls::default();
    assert_eq!(calls.path(i32::MAX), None);
    assert!(calls.argument_bytes(i32::MAX).is_empty());
    assert!(calls.ports(i32::MAX).is_empty());
}

#[test]
fn system_process_calls_find_own_listener() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    assert!(SystemCalls::default().ports(std::process::id() as i32).contains(&port));
}

#[test]
fn system_scanner_reads_own_listener() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    assert!(
        Scanner::system()
            .listening_ports(std::process::id() as i32)
            .contains(&port)
    );
}

#[test]
fn system_process_calls_reject_failed_argument_reads() {
    let mut native = MockNativeCalls::new();
    native.expect_argument_size().return_const(Some(8));
    native.expect_read_arguments().return_const(None);
    assert!(SystemCalls::with_native(native).argument_bytes(7).is_empty());
}

#[test]
fn system_process_calls_reject_empty_argument_sizes() {
    let mut native = MockNativeCalls::new();
    native.expect_argument_size().return_const(Some(0));
    assert!(SystemCalls::with_native(native).argument_bytes(7).is_empty());
}

#[test]
fn system_process_calls_truncate_arguments_to_bytes_written() {
    let mut native = MockNativeCalls::new();
    native.expect_argument_size().return_const(Some(8));
    native.expect_read_arguments().returning(|_, bytes| {
        bytes[..3].copy_from_slice(b"agy");
        Some(3)
    });
    assert_eq!(SystemCalls::with_native(native).argument_bytes(7), b"agy");
}

#[test]
fn system_process_calls_reject_failed_descriptor_reads() {
    let mut native = MockNativeCalls::new();
    native.expect_fd_count().return_const(Some(3));
    native.expect_descriptors().return_const(None);
    assert!(SystemCalls::with_native(native).ports(7).is_empty());
}

#[test]
fn system_process_calls_filter_sockets_and_states() {
    let mut native = MockNativeCalls::new();
    native.expect_fd_count().return_const(Some(5));
    native.expect_descriptors().return_const(Some(vec![
        Descriptor { fd: 1, socket: false },
        Descriptor { fd: 2, socket: true },
        Descriptor { fd: 3, socket: true },
        Descriptor { fd: 4, socket: true },
        Descriptor { fd: 5, socket: true },
    ]));
    native.expect_socket().returning(|_, fd| match fd {
        2 => None,
        3 => Some(Socket::Other),
        4 => Some(Socket::Tcp {
            listening: false,
            port: 8080,
        }),
        _ => Some(Socket::Tcp {
            listening: true,
            port: 9090,
        }),
    });
    assert_eq!(SystemCalls::with_native(native).ports(7), [9090]);
}

#[test]
fn native_calls_read_own_arguments_and_descriptors() {
    let native = LibprocCalls;
    let pid = std::process::id() as i32;
    let mut bytes = vec![0; native.argument_size(pid).unwrap()];
    let size = native.read_arguments(pid, &mut bytes).unwrap();
    assert!(size > 4);
    assert!(
        !native
            .descriptors(pid, native.fd_count(pid).unwrap())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn native_calls_reject_missing_process_and_descriptor() {
    let native = LibprocCalls;
    assert_eq!(native.argument_size(i32::MAX), None);
    assert_eq!(native.read_arguments(i32::MAX, &mut [0; 8]), None);
    assert_eq!(native.fd_count(i32::MAX), None);
    assert_eq!(native.descriptors(i32::MAX, 1), None);
    assert_eq!(native.socket(std::process::id() as i32, i32::MAX), None);
}

#[test]
fn native_calls_distinguish_tcp_and_unix_sockets() {
    let tcp = TcpListener::bind("127.0.0.1:0").unwrap();
    let stream = TcpStream::connect(tcp.local_addr().unwrap()).unwrap();
    let _accepted = tcp.accept().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&root).unwrap();
    let dir = tempfile::tempdir_in(root).unwrap();
    let unix = UnixListener::bind(dir.path().join("socket")).unwrap();
    let native = LibprocCalls;
    let pid = std::process::id() as i32;
    let sockets = native
        .descriptors(pid, native.fd_count(pid).unwrap())
        .unwrap()
        .into_iter()
        .filter(|fd| fd.socket)
        .filter_map(|fd| native.socket(pid, fd.fd))
        .collect::<Vec<_>>();
    assert!(sockets.contains(&Socket::Tcp {
        listening: true,
        port: tcp.local_addr().unwrap().port(),
    }));
    assert!(
        sockets
            .iter()
            .any(|socket| matches!(socket, Socket::Tcp { listening: false, .. }))
    );
    assert!(sockets.contains(&Socket::Other));
    drop(stream);
    drop(unix);
}
