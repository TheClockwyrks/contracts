//! Tests for reading a run's `writeup.md` into a [`Writeup`] and rendering the
//! canonical file back out.

use super::*;

/// A passing verdict on `id`, with no note.
fn pass(id: &str) -> ReviewVerdict {
    ReviewVerdict {
        id: id.to_string(),
        status: VerdictStatus::Pass,
        note: None,
    }
}

#[test]
fn parses_rating_and_body() {
    let raw =
        "---\nrating.gameplay: great\n---\n\nMovement feels right.\nThe pause menu is rough.\n";
    let writeup = parse_writeup(raw).expect("parse");
    assert_eq!(
        writeup.ratings,
        vec![DomainRating {
            domain: "gameplay".to_string(),
            rating: Rating::Great,
        }]
    );
    assert_eq!(
        writeup.body,
        "Movement feels right.\nThe pause menu is rough."
    );
}

#[test]
fn rating_token_is_case_and_whitespace_insensitive() {
    let raw = "---\nrating.gameplay:   Broken \n---\n\nUnplayable.\n";
    assert_eq!(
        parse_writeup(raw).expect("parse").overall_rating(),
        Some(Rating::Broken)
    );
}

#[test]
fn tolerates_a_bom_and_leading_blank_lines() {
    let raw = "\u{feff}\n\n---\nrating.gameplay: flawless\n---\n\nSpotless.\n";
    assert_eq!(
        parse_writeup(raw).expect("parse").overall_rating(),
        Some(Rating::Flawless)
    );
}

#[test]
fn overall_rating_is_the_worst_across_domains() {
    let raw =
        "---\nrating.single-player: flawless\nrating.versus: scuffed\n---\n\nVersus is rough.\n";
    let writeup = parse_writeup(raw).expect("parse");
    assert_eq!(writeup.ratings.len(), 2);
    // The overall rating is the worst across the two domains.
    assert_eq!(writeup.overall_rating(), Some(Rating::Scuffed));
}

#[test]
fn rejects_a_missing_frontmatter_block() {
    let err = parse_writeup("Just some prose, no frontmatter.\n").expect_err("no frontmatter");
    assert!(err.to_string().contains("frontmatter"));
}

#[test]
fn rejects_an_unterminated_frontmatter_block() {
    let err = parse_writeup("---\nrating.gameplay: great\n\nbody with no closing fence\n")
        .expect_err("unterminated");
    assert!(err.to_string().contains("not closed"));
}

#[test]
fn rejects_a_missing_rating() {
    let err = parse_writeup("---\nnote: hi\n---\n\nBody.\n").expect_err("no rating");
    assert!(err.to_string().contains("rating"));
}

#[test]
fn rejects_an_unknown_rating() {
    let err =
        parse_writeup("---\nrating.gameplay: amazing\n---\n\nBody.\n").expect_err("unknown rating");
    assert!(err.to_string().contains("amazing"));
}

#[test]
fn rejects_an_empty_body() {
    let err = parse_writeup("---\nrating.gameplay: great\n---\n\n   \n").expect_err("empty body");
    assert!(err.to_string().contains("body"));
}

#[test]
fn renders_a_canonical_file_that_reparses() {
    let writeup = Writeup {
        aesthetic: None,
        ratings: vec![DomainRating {
            domain: "gameplay".to_string(),
            rating: Rating::Scuffed,
        }],
        body: "Plays, but the score resets on pause.".to_string(),
        checklist: vec![],
    };
    let file = writeup.to_file_string();
    assert_eq!(
        file,
        "---\nrating.gameplay: scuffed\n---\n\nPlays, but the score resets on pause.\n"
    );
    assert_eq!(parse_writeup(&file).expect("reparse"), writeup);
}

#[test]
fn renders_multiple_domain_ratings_in_order() {
    let writeup = Writeup {
        aesthetic: None,
        ratings: vec![
            DomainRating {
                domain: "single-player".to_string(),
                rating: Rating::Flawless,
            },
            DomainRating {
                domain: "versus".to_string(),
                rating: Rating::Broken,
            },
        ],
        body: "Single player is great; versus is broken.".to_string(),
        checklist: vec![],
    };
    let file = writeup.to_file_string();
    assert!(file.contains("rating.single-player: flawless\n"));
    assert!(file.contains("rating.versus: broken\n"));
    assert_eq!(parse_writeup(&file).expect("reparse"), writeup);
}

#[test]
fn parses_checklist_verdicts_with_and_without_notes() {
    let raw = "---\nrating.gameplay: great\n\
               review.ball-spin: pass\n\
               review.bank-shot: fail ball clips the top obstacle\n\
               ---\n\nPlays well.\n";
    let writeup = parse_writeup(raw).expect("parse");
    assert_eq!(
        writeup.checklist,
        vec![
            ReviewVerdict {
                id: "ball-spin".to_string(),
                status: VerdictStatus::Pass,
                note: None,
            },
            ReviewVerdict {
                id: "bank-shot".to_string(),
                status: VerdictStatus::Fail,
                note: Some("ball clips the top obstacle".to_string()),
            },
        ]
    );
}

#[test]
fn a_writeup_with_verdicts_round_trips() {
    let writeup = Writeup {
        aesthetic: None,
        ratings: vec![DomainRating {
            domain: "gameplay".to_string(),
            rating: Rating::Scuffed,
        }],
        body: "Bank shots are off.".to_string(),
        checklist: vec![
            ReviewVerdict {
                id: "ball-spin".to_string(),
                status: VerdictStatus::Pass,
                note: None,
            },
            ReviewVerdict {
                id: "bank-shot".to_string(),
                status: VerdictStatus::Fail,
                note: Some("clips the corner".to_string()),
            },
        ],
    };
    let file = writeup.to_file_string();
    assert_eq!(parse_writeup(&file).expect("reparse"), writeup);
}

#[test]
fn a_note_with_a_stray_newline_is_normalized_to_one_line() {
    // A note must never break the frontmatter block: newlines collapse to spaces.
    let writeup = Writeup {
        aesthetic: None,
        ratings: vec![DomainRating {
            domain: "gameplay".to_string(),
            rating: Rating::Broken,
        }],
        body: "Unplayable.".to_string(),
        checklist: vec![ReviewVerdict {
            id: "load".to_string(),
            status: VerdictStatus::Fail,
            note: Some("throws\non load".to_string()),
        }],
    };
    let file = writeup.to_file_string();
    assert!(file.contains("review.load: fail throws on load\n"));
    let reparsed = parse_writeup(&file).expect("reparse");
    assert_eq!(
        reparsed.checklist[0].note.as_deref(),
        Some("throws on load")
    );
}

#[test]
fn an_unrecognized_verdict_status_is_rejected() {
    let raw = "---\nrating.gameplay: great\nreview.x: maybe\n---\n\nBody.\n";
    let err = parse_writeup(raw).expect_err("bad status");
    assert!(err.to_string().contains("pass or fail"));
}

#[test]
fn na_is_no_longer_a_valid_verdict() {
    // Verdicts are binary now; `na` must be rejected so every item counts toward
    // the score one way or the other.
    let raw = "---\nrating.gameplay: great\nreview.x: na\n---\n\nBody.\n";
    assert!(parse_writeup(raw).is_err());
}

#[test]
fn writeup_with_only_graded_verdicts_parses() {
    // A game-jam writeup has no `rating.<domain>` lines — only graded categories
    // and the overall mark — and must still parse.
    let raw = "---\nreview.fun: great\nreview.overall: incredible\n---\n\nGreat jam entry.\n";
    let writeup = parse_writeup(raw).expect("jam writeup parses");
    assert!(writeup.ratings.is_empty());
    assert_eq!(writeup.overall_grade(), Some(VerdictStatus::Incredible));
    assert_eq!(writeup.checklist.len(), 2);
}

#[test]
fn a_writeup_parses_and_round_trips_the_run_wide_aesthetic() {
    let raw = "---\naesthetic: Amazing\n---\n\nLovely.\n";
    let writeup = parse_writeup(raw).expect("parse");
    assert!(writeup.ratings.is_empty());
    assert_eq!(writeup.aesthetic, Some(AestheticRating::Amazing));
    assert_eq!(writeup.overall_rating(), None);
    assert_eq!(writeup.overall_aesthetic(), Some(AestheticRating::Amazing));

    let rendered = writeup.to_file_string();
    assert_eq!(rendered, "---\naesthetic: amazing\n---\n\nLovely.\n");
    assert_eq!(parse_writeup(&rendered).expect("reparse"), writeup);
}

#[test]
fn legacy_per_domain_aesthetic_lines_collapse_to_the_worst_tier() {
    // A writeup written when the channel was per-domain still parses; its lines
    // collapse to the worst tier — which equals the old aggregation, so the
    // displayed value for an old review does not change.
    let raw =
        "---\naesthetic.single-player: amazing\naesthetic.versus: Legendary\n---\n\nLovely.\n";
    let writeup = parse_writeup(raw).expect("parse");
    assert_eq!(writeup.aesthetic, Some(AestheticRating::Amazing));

    // Re-rendering emits only the new bare form.
    assert_eq!(
        writeup.to_file_string(),
        "---\naesthetic: amazing\n---\n\nLovely.\n"
    );

    // A mixed file (bare + legacy lines) collapses across all of them too.
    let mixed = "---\naesthetic: good\naesthetic.versus: okay\n---\n\nBody.\n";
    assert_eq!(
        parse_writeup(mixed).expect("parse").aesthetic,
        Some(AestheticRating::Okay)
    );
}

#[test]
fn a_writeup_renders_ratings_then_the_aesthetic_then_verdicts() {
    let writeup = Writeup {
        ratings: vec![DomainRating {
            domain: "gameplay".to_string(),
            rating: Rating::Great,
        }],
        aesthetic: Some(AestheticRating::Good),
        body: "Body.".to_string(),
        checklist: vec![pass("a")],
    };
    assert_eq!(
        writeup.to_file_string(),
        "---\nrating.gameplay: great\naesthetic: good\nreview.a: pass\n---\n\nBody.\n"
    );
    assert_eq!(
        parse_writeup(&writeup.to_file_string()).expect("reparse"),
        writeup
    );
}

#[test]
fn an_unknown_aesthetic_tier_or_empty_domain_is_rejected() {
    let err = parse_writeup("---\naesthetic: flawless\n---\n\nBody.\n").unwrap_err();
    assert!(
        format!("{err}").contains("`aesthetic` must be one of legendary, amazing"),
        "got: {err}"
    );
    // A legacy per-domain line's errors still name the line.
    let err = parse_writeup("---\naesthetic.gameplay: flawless\n---\n\nBody.\n").unwrap_err();
    assert!(
        format!("{err}").contains("`aesthetic.gameplay` must be one of legendary, amazing"),
        "got: {err}"
    );
    let err = parse_writeup("---\naesthetic.: good\n---\n\nBody.\n").unwrap_err();
    assert!(format!("{err}").contains("empty domain id"), "got: {err}");
}
