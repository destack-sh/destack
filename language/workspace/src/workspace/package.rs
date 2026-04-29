use std::path::PathBuf;

use destack_source::{FileId, PackageId, TargetId, Uri};
use indexmap::IndexMap;

use crate::config::Target;

/// The ownership kind for a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackageKind {
    /// Declared package rooted by authored workspace config.
    Declared,
    /// Implicit package rooted by one loose-file directory.
    Implicit,
    /// Builtin package for language primitives.
    Builtin,
}

/// One package of modules.
#[derive(Debug, Clone)]
pub struct Package {
    /// The package id.
    pub id: PackageId,
    /// The ownership kind.
    pub kind: PackageKind,
    /// The package uri.
    pub uri: Uri,
    /// The package directory when filesystem backed.
    pub path: Option<PathBuf>,
    /// The package name.
    pub name: Option<String>,
    /// The package version.
    pub version: Option<String>,
    /// The package.json declaration file id when present.
    pub package_file_id: Option<FileId>,
    /// The destack.json declaration file id when present.
    pub destack_file_id: Option<FileId>,
    /// The root tsconfig file for the package when present.
    pub tsconfig_file_id: Option<FileId>,
    /// The package targets.
    pub targets: IndexMap<TargetId, Target>,
}

impl Package {
    /// Get one target by id.
    pub fn target(&self, target: &TargetId) -> Option<&Target> {
        self.targets.get(target)
    }
}
