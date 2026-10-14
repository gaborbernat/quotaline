//! macOS notice visibility bridge cases.

use super::*;

#[test]
fn visibility_bridge_keeps_general_and_hidden_limit_notices() {
    let notices = vec![
        NoticeCandidate {
            limit_reached: true,
            window_id: Some("full".into()),
        },
        NoticeCandidate {
            limit_reached: true,
            window_id: Some("hidden".into()),
        },
        NoticeCandidate {
            limit_reached: true,
            window_id: None,
        },
        NoticeCandidate {
            limit_reached: false,
            window_id: Some("full".into()),
        },
    ];
    assert_eq!(visible_notice_indices(notices, vec!["full".into()]), [1, 2, 3]);
}
