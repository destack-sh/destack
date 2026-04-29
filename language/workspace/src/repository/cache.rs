use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use dashmap::DashMap;
use destack_source::{FileContentId, ProfileId};
use im::OrdMap;
use rustc_hash::FxHashSet;

use crate::{DestackDeclaration, PackageDeclaration, Profile, TsConfigDeclaration, Workspace};

/// Derived cache data for one immutable revision.
#[derive(Debug, Default)]
pub(crate) struct RevisionCache {
    /// The workspace index.
    pub(crate) workspace: OnceLock<Arc<Workspace>>,
    /// The profile index.
    pub(crate) profiles: OnceLock<Arc<OrdMap<ProfileId, Arc<Profile>>>>,
    /// The workspace directory set.
    pub(crate) directory_paths: OnceLock<Arc<FxHashSet<PathBuf>>>,
}

impl RevisionCache {
    /// Create one empty revision cache.
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

/// Parsed file data keyed by exact file content.
#[derive(Debug, Default)]
pub(crate) struct FileCache {
    /// The package declaration parse result by exact content.
    pub(crate) package_declarations:
        DashMap<FileContentId, Result<Arc<PackageDeclaration>, String>>,
    /// The destack declaration parse result by exact content.
    pub(crate) destack_declarations:
        DashMap<FileContentId, Result<Arc<DestackDeclaration>, String>>,
    /// The tsconfig declaration parse result by exact content.
    pub(crate) tsconfig_declarations:
        DashMap<FileContentId, Result<Arc<TsConfigDeclaration>, String>>,
}

impl FileCache {
    /// Create one empty file cache.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Drop entries for file contents that are no longer reachable.
    pub(crate) fn retain_file_contents(&self, reachable: &HashSet<FileContentId>) {
        self.package_declarations
            .retain(|content_id, _| reachable.contains(content_id));
        self.destack_declarations
            .retain(|content_id, _| reachable.contains(content_id));
        self.tsconfig_declarations
            .retain(|content_id, _| reachable.contains(content_id));
    }
}
