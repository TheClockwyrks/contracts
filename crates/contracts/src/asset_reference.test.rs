//! Tests for the published reference frames' object-key layout.

use super::*;

#[test]
fn image_keys_round_trip_through_the_parser() {
    let key = reference_image_key("lattice-belt", "v1.0.0", "base", 3);
    assert_eq!(
        key,
        "media/references/lattice-belt/v1.0.0/base/frames/3.png"
    );
    assert_eq!(
        parse_reference_image_key(&key),
        Some(ReferenceMediaKey {
            slug: "lattice-belt".to_string(),
            version: "v1.0.0".to_string(),
            variant: "base".to_string(),
            index: 3,
        })
    );
}

#[test]
fn only_image_keys_parse() {
    // Action logs share the prefix but are not what proves a frame was published,
    // so the discovery side must skip them rather than count them as frames.
    assert_eq!(
        parse_reference_image_key(&reference_actions_key("lattice-belt", "v1.0.0", "base", 3)),
        None
    );
    // Run media lives under a sibling prefix and must never be mistaken for a
    // reference.
    assert_eq!(
        parse_reference_image_key("media/runs/abc/asset/regenerated-3.png"),
        None
    );
    // Structurally wrong keys under the right prefix are rejected, not guessed at.
    assert_eq!(
        parse_reference_image_key("media/references/slug/v1/base/3.png"),
        None
    );
    assert_eq!(
        parse_reference_image_key("media/references/slug/v1/base/frames/notanumber.png"),
        None
    );
}
