use std::path::PathBuf;

use serde_json::json;

use super::*;

const ENGINE: &str = "@clockwyrks/simple-2d";

fn package_json() -> Value {
    json!({
        "name": "game",
        "private": true,
        "dependencies": {
            ENGINE: "file:./.vendor/packages/@clockwyrks/simple-2d",
            "ts-essentials": "^10.2.1"
        },
        "devDependencies": { "vite": "^8" }
    })
}

/// The lockfile npm writes for [`package_json`].
fn lockfile() -> Value {
    json!({
        "name": "game",
        "lockfileVersion": 3,
        "requires": true,
        "packages": {
            "": {
                "name": "game",
                "dependencies": {
                    ENGINE: "file:./.vendor/packages/@clockwyrks/simple-2d",
                    "ts-essentials": "^10.2.1"
                },
                "devDependencies": { "vite": "^8" }
            },
            ".vendor/packages/@clockwyrks/simple-2d": {
                "name": ENGINE,
                "version": "1.1.1",
                "dependencies": { "ts-essentials": "^10.2.1" }
            },
            "node_modules/@clockwyrks/simple-2d": {
                "resolved": ".vendor/packages/@clockwyrks/simple-2d",
                "link": true
            },
            "node_modules/ts-essentials": { "version": "10.2.1" },
            "node_modules/vite": { "version": "8.0.0", "dev": true }
        }
    })
}

fn check(package_json: &Value, lockfile: &Value) -> Result<(), LockfileError> {
    let manifest = PackageManifest::parse(&package_json.to_string())?;
    Lockfile::parse(&lockfile.to_string())?.check_agrees(&manifest)
}

fn problems(package_json: &Value, lockfile: &Value) -> Vec<String> {
    match check(package_json, lockfile) {
        Err(LockfileError::Disagrees { problems }) => problems,
        other => panic!("expected a disagreement, got {other:?}"),
    }
}

#[test]
fn a_lockfile_npm_wrote_for_the_package_json_agrees_with_it() {
    check(&package_json(), &lockfile()).expect("agrees");
}

#[test]
fn a_dependency_missing_from_the_root_entry_is_named_with_its_spec() {
    let mut lock = lockfile();
    lock["packages"][""]["dependencies"]
        .as_object_mut()
        .unwrap()
        .remove("ts-essentials");
    let problems = problems(&package_json(), &lock);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("`ts-essentials`"), "{problems:?}");
    assert!(problems[0].contains("`^10.2.1`"), "{problems:?}");
    assert!(problems[0].contains("does not"), "{problems:?}");
}

#[test]
fn a_dependency_only_the_lockfile_declares_is_a_disagreement() {
    let mut lock = lockfile();
    lock["packages"][""]["devDependencies"]["eslint"] = json!("^10");
    lock["packages"]["node_modules/eslint"] = json!({ "version": "10.0.0", "dev": true });
    let problems = problems(&package_json(), &lock);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("`eslint`"), "{problems:?}");
    assert!(problems[0].contains("`devDependencies`"), "{problems:?}");
}

#[test]
fn a_dependency_with_a_different_spec_names_both_specs() {
    let mut manifest = package_json();
    manifest["devDependencies"]["vite"] = json!("^9");
    let problems = problems(&manifest, &lockfile());
    assert_eq!(
        problems,
        vec![
            "`devDependencies` declares `vite` as `^9` in package.json but as `^8` in \
             package-lock.json"
                .to_string()
        ]
    );
}

#[test]
fn a_dependency_moved_between_maps_is_a_disagreement() {
    let mut manifest = package_json();
    manifest["devDependencies"]
        .as_object_mut()
        .unwrap()
        .remove("vite");
    manifest["dependencies"]["vite"] = json!("^8");
    let problems = problems(&manifest, &lockfile());
    assert_eq!(problems.len(), 2, "{problems:?}");
}

#[test]
fn a_direct_dependency_without_a_node_modules_entry_is_a_disagreement() {
    let mut lock = lockfile();
    lock["packages"]
        .as_object_mut()
        .unwrap()
        .remove("node_modules/vite");
    let problems = problems(&package_json(), &lock);
    assert_eq!(
        problems,
        vec!["`vite` (`^8`) has no `node_modules/vite` entry in package-lock.json".to_string()]
    );
}

#[test]
fn a_file_dependency_that_is_not_a_link_is_a_disagreement() {
    let mut lock = lockfile();
    lock["packages"]["node_modules/@clockwyrks/simple-2d"] = json!({ "version": "1.1.1" });
    let problems = problems(&package_json(), &lock);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("is not a link"), "{problems:?}");
}

#[test]
fn a_file_dependency_linked_elsewhere_names_both_paths() {
    let mut lock = lockfile();
    lock["packages"]["node_modules/@clockwyrks/simple-2d"]["resolved"] =
        json!(".vendor/engine/@clockwyrks/simple-2d");
    let problems = problems(&package_json(), &lock);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].contains("`file:./.vendor/packages/@clockwyrks/simple-2d`"),
        "{problems:?}"
    );
    assert!(
        problems[0].contains("`.vendor/engine/@clockwyrks/simple-2d`"),
        "{problems:?}"
    );
}

#[test]
fn a_link_whose_target_has_no_entry_is_a_disagreement() {
    let mut lock = lockfile();
    lock["packages"]
        .as_object_mut()
        .unwrap()
        .remove(".vendor/packages/@clockwyrks/simple-2d");
    let problems = problems(&package_json(), &lock);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("no entry"), "{problems:?}");
}

#[test]
fn a_link_whose_target_carries_no_version_is_a_disagreement() {
    let mut lock = lockfile();
    lock["packages"][".vendor/packages/@clockwyrks/simple-2d"]
        .as_object_mut()
        .unwrap()
        .remove("version");
    let problems = problems(&package_json(), &lock);
    assert_eq!(problems.len(), 1, "{problems:?}");
}

#[test]
fn a_file_spec_without_the_leading_dot_slash_links_the_same_path() {
    let mut manifest = package_json();
    manifest["dependencies"][ENGINE] = json!("file:.vendor/packages/@clockwyrks/simple-2d");
    let mut lock = lockfile();
    lock["packages"][""]["dependencies"][ENGINE] =
        json!("file:.vendor/packages/@clockwyrks/simple-2d");
    check(&manifest, &lock).expect("agrees");
}

#[test]
fn a_tarball_file_dependency_is_not_held_to_a_link() {
    let mut manifest = package_json();
    manifest["dependencies"]["local"] = json!("file:vendor/local-1.0.0.tgz");
    let mut lock = lockfile();
    lock["packages"][""]["dependencies"]["local"] = json!("file:vendor/local-1.0.0.tgz");
    lock["packages"]["node_modules/local"] =
        json!({ "version": "1.0.0", "resolved": "file:vendor/local-1.0.0.tgz" });
    check(&manifest, &lock).expect("agrees");
}

#[test]
fn every_disagreement_is_reported() {
    let mut manifest = package_json();
    manifest["devDependencies"]["vite"] = json!("^9");
    let mut lock = lockfile();
    lock["packages"]
        .as_object_mut()
        .unwrap()
        .remove("node_modules/ts-essentials");
    assert_eq!(problems(&manifest, &lock).len(), 2);
}

#[test]
fn a_version_one_lockfile_is_refused() {
    let lock = json!({ "lockfileVersion": 1, "dependencies": {} });
    let err = Lockfile::parse(&lock.to_string()).unwrap_err();
    assert!(err.to_string().contains("lockfileVersion 1"), "{err}");
}

#[test]
fn a_lockfile_without_a_root_entry_is_refused() {
    let lock = json!({ "lockfileVersion": 3, "packages": {} });
    let err = Lockfile::parse(&lock.to_string()).unwrap_err();
    assert!(err.to_string().contains("root entry"), "{err}");
}

#[test]
fn a_non_string_spec_is_refused() {
    let manifest = json!({ "dependencies": { "vite": 8 } });
    let err = PackageManifest::parse(&manifest.to_string()).unwrap_err();
    assert!(err.to_string().contains("`vite`"), "{err}");
}

#[test]
fn the_linked_version_is_read_at_a_vendored_path() {
    let lock = Lockfile::parse(&lockfile().to_string()).unwrap();
    assert_eq!(
        lock.version_at("./.vendor/packages/@clockwyrks/simple-2d"),
        Some("1.1.1")
    );
    assert_eq!(lock.version_at(".vendor/packages/@clockwyrks/other"), None);
    assert!(lock.entries().any(
        |(path, version)| path == ".vendor/packages/@clockwyrks/simple-2d"
            && version == Some("1.1.1")
    ));
}

#[test]
fn a_dependency_is_found_in_whichever_map_declares_it() {
    let manifest = PackageManifest::parse(&package_json().to_string()).unwrap();
    assert_eq!(manifest.dependency("vite"), Some(("devDependencies", "^8")));
    assert_eq!(
        manifest.dependency(ENGINE),
        Some((
            "dependencies",
            "file:./.vendor/packages/@clockwyrks/simple-2d"
        ))
    );
    assert_eq!(manifest.dependency("eslint"), None);
}

#[test]
fn relative_paths_normalize_to_the_lockfile_form() {
    assert_eq!(
        normalize_relative("./.vendor/packages/x/"),
        ".vendor/packages/x"
    );
    assert_eq!(normalize_relative("a/./b/../c"), "a/c");
    assert_eq!(normalize_relative("../../packages/x"), "../../packages/x");
    assert_eq!(file_directory_spec("^1"), None);
    assert_eq!(file_directory_spec("file:x.tgz"), None);
    assert_eq!(
        file_directory_spec("file:./.vendor/engine/@clockwyrks/x"),
        Some(".vendor/engine/@clockwyrks/x".to_string())
    );
}

fn write_workspace(files: &[(&str, &Value)]) -> (tempfile::TempDir, Vec<WorkspaceFile>) {
    let dir = tempfile::tempdir().unwrap();
    let mut out = Vec::new();
    for (name, value) in files {
        let path = dir.path().join(name);
        fs::write(&path, serde_json::to_string_pretty(value).unwrap()).unwrap();
        out.push(WorkspaceFile {
            source_path: path,
            dest: PathBuf::from(name),
        });
    }
    (dir, out)
}

#[test]
fn a_workspace_without_a_lockfile_is_accepted() {
    let (_dir, files) = write_workspace(&[(PACKAGE_JSON, &package_json())]);
    check_workspace(&files).expect("accepted");
}

#[test]
fn a_workspace_whose_lockfile_agrees_is_accepted() {
    let (_dir, files) =
        write_workspace(&[(PACKAGE_JSON, &package_json()), (PACKAGE_LOCK, &lockfile())]);
    check_workspace(&files).expect("accepted");
}

#[test]
fn a_workspace_whose_lockfile_disagrees_is_refused() {
    let mut manifest = package_json();
    manifest["dependencies"]["ts-essentials"] = json!("^11");
    let (_dir, files) = write_workspace(&[(PACKAGE_JSON, &manifest), (PACKAGE_LOCK, &lockfile())]);
    let err = check_workspace(&files).unwrap_err();
    assert!(err.to_string().contains("`^11`"), "{err}");
}

#[test]
fn a_workspace_shipping_a_lockfile_without_a_package_json_is_refused() {
    let (_dir, files) = write_workspace(&[(PACKAGE_LOCK, &lockfile())]);
    assert!(matches!(
        check_workspace(&files),
        Err(LockfileError::Disagrees { .. })
    ));
}

#[test]
fn a_nested_lockfile_is_not_the_workspaces() {
    let (_dir, mut files) = write_workspace(&[(PACKAGE_JSON, &package_json())]);
    let (_other, nested) = write_workspace(&[(PACKAGE_LOCK, &json!({}))]);
    files.push(WorkspaceFile {
        source_path: nested[0].source_path.clone(),
        dest: PathBuf::from("tools/package-lock.json"),
    });
    check_workspace(&files).expect("only the root lockfile is checked");
}
