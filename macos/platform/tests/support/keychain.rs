use mockall::mock;
use quotaline_macos_platform::keychain::{KeychainValue, Reader};

mock! {
    pub Calls {}
    impl Reader for Calls {
        fn load(&self, service: &str, account: Option<String>) -> Result<Option<KeychainValue>, i32>;
    }
}
