//! Tests for the run-tree layout rules: where a run's static build is found and
//! how a proof is named.

use std::fs;

use tempfile::TempDir;

use super::*;

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
fn find_build_output_prefers_dist_then_build_then_out() {
    let dir = impl_with_build("out", &[("index.html", "<html></html>")]);
    let impl_dir = dir.path().join("implementation");
    assert_eq!(find_build_output(&impl_dir), Some(impl_dir.join("out")));

    // `dist` wins when several exist, matching BUILD_OUTPUTS priority.
    fs::create_dir_all(impl_dir.join("build")).unwrap();
    fs::create_dir_all(impl_dir.join("dist")).unwrap();
    assert_eq!(find_build_output(&impl_dir), Some(impl_dir.join("dist")));
}

#[test]
fn find_build_output_is_none_without_a_build() {
    let dir = TempDir::new().unwrap();
    let impl_dir = dir.path().join("implementation");
    fs::create_dir_all(&impl_dir).unwrap();
    assert_eq!(find_build_output(&impl_dir), None);
}

#[test]
fn published_extension_is_mp4_for_video_and_verbatim_for_images() {
    // The public snapshot serves a video proof as mp4 (transcoded from webm)
    // regardless of the captured extension; an image keeps its own extension.
    assert_eq!(
        proof_published_extension(MediaKind::Video, "proof/rally.webm"),
        "mp4"
    );
    assert_eq!(
        proof_published_extension(MediaKind::Video, "proof/rally.mp4"),
        "mp4"
    );
    assert_eq!(
        proof_published_extension(MediaKind::Image, "proof/title.png"),
        "png"
    );
}

#[test]
fn a_proof_is_served_under_its_dests_lowercased_extension() {
    assert_eq!(proof_served_extension("proof/Title.PNG"), "png");
    assert_eq!(proof_served_extension("shots/rally.json.gz"), "gz");
    assert_eq!(proof_served_extension("proof/rally.webm"), "webm");
}

#[test]
fn a_dest_with_no_usable_extension_is_served_as_png() {
    assert_eq!(proof_served_extension("proof/title"), "png");
    assert_eq!(proof_served_extension("proof/.hidden"), "png");
    assert_eq!(proof_served_extension("proof/trailing."), "png");
}

#[test]
fn a_proof_is_labelled_from_its_dest_unless_the_dest_has_no_extension() {
    assert_eq!(
        proof_labelled_name("shots/rally.json.gz", "rally.gz"),
        "rally.json.gz"
    );
    assert_eq!(proof_labelled_name("shots/rally", "rally.png"), "rally.png");
}
