use super::*;
use security_framework::os::macos::keychain::CreateOptions;

fn query_value(query: &CFDictionary<CFString, CFType>, field: CFStringRef) -> Option<CFType> {
    let field = unsafe { CFString::wrap_under_get_rule(field) };
    query.find(&field).map(|value| (*value).clone())
}

fn isolated_store() -> (tempfile::TempDir, Store) {
    let temporary = std::env::current_dir().unwrap().join("target/tmp");
    std::fs::create_dir_all(&temporary).unwrap();
    let directory = tempfile::Builder::new()
        .prefix("keychain-")
        .tempdir_in(temporary)
        .unwrap();
    let keychain = CreateOptions::new()
        .password("test-only-password")
        .create(directory.path().join("isolated.keychain"))
        .unwrap();
    (directory, Store::in_keychain(keychain))
}

#[test]
fn system_keychain_missing_service_returns_none_without_writing() {
    let service = format!(
        "tech.bernat.quotaline.probe.{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    assert!(Store::system().load(&service, None).unwrap().is_none());
}

#[test]
fn isolated_keychain_adds_and_loads_a_password() {
    let (_directory, store) = isolated_store();
    assert!(store.load("test-service", Some("test-account")).unwrap().is_none());
    store.save(b"first", "test-service", "test-account").unwrap();
    let item = store.load("test-service", Some("test-account")).unwrap().unwrap();
    assert_eq!(item.data, b"first");
    assert_eq!(item.account.as_deref(), Some("test-account"));
}

#[test]
fn isolated_keychain_updates_an_existing_password() {
    let (_directory, store) = isolated_store();
    store.save(b"first", "test-service", "test-account").unwrap();
    store.save(b"second", "test-service", "test-account").unwrap();
    assert_eq!(store.load("test-service", None).unwrap().unwrap().data, b"second");
}

#[test]
fn isolated_keychain_lists_only_matching_services() {
    let (_directory, store) = isolated_store();
    assert!(store.service_names("Claude").unwrap().is_empty());
    store.save(b"a", "Claude-b", "account").unwrap();
    store.save(b"b", "Claude-a", "account").unwrap();
    store.save(b"c", "Codex", "account").unwrap();
    assert_eq!(store.service_names("Claude").unwrap(), ["Claude-a", "Claude-b"]);
}

#[test]
fn keychain_save_without_selected_keychain_adds_missing_password() {
    let mut calls = MockItemCalls::new();
    calls.expect_update().return_const(errSecItemNotFound);
    calls.expect_add().return_const(errSecSuccess);
    let store = Store::with_calls(calls);
    assert_eq!(store.save(b"secret", "service", "account"), Ok(()));
}

#[test]
fn keychain_save_reports_update_error() {
    let mut calls = MockItemCalls::new();
    calls.expect_update().return_const(errSecParam);
    calls.expect_add().times(0);
    let store = Store::with_calls(calls);
    assert_eq!(store.save(b"secret", "service", "account"), Err(errSecParam));
}

#[test]
fn keychain_load_reports_security_error() {
    let mut calls = MockItemCalls::new();
    calls.expect_copy().returning(|_| Err(errSecParam));
    let store = Store::with_calls(calls);
    assert_eq!(store.load("service", None).err(), Some(errSecParam));
}

#[test]
fn malformed_security_query_reports_parameter_error() {
    let query = CFDictionary::<CFString, CFType>::from_CFType_pairs(&[]);
    assert_eq!(SystemCalls.copy(&query).err(), Some(errSecParam));
}

#[test]
fn keychain_load_uses_noninteractive_security_query() {
    let mut calls = MockItemCalls::new();
    calls
        .expect_copy()
        .withf(|query| {
            let context = query_value(query, unsafe { kSecUseAuthenticationContext }).unwrap();
            let context = context.as_CFTypeRef().cast::<LAContext>();
            (unsafe { (*context).interactionNotAllowed() })
                && query_value(query, unsafe { kSecAttrService })
                    .and_then(|value| value.downcast::<CFString>())
                    .is_some_and(|value| value == "service")
                && query_value(query, unsafe { kSecAttrAccount })
                    .and_then(|value| value.downcast::<CFString>())
                    .is_some_and(|value| value == "account")
                && query_value(query, unsafe { kSecReturnAttributes }).is_some()
                && query_value(query, unsafe { kSecReturnData }).is_some()
                && query_value(query, unsafe { kSecUseAuthenticationUI })
                    .and_then(|value| value.downcast::<CFString>())
                    .is_some_and(|value| value == "u_AuthUIF")
        })
        .returning(|_| Ok(None));
    assert!(
        Store::with_calls(calls)
            .load("service", Some("account"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn keychain_services_query_all_attributes_without_data() {
    let mut calls = MockItemCalls::new();
    calls
        .expect_copy()
        .withf(|query| {
            query_value(query, unsafe { kSecAttrService }).is_none()
                && query_value(query, unsafe { kSecAttrAccount }).is_none()
                && query_value(query, unsafe { kSecReturnData }).is_none()
                && query_value(query, unsafe { kSecReturnAttributes }).is_some()
                && query_value(query, unsafe { kSecMatchLimit })
                    .and_then(|value| value.downcast::<CFString>())
                    .is_some_and(|value| value == unsafe { CFString::wrap_under_get_rule(kSecMatchLimitAll) })
        })
        .returning(|_| Ok(None));
    assert!(Store::with_calls(calls).service_names("Claude").unwrap().is_empty());
}
