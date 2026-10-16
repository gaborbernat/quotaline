use super::{ProviderRediscoveryState, ProviderRediscoveryTrigger, default_provider_rediscovery_interval};

#[test]
fn bridge_throttles_activation_and_records_manual_discovery() {
    let state = ProviderRediscoveryState::new(default_provider_rediscovery_interval(), None);
    assert_eq!(default_provider_rediscovery_interval(), 60.0);
    assert!(state.begin(ProviderRediscoveryTrigger::ApplicationActivated, 100.0));
    assert!(!state.begin(ProviderRediscoveryTrigger::ApplicationActivated, 101.0));
    assert!(state.begin(ProviderRediscoveryTrigger::UserInitiated, 101.0));
    state.record_discovery(200.0);
    assert!(!state.begin(ProviderRediscoveryTrigger::ApplicationActivated, 201.0));
}
