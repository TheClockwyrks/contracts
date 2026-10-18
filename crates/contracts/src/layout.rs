//! Layout: the names of the files and directories a run tree, a case version and
//! the test-cases catalog are made of, and the bounds on what a showcase may hold.
//!
//! Each is read on more than one side: by the run that writes the tree, by the
//! validator that reads it back, by the backend and the snapshot builder that serve
//! and publish it, and by the content digest that decides whether it changed. One
//! definition here keeps them agreeing, along with the rules that find a run's
//! static build and name a proof's served file.

use std::path::{Path, PathBuf};

use crate::test_case::MediaKind;

// --- showcase bounds ---------------------------------------------------------

/// The largest showcase description, in bytes. The description is a store-page
/// blurb; this cap keeps a pathological one from bloating every store downstream.
/// The run-side capture truncates a longer one on a char boundary with a trailing
/// marker; the case-side resolution (an authored, committed showcase — see
/// `test_case::Variant::showcase`) hard-fails instead. Public because the bound is
/// shared by all three showcases, the suite's
/// included (`test_suite::ShowcaseManifest`), and The Spec Cabinet enforces it on
/// the one it authors.
pub const MAX_SHOWCASE_DESCRIPTION_BYTES: usize = 64 * 1024;

/// The most media entries a showcase carousel may hold. The run-side capture
/// drops the excess with a warning — the carousel is a highlight reel, not an
/// archive — while the case-side resolution hard-fails on it. Public for the
/// reason [`MAX_SHOWCASE_DESCRIPTION_BYTES`] is: the bound is shared by all three
/// showcases, the suite's included.
pub const MAX_SHOWCASE_MEDIA_ENTRIES: usize = 10;

/// The largest media file a showcase carousel entry may name, in bytes. An entry
/// naming a larger file is dropped with a warning, since the file travels the
/// per-run media path and a pathological one would bloat every store downstream.
/// Public because the driver's backend-store mirror applies the same cap to the
/// directory it uploads, so a file the capture refused never ships either.
pub const MAX_SHOWCASE_MEDIA_FILE_BYTES: u64 = 25 * 1024 * 1024;

// --- build outputs -----------------------------------------------------------

/// Candidate static build-output directory names a run's implementation may
/// produce, in priority order. The validator builds into whichever a project's
/// tooling is configured for; `dist` is Vite's default. Shared so the publish
/// path (which deploys the build) and the local-serving path agree on what a
/// build output is.
///
/// Public also because the code analyzer (`test_cabinet_driver::analyzer`) removes
/// exactly these names from the tree it measures, and the two must agree: a
/// directory the validator will serve a build out of is, by definition, build output
/// rather than code the model wrote. One list, read from every side, so adding a
/// fourth cannot silently start counting it.
pub const BUILD_OUTPUTS: [&str; 3] = ["dist", "build", "out"];

/// Find a deployable static build output beside a run's implementation, if one
/// was produced. Returns the first of [`BUILD_OUTPUTS`] that exists as a
/// directory, or `None` when the run produced no static build.
pub fn find_build_output(impl_dir: &Path) -> Option<PathBuf> {
    BUILD_OUTPUTS
        .iter()
        .map(|name| impl_dir.join(name))
        .find(|candidate| candidate.is_dir())
}

/// Serving one file of a build under the per-run base path it is mounted at; see
/// [`serve_build_file`].
#[path = "layout.build.rs"]
mod build;
pub use build::{ServedBuildFile, serve_build_file};

// --- proof naming ------------------------------------------------------------

/// The name a proof's bytes are labelled from: its recorded `dest`, falling back to
/// the requested name when the dest carries no usable extension.
///
/// A proof is *addressed* as `<proof-id>.<ext>`, and that name keeps only the last
/// extension — `<proof-id>.gz` for a `dest` of `shots/rally.json.gz`, which cannot
/// say whether the gzip frames a document or is one. The dest carries the whole
/// compound suffix, so the labels come from it (see the suites crate's `content_labels`).
pub fn proof_labelled_name<'a>(dest: &'a str, file: &'a str) -> &'a str {
    let base = dest.rsplit('/').next().unwrap_or(dest);
    match base.rfind('.') {
        Some(dot) if dot > 0 && dot + 1 < base.len() => base,
        _ => file,
    }
}

/// The file extension a proof is served under, derived from its `dest` path: the
/// lowercased extension of the dest's final path segment, defaulting to `png` when
/// there is none usable (a leading- or trailing-dot segment, or no dot at all).
///
/// A proof is addressed everywhere as `<proof-id>.<ext>` — this computes the `<ext>`
/// from the recorded `dest` so the served name lines up across the producers and
/// consumers of that name: the driver's backend-store mirror, the backend snapshot
/// builder, and the gallery's `extensionFor` (`packages/ui/.../proofMedia.ts`). The
/// extension is cosmetic (the file is resolved by its proof-id stem), but it must
/// agree on all sides or a snapshot key misses the URL the UI requests.
pub fn proof_served_extension(dest: &str) -> String {
    let base = dest.rsplit('/').next().unwrap_or(dest);
    match base.rfind('.') {
        Some(dot) if dot > 0 && dot + 1 < base.len() => base[dot + 1..].to_ascii_lowercase(),
        _ => "png".to_string(),
    }
}

/// The file extension a proof is served under **in the public snapshot** — as
/// opposed to [`proof_served_extension`], which is how it is served live from the
/// produced tree.
///
/// They differ for video only. A run captures its clip as the `.webm` Playwright
/// records natively (the single format a case's `dest` names), which the live
/// console serves verbatim; but the snapshot builder transcodes it to H.264
/// `.mp4` so the public gallery plays on every browser (webm/VP8 does not on
/// iOS/Safari). So a `Video` proof publishes as `mp4` regardless of its `dest`,
/// while an `Image` or `Replay` proof publishes under its recorded extension
/// unchanged — there is no second format either of those is converted into.
pub fn proof_published_extension(kind: MediaKind, dest: &str) -> String {
    match kind {
        MediaKind::Video => "mp4".to_string(),
        MediaKind::Image | MediaKind::Replay => proof_served_extension(dest),
    }
}

// --- reference builds --------------------------------------------------------

/// The filename, relative to the test-cases catalog root, the CLI writes and the
/// backend reads. It lives beside the catalog (not under a version folder) because
/// its entries span every case, version, and environment.
pub const REFERENCE_LOCK_FILENAME: &str = "reference-builds.lock.json";

// --- validation media and scripts --------------------------------------------

/// The run-root-relative directory synthesized *actual* validation media is
/// collected under, so it travels with the published implementation and is served
/// by `test_cabinet_artifacts::playable::serve_validation_file`.
pub const VALIDATION_MEDIA_DIR: &str = ".vendor/validation";

/// The directory a case version's **baseline** validation media lives under, one
/// sub-directory per engine and variant: `validation-baseline/<engine>/<variant>/`.
/// Synthesized once at capture-baselines time from the reference implementation and
/// committed to the cold-storage submodule beneath the version's mirrored path (see
/// [`crate::ColdStorage::validation_baseline_dir`]). Ingest copies it into the stored
/// version under this same name, and the backend serves it case-scoped from there —
/// the invariant counterpart to the per-run `VALIDATION_MEDIA_DIR` *actual* media.
///
/// The engine comes first because it is what makes two captures of the same variant
/// different media: a variant has one reference implementation PER ENGINE, and the
/// two draw the same game through different runtimes, so their recordings are not
/// interchangeable. A reviewer comparing a `simple-2d` run against an engineless
/// build's frames would be shown a difference between two runtimes and read it as a
/// difference in the build.
pub const VALIDATION_BASELINE_DIR: &str = "validation-baseline";

/// The version-folder-relative directory a case's reporter-side automated-validation
/// debug scripts live under (`validation/<item>.mjs`, plus any shared modules those
/// scripts import — e.g. `validation/_helpers.mjs`). Reporter-side and **never**
/// seeded into the model's run container; the whole directory is materialized into a
/// backend-driven run's definition store (see `test_cabinet_driver::materialize_version`) so a
/// script's sibling imports resolve when the validator runs it.
pub const VALIDATION_SCRIPT_DIR: &str = "validation";

/// The driver's `--outputs` kind tag for a media kind.
pub fn media_kind_tag(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Image => "image",
        MediaKind::Video => "video",
        MediaKind::Replay => "replay",
    }
}

/// The file extension a synthesized output is captured under, by kind: a still is a
/// PNG, a clip is the `.webm` Playwright records natively, and a draw-command
/// recording is the gzipped JSON document the engine's recorder hands back.
///
/// A recording is stored compressed because its format is deliberately repetitive.
/// Every frame restates the drawing state it inherited so that any frame can be
/// drawn without drawing the frames before it, and consecutive frames of a game
/// issue very nearly the same operations as each other. That redundancy is what
/// makes seeking and side-by-side scrubbing work at all, and it is also exactly
/// what gzip removes: a real capture stores tens of times smaller, which is the
/// difference between a run whose recordings are tens of megabytes and one whose
/// recordings are a few. The name carries both extensions, so what the bytes
/// are and how they are framed are each readable off the file.
pub fn validation_output_extension(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Image => "png",
        MediaKind::Video => "webm",
        MediaKind::Replay => "json.gz",
    }
}

/// The file extension a synthesized validation output is published under **in the
/// public snapshot** — the counterpart to [`validation_output_extension`], which is
/// how it is captured on disk and served live.
///
/// They differ for video only. A clip is captured as the `.webm` Playwright records
/// natively (the on-disk name both the run-scoped *actual* and case-scoped *baseline*
/// media use, and what the live console/artifact service serve verbatim); but the
/// snapshot builder transcodes it to H.264 `.mp4` so the public gallery plays on every
/// browser (webm/VP8 does not on iOS/Safari) — exactly as a video proof is published
/// (see [`proof_published_extension`]). A still publishes as its captured PNG
/// unchanged, and so does a recording: a `.json.gz` document is inflated by the
/// browser and drawn by the console's own player, so there is no format the gallery
/// would need it converted into and no reason to publish it any larger than it is
/// stored.
pub fn validation_published_extension(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Image => "png",
        MediaKind::Video => "mp4",
        MediaKind::Replay => "json.gz",
    }
}

/// The flat, addressable file name a synthesized output is stored and served under:
/// `<verdict>__<output>.<ext>`. Kept flat (one path segment) so it routes through the
/// one-segment `/validation/{file}` endpoints unchanged, and shared with
/// `test_cabinet_artifacts::playable::serve_validation_file` and the gallery URL resolver.
///
/// `verdict_id` is the id of the verdict unit the output backs — a whole item's own id
/// (`<item>`, for an item validated as a whole) or a sub-item's composite id
/// (`<item>.<sub>`, since validation and its proof media attach per sub-item once an
/// item is sub-divided). It contains no `/` (ids are plain slugs joined by a single
/// `.`), so the name stays a single path segment and cannot escape the media directory.
///
/// Both the model's *actual* media (under a run's `VALIDATION_MEDIA_DIR`) and a
/// case's *baseline* media (under the version's [`VALIDATION_BASELINE_DIR`]`/
/// <engine>/<variant>/`) use this same name; the directory, not the name, tells them
/// apart.
pub fn validation_media_name(verdict_id: &str, output_id: &str, kind: MediaKind) -> String {
    let ext = validation_output_extension(kind);
    format!("{verdict_id}__{output_id}.{ext}")
}

/// The prefix every file of a recording's **shared image store** carries:
/// `img.<id>.png` for a bitmap and `img.<id>.bin` for a raw RGBA pixel buffer, where
/// `<id>` is derived from the bytes themselves. Only the first is written today —
/// see [`is_validation_image_name`].
///
/// A draw-command recording's images are the bulk of its weight — PNG payloads that
/// gzip cannot compress — and the same sprite is drawn by dozens of a run's
/// recordings. So the harness writes each *unique* image once as a flat file beside
/// the recordings and the entry inside the document names that file instead of
/// carrying base64 of it. The producer is `@clockwyrks/case-harness`'s
/// `replay/store.ts`, which mirrors this constant as `IMAGE_STORE_PREFIX`; the two
/// spellings must agree, and there is no negotiation between them — a name that does
/// not match here simply does not travel.
///
/// A store file deliberately shares the flat namespace of
/// `validation_media_name`'s `<verdict>__<output>.<ext>`, and can never collide
/// with one: a declared output's name always carries `__`, and a store file's never
/// does. That is what lets it route through the one-segment `/validation/{file}`
/// endpoints, publish through the media paths, and resolve in every console through
/// the very resolver the recording it belongs to came from — with no new route, key
/// shape, or lookup anywhere.
pub const VALIDATION_IMAGE_PREFIX: &str = "img.";

/// Whether `file` names a file of a recording's shared image store rather than a
/// declared output.
///
/// Both publish paths — the driver's mirror into the backend store and the snapshot
/// builder's upload — are driven off the run record's *declared* outputs. A store
/// file is on no record: it backs no verdict and is named by its own bytes, so it
/// has to be recognized off the directory instead, and this is the one place that
/// judgement is written down.
///
/// The extension is part of the check and not decoration. The namespace admits
/// exactly two shapes of bytes — a PNG bitmap and a headerless RGBA buffer — so a
/// name carrying neither extension is not something this side of the contract knows
/// how to serve a content type for, and it is left where it is rather than published
/// as an unlabelled blob. Today only the first is ever written: the harness leaves a
/// pixel buffer inline, where the recording's own gzip compresses raw RGBA far
/// better than a flat file could be served. `.bin` is recognized here because the
/// recording format admits a stored buffer and a player resolves one, so the day
/// that becomes worth writing it travels without this side changing.
pub fn is_validation_image_name(file: &str) -> bool {
    file.starts_with(VALIDATION_IMAGE_PREFIX) && (file.ends_with(".png") || file.ends_with(".bin"))
}

#[cfg(test)]
#[path = "layout.test.rs"]
mod tests;
