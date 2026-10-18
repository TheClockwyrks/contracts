//! The color parsers a test case manifest is checked with.
//!
//! A manifest names a background in four places: a drawing case's
//! `[canvas] background`, and the `background` of a voxel, material or particle
//! case's rendered preview. Resolution refuses a value the tool that later renders
//! it would refuse, so an authoring mistake costs a `tcab validate` rather than a
//! run. The tools hold their own parsers (the drawing tool's `Background`, the
//! model core's `PreviewBackground`), because they turn the value into pixels;
//! these are the same grammars, read only to accept or refuse a manifest's value.
//! Each refusal reads exactly as the tool's does.
//!
//! - A canvas background is `transparent` or a `#rrggbb` / `#rrggbbaa` hex color:
//!   a drawn sprite may start translucent.
//! - A preview background is `transparent` or an opaque `#rrggbb` hex color: the
//!   volume behind a voxel, material or particle preview carries no alpha, so a
//!   `#rrggbbaa` form is refused rather than silently truncated.
//!
//! The literal `transparent` is matched without regard to case, and so are the hex
//! digits; the leading `#` is required.

use thiserror::Error;

/// A background value a manifest declares that its grammar refuses.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ColorError {
    /// A canvas background that is neither `transparent` nor a `#rrggbb` or
    /// `#rrggbbaa` hex color.
    #[error("invalid color `{0}` (expected `#rrggbb`, `#rrggbbaa`, or `transparent`)")]
    Canvas(String),
    /// A preview background that is neither `transparent` nor an opaque `#rrggbb`
    /// hex color.
    #[error("invalid color `{0}` (expected `#rrggbb` or `transparent`)")]
    Preview(String),
}

/// A drawing case's `[canvas] background`: what every pixel starts as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasBackground {
    /// Every pixel starts fully transparent.
    Transparent,
    /// Every pixel starts as this straight (non-premultiplied) RGBA color.
    Color([u8; 4]),
}

impl CanvasBackground {
    /// Parse a canvas background: the literal `transparent`, a `#rrggbb` color
    /// (opaque), or a `#rrggbbaa` color.
    pub fn parse(value: &str) -> Result<Self, ColorError> {
        if value.eq_ignore_ascii_case("transparent") {
            return Ok(Self::Transparent);
        }
        let invalid = || ColorError::Canvas(value.to_string());
        match hex_bytes(value).ok_or_else(invalid)?.as_slice() {
            [r, g, b] => Ok(Self::Color([*r, *g, *b, 0xff])),
            [r, g, b, a] => Ok(Self::Color([*r, *g, *b, *a])),
            _ => Err(invalid()),
        }
    }
}

/// The background behind a rendered voxel, material or particle preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewBackground {
    /// The preview's empty area is fully transparent.
    Transparent,
    /// The preview's empty area is this opaque RGB color.
    Color([u8; 3]),
}

impl PreviewBackground {
    /// Parse a preview background: the literal `transparent` or an opaque
    /// `#rrggbb` color. A `#rrggbbaa` color is refused.
    pub fn parse(value: &str) -> Result<Self, ColorError> {
        if value.eq_ignore_ascii_case("transparent") {
            return Ok(Self::Transparent);
        }
        let invalid = || ColorError::Preview(value.to_string());
        match hex_bytes(value).ok_or_else(invalid)?.as_slice() {
            [r, g, b] => Ok(Self::Color([*r, *g, *b])),
            _ => Err(invalid()),
        }
    }
}

/// The bytes a `#`-prefixed hex color spells, or `None` when the value has no
/// leading `#`, holds a character that is not a hex digit, or has an odd number
/// of digits.
fn hex_bytes(value: &str) -> Option<Vec<u8>> {
    let hex = value.strip_prefix('#')?;
    if hex.is_empty() || hex.len() % 2 != 0 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
#[path = "color.test.rs"]
mod tests;
