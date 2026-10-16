use mockall::mock;
use quotaline_macos_ffi::credentials::{KeychainLookup, KeychainReader};

mock! {
    pub Calls {}
    impl KeychainReader for Calls {
        fn load(&self, service: String, account: Option<String>) -> KeychainLookup;
    }
}
