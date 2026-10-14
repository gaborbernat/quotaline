//! Exercises the UniFFI credential source callback boundary used by Swift.

use std::ptr::NonNull;

use quotaline_macos_ffi::{
    UniFfiTag,
    credentials::{
        CredentialSourcePolicy, CredentialSourceReader, CredentialSourceScan, CredentialSourceSelection,
        UniFfiTraitVtableCredentialSourceReader, select_credential_source,
        uniffi_quotaline_macos_ffi_fn_init_callback_vtable_credentialsourcereader,
    },
};
use uniffi::{FfiConverterArc, Lower, RustBuffer, RustCallStatus};

extern "C" fn free(_: u64) {}

extern "C" fn clone(handle: u64) -> u64 {
    handle
}

extern "C" fn load(handle: u64, index: u32, output: &mut RustBuffer, _: &mut RustCallStatus) {
    assert_eq!(handle, 11);
    *output = <CredentialSourceScan as Lower<UniFfiTag>>::lower(match index {
        0 => CredentialSourceScan::Missing,
        1 => CredentialSourceScan::Present {
            access_token: "token".into(),
            refresh_token: None,
            expires_at: None,
        },
        _ => CredentialSourceScan::Failed,
    });
}

static VTABLE: UniFfiTraitVtableCredentialSourceReader = UniFfiTraitVtableCredentialSourceReader {
    uniffi_free: free,
    uniffi_clone: clone,
    load,
};

#[test]
fn foreign_source_callback_selects_credential() {
    uniffi_quotaline_macos_ffi_fn_init_callback_vtable_credentialsourcereader(NonNull::from(&VTABLE));
    let reader =
        <dyn CredentialSourceReader as FfiConverterArc<UniFfiTag>>::try_lift(uniffi::Handle::from_raw_unchecked(11))
            .unwrap();
    assert_eq!(
        select_credential_source(CredentialSourcePolicy::First, 3, reader.clone()),
        CredentialSourceSelection {
            selected: Some(1),
            error: None
        }
    );
    let mut bytes = Vec::new();
    <dyn CredentialSourceReader as FfiConverterArc<UniFfiTag>>::write(reader, &mut bytes);
    let mut encoded = bytes.as_slice();
    let restored = <dyn CredentialSourceReader as FfiConverterArc<UniFfiTag>>::try_read(&mut encoded).unwrap();
    assert_eq!(restored.load(0), CredentialSourceScan::Missing);
    assert_eq!(restored.load(2), CredentialSourceScan::Failed);
    assert_eq!(
        <dyn CredentialSourceReader as FfiConverterArc<UniFfiTag>>::lower(restored).as_raw(),
        11
    );
}
