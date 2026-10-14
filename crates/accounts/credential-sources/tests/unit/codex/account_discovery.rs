use super::*;
use quotaline_credentials::source_chain::SourceState;

fn discovery() -> AccountDiscovery {
    AccountDiscovery::new(vec!["first".into(), "second".into(), "third".into()])
}

#[test]
fn scan_collects_usable_accounts_and_first_error() {
    let states = [SourceState::Failed, SourceState::Present, SourceState::Failed];
    assert_eq!(
        discovery().scan(|index| states[index]),
        AccountScan {
            available: vec![1],
            first_error: Some(0)
        }
    );
}

#[test]
fn scan_keeps_empty_and_missing_results_distinct() {
    assert_eq!(
        AccountDiscovery::new(vec![]).scan(|_| panic!("empty discovery must not read")),
        AccountScan {
            available: vec![],
            first_error: None
        }
    );
    assert_eq!(
        discovery().scan(|_| SourceState::Missing),
        AccountScan {
            available: vec![],
            first_error: None
        }
    );
}

#[test]
fn selected_home_stays_sticky_when_other_accounts_appear() {
    let discovery = discovery();
    assert_eq!(discovery.selected_home(), None);
    assert_eq!(discovery.select(&[1, 2]), Some(1));
    assert_eq!(discovery.selected_home(), Some("second".into()));
    assert_eq!(discovery.select(&[0, 1, 2]), Some(1));
    assert_eq!(discovery.select(&[0, 2]), None);
    assert_eq!(discovery.selected_home(), Some("second".into()));
}

#[test]
fn save_target_uses_selected_or_first_configured_home() {
    let discovery = discovery();
    assert_eq!(discovery.save_target(&[]), Some(0));
    assert_eq!(discovery.save_target(&[2]), Some(2));
    assert_eq!(discovery.save_target(&[]), Some(0));
    assert_eq!(AccountDiscovery::new(vec![]).save_target(&[]), None);
}
