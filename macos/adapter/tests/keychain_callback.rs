//! Exercises the UniFFI Keychain callback boundary used by Swift.

use std::ptr::NonNull;

use quotaline_macos_ffi::{
    UniFfiTag,
    credentials::{
        KeychainLookup, KeychainReader, UniFfiTraitVtableKeychainReader,
        uniffi_quotaline_macos_ffi_fn_init_callback_vtable_keychainreader,
    },
};
use uniffi::{FfiConverterArc, Lift, Lower, RustBuffer, RustCallStatus};

extern "C" fn free(_: u64) {}

extern "C" fn clone(handle: u64) -> u64 {
    handle
}

extern "C" fn load(
    handle: u64,
    service: RustBuffer,
    account: RustBuffer,
    output: &mut RustBuffer,
    _: &mut RustCallStatus,
) {
    assert_eq!(handle, 7);
    assert_eq!(<String as Lift<UniFfiTag>>::try_lift(service).unwrap(), "copilot-cli");
    assert_eq!(
        <Option<String> as Lift<UniFfiTag>>::try_lift(account).unwrap(),
        Some("ada".into())
    );
    *output = <KeychainLookup as Lower<UniFfiTag>>::lower(KeychainLookup::Item {
        data: b"secret".to_vec(),
        account: Some("ada".into()),
    });
}

static VTABLE: UniFfiTraitVtableKeychainReader = UniFfiTraitVtableKeychainReader {
    uniffi_free: free,
    uniffi_clone: clone,
    load,
};

#[test]
fn foreign_keychain_callback_returns_the_item() {
    uniffi_quotaline_macos_ffi_fn_init_callback_vtable_keychainreader(NonNull::from(&VTABLE));
    let reader =
        <dyn KeychainReader as FfiConverterArc<UniFfiTag>>::try_lift(uniffi::Handle::from_raw_unchecked(7)).unwrap();
    assert_eq!(
        reader.load("copilot-cli".into(), Some("ada".into())),
        KeychainLookup::Item {
            data: b"secret".to_vec(),
            account: Some("ada".into()),
        }
    );
    let mut bytes = Vec::new();
    <dyn KeychainReader as FfiConverterArc<UniFfiTag>>::write(reader.clone(), &mut bytes);
    let mut encoded = bytes.as_slice();
    let restored = <dyn KeychainReader as FfiConverterArc<UniFfiTag>>::try_read(&mut encoded).unwrap();
    assert_eq!(
        restored.load("copilot-cli".into(), Some("ada".into())),
        reader.load("copilot-cli".into(), Some("ada".into()))
    );
    assert_eq!(
        <dyn KeychainReader as FfiConverterArc<UniFfiTag>>::lower(reader).as_raw(),
        7
    );
}
