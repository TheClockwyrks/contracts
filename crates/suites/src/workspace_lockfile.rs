//! A workspace's `package.json` and the `package-lock.json` that pins it.
//!
//! See `docs/testing/end-to-end/overview.md`, sections "Workspace", "Packages" and
//! "Engines". A workspace with a `package.json` ships the lockfile that pins it,
//! so a run begins from a committed lockfile its init installs with `npm ci`. The
//! lockfile pins the registry dependencies and links each package and engine
//! dependency at its vendored `file:` path, and a workspace whose lockfile
//! disagrees with its `package.json` is rejected when the case resolves, before a
//! run is spent. This module is that check, and the reading of a lockfile the
//! seeder needs to hold a vendored copy to the version the lockfile links.
//!
//! The agreement checked is the part `npm ci` itself refuses on, read from the
//! files rather than by running npm:
//!
//! - the lockfile is `lockfileVersion` 2 or later, so it carries the `packages`
//!   map every check below reads;
//! - its root entry declares the same `dependencies`, `devDependencies`,
//!   `optionalDependencies` and `peerDependencies` as the `package.json`, name for
//!   name and spec for spec;
//! - every direct dependency the install places has a `node_modules/<name>` entry;
//! - every `file:` dependency on a directory is a `link: true` entry whose
//!   `resolved` is the dependency's path, and that path has an entry of its own
//!   carrying the version the link installs.
//!
//! A workspace without a lockfile is not checked here: it stays resolvable, and
//! its init is accepted unchecked (overview, "Init").

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::{Map, Value};
use test_cabinet_contracts::test_case::WorkspaceFile;
use thiserror::Error;

/// The manifest a workspace declares its dependencies in, at its root.
pub const PACKAGE_JSON: &str = "package.json";

/// The lockfile that pins a workspace's [`PACKAGE_JSON`], beside it.
pub const PACKAGE_LOCK: &str = "package-lock.json";

/// The dependency maps a `package.json` declares and a lockfile's root entry
/// repeats, in the order a mismatch is reported.
pub const DEPENDENCY_FIELDS: [&str; 4] = [
    "dependencies",
    "devDependencies",
    "optionalDependencies",
    "peerDependencies",
];

/// The dependency maps whose entries an install places under `node_modules/`. A
/// peer dependency is the consumer's to provide, so it is held to agreeing with
/// the root entry but not to having an entry of its own.
const INSTALLED_FIELDS: [&str; 3] = ["dependencies", "devDependencies", "optionalDependencies"];

/// The oldest `lockfileVersion` that carries the `packages` map (npm 7).
const MIN_LOCKFILE_VERSION: u64 = 2;

/// Why a workspace's lockfile could not be read or does not agree with its
/// `package.json`. The message names the file and every disagreement, for the
/// caller to place behind the workspace it came from.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LockfileError {
    /// A file could not be read, or is not the JSON object its format is.
    #[error("`{file}` {detail}")]
    Malformed {
        /// The file name, [`PACKAGE_JSON`] or [`PACKAGE_LOCK`].
        file: &'static str,
        /// What is wrong with it.
        detail: String,
    },

    /// The lockfile was read and disagrees with the `package.json`, once per
    /// problem, in a stable order.
    #[error("`package-lock.json` disagrees with `package.json`: {}", problems.join("; "))]
    Disagrees {
        /// Each disagreement, naming the dependency and both sides.
        problems: Vec<String>,
    },
}

impl LockfileError {
    fn malformed(file: &'static str, detail: impl Into<String>) -> Self {
        Self::Malformed {
            file,
            detail: detail.into(),
        }
    }
}

/// A `package.json`'s dependency maps, field by field, name to spec.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackageManifest {
    fields: BTreeMap<&'static str, BTreeMap<String, String>>,
}

impl PackageManifest {
    /// Parse a `package.json`'s source. Each of [`DEPENDENCY_FIELDS`] that is
    /// present must be an object of string specs.
    pub fn parse(raw: &str) -> Result<Self, LockfileError> {
        let value: Value = serde_json::from_str(raw).map_err(|err| {
            LockfileError::malformed(PACKAGE_JSON, format!("is not valid JSON: {err}"))
        })?;
        let object = value
            .as_object()
            .ok_or_else(|| LockfileError::malformed(PACKAGE_JSON, "must be a JSON object"))?;
        Ok(Self {
            fields: read_dependency_fields(object, PACKAGE_JSON, "")?,
        })
    }

    /// Read and parse the `package.json` at `path`.
    pub fn read(path: &Path) -> Result<Self, LockfileError> {
        Self::parse(&read_file(path, PACKAGE_JSON)?)
    }

    /// The specs `field` declares, empty when the field is absent.
    pub fn field(&self, field: &str) -> &BTreeMap<String, String> {
        static EMPTY: BTreeMap<String, String> = BTreeMap::new();
        self.fields.get(field).unwrap_or(&EMPTY)
    }

    /// The spec `name` is declared with, read from the first of
    /// [`DEPENDENCY_FIELDS`] that declares it, with that field's name.
    pub fn dependency(&self, name: &str) -> Option<(&'static str, &str)> {
        DEPENDENCY_FIELDS.iter().find_map(|field| {
            self.field(field)
                .get(name)
                .map(|spec| (*field, spec.as_str()))
        })
    }
}

/// A parsed `package-lock.json`: its `packages` map, keyed by the path of each
/// entry relative to the lockfile (`""` is the root project).
#[derive(Debug, Clone, PartialEq)]
pub struct Lockfile {
    packages: Map<String, Value>,
}

impl Lockfile {
    /// Parse a lockfile's source. It must be `lockfileVersion` 2 or later, with a
    /// `packages` map holding the root entry.
    pub fn parse(raw: &str) -> Result<Self, LockfileError> {
        let value: Value = serde_json::from_str(raw).map_err(|err| {
            LockfileError::malformed(PACKAGE_LOCK, format!("is not valid JSON: {err}"))
        })?;
        let object = value
            .as_object()
            .ok_or_else(|| LockfileError::malformed(PACKAGE_LOCK, "must be a JSON object"))?;
        let version = object.get("lockfileVersion").and_then(Value::as_u64);
        match version {
            Some(version) if version >= MIN_LOCKFILE_VERSION => {}
            _ => {
                let found = object
                    .get("lockfileVersion")
                    .map_or_else(|| "none".to_string(), Value::to_string);
                return Err(LockfileError::malformed(
                    PACKAGE_LOCK,
                    format!(
                        "has lockfileVersion {found}; a workspace lockfile must be version \
                         {MIN_LOCKFILE_VERSION} or later (written by npm 7 or later)"
                    ),
                ));
            }
        }
        let packages = object
            .get("packages")
            .and_then(Value::as_object)
            .ok_or_else(|| LockfileError::malformed(PACKAGE_LOCK, "has no `packages` map"))?;
        if !packages.get("").is_some_and(Value::is_object) {
            return Err(LockfileError::malformed(
                PACKAGE_LOCK,
                "has no root entry (`\"\"`) in its `packages` map",
            ));
        }
        for (key, entry) in packages {
            if !entry.is_object() {
                return Err(LockfileError::malformed(
                    PACKAGE_LOCK,
                    format!("entry `{key}` is not an object"),
                ));
            }
        }
        Ok(Self {
            packages: packages.clone(),
        })
    }

    /// Read and parse the lockfile at `path`.
    pub fn read(path: &Path) -> Result<Self, LockfileError> {
        Self::parse(&read_file(path, PACKAGE_LOCK)?)
    }

    /// The version the entry at `path` (relative to the lockfile, for example
    /// `.vendor/packages/@clockwyrks/simple-2d`) records, or `None` when there is
    /// no such entry or it carries no version. `path` is normalized first, so
    /// `./a/b` and `a/b` name one entry.
    pub fn version_at(&self, path: &str) -> Option<&str> {
        self.entry(&normalize_relative(path))?
            .get("version")
            .and_then(Value::as_str)
    }

    /// Every entry's path with the version it records, in the lockfile's order.
    /// The root entry (`""`) is included.
    pub fn entries(&self) -> impl Iterator<Item = (&str, Option<&str>)> {
        self.packages
            .iter()
            .map(|(path, entry)| (path.as_str(), entry.get("version").and_then(Value::as_str)))
    }

    /// Check that this lockfile pins `manifest`: see the module documentation for
    /// what agreeing means. Every disagreement is reported, not only the first.
    pub fn check_agrees(&self, manifest: &PackageManifest) -> Result<(), LockfileError> {
        let root = self
            .entry("")
            .and_then(Value::as_object)
            .expect("parse requires an object root entry");
        let locked = read_dependency_fields(root, PACKAGE_LOCK, "the root entry's ")?;
        static EMPTY: BTreeMap<String, String> = BTreeMap::new();
        let mut problems = Vec::new();

        for field in DEPENDENCY_FIELDS {
            let declared = manifest.field(field);
            let pinned = locked.get(field).unwrap_or(&EMPTY);
            for (name, spec) in declared {
                match pinned.get(name) {
                    Some(locked_spec) if locked_spec == spec => {}
                    Some(locked_spec) => problems.push(format!(
                        "`{field}` declares `{name}` as `{spec}` in package.json but as \
                         `{locked_spec}` in package-lock.json"
                    )),
                    None => problems.push(format!(
                        "`{field}` declares `{name}` as `{spec}` in package.json but \
                         package-lock.json's root entry does not"
                    )),
                }
            }
            for (name, locked_spec) in pinned {
                if !declared.contains_key(name) {
                    problems.push(format!(
                        "package-lock.json's root entry declares `{name}` as `{locked_spec}` \
                         in `{field}`, but package.json does not"
                    ));
                }
            }
        }

        for field in INSTALLED_FIELDS {
            for (name, spec) in manifest.field(field) {
                let key = format!("node_modules/{name}");
                let Some(entry) = self.entry(&key) else {
                    problems.push(format!(
                        "`{name}` (`{spec}`) has no `{key}` entry in package-lock.json"
                    ));
                    continue;
                };
                let Some(path) = file_directory_spec(spec) else {
                    continue;
                };
                if entry.get("link").and_then(Value::as_bool) != Some(true) {
                    problems.push(format!(
                        "`{name}` is the `file:` dependency `{spec}`, but its `{key}` entry \
                         in package-lock.json is not a link (`\"link\": true`)"
                    ));
                    continue;
                }
                let resolved = entry.get("resolved").and_then(Value::as_str);
                if resolved.map(normalize_relative).as_deref() != Some(path.as_str()) {
                    problems.push(format!(
                        "`{name}` is the `file:` dependency `{spec}`, but package-lock.json \
                         links it to `{}`",
                        resolved.unwrap_or("nothing")
                    ));
                    continue;
                }
                if self.version_at(&path).is_none() {
                    problems.push(format!(
                        "`{name}` is linked to `{path}`, but package-lock.json has no entry \
                         for `{path}` carrying the version it installs"
                    ));
                }
            }
        }

        if problems.is_empty() {
            Ok(())
        } else {
            Err(LockfileError::Disagrees { problems })
        }
    }

    fn entry(&self, key: &str) -> Option<&Value> {
        self.packages.get(key)
    }
}

/// Check one workspace's files: when they include a [`PACKAGE_LOCK`] at the root,
/// it must parse and agree with the [`PACKAGE_JSON`] beside it. A workspace
/// without a lockfile is accepted, and a lockfile without a `package.json` is a
/// disagreement.
pub fn check_workspace(files: &[WorkspaceFile]) -> Result<(), LockfileError> {
    let at_root = |name: &str| files.iter().find(|file| file.dest == Path::new(name));
    let Some(lockfile) = at_root(PACKAGE_LOCK) else {
        return Ok(());
    };
    let Some(package_json) = at_root(PACKAGE_JSON) else {
        return Err(LockfileError::Disagrees {
            problems: vec!["the workspace ships package-lock.json without a package.json".into()],
        });
    };
    let manifest = PackageManifest::read(&package_json.source_path)?;
    Lockfile::read(&lockfile.source_path)?.check_agrees(&manifest)
}

/// The directory a `file:` spec names, normalized relative to the workspace
/// root (`file:./.vendor/packages/x` is `.vendor/packages/x`), or `None` when the
/// spec is not `file:` or names a tarball, which npm installs rather than links.
pub fn file_directory_spec(spec: &str) -> Option<String> {
    let path = spec.strip_prefix("file:")?;
    let is_tarball = [".tgz", ".tar.gz", ".tar"]
        .iter()
        .any(|suffix| path.ends_with(suffix));
    (!is_tarball).then(|| normalize_relative(path))
}

/// A relative path in the form a lockfile keys and resolves it: forward slashes,
/// no `.` segments, no trailing slash, and `..` folded into the segment before it
/// where there is one (a leading `..` is kept).
pub fn normalize_relative(path: &str) -> String {
    let unified = path.replace('\\', "/");
    // An absolute path is never what a workspace declares; keep its leading `/`
    // so it compares unequal to every relative one.
    let mut parts: Vec<&str> = Vec::new();
    if unified.starts_with('/') {
        parts.push("");
    }
    for segment in unified.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if parts
                    .last()
                    .is_some_and(|last| !last.is_empty() && *last != "..")
                {
                    parts.pop();
                } else {
                    parts.push("..");
                }
            }
            part => parts.push(part),
        }
    }
    parts.join("/")
}

fn read_file(path: &Path, file: &'static str) -> Result<String, LockfileError> {
    fs::read_to_string(path)
        .map_err(|err| LockfileError::malformed(file, format!("could not be read: {err}")))
}

/// The [`DEPENDENCY_FIELDS`] of a `package.json` or a lockfile's root entry.
/// `place` prefixes the field in a malformed-field message.
fn read_dependency_fields(
    object: &Map<String, Value>,
    file: &'static str,
    place: &str,
) -> Result<BTreeMap<&'static str, BTreeMap<String, String>>, LockfileError> {
    let mut fields = BTreeMap::new();
    for field in DEPENDENCY_FIELDS {
        let Some(value) = object.get(field) else {
            continue;
        };
        let map = value.as_object().ok_or_else(|| {
            LockfileError::malformed(file, format!("{place}`{field}` must be an object"))
        })?;
        let mut specs = BTreeMap::new();
        for (name, spec) in map {
            let spec = spec.as_str().ok_or_else(|| {
                LockfileError::malformed(
                    file,
                    format!("{place}`{field}` gives `{name}` a spec that is not a string"),
                )
            })?;
            specs.insert(name.clone(), spec.to_string());
        }
        fields.insert(field, specs);
    }
    Ok(fields)
}

#[cfg(test)]
#[path = "workspace_lockfile.test.rs"]
mod tests;
