use super::*;

#[test]
fn reading_selection_follows_each_event_and_clamps_to_the_timeline() {
    for (name, count, selected, directions, expected) in [
        ("empty timeline", 0, None, vec![1], None),
        ("no events", 3, None, vec![], None),
        ("first forward", 3, None, vec![1], Some(0)),
        ("first backward", 3, None, vec![-1], Some(2)),
        ("next", 3, Some(0), vec![1], Some(1)),
        ("previous", 3, Some(2), vec![-1], Some(1)),
        ("clamp forward", 3, Some(2), vec![1], Some(2)),
        ("clamp backward", 3, Some(0), vec![-1], Some(0)),
        ("reverse at first", 3, None, vec![1, -1], Some(0)),
        ("reverse at last", 3, None, vec![-1, 1], Some(2)),
        ("several steps", 3, Some(0), vec![1, 1], Some(2)),
        ("zero from missing", 3, None, vec![0], Some(2)),
    ] {
        assert_eq!(step_reading(count, selected, &directions), expected, "{name}");
    }
}
