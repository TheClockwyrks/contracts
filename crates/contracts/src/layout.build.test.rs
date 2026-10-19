//! Tests for serving a static build under a per-run base path.

use super::*;
use std::fs;
use tempfile::TempDir;

/// Build a fake implementation directory containing one named build output with
/// the given files (path within the build → contents).
fn impl_with_build(output: &str, files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    let build = dir.path().join("implementation").join(output);
    for (rel, contents) in files {
        let target = build.join(rel);
        fs::create_dir_all(target.parent().unwrap()).expect("create dirs");
        fs::write(target, contents).expect("write file");
    }
    dir
}

#[test]
fn empty_path_serves_index_html() {
    let dir = impl_with_build("dist", &[("index.html", "<html><head></head></html>")]);
    let build = dir.path().join("implementation").join("dist");
    let served = serve_build_file(&build, "", "/runs/abc/build/").expect("index served");
    assert!(served.content_type.starts_with("text/html"));
    assert!(String::from_utf8(served.body).unwrap().contains("<base"));
}

#[test]
fn html_is_relocated_under_the_base() {
    let html = "<html><head>\n<script type=\"module\" src=\"/assets/index-AAA.js\"></script>\
        </head><body></body></html>";
    let dir = impl_with_build("dist", &[("index.html", html)]);
    let build = dir.path().join("implementation").join("dist");
    let served = serve_build_file(&build, "index.html", "/runs/abc/build/").unwrap();
    let body = String::from_utf8(served.body).unwrap();
    // The base is injected and the absolute asset reference is de-absolutized so
    // the base applies to it.
    assert!(body.contains("<base href=\"/runs/abc/build/\">"));
    assert!(body.contains("src=\"assets/index-AAA.js\""));
    assert!(!body.contains("src=\"/assets/index-AAA.js\""));
}

#[test]
fn protocol_relative_and_scheme_urls_are_left_alone() {
    let html = "<head></head><body>\
        <img src=\"//cdn.example/x.png\"><a href=\"https://example.com\">x</a></body>";
    let dir = impl_with_build("dist", &[("index.html", html)]);
    let build = dir.path().join("implementation").join("dist");
    let body = serve_build_file(&build, "", "/b/").unwrap().body;
    let body = String::from_utf8(body).unwrap();
    assert!(body.contains("src=\"//cdn.example/x.png\""));
    assert!(body.contains("href=\"https://example.com\""));
}

#[test]
fn non_html_assets_are_served_verbatim_with_a_mime_type() {
    let dir = impl_with_build(
        "dist",
        &[("assets/index-AAA.js", "console.log('/not/rewritten')")],
    );
    let build = dir.path().join("implementation").join("dist");
    let served = serve_build_file(&build, "assets/index-AAA.js", "/b/").unwrap();
    assert!(served.content_type.starts_with("text/javascript"));
    // JS is byte-for-byte: an absolute path in a string literal is not touched.
    assert_eq!(served.body, b"console.log('/not/rewritten')");
}

#[test]
fn traversal_outside_the_build_is_refused() {
    let dir = impl_with_build("dist", &[("index.html", "<html></html>")]);
    // A secret beside (not inside) the build.
    fs::write(dir.path().join("implementation").join("secret"), "nope").unwrap();
    let build = dir.path().join("implementation").join("dist");
    assert_eq!(serve_build_file(&build, "../secret", "/b/"), None);
}

#[test]
fn missing_file_is_none() {
    let dir = impl_with_build("dist", &[("index.html", "<html></html>")]);
    let build = dir.path().join("implementation").join("dist");
    assert_eq!(serve_build_file(&build, "does-not-exist.js", "/b/"), None);
}

#[test]
fn normalize_build_path_skips_empty_and_dot_segments() {
    assert_eq!(normalize_build_path(""), Some(String::new()));
    assert_eq!(normalize_build_path("/"), Some(String::new()));
    assert_eq!(normalize_build_path("./"), Some(String::new()));
    assert_eq!(
        normalize_build_path("/assets//./index-AAA.js"),
        Some("assets/index-AAA.js".to_owned())
    );
    assert_eq!(normalize_build_path("sub/"), Some("sub".to_owned()));
}

#[test]
fn normalize_build_path_refuses_any_parent_segment() {
    assert_eq!(normalize_build_path(".."), None);
    assert_eq!(normalize_build_path("../secret"), None);
    // Refused outright even when the result would stay inside the build.
    assert_eq!(normalize_build_path("assets/../index.html"), None);
    // A dotted name that is not exactly `..` is an ordinary segment.
    assert_eq!(normalize_build_path("..a/b.."), Some("..a/b..".to_owned()));
}

#[test]
fn content_type_for_reads_the_extension_of_the_last_segment() {
    assert_eq!(content_type_for("index.html"), "text/html; charset=utf-8");
    assert_eq!(
        content_type_for("assets/index-AAA.JS"),
        "text/javascript; charset=utf-8"
    );
    assert_eq!(content_type_for("game.wasm"), "application/wasm");
    // The extension is the last one, and a lone leading dot names none.
    assert_eq!(content_type_for("data.json.gz"), "application/octet-stream");
    assert_eq!(content_type_for(".htaccess"), "application/octet-stream");
    assert_eq!(
        content_type_for("dir.html/LICENSE"),
        "application/octet-stream"
    );
    assert_eq!(content_type_for(""), "application/octet-stream");
}

#[test]
fn rewrite_html_base_is_the_rewrite_serve_build_file_applies() {
    let html = "<!doctype html><html><head lang=\"en\">\
        <link href=\"/style.css\"><script src='/main.js'></script></head></html>";
    let dir = impl_with_build("dist", &[("index.html", html)]);
    let build = dir.path().join("implementation").join("dist");
    let served = serve_build_file(&build, "", "/runs/abc/build/").unwrap();
    let rewritten = rewrite_html_base(html.as_bytes(), "/runs/abc/build/");
    assert_eq!(served.body, rewritten);
    assert_eq!(
        String::from_utf8(rewritten).unwrap(),
        "<!doctype html><html><head lang=\"en\"><base href=\"/runs/abc/build/\">\
        <link href=\"style.css\"><script src='main.js'></script></head></html>"
    );
}

#[test]
fn rewrite_html_base_prepends_without_a_head_and_keeps_non_utf8() {
    assert_eq!(
        rewrite_html_base(b"<p><img src=\"/a.png\"></p>", "/b/"),
        b"<base href=\"/b/\"><p><img src=\"a.png\"></p>".to_vec()
    );
    let invalid = [0xff, 0xfe, b'<', b'h'];
    assert_eq!(rewrite_html_base(&invalid, "/b/"), invalid.to_vec());
}
