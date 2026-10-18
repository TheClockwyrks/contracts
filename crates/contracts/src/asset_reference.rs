//! The object-key layout an asset-generation case's published reference frames
//! live under, `media/references/<slug>/<version>/<variant>/frames/`.
//!
//! The CLI's `publish-asset-reference` writes keys under it and the backend lists
//! the same prefix to learn which references exist, so the writer and the reader
//! read the one definition here and cannot drift apart. Building the reference
//! (running its script through the case's drawing binary) is the driver's.

/// The object-store prefix an asset reference's frames are published under.
///
/// Sits beside the snapshot's `media/runs/…` rather than inside a per-snapshot
/// prefix, because a reference belongs to a case *version*, not to whichever
/// snapshot happened to be current when it was published — the objects must
/// survive the next refresh rewriting `snapshots/…`.
pub const REFERENCE_MEDIA_PREFIX: &str = "media/references";

/// The prefix holding one variant's published reference frames:
/// `media/references/<slug>/<version>/<variant>/`.
///
/// This is the single place the layout is defined. `tcab publish-reference` writes
/// keys under it and the backend lists the same prefix to learn which references
/// exist, so the writer and the reader cannot drift apart.
pub fn reference_prefix(slug: &str, version: &str, variant: &str) -> String {
    format!("{REFERENCE_MEDIA_PREFIX}/{slug}/{version}/{variant}")
}

/// The key of one published frame's rendered image.
pub fn reference_image_key(slug: &str, version: &str, variant: &str, index: u32) -> String {
    format!(
        "{}/frames/{index}.png",
        reference_prefix(slug, version, variant)
    )
}

/// The key of one published frame's recorded action log — kept beside the image so
/// a reviewer can read how the reference was drawn, not just what it looks like.
pub fn reference_actions_key(slug: &str, version: &str, variant: &str, index: u32) -> String {
    format!(
        "{}/frames/{index}.actions.json",
        reference_prefix(slug, version, variant)
    )
}

/// One published reference frame image, recovered from its object key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceMediaKey {
    /// The case slug the reference belongs to.
    pub slug: String,
    /// The case version.
    pub version: String,
    /// The variant slug.
    pub variant: String,
    /// The frame index.
    pub index: u32,
}

/// Recover the case/version/variant/frame a reference **image** key addresses, or
/// `None` when the key is not one (a log, or anything else under the prefix).
///
/// Images alone are parsed because they are what proves a frame was published: the
/// UI needs an image per frame, and a log without one would render nothing.
pub fn parse_reference_image_key(key: &str) -> Option<ReferenceMediaKey> {
    let rest = key
        .strip_prefix(REFERENCE_MEDIA_PREFIX)?
        .strip_prefix('/')?;
    let parts: Vec<&str> = rest.split('/').collect();
    // <slug>/<version>/<variant>/frames/<index>.png
    let [slug, version, variant, "frames", file] = parts.as_slice() else {
        return None;
    };
    let index: u32 = file.strip_suffix(".png")?.parse().ok()?;
    Some(ReferenceMediaKey {
        slug: (*slug).to_string(),
        version: (*version).to_string(),
        variant: (*variant).to_string(),
        index,
    })
}

#[cfg(test)]
#[path = "asset_reference.test.rs"]
mod tests;
