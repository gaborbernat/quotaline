use super::*;
use core_foundation::{base::CFType, dictionary::CFDictionary, string::CFString};
use mockall::mock;
use quotaline_macos_platform::keychain::native::{ItemCalls, Store};
use security_framework::os::macos::keychain::CreateOptions;
use security_framework_sys::base::errSecParam;

mock! {
    Calls {}
    impl ItemCalls for Calls {
        fn copy(&self, query: &CFDictionary<CFString, CFType>) -> Result<Option<CFType>, i32>;
        fn update(&self, query: &CFDictionary<CFString, CFType>, value: &CFDictionary<CFString, CFType>) -> i32;
        fn add(&self, query: &CFDictionary<CFString, CFType>) -> i32;
    }
}

fn isolated_access() -> (tempfile::TempDir, KeychainAccess) {
    let directory = temporary_directory();
    let keychain = CreateOptions::new()
        .password("test-only-password")
        .create(directory.path().join("isolated.keychain"))
        .unwrap();
    let access = KeychainAccess::with_store(Store::in_keychain(keychain));
    (directory, access)
}

fn temporary_directory() -> tempfile::TempDir {
    let temporary = std::env::current_dir().unwrap().join("target/tmp");
    std::fs::create_dir_all(&temporary).unwrap();
    tempfile::Builder::new()
        .prefix("keychain-ffi-")
        .tempdir_in(temporary)
        .unwrap()
}

#[test]
fn ffi_keychain_reads_and_writes_isolated_item() {
    let (_directory, access) = isolated_access();
    assert!(access.load("service".into(), Some("account".into())).unwrap().is_none());
    access
        .save(b"secret".to_vec(), "service".into(), "account".into())
        .unwrap();
    assert_eq!(access.load("service".into(), None).unwrap().unwrap().data, b"secret");
}

#[test]
fn ffi_keychain_lists_isolated_services() {
    let (_directory, access) = isolated_access();
    assert!(access.services("Claude".into()).unwrap().is_empty());
    access.save(b"a".to_vec(), "Claude-b".into(), "account".into()).unwrap();
    access.save(b"b".to_vec(), "Claude-a".into(), "account".into()).unwrap();
    assert_eq!(access.services("Claude".into()).unwrap(), ["Claude-a", "Claude-b"]);
}

#[test]
fn ffi_system_keychain_read_only_probe_is_missing() {
    let service = format!(
        "tech.bernat.quotaline.ffi-probe.{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    assert!(KeychainAccess::default().load(service, None).unwrap().is_none());
}

#[test]
fn ffi_keychain_load_reports_security_status() {
    let mut calls = MockCalls::new();
    calls.expect_copy().returning(|_| Err(errSecParam));
    let access = KeychainAccess::with_store(Store::with_calls(calls));
    assert_eq!(
        access.load("service".into(), None).err().map(|error| match error {
            KeychainAccessError::Status { status } => status,
        }),
        Some(errSecParam)
    );
}

#[test]
fn ffi_keychain_services_report_security_status() {
    let mut calls = MockCalls::new();
    calls.expect_copy().returning(|_| Err(errSecParam));
    let access = KeychainAccess::with_store(Store::with_calls(calls));
    assert_eq!(
        access.services("Claude".into()).err().map(|error| match error {
            KeychainAccessError::Status { status } => status,
        }),
        Some(errSecParam)
    );
}

#[test]
fn ffi_keychain_save_reports_security_status() {
    let mut calls = MockCalls::new();
    calls.expect_update().return_const(errSecParam);
    let access = KeychainAccess::with_store(Store::with_calls(calls));
    assert_eq!(
        access
            .save(b"secret".to_vec(), "service".into(), "account".into())
            .err()
            .map(|error| match error {
                KeychainAccessError::Status { status } => status,
            }),
        Some(errSecParam)
    );
}
