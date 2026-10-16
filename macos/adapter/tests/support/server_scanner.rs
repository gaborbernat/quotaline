use crate::antigravity::{AntigravityLocalServer, AntigravityServerScanner};

mockall::mock! {
    pub ServerScanner {}
    impl AntigravityServerScanner for ServerScanner {
        fn discover_servers(&self) -> Vec<AntigravityLocalServer>;
    }
}
