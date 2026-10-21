//! The cold-storage checkout: large generated media kept out of the catalog.
//!
//! The catalog's captured baseline validation media lives in the `cold-storage`
//! repository, checked out beside the `test-suites` checkout that holds the catalog
//! (both are submodules of the superrepo), rather than beside each test case. Its
//! tree mirrors the catalog checkout's, so a case version at
//! `test-cases/<type>/<difficulty>/<slug>/<version>/` keeps its baselines at
//! `cold-storage/test-cases/<type>/<difficulty>/<slug>/<version>/validation-baseline/`.
//!
//! [`ColdStorage::validation_baseline_dir`] is the one place that path is worked
//! out. `tcab capture-baselines` and `tcab publish-reference` write through it, and
//! backend ingest reads through it. The root defaults to the `cold-storage`
//! directory beside the checkout, and [`COLD_STORAGE_DIR_ENV`] points it anywhere
//! else.

use std::path::{Component, Path, PathBuf};

use crate::layout::VALIDATION_BASELINE_DIR;

/// The directory the cold-storage repository is checked out at, beside the catalog
/// checkout.
pub const COLD_STORAGE_DIR: &str = "cold-storage";

/// The environment variable that overrides the cold-storage root. When set and
/// non-empty it replaces the `cold-storage` beside the checkout; a relative value
/// resolves against the working directory.
pub const COLD_STORAGE_DIR_ENV: &str = "TCAB_COLD_STORAGE_DIR";

/// A checkout's cold-storage root, and the resolver from a case version to its
/// media beneath it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColdStorage {
    checkout: PathBuf,
    root: PathBuf,
}

impl ColdStorage {
    /// The cold storage of the catalog checkout at `checkout`: [`COLD_STORAGE_DIR_ENV`]
    /// when set, otherwise the `cold-storage` directory beside the checkout.
    pub fn for_checkout(checkout: impl Into<PathBuf>) -> Self {
        let checkout = checkout.into();
        let root = std::env::var_os(COLD_STORAGE_DIR_ENV)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| beside(&checkout));
        Self { checkout, root }
    }

    /// The cold storage of the checkout whose `test-cases/` catalog is rooted at
    /// `catalog_root`. The checkout is the catalog root's parent.
    pub fn for_catalog(catalog_root: &Path) -> Self {
        Self::for_checkout(catalog_root.parent().unwrap_or_else(|| Path::new("")))
    }

    /// Cold storage at an explicit `root` for the checkout at `checkout`, ignoring
    /// the environment.
    pub fn at(checkout: impl Into<PathBuf>, root: impl Into<PathBuf>) -> Self {
        Self {
            checkout: checkout.into(),
            root: root.into(),
        }
    }

    /// The cold-storage root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Whether the cold storage is checked out: its root holds something other
    /// than git's own bookkeeping.
    ///
    /// The submodule is optional, so a root that is missing, or is the empty
    /// directory git leaves for a submodule it has not fetched (with or without a
    /// `.git` entry from a fetch that failed part-way), is cold storage that says
    /// nothing about any version. A reader has to tell that apart from cold
    /// storage that is present and simply holds no baselines for one version.
    pub fn is_checked_out(&self) -> bool {
        std::fs::read_dir(&self.root).is_ok_and(|entries| {
            entries
                .flatten()
                .any(|entry| entry.file_name() != std::ffi::OsStr::new(".git"))
        })
    }

    /// The directory a case version's baseline validation media lives under, given
    /// the version's folder in the checkout: the folder's checkout-relative path
    /// re-rooted beneath the cold-storage root, joined with
    /// [`VALIDATION_BASELINE_DIR`]. Engine and variant directories sit beneath it.
    ///
    /// The path is worked out lexically, so it resolves whether or not the
    /// submodule is checked out. A `..` in the folder path is folded first, which
    /// places a game jam reached as `test-cases/../game-jams/<slug>/<version>` at
    /// `game-jams/<slug>/<version>`. `None` when the folder does not sit inside the
    /// checkout.
    pub fn validation_baseline_dir(&self, version_root: &Path) -> Option<PathBuf> {
        let checkout = normalize(&self.checkout)?;
        let version_root = normalize(version_root)?;
        let relative = version_root.strip_prefix(&checkout).ok()?;
        if relative.as_os_str().is_empty() {
            return None;
        }
        Some(self.root.join(relative).join(VALIDATION_BASELINE_DIR))
    }
}

/// The `cold-storage` directory beside `checkout`, worked out lexically: the
/// checkout's parent when its last component names a directory, and `..` from it
/// otherwise, so an empty (the working directory) or `.` checkout reaches the
/// working directory's parent.
fn beside(checkout: &Path) -> PathBuf {
    match (checkout.components().next_back(), checkout.parent()) {
        (Some(Component::Normal(_)), Some(parent)) => parent.join(COLD_STORAGE_DIR),
        _ => checkout.join("..").join(COLD_STORAGE_DIR),
    }
}

/// Fold `.` and `..` components out of `path` without touching the filesystem.
/// `None` when a `..` would climb above the path's start.
fn normalize(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !matches!(out.components().next_back(), Some(Component::Normal(_))) {
                    return None;
                }
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    Some(out)
}

#[cfg(test)]
#[path = "cold_storage.test.rs"]
mod tests;
