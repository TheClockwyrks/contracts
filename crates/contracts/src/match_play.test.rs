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

#[test]
fn a_tournament_state_round_trips_its_wire_spelling() {
    for state in [
        TournamentState::Queued,
        TournamentState::Running,
        TournamentState::Succeeded,
        TournamentState::Failed,
    ] {
        let wire = serde_json::to_value(state).unwrap();
        assert_eq!(wire, serde_json::json!(state.as_str()));
        assert_eq!(TournamentState::parse(state.as_str()), Some(state));
    }
    assert_eq!(TournamentState::parse("published"), None);
    assert!(TournamentState::Succeeded.is_terminal());
    assert!(TournamentState::Failed.is_terminal());
    assert!(!TournamentState::Queued.is_terminal());
    assert!(!TournamentState::Running.is_terminal());
}

#[test]
fn a_transition_is_tagged_by_the_state_it_moves_to() {
    let running = serde_json::to_value(TournamentTransition::Running).unwrap();
    assert_eq!(running, serde_json::json!({ "state": "running" }));
    let failed: TournamentTransition =
        serde_json::from_value(serde_json::json!({ "state": "failed", "detail": "engine" }))
            .unwrap();
    assert_eq!(failed.state(), TournamentState::Failed);
    let succeeded: TournamentTransition =
        serde_json::from_value(serde_json::json!({ "state": "succeeded", "standings": [] }))
            .unwrap();
    assert_eq!(succeeded.state(), TournamentState::Succeeded);
}

#[test]
fn the_unplayed_pairings_are_those_without_a_recorded_summary() {
    let pairing = |red: &str, blue: &str| TournamentPairing {
        match_id: match_id(red, blue),
        red_id: red.to_string(),
        blue_id: blue.to_string(),
    };
    let summary = MatchSummary {
        match_id: match_id("a", "b"),
        red_id: "a".to_string(),
        blue_id: "b".to_string(),
        winner: Some("a".to_string()),
        win_type: "swept".to_string(),
        outcome_for_red: AdversarialOutcome::Win,
        red_score: 1,
        blue_score: 0,
        ticks: 10,
        red_kills: 0,
        blue_kills: 0,
        red_fuel: 0,
        blue_fuel: 0,
        replay_key: Some(match_id("a", "b")),
        detail: None,
    };
    let recorded = RecordedTournament {
        record: TournamentRecord {
            id: "t".to_string(),
            created_at: "2026-10-26T00:00:00Z".to_string(),
            test_case_slug: "foray".to_string(),
            test_case_version: "v1".to_string(),
            variant: "base".to_string(),
            participants: Vec::new(),
            standings: Vec::new(),
            matches: vec![summary],
        },
        state: TournamentState::Running,
        detail: None,
        pairings: vec![pairing("a", "b"), pairing("a", "c"), pairing("b", "c")],
        published_at: "2026-10-26T00:00:00Z".to_string(),
    };
    let unplayed: Vec<String> = recorded
        .unplayed_pairings()
        .into_iter()
        .map(|pairing| pairing.match_id)
        .collect();
    assert_eq!(unplayed, vec![match_id("a", "c"), match_id("b", "c")]);
}
