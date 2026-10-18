//! Tests for the match-play shapes' opponent lists and storage names.

use super::*;

#[test]
fn match_id_is_stable_and_segment_safe() {
    let id = match_id("border-soldier", "random");
    assert_eq!(id, "border-soldier__vs__random");
    assert!(!id.contains('/'), "a match id must be a safe path segment");
}

#[test]
fn arena_opponents_are_the_baselines_plus_the_hidden_references() {
    // `ARENA_OPPONENT_IDS` is the resolve allowlist; it must stay exactly the
    // model-facing baselines followed by the hidden references, or a hidden
    // opponent silently becomes unresolvable (or a baseline leaks its hidden
    // status). Asserting the concatenation keeps the three constants in sync.
    let expected: Vec<&str> = BASELINE_IDS
        .iter()
        .chain(HIDDEN_OPPONENT_IDS.iter())
        .copied()
        .collect();
    assert_eq!(ARENA_OPPONENT_IDS.to_vec(), expected);
}

#[test]
fn auto_replay_opponents_are_arena_opponents_and_only_random_is_unscored() {
    // The canonical opponent must be `border-soldier`, every auto-replay opponent
    // must be resolvable (in the arena allowlist), and the only unscored
    // exhibition is `random`.
    assert_eq!(AUTO_REPLAY_OPPONENTS[0].0, "border-soldier");
    assert!(AUTO_REPLAY_OPPONENTS[0].1, "canonical is scored");
    for (id, scored) in AUTO_REPLAY_OPPONENTS {
        assert!(
            ARENA_OPPONENT_IDS.contains(id),
            "auto-replay opponent `{id}` must be a resolvable arena opponent",
        );
        assert_eq!(*scored, *id != "random", "only random is unscored: {id}");
    }
}

#[test]
fn replay_filename_is_canonical_then_indexed() {
    assert_eq!(replay_filename(0), "replay.json");
    assert_eq!(replay_filename(1), "replay-1.json");
    assert_eq!(replay_filename(3), "replay-3.json");
}
