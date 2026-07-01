use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use destack_source::{FileId, ProfileId};
use im::OrdMap;
use rustc_hash::FxHashSet;

use crate::repository::FileEntry;
use crate::{ModuleIndex, PackageIndex, Profile, Root};

/// Lazily derived data for one revision identity.
#[derive(Debug, Default)]
pub(crate) struct RevisionCache {
    /// Root metadata.
    pub(crate) root: OnceLock<Arc<Root>>,
    /// Editable file bindings in key order.
    pub(crate) files: OnceLock<Arc<[(FileId, FileEntry)]>>,
    /// Package lookup data.
    pub(crate) packages: OnceLock<Arc<PackageIndex>>,
    /// Module lookup data.
    pub(crate) modules: OnceLock<Arc<ModuleIndex>>,
    /// Profile lookup data.
    pub(crate) profiles: OnceLock<Arc<OrdMap<ProfileId, Arc<Profile>>>>,
    /// Root directory paths.
    pub(crate) directory_paths: OnceLock<Arc<FxHashSet<PathBuf>>>,
}

impl RevisionCache {
    /// Create one empty revision cache.
    pub(crate) fn new() -> Self {
        Self::default()
    }
}
