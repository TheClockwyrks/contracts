//! Serving one file of a static build under a per-run base path.
//!
//! A Vite build emitted with the default `base: "/"` references its assets
//! absolutely (`/assets/index-….js`), which would escape a per-run base such as
//! `/runs/<id>/build/` and 404. So an HTML document is relocated under the base it
//! is served at: a `<base href>` is injected and root-relative `src`/`href`
//! references are de-absolutized so the injected base applies (a `<base href>` does
//! not affect already-absolute `/…` URLs). Every other asset is served
//! byte-for-byte.
//!
//! The rule is shared by every side that serves a build under a sub-path: the
//! artifacts service serves a run's build, and the backend serves a suite's
//! reference builds.

use std::path::{Path, PathBuf};

/// A file resolved from a build, ready to write to an HTTP or IPC response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServedBuildFile {
    /// The `Content-Type` to send, derived from the file extension.
    pub content_type: &'static str,
    /// The file body, with HTML relocated under the build base (see module docs).
    pub body: Vec<u8>,
}

/// Resolve and read one file from a run's build directory for serving under
/// `base_href` — the per-run URL path the build is mounted at (with a trailing
/// slash), e.g. `/runs/<id>/build/`.
///
/// `rel_path` is the request path *within* the build, without a leading slash; an
/// empty path or one naming a directory resolves to that directory's
/// `index.html`. HTML responses are relocated under `base_href`; every other
/// asset is returned verbatim. Returns `None` when the file does not exist or the
/// path escapes the build directory (a traversal attempt) — the caller maps that
/// to a 404.
pub fn serve_build_file(
    build_dir: &Path,
    rel_path: &str,
    base_href: &str,
) -> Option<ServedBuildFile> {
    let target = resolve_within(build_dir, rel_path)?;
    let bytes = std::fs::read(&target).ok()?;
    let content_type = content_type_for(&target.to_string_lossy());
    let body = if content_type.starts_with("text/html") {
        rewrite_html_base(&bytes, base_href)
    } else {
        bytes
    };
    Some(ServedBuildFile { content_type, body })
}

/// Normalize a request path *within* a build into the relative path it names,
/// segments joined by `/`, or `None` when it would escape the build (a `..`
/// segment, refused outright rather than resolved).
///
/// Empty segments (a leading or trailing slash, a `//`) and `.` are skipped, so
/// `""`, `"/"` and `"./"` all normalize to `""`, the build root. The rule is
/// pure: it reads no file system, so a store that keeps a build as objects
/// rather than files applies the same traversal guard to its keys that
/// [`serve_build_file`] applies to paths. Which file a directory request serves
/// (its `index.html`) is left to the caller, which alone knows what exists.
pub fn normalize_build_path(rel_path: &str) -> Option<String> {
    let mut segments: Vec<&str> = Vec::new();
    for segment in rel_path.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            return None;
        }
        segments.push(segment);
    }
    Some(segments.join("/"))
}

/// Resolve a request path to a real file inside `build_dir`, refusing anything
/// that would escape it. An empty path or a directory resolves to `index.html`.
fn resolve_within(build_dir: &Path, rel_path: &str) -> Option<PathBuf> {
    let mut path = build_dir.to_path_buf();
    // The lexical guard first: `..` is refused rather than relying on
    // canonicalization alone to catch traversal.
    let normalized = normalize_build_path(rel_path)?;
    for segment in normalized.split('/').filter(|s| !s.is_empty()) {
        path.push(segment);
    }
    // A directory request (including the build root) serves its index.html.
    if path.is_dir() {
        path.push("index.html");
    }
    // Defense in depth: the resolved real path must stay within the build dir, so
    // a symlink inside the build cannot point the response outside it.
    let base = build_dir.canonicalize().ok()?;
    let resolved = path.canonicalize().ok()?;
    resolved.starts_with(&base).then_some(resolved)
}

/// The `Content-Type` for a built asset, by the extension of its name. Covers
/// what a Vite game build emits (the entry HTML and JS, plus any bundled CSS,
/// wasm, images, fonts and audio); anything unrecognized falls back to a binary
/// stream.
///
/// `name` is a file name or a `/`-separated relative path (an object key within
/// a build); only its last segment is read. The extension is what follows the
/// last `.` of that segment, as [`Path::extension`] reads it: a leading dot
/// alone (`.htaccess`) names no extension. Matching ignores case.
pub fn content_type_for(name: &str) -> &'static str {
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    match ext.as_deref() {
        Some("html" | "htm") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json" | "map") => "application/json; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        Some("mp3") => "audio/mpeg",
        Some("ogg") => "audio/ogg",
        Some("wav") => "audio/wav",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// Relocate an HTML document under `base_href`: inject a `<base href>` at the
/// start of `<head>` and de-absolutize root-relative `src`/`href` references so
/// the base applies to them. Non-UTF-8 input is returned unchanged.
///
/// `base_href` is the URL path the build is mounted at, with a trailing slash,
/// e.g. `/runs/<id>/build/`. This is the rewrite [`serve_build_file`] applies
/// to an HTML response, public so a server reading the build from somewhere
/// other than a directory relocates it the same way.
pub fn rewrite_html_base(bytes: &[u8], base_href: &str) -> Vec<u8> {
    let Ok(html) = std::str::from_utf8(bytes) else {
        return bytes.to_vec();
    };
    // De-absolutize the document's own references first, then inject the base —
    // injecting first would let de-absolutization strip the leading slash off the
    // base tag's own (intentionally absolute) `href`.
    let deabsolutized = deabsolutize(html);
    let base_tag = format!("<base href=\"{base_href}\">");
    match head_insert_index(&deabsolutized) {
        Some(idx) => {
            let mut out = String::with_capacity(deabsolutized.len() + base_tag.len());
            out.push_str(&deabsolutized[..idx]);
            out.push_str(&base_tag);
            out.push_str(&deabsolutized[idx..]);
            out.into_bytes()
        }
        // No <head> to anchor to: prepend the base so relative refs still resolve.
        None => format!("{base_tag}{deabsolutized}").into_bytes(),
    }
}

/// The byte offset just after the opening `<head …>` tag, or `None` when the
/// document has no head.
fn head_insert_index(html: &str) -> Option<usize> {
    let lower = html.to_ascii_lowercase();
    let head = lower.find("<head")?;
    let close = lower[head..].find('>')?;
    Some(head + close + 1)
}

/// Strip the leading slash from root-relative `src`/`href` attribute values so a
/// `<base href>` resolves them under the build path. Protocol-relative `//host`
/// references and absolute URLs with a scheme are left untouched.
fn deabsolutize(html: &str) -> String {
    const PREFIXES: [&str; 4] = ["src=\"/", "src='/", "href=\"/", "href='/"];
    let bytes = html.as_bytes();
    let mut out = String::with_capacity(html.len());
    let mut i = 0;
    'scan: while i < bytes.len() {
        for prefix in PREFIXES {
            let pat = prefix.as_bytes();
            if bytes[i..].starts_with(pat) {
                // Leave protocol-relative `//` alone — only a single leading
                // slash is a root-relative path to de-absolutize.
                let after = i + pat.len();
                if bytes.get(after) != Some(&b'/') {
                    // Emit the attribute prefix without its trailing slash, then
                    // resume just past the slash we dropped.
                    out.push_str(&prefix[..prefix.len() - 1]);
                    i = after;
                    continue 'scan;
                }
            }
        }
        // Copy one whole UTF-8 character so multibyte content survives intact.
        let len = utf8_len(bytes[i]);
        out.push_str(&html[i..i + len]);
        i += len;
    }
    out
}

/// The byte length of a UTF-8 character from its leading byte.
fn utf8_len(lead: u8) -> usize {
    match lead {
        b if b < 0x80 => 1,
        b if b >> 5 == 0b110 => 2,
        b if b >> 4 == 0b1110 => 3,
        b if b >> 3 == 0b11110 => 4,
        // A stray continuation byte is copied alone rather than panicking; the
        // input came from `from_utf8` so this is unreachable in practice.
        _ => 1,
    }
}

#[cfg(test)]
#[path = "layout.build.test.rs"]
mod tests;
