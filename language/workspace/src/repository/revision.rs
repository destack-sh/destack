use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use im::OrdMap;
use parking_lot::RwLock;
use rustc_hash::{FxHashMap, FxHashSet, FxHasher};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::{DestackDeclaration, PackageDeclaration, Profile, TsConfigDeclaration, Workspace};
use destack_artifact::{ArtifactKey, ArtifactStamp};
use destack_source::{FileId, FileType, ProfileId};

use crate::repository::{AmbientSnapshot, FileContentId, QueryIndex};

/// A ref names one movable repository tip.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ref(String);

impl Ref {
    /// Build a ref from one name.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Build the canonical ref for one workspace root.
    pub fn for_workspace_root(root: &Path) -> Self {
        Self::new(format!("root:{}", root.display()))
    }

    /// Return the ref name.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the ref into its name.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for Ref {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<String> for Ref {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Ref {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// The immutable handle for one published source snapshot.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Revision(pub u64);

impl Revision {
    /// The empty root revision handle.
    pub const INITIAL: Self = Self(0);

    /// Build one revision from one raw hash value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

impl Display for Revision {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "r{:016x}", self.0)
    }
}

/// The source-state mutability for one revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RevisionMode {
    /// One mutable working revision.
    Mutable,
    /// One immutable semantic revision.
    Immutable,
}

/// The origin for one file in one revision.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileOrigin {
    /// One workspace file keyed by logical path.
    Workspace { logical_path: String },
    /// One builtin file keyed by builtin module path.
    Builtin { logical_path: String },
    /// One synthetic file keyed by explicit logical path, kind, and file type.
    Synthetic {
        logical_path: String,
        kind: SyntheticFileKind,
        file_type: FileType,
    },
}

/// The synthetic role for one synthetic file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyntheticFileKind {
    /// One named synthetic file.
    Named,
    /// The synthetic root file.
    Root,
}

impl FileOrigin {
    /// Build one workspace file origin.
    pub fn workspace(logical_path: impl Into<String>) -> Self {
        Self::Workspace {
            logical_path: logical_path.into(),
        }
    }

    /// Build one builtin file origin.
    pub fn builtin(logical_path: impl Into<String>) -> Self {
        Self::Builtin {
            logical_path: logical_path.into(),
        }
    }

    /// Build one synthetic file origin.
    pub fn synthetic(logical_path: impl Into<String>, file_type: FileType) -> Self {
        Self::Synthetic {
            logical_path: logical_path.into(),
            kind: SyntheticFileKind::Named,
            file_type,
        }
    }

    /// Build the synthetic root file origin.
    pub fn root() -> Self {
        Self::Synthetic {
            logical_path: "root".to_string(),
            kind: SyntheticFileKind::Root,
            file_type: FileType::Destack,
        }
    }

    /// Return the logical path for this origin.
    pub fn logical_path(&self) -> &str {
        match self {
            Self::Workspace { logical_path }
            | Self::Builtin { logical_path }
            | Self::Synthetic { logical_path, .. } => logical_path,
        }
    }

    /// Return true when this origin is the synthetic root file.
    pub fn is_root(&self) -> bool {
        matches!(
            self,
            Self::Synthetic {
                kind: SyntheticFileKind::Root,
                ..
            }
        )
    }
}

/// The file binding for one file in one revision.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileEntry {
    /// The origin for this file in this revision.
    pub origin: FileOrigin,
    /// The fixed content binding for this file.
    pub content_id: FileContentId,
}

impl FileEntry {
    /// Build one loaded file entry.
    pub fn loaded(origin: FileOrigin, content_id: FileContentId) -> Self {
        Self { origin, content_id }
    }
}

/// The structurally shared source map for one revision.
pub type SourceMap = OrdMap<FileId, FileEntry>;

/// The immutable state behind one revision.
#[derive(Debug, Clone)]
pub struct RevisionState {
    /// The source-state mutability for this revision.
    pub mode: RevisionMode,
    /// The parent revisions for this snapshot.
    pub parents: SmallVec<[Revision; 2]>,

    /// The source map for this revision.
    pub source: Arc<SourceMap>,
    /// The ambient semantic snapshot for this revision.
    pub ambient: Arc<AmbientSnapshot>,

    /// The cached workspace view for this revision.
    pub workspace: OnceLock<Arc<Workspace>>,
    /// The cached profile view for this revision.
    pub profiles: OnceLock<Arc<OrdMap<ProfileId, Profile>>>,
    /// The cached artifact stamp for each revision local artifact key.
    pub artifact_stamps: OnceLock<Arc<RwLock<FxHashMap<ArtifactKey, ArtifactStamp>>>>,
    /// The cached query index for this revision.
    pub query_index: OnceLock<Arc<RwLock<QueryIndex>>>,

    /// The cached package declarations for this revision.
    pub package_declarations:
        OnceLock<Arc<RwLock<FxHashMap<FileId, Option<Arc<PackageDeclaration>>>>>>,
    /// The cached destack declarations for this revision.
    pub destack_declarations:
        OnceLock<Arc<RwLock<FxHashMap<FileId, Option<Arc<DestackDeclaration>>>>>>,
    /// The cached tsconfig declarations for this revision.
    pub tsconfig_declarations:
        OnceLock<Arc<RwLock<FxHashMap<FileId, Option<Arc<TsConfigDeclaration>>>>>>,
    /// The cached workspace directory paths for this revision.
    pub directory_paths: OnceLock<Arc<FxHashSet<PathBuf>>>,
}

impl RevisionState {
    /// Build one revision state record from explicit parts.
    pub fn new(
        parents: SmallVec<[Revision; 2]>,
        source: Arc<SourceMap>,
        ambient: Arc<AmbientSnapshot>,
    ) -> Self {
        Self::with_mode(RevisionMode::Immutable, parents, source, ambient)
    }

    /// Build one mutable revision state record from explicit parts.
    pub fn new_mutable(
        parents: SmallVec<[Revision; 2]>,
        source: Arc<SourceMap>,
        ambient: Arc<AmbientSnapshot>,
    ) -> Self {
        Self::with_mode(RevisionMode::Mutable, parents, source, ambient)
    }

    /// Build one revision state record from one explicit mode.
    pub fn with_mode(
        mode: RevisionMode,
        parents: SmallVec<[Revision; 2]>,
        source: Arc<SourceMap>,
        ambient: Arc<AmbientSnapshot>,
    ) -> Self {
        Self {
            mode,
            parents,
            source,
            ambient,
            workspace: OnceLock::new(),
            profiles: OnceLock::new(),
            artifact_stamps: OnceLock::new(),
            query_index: OnceLock::new(),
            package_declarations: OnceLock::new(),
            destack_declarations: OnceLock::new(),
            tsconfig_declarations: OnceLock::new(),
            directory_paths: OnceLock::new(),
        }
    }

    /// Return true when this revision is mutable.
    pub fn is_mutable(&self) -> bool {
        self.mode == RevisionMode::Mutable
    }

    /// Return true when this revision is immutable.
    pub fn is_immutable(&self) -> bool {
        self.mode == RevisionMode::Immutable
    }

    /// Return the file content id for one file.
    pub fn file_content_id(&self, file_id: FileId) -> Option<FileContentId> {
        self.source.get(&file_id).map(|entry| entry.content_id)
    }

    /// Return the file entry for one file.
    pub fn file_entry(&self, file_id: FileId) -> Option<FileEntry> {
        self.source.get(&file_id).cloned()
    }

    /// Return whether one file exists in this revision.
    pub fn contains_file(&self, file_id: FileId) -> bool {
        self.source.contains_key(&file_id)
    }

    /// Compute the deterministic identity for this revision state.
    pub fn revision(&self) -> Revision {
        let mut hasher = FxHasher::default();

        // revision mode
        self.mode.hash(&mut hasher);

        // canonical file map
        self.source.len().hash(&mut hasher);
        for (file_id, entry) in self.source.iter() {
            file_id.hash(&mut hasher);
            entry.hash(&mut hasher);
        }

        // ambient semantic inputs
        self.ambient.hash(&mut hasher);

        Revision::new(hasher.finish())
    }
}
