//! Tests for what resolution holds a workspace's `package.json` and
//! `package-lock.json` to: a lockfile that agrees with the `package.json` beside
//! it, and a runtime engine's workspace declaring the engine's package at the path
//! its runtime is vendored at. See `docs/testing/end-to-end/overview.md`,
//! "Workspace", "Packages" and "Engines".

use serde_json::{Value, json};

use super::tests::{catalog_with_files, engines_manifest_with, manifest_with};
use super::*;

const ENGINE: &str = "@clockwyrks/simple-2d";

/// A `simple-2d` workspace `package.json` declaring the engine as `spec`.
fn engine_package_json(spec: &str) -> String {
    json!({
        "name": "demo",
        "private": true,
        "dependencies": { ENGINE: spec },
        "devDependencies": { "vite": "^8" }
    })
    .to_string()
}

/// The lockfile npm writes for [`engine_package_json`] of `spec`, linking the
/// engine at the path the spec names.
fn engine_lockfile(spec: &str) -> Value {
    let path = spec.strip_prefix("file:./").expect("an in-repo file: spec");
    json!({
        "name": "demo",
        "lockfileVersion": 3,
        "requires": true,
        "packages": {
            "": {
                "name": "demo",
                "dependencies": { ENGINE: spec },
                "devDependencies": { "vite": "^8" }
            },
            path: { "name": ENGINE, "version": "1.1.1" },
            "node_modules/@clockwyrks/simple-2d": { "resolved": path, "link": true },
            "node_modules/vite": { "version": "8.0.0", "dev": true }
        }
    })
}

const VENDORED: &str = "file:./.vendor/packages/@clockwyrks/simple-2d";
const LEGACY: &str = "file:./.vendor/engine/@clockwyrks/simple-2d";

/// Resolve a case supporting `none` and `simple-2d`, its workspaces holding
/// `files` (paths relative to the version folder).
fn resolve_engine_case(files: &[(&str, &str)]) -> Result<TestCaseVersion> {
    resolve_engine_case_with("", files)
}

/// [`resolve_engine_case`] with `variant` as the `base` variant's file.
fn resolve_engine_case_with(variant: &str, files: &[(&str, &str)]) -> Result<TestCaseVersion> {
    let manifest = engines_manifest_with(
        "engines = [\"none\", \"simple-2d\"]\n",
        "",
        &["none", "simple-2d"],
    );
    let mut all: Vec<(&str, &str)> = files.to_vec();
    let variant = format!("slug = \"base\"\n{variant}");
    all.push(("variants/base.toml", &variant));
    let (_dir, catalog) = catalog_with_files(&manifest, &all);
    catalog.resolve("demo", "v1.0.0")
}

fn rejection(result: Result<TestCaseVersion>) -> String {
    match result {
        Ok(_) => panic!("expected the case to be refused"),
        Err(err) => err.to_string(),
    }
}

// --- the engine's declaration -------------------------------------------------

#[test]
fn an_engine_workspace_declaring_the_engine_under_vendor_packages_resolves() {
    let package_json = engine_package_json(VENDORED);
    let version = resolve_engine_case(&[
        ("workspaces/none/package.json", "{}"),
        ("workspaces/simple-2d/package.json", &package_json),
    ])
    .expect("resolve");
    assert!(version.supports_engine("simple-2d"));
}

#[test]
fn an_engine_workspace_declaring_the_legacy_engine_path_resolves() {
    // A version written before the engine moved under `.vendor/packages/` declares
    // it under `.vendor/engine/`, and still resolves.
    let package_json = engine_package_json(LEGACY);
    resolve_engine_case(&[
        ("workspaces/none/package.json", "{}"),
        ("workspaces/simple-2d/package.json", &package_json),
    ])
    .expect("resolve");
}

#[test]
fn an_engine_declared_as_a_dev_dependency_resolves() {
    let package_json = json!({ "devDependencies": { ENGINE: VENDORED } }).to_string();
    resolve_engine_case(&[
        ("workspaces/none/package.json", "{}"),
        ("workspaces/simple-2d/package.json", &package_json),
    ])
    .expect("resolve");
}

#[test]
fn an_engine_workspace_not_declaring_the_engine_is_refused() {
    let err = rejection(resolve_engine_case(&[
        ("workspaces/none/package.json", "{}"),
        ("workspaces/simple-2d/package.json", r#"{"name":"demo"}"#),
    ]));
    assert!(err.contains("engine `simple-2d`"), "got: {err}");
    assert!(err.contains("does not declare"), "got: {err}");
    assert!(err.contains(VENDORED), "got: {err}");
}

#[test]
fn an_engine_declared_from_the_registry_is_refused_naming_both_specs() {
    let package_json = engine_package_json("^1.1.0");
    let err = rejection(resolve_engine_case(&[
        ("workspaces/none/package.json", "{}"),
        ("workspaces/simple-2d/package.json", &package_json),
    ]));
    assert!(err.contains("`^1.1.0`"), "got: {err}");
    assert!(err.contains(VENDORED), "got: {err}");
}

#[test]
fn an_engine_declared_elsewhere_under_vendor_is_refused() {
    let package_json = engine_package_json("file:./.vendor/other/@clockwyrks/simple-2d");
    let err = rejection(resolve_engine_case(&[
        ("workspaces/none/package.json", "{}"),
        ("workspaces/simple-2d/package.json", &package_json),
    ]));
    assert!(err.contains(".vendor/other"), "got: {err}");
}

#[test]
fn an_engine_declared_only_as_a_peer_is_refused() {
    let package_json = json!({ "peerDependencies": { ENGINE: VENDORED } }).to_string();
    let err = rejection(resolve_engine_case(&[
        ("workspaces/none/package.json", "{}"),
        ("workspaces/simple-2d/package.json", &package_json),
    ]));
    assert!(err.contains("does not declare"), "got: {err}");
}

#[test]
fn the_engineless_workspace_owes_no_engine_declaration() {
    // `none` vendors no runtime, so only the `simple-2d` workspace is held to
    // declaring one; the engineless project here depends on nothing.
    let package_json = engine_package_json(VENDORED);
    resolve_engine_case(&[
        ("workspaces/none/README.md", "hi"),
        ("workspaces/simple-2d/package.json", &package_json),
    ])
    .expect("resolve");
}

#[test]
fn a_variant_workspace_replacing_the_engine_project_owes_the_declaration() {
    // The variant's workspace replaces the case's, so it is the one the run is
    // seeded with and the one that must declare the engine.
    let package_json = engine_package_json(VENDORED);
    let err = rejection(resolve_engine_case_with(
        "[workspaces]\nnone = \"variant-ws/none\"\n\"simple-2d\" = \"variant-ws/simple-2d\"\n",
        &[
            ("workspaces/none/package.json", "{}"),
            ("workspaces/simple-2d/package.json", &package_json),
            ("variant-ws/none/package.json", "{}"),
            ("variant-ws/simple-2d/package.json", "{}"),
        ],
    ));
    assert!(err.contains("variant `base`"), "got: {err}");
    assert!(err.contains("does not declare"), "got: {err}");
}

// --- the lockfile -----------------------------------------------------------

#[test]
fn a_workspace_whose_lockfile_agrees_resolves_and_seeds_it() {
    let package_json = engine_package_json(VENDORED);
    let lockfile = engine_lockfile(VENDORED).to_string();
    let version = resolve_engine_case(&[
        ("workspaces/none/package.json", "{}"),
        ("workspaces/simple-2d/package.json", &package_json),
        ("workspaces/simple-2d/package-lock.json", &lockfile),
    ])
    .expect("resolve");
    assert!(
        version
            .common_workspace
            .get("simple-2d")
            .iter()
            .any(|file| file.dest == Path::new("package-lock.json")),
        "the lockfile is a workspace file, seeded like every other"
    );
}

#[test]
fn a_legacy_lockfile_linking_the_engine_under_vendor_engine_resolves() {
    let package_json = engine_package_json(LEGACY);
    let lockfile = engine_lockfile(LEGACY).to_string();
    resolve_engine_case(&[
        ("workspaces/none/package.json", "{}"),
        ("workspaces/simple-2d/package.json", &package_json),
        ("workspaces/simple-2d/package-lock.json", &lockfile),
    ])
    .expect("resolve");
}

#[test]
fn a_workspace_whose_lockfile_disagrees_is_refused_naming_it() {
    let package_json = engine_package_json(VENDORED);
    let mut lockfile = engine_lockfile(VENDORED);
    lockfile["packages"][""]["devDependencies"]["vite"] = json!("^7");
    let lockfile = lockfile.to_string();
    let err = rejection(resolve_engine_case(&[
        ("workspaces/none/package.json", "{}"),
        ("workspaces/simple-2d/package.json", &package_json),
        ("workspaces/simple-2d/package-lock.json", &lockfile),
    ]));
    assert!(err.contains("`workspaces/simple-2d`"), "got: {err}");
    assert!(err.contains("`vite`"), "got: {err}");
    assert!(err.contains("`^8`") && err.contains("`^7`"), "got: {err}");
}

#[test]
fn a_lockfile_linking_the_engine_elsewhere_is_refused() {
    // The `package.json` names `.vendor/packages/`, the lockfile still links the
    // legacy path: `npm ci` would refuse it, so resolution does.
    let package_json = engine_package_json(VENDORED);
    let mut lockfile = engine_lockfile(VENDORED);
    lockfile["packages"]["node_modules/@clockwyrks/simple-2d"]["resolved"] =
        json!(".vendor/engine/@clockwyrks/simple-2d");
    let lockfile = lockfile.to_string();
    let err = rejection(resolve_engine_case(&[
        ("workspaces/none/package.json", "{}"),
        ("workspaces/simple-2d/package.json", &package_json),
        ("workspaces/simple-2d/package-lock.json", &lockfile),
    ]));
    assert!(
        err.contains(".vendor/engine/@clockwyrks/simple-2d"),
        "got: {err}"
    );
}

#[test]
fn a_variant_workspace_whose_lockfile_disagrees_is_refused_naming_the_variant() {
    let package_json = engine_package_json(VENDORED);
    let lockfile = engine_lockfile(VENDORED);
    let mut stale = lockfile.clone();
    stale["packages"]
        .as_object_mut()
        .unwrap()
        .remove("node_modules/vite");
    let (lockfile, stale) = (lockfile.to_string(), stale.to_string());
    let err = rejection(resolve_engine_case_with(
        "[workspaces]\nnone = \"variant-ws/none\"\n\"simple-2d\" = \"variant-ws/simple-2d\"\n",
        &[
            ("workspaces/none/package.json", "{}"),
            ("workspaces/simple-2d/package.json", &package_json),
            ("workspaces/simple-2d/package-lock.json", &lockfile),
            ("variant-ws/none/package.json", "{}"),
            ("variant-ws/simple-2d/package.json", &package_json),
            ("variant-ws/simple-2d/package-lock.json", &stale),
        ],
    ));
    assert!(err.contains("variant `base`'s workspace"), "got: {err}");
    assert!(err.contains("node_modules/vite"), "got: {err}");
}

#[test]
fn an_engineless_workspace_whose_lockfile_disagrees_is_refused() {
    let manifest = manifest_with("workspace = \"workspaces/base\"\n", "");
    let package_json = json!({ "devDependencies": { "vite": "^8" } }).to_string();
    let lockfile = json!({
        "lockfileVersion": 3,
        "packages": { "": { "devDependencies": { "vite": "^8" } } }
    })
    .to_string();
    let (_dir, catalog) = catalog_with_files(
        &manifest,
        &[
            ("workspaces/base/package.json", &package_json),
            ("workspaces/base/package-lock.json", &lockfile),
        ],
    );
    let err = rejection(catalog.resolve("demo", "v1.0.0"));
    assert!(err.contains("`workspaces/base`"), "got: {err}");
    assert!(err.contains("no `node_modules/vite` entry"), "got: {err}");
}

#[test]
fn a_version_one_lockfile_is_refused() {
    let manifest = manifest_with("workspace = \"workspaces/base\"\n", "");
    let (_dir, catalog) = catalog_with_files(
        &manifest,
        &[
            ("workspaces/base/package.json", "{}"),
            (
                "workspaces/base/package-lock.json",
                r#"{"lockfileVersion":1,"dependencies":{}}"#,
            ),
        ],
    );
    let err = rejection(catalog.resolve("demo", "v1.0.0"));
    assert!(err.contains("lockfileVersion 1"), "got: {err}");
}

#[test]
fn a_workspace_without_a_lockfile_resolves() {
    // A version authored before workspaces shipped a lockfile still resolves; its
    // init is accepted unchecked.
    let manifest = manifest_with("workspace = \"workspaces/base\"\n", "");
    let (_dir, catalog) = catalog_with_files(&manifest, &[("workspaces/base/package.json", "{}")]);
    catalog.resolve("demo", "v1.0.0").expect("resolve");
}
