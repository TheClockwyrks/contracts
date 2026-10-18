//! A run's `writeup.md`: the [`Writeup`] a review is parsed from and rendered
//! back to.
//!
//! The file is Markdown whose `---` frontmatter carries the per-domain
//! `rating.<domain>` lines, the run-wide `aesthetic` tier and the
//! `review.<id>` checklist verdicts, followed by the prose. The CLI reads it to
//! publish a run, and the backend and the console write and read the same
//! review, so its format is defined once here.

use super::{
    AestheticRating, DomainRating, OVERALL_VERDICT_ID, Rating, ReviewVerdict, VerdictStatus,
};

/// Why a `writeup.md` could not be read as a [`Writeup`]: a missing or unclosed
/// frontmatter block, a malformed rating, aesthetic tier or verdict line, or an
/// empty body. The message says which, in the words the publish gate reports.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct WriteupError(pub String);

/// A parsed review: per-domain [`Rating`]s and/or a run-wide [`AestheticRating`],
/// the writeup prose it accompanies, and the reviewer's verdicts on the case's
/// declared checklist items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Writeup {
    /// The reviewer's functional rating per scoring domain, in the order they
    /// appear in the writeup's frontmatter. On a legacy run a review is only
    /// complete once every declared domain has a rating here (the scorer's
    /// `missing_ratings`); the run's overall rating is the worst across them
    /// ([`Self::overall_rating`]). Empty on a review of a
    /// [validator-rated](crate::test_case::TestCaseVersion::validator_rated) run,
    /// whose functional rating is not the reviewer's to give.
    pub ratings: Vec<DomainRating>,
    /// The reviewer's run-wide **aesthetic** rating — one tier for the whole
    /// build, from the writeup's bare `aesthetic: <tier>` frontmatter line (a
    /// legacy file's per-domain `aesthetic.<domain>` lines parse too, collapsed to
    /// their worst tier). On a validator-rated run a review is only complete once
    /// it carries one (the scorer's `needs_aesthetic`); the run's aesthetic rating
    /// is the worst across its reviews' tiers (the scorer's `aggregate_aesthetic`).
    /// `None` on a legacy run's review, which has no aesthetic channel.
    pub aesthetic: Option<AestheticRating>,
    /// The writeup body — the Markdown prose shown before the playable build,
    /// with the frontmatter stripped and surrounding whitespace trimmed.
    pub body: String,
    /// The reviewer's verdicts on the case's declared checklist items, in the
    /// order they appear in the writeup's frontmatter. Empty for a case that
    /// declares no items, or a writeup that records none. A run is only ready to
    /// publish once every declared item has a verdict here (the scorer's
    /// `missing_verdicts`).
    pub checklist: Vec<ReviewVerdict>,
}

impl Writeup {
    /// The run's overall rating: the worst across its domain ratings, or `None`
    /// when it records none. A flawless mode cannot mask a broken one.
    pub fn overall_rating(&self) -> Option<Rating> {
        Rating::worst(self.ratings.iter().map(|domain| domain.rating))
    }

    /// This review's run-wide aesthetic rating, or `None` when it records none.
    /// The parser already collapsed a legacy file's per-domain lines to their
    /// worst tier, so this is simply [`Self::aesthetic`].
    pub fn overall_aesthetic(&self) -> Option<AestheticRating> {
        self.aesthetic
    }

    /// The reviewer's whole-game overall grade, from the reserved
    /// [`OVERALL_VERDICT_ID`] checklist verdict (a [game jam](crate::test_case::TestType::GameJam)
    /// review), or `None` when the review records none.
    pub fn overall_grade(&self) -> Option<VerdictStatus> {
        self.checklist
            .iter()
            .find(|verdict| verdict.id == OVERALL_VERDICT_ID)
            .map(|verdict| verdict.status)
            .filter(|status| status.is_grade())
    }

    /// Render this review to its canonical `writeup.md` file contents: a
    /// frontmatter block of per-domain `rating.<domain>` lines and the bare
    /// run-wide `aesthetic: <tier>` line, followed by the body. Only the bare
    /// aesthetic form is ever emitted — a legacy file's per-domain lines were
    /// collapsed at parse time.
    ///
    /// Reconstructing the file from the parsed parts normalizes whatever spacing
    /// the author used, so every published writeup has identical framing.
    ///
    /// Checklist verdicts follow the ratings in the frontmatter, one per line as
    /// `review.<id>: <status> [note]`. A note is normalized to a single line so a
    /// stray newline can never break the frontmatter block.
    pub fn to_file_string(&self) -> String {
        let mut frontmatter = String::new();
        for domain in &self.ratings {
            frontmatter.push_str(&format!(
                "rating.{}: {}\n",
                domain.domain,
                domain.rating.as_str()
            ));
        }
        if let Some(aesthetic) = self.aesthetic {
            frontmatter.push_str(&format!("aesthetic: {}\n", aesthetic.as_str()));
        }
        for verdict in &self.checklist {
            frontmatter.push_str(&format!(
                "review.{}: {}",
                verdict.id,
                verdict.status.as_str()
            ));
            if let Some(note) = &verdict.note {
                let note = note.split_whitespace().collect::<Vec<_>>().join(" ");
                if !note.is_empty() {
                    frontmatter.push(' ');
                    frontmatter.push_str(&note);
                }
            }
            frontmatter.push('\n');
        }
        format!("---\n{frontmatter}---\n\n{}\n", self.body)
    }
}

/// Parse a `writeup.md` file: its per-domain `rating.<domain>` and run-wide
/// `aesthetic` frontmatter and its prose body.
///
/// The file must open with a `---` fenced YAML frontmatter block carrying at
/// least one of a `rating.<domain>` (a [`Rating`] tier), an `aesthetic` (an
/// [`AestheticRating`] tier), or a `review.<id>` checklist verdict, and must
/// have a non-empty body after the frontmatter. Anything else is an
/// [`WriteupError`] explaining what was missing — this is what the publish gate
/// reports.
pub fn parse_writeup(raw: &str) -> Result<Writeup, WriteupError> {
    let (frontmatter, body) = split_frontmatter(raw)?;

    let ratings = parse_ratings(frontmatter)?;
    let aesthetic = parse_aesthetic(frontmatter)?;
    let checklist = parse_checklist(frontmatter)?;
    // A legacy domain-scored case rates at least one `rating.<domain>`; a
    // validator-rated run's review rates the run-wide `aesthetic`; a game jam
    // rates neither but records its graded categories and overall mark as
    // `review.<id>` verdicts. A writeup carrying none is empty of judgement and
    // rejected.
    if ratings.is_empty() && aesthetic.is_none() && checklist.is_empty() {
        return Err(WriteupError(
            "writeup frontmatter is missing a `rating.<domain>` entry, an `aesthetic` tier, or a \
             `review.<id>` verdict"
                .to_string(),
        ));
    }

    let body = body.trim();
    if body.is_empty() {
        return Err(WriteupError(
            "writeup has no body — add the report prose after the frontmatter".to_string(),
        ));
    }

    Ok(Writeup {
        ratings,
        aesthetic,
        body: body.to_string(),
        checklist,
    })
}

/// Parse the per-domain ratings from a frontmatter block: every `rating.<domain>`
/// line, in order. The value must be one of the [`Rating`] tiers. An empty domain
/// id or an unrecognized tier is a [`WriteupError`] so a malformed rating is
/// reported rather than silently dropped.
fn parse_ratings(frontmatter: &str) -> Result<Vec<DomainRating>, WriteupError> {
    let mut ratings = Vec::new();
    for line in frontmatter.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let Some(domain) = name.trim().strip_prefix("rating.") else {
            continue;
        };
        let domain = domain.trim();
        if domain.is_empty() {
            return Err(WriteupError(
                "writeup has a `rating.` line with an empty domain id".to_string(),
            ));
        }
        let rating = Rating::parse(value).ok_or_else(|| {
            WriteupError(format!(
                "writeup `rating.{domain}` must be one of flawless, great, passable, scuffed, \
                 broken (got `{}`)",
                value.trim()
            ))
        })?;
        ratings.push(DomainRating {
            domain: domain.to_string(),
            rating,
        });
    }
    Ok(ratings)
}

/// Parse the run-wide aesthetic rating from a frontmatter block: the bare
/// `aesthetic: <tier>` line, plus any legacy per-domain `aesthetic.<domain>`
/// lines, collapsed together to the **worst** tier — which equals the old
/// per-domain aggregation, so a legacy file's displayed value does not change.
/// The value must be one of the [`AestheticRating`] tiers. An empty domain id or
/// an unrecognized tier is a [`WriteupError`] so a malformed rating is
/// reported rather than silently dropped.
fn parse_aesthetic(frontmatter: &str) -> Result<Option<AestheticRating>, WriteupError> {
    let mut tiers = Vec::new();
    for line in frontmatter.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        let key = if name == "aesthetic" {
            "aesthetic".to_string()
        } else if let Some(domain) = name.strip_prefix("aesthetic.") {
            // A legacy per-domain line: parsed for compatibility, collapsed below.
            let domain = domain.trim();
            if domain.is_empty() {
                return Err(WriteupError(
                    "writeup has an `aesthetic.` line with an empty domain id".to_string(),
                ));
            }
            format!("aesthetic.{domain}")
        } else {
            continue;
        };
        let rating = AestheticRating::parse(value).ok_or_else(|| {
            WriteupError(format!(
                "writeup `{key}` must be one of legendary, amazing, good, okay, slop (got `{}`)",
                value.trim()
            ))
        })?;
        tiers.push(rating);
    }
    Ok(AestheticRating::worst(tiers))
}

/// Parse the checklist verdicts from a frontmatter block: every `review.<id>`
/// line, in order. The value's first whitespace-delimited token is the status
/// (`pass` or `fail`) and the remainder, if any, is the reviewer's note. An empty
/// id or an unrecognized status is a [`WriteupError`] so a malformed verdict is
/// reported rather than silently dropped.
fn parse_checklist(frontmatter: &str) -> Result<Vec<ReviewVerdict>, WriteupError> {
    let mut verdicts = Vec::new();
    for line in frontmatter.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let Some(id) = name.trim().strip_prefix("review.") else {
            continue;
        };
        let id = id.trim();
        if id.is_empty() {
            return Err(WriteupError(
                "writeup has a `review.` checklist line with an empty item id".to_string(),
            ));
        }
        let value = value.trim();
        let (status_token, note) = match value.split_once(char::is_whitespace) {
            Some((status, rest)) => (status, rest.trim()),
            None => (value, ""),
        };
        let status = VerdictStatus::parse(status_token).ok_or_else(|| {
            WriteupError(format!(
                "writeup checklist item `{id}` has status `{status_token}`; \
                 expected pass or fail"
            ))
        })?;
        verdicts.push(ReviewVerdict {
            id: id.to_string(),
            status,
            note: (!note.is_empty()).then(|| note.to_string()),
        });
    }
    Ok(verdicts)
}

/// Split a Markdown document into its leading `---` frontmatter block and the
/// body that follows. Returns a [`WriteupError`] when no frontmatter is found.
fn split_frontmatter(raw: &str) -> Result<(&str, &str), WriteupError> {
    // Tolerate a UTF-8 BOM and leading blank lines before the opening fence.
    let trimmed = raw.trim_start_matches('\u{feff}').trim_start();
    let after_open = trimmed
        .strip_prefix("---\n")
        .or_else(|| trimmed.strip_prefix("---\r\n"))
        .ok_or_else(|| {
            WriteupError(
                "writeup is missing its `---` frontmatter block with a `rating` or `aesthetic`"
                    .to_string(),
            )
        })?;

    // The closing fence is a line that is exactly `---`. Walk the lines tracking
    // each one's byte offset so the body can be split off after the fence.
    let mut offset = 0;
    for line in after_open.split_inclusive('\n') {
        if line.trim_end_matches('\n').trim_end_matches('\r') == "---" {
            let frontmatter = &after_open[..offset];
            let body = &after_open[offset + line.len()..];
            return Ok((frontmatter, body));
        }
        offset += line.len();
    }

    Err(WriteupError(
        "writeup frontmatter is not closed with a `---` line".to_string(),
    ))
}
