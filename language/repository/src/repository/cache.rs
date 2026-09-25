use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use im::OrdMap;
use parking_lot::Mutex;
use rustc_hash::{FxHashMap, FxHashSet};
use tspp_artifact::{ModuleSetFingerprint, PackageSetFingerprint};
use tspp_source::{DiagnosticCollection, FileId, ProfileId};

use crate::repository::{FileEntry, RepositoryError};
use crate::{ModuleIndex, PackageIndex, Profile, Root};

/// Lazily derived data for one revision identity.
#[derive(Debug, Default)]
pub(crate) struct RevisionCache {
    /// Root metadata.
    pub(crate) root: OnceLock<Arc<Root>>,
    /// Editable file bindings in key order.
    pub(crate) files: OnceLock<Arc<[(FileId, FileEntry)]>>,
    /// Package discovery result.
    pub(crate) packages: OnceLock<Result<Arc<PackageIndex>, RepositoryError>>,
    /// Module discovery result.
    pub(crate) modules: OnceLock<Result<Arc<ModuleIndex>, RepositoryError>>,
    /// Profile lookup data.
    pub(crate) profiles: OnceLock<Arc<OrdMap<ProfileId, Arc<Profile>>>>,
    /// Root directory paths.
    pub(crate) directory_paths: OnceLock<Arc<FxHashSet<PathBuf>>>,

    /// The fingerprint over this revision's module ids, matching Modules observations.
    pub(crate) modules_fingerprint: OnceLock<ModuleSetFingerprint>,
    /// The fingerprint over this revision's package ids, matching Packages observations.
    pub(crate) packages_fingerprint: OnceLock<PackageSetFingerprint>,
    /// Terminal dependency-closure diagnostics by requested artifact key set.
    pub(crate) diagnostics: Mutex<FxHashMap<u64, Arc<DiagnosticCollection>>>,
}

impl RevisionCache {
    /// Create one empty revision cache.
    pub(crate) fn new() -> Self {
        Self::default()
    }
}
