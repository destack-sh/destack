use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use indexmap::IndexMap;
use postcard::Error as PostcardError;
use rustc_hash::FxHasher;
use serde::{Deserialize, Serialize};

use destack_core::StringPool;
use destack_source::{
    FileContent, FileId, FileMetadata, FileRegistry, FileSystem, FileVersion, ModuleId,
    ModuleVersion, strip_json,
};

use crate::{
    CacheScope, CacheValidate, Destack, Program, Workspace, hash_bytes, hash_json_value,
    resolve_cache_dir, resolve_cache_root_for_scope,
};

/// Magic prefix for workspace index snapshots.
const WORKSPACE_INDEX_MAGIC: [u8; 4] = *b"DSWI";

/// Workspace index snapshot format version.
const WORKSPACE_INDEX_FORMAT_VERSION: u32 = 2;

/// Compute a hash for the file contents at a path.
pub(crate) fn file_content_hash_for_path(
    fs: &dyn FileSystem,
    files: &FileRegistry,
    path: &Path,
) -> Option<u64> {
    // reuse loaded file contents when available
    if let Some(file) = files.get_by_path(path) {
        match &file.content {
            FileContent::Text { content } => {
                return Some(hash_bytes(content.as_bytes()));
            }
            FileContent::Json { content, .. } => {
                return Some(hash_bytes(content.as_bytes()));
            }
            FileContent::Binary { content } => {
                return Some(hash_bytes(content));
            }
            FileContent::Missing => return None,
            FileContent::Unloaded => {}
        }
    }

    // fall back to reading from the file system
    let bytes = fs.read(path).ok()?;
    Some(hash_bytes(&bytes))
}

/// Header for workspace index snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceIndexHeader {
    /// The magic prefix used to identify cache files.
    pub magic: [u8; 4],
    /// The cache header format version.
    pub format_version: u32,
    /// The compiler version that produced the snapshot.
    pub compiler_version: String,
    /// The workspace root used for the snapshot.
    pub workspace_root: PathBuf,
    /// Hash of the workspace configuration.
    pub config_hash: Option<u64>,
    /// Hash of compiler options affecting the workspace cache.
    pub compiler_options_hash: u64,
    /// Hash of resolver options affecting the workspace cache.
    pub resolve_options_hash: u64,
    /// Cache validation strategy for the snapshot.
    pub cache_validate: CacheValidate,
}

impl WorkspaceIndexHeader {
    /// Create a new workspace index header.
    pub fn new(
        compiler_version: String,
        workspace_root: PathBuf,
        config_hash: Option<u64>,
        compiler_options_hash: u64,
        resolve_options_hash: u64,
        cache_validate: CacheValidate,
    ) -> Self {
        Self {
            magic: WORKSPACE_INDEX_MAGIC,
            format_version: WORKSPACE_INDEX_FORMAT_VERSION,
            compiler_version,
            workspace_root,
            config_hash,
            compiler_options_hash,
            resolve_options_hash,
            cache_validate,
        }
    }

    /// Check whether this header matches the expected metadata.
    pub fn matches(&self, expected: &Self) -> bool {
        if self.magic != expected.magic {
            return false;
        }

        if self.format_version != expected.format_version {
            return false;
        }

        if self.compiler_version != expected.compiler_version {
            return false;
        }

        if self.workspace_root != expected.workspace_root {
            return false;
        }

        self.config_hash == expected.config_hash
            && self.compiler_options_hash == expected.compiler_options_hash
            && self.resolve_options_hash == expected.resolve_options_hash
            && self.cache_validate == expected.cache_validate
    }
}

/// Cached file entry for workspace index snapshots.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WorkspaceFileEntry {
    /// File version at the time of the snapshot.
    pub version: FileVersion,
    /// Whether the file existed when the snapshot was produced.
    pub exists: bool,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Last modified timestamp in nanoseconds since unix epoch.
    pub modified_ns: Option<u64>,
    /// Content hash for strict validation when available.
    pub content_hash: Option<u64>,
}

impl WorkspaceFileEntry {
    /// Create a file entry from filesystem metadata.
    pub fn from_metadata(
        version: FileVersion,
        metadata: &FileMetadata,
        content_hash: Option<u64>,
    ) -> Self {
        let modified_ns = system_time_to_nanos(metadata.modified_at);
        Self {
            version,
            exists: metadata.is_file,
            size_bytes: metadata.size_bytes,
            modified_ns,
            content_hash,
        }
    }

    /// Create a missing file entry.
    pub fn missing(version: FileVersion) -> Self {
        Self {
            version,
            exists: false,
            size_bytes: 0,
            modified_ns: None,
            content_hash: None,
        }
    }

    /// Check whether metadata matches the stored stamp.
    pub fn matches_metadata(&self, metadata: &FileMetadata) -> bool {
        if !self.exists || !metadata.is_file {
            return false;
        }

        if self.size_bytes != metadata.size_bytes {
            return false;
        }

        let modified_ns = system_time_to_nanos(metadata.modified_at);
        self.modified_ns == modified_ns
    }

    /// Return the version to use when metadata is missing.
    pub fn version_for_missing(&self) -> FileVersion {
        if self.exists {
            return self.version.next();
        }

        self.version
    }

    /// Return the version to use for the provided metadata.
    pub fn version_for_metadata(&self, metadata: &FileMetadata) -> FileVersion {
        if self.matches_metadata(metadata) {
            return self.version;
        }

        self.version.next()
    }
}

/// Cached module entry for workspace index snapshots.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WorkspaceModuleEntry {
    /// Module version at the time of the snapshot.
    pub version: ModuleVersion,
    /// File version used to compute the module version.
    pub source_version: FileVersion,
}

impl WorkspaceModuleEntry {
    /// Create a new module entry.
    pub fn new(version: ModuleVersion, source_version: FileVersion) -> Self {
        Self {
            version,
            source_version,
        }
    }
}

/// Snapshot of workspace index data for caching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceIndexSnapshot {
    /// Header metadata for the snapshot.
    pub header: WorkspaceIndexHeader,
    /// The shared workspace string pool image.
    pub strings: StringPool,
    /// File entries keyed by path.
    pub files: IndexMap<PathBuf, WorkspaceFileEntry>,
    /// Module entries keyed by module id.
    pub modules: IndexMap<ModuleId, WorkspaceModuleEntry>,
}

impl WorkspaceIndexSnapshot {
    /// Build a snapshot from the current program state.
    pub fn from_program(
        program: &Program,
        header: WorkspaceIndexHeader,
    ) -> Result<Self, WorkspaceIndexError> {
        // collect file and module entries from program modules
        let mut files = IndexMap::new();
        let mut modules = IndexMap::new();

        // walk modules and snapshot file/module metadata
        for module in program.modules.iter() {
            // snapshot module metadata
            let module = module.as_ref();
            modules.insert(
                module.id,
                WorkspaceModuleEntry::new(
                    program.modules.version(module.id),
                    program.modules.source_version(module.id),
                ),
            );

            // skip modules without file paths
            let Some(path) = module.path.as_ref() else {
                continue;
            };

            // skip duplicate file paths
            if files.contains_key(path) {
                continue;
            }

            // collect file metadata and content hashes
            let file = program.files.get(module.file_id);
            let entry = match program.fs.metadata(path) {
                Ok(metadata) => {
                    let content_hash = if metadata.is_file {
                        Some(hash_file_content(program, module.file_id, path)?)
                    } else {
                        None
                    };
                    WorkspaceFileEntry::from_metadata(file.version, &metadata, content_hash)
                }
                Err(_) => WorkspaceFileEntry::missing(file.version),
            };

            // record the file entry
            files.insert(path.clone(), entry);
        }

        Ok(Self {
            header,
            strings: program.strings.as_ref().clone(),
            files,
            modules,
        })
    }
}

/// Errors produced while hashing workspace configuration.
#[derive(Debug)]
pub enum WorkspaceConfigError {
    /// The workspace config is missing.
    Missing { path: PathBuf },
    /// The workspace config could not be read.
    Read {
        path: PathBuf,
        error: std::io::Error,
    },
    /// The workspace config could not be stripped of comments.
    Strip {
        path: PathBuf,
        error: std::io::Error,
    },
    /// The workspace config is not valid JSON.
    Parse {
        path: PathBuf,
        error: serde_json::Error,
    },
    /// The workspace config has an invalid extends entry.
    Extends { path: PathBuf, reason: String },
}

impl std::fmt::Display for WorkspaceConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // format workspace config errors
        match self {
            WorkspaceConfigError::Missing { path } => {
                write!(f, "workspace config missing at {}", path.display())
            }
            WorkspaceConfigError::Read { path, error } => {
                write!(
                    f,
                    "workspace config read error at {}: {error}",
                    path.display()
                )
            }
            WorkspaceConfigError::Strip { path, error } => {
                write!(
                    f,
                    "workspace config comment strip error at {}: {error}",
                    path.display()
                )
            }
            WorkspaceConfigError::Parse { path, error } => {
                write!(
                    f,
                    "workspace config parse error at {}: {error}",
                    path.display()
                )
            }
            WorkspaceConfigError::Extends { path, reason } => {
                write!(
                    f,
                    "workspace config extends error at {}: {reason}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for WorkspaceConfigError {}

/// Errors produced while reading or writing the workspace index.
#[derive(Debug)]
pub enum WorkspaceIndexError {
    /// The workspace index failed to serialize.
    Serialize(PostcardError),
    /// The workspace index failed to deserialize.
    Deserialize(PostcardError),
    /// The workspace index exceeded the configured size limit.
    SizeLimitExceeded { limit: u64, actual: u64 },
    /// The workspace index failed to read or write.
    Io(std::io::Error),
    /// Workspace config hashing failed.
    Config(WorkspaceConfigError),
    /// Workspace index file hashing failed.
    ContentHash { path: PathBuf, reason: String },
}

impl std::fmt::Display for WorkspaceIndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // format workspace index errors
        match self {
            WorkspaceIndexError::Serialize(error) => {
                write!(f, "failed to serialize workspace index: {error}")
            }
            WorkspaceIndexError::Deserialize(error) => {
                write!(f, "failed to deserialize workspace index: {error}")
            }
            WorkspaceIndexError::SizeLimitExceeded { limit, actual } => {
                write!(
                    f,
                    "workspace index exceeded size limit, limit {limit}, actual {actual}"
                )
            }
            WorkspaceIndexError::Io(error) => write!(f, "workspace index io error: {error}"),
            WorkspaceIndexError::Config(error) => {
                write!(f, "workspace index config error: {error}")
            }
            WorkspaceIndexError::ContentHash { path, reason } => {
                write!(
                    f,
                    "workspace index content hash error at {}: {reason}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for WorkspaceIndexError {}

impl From<WorkspaceConfigError> for WorkspaceIndexError {
    fn from(error: WorkspaceConfigError) -> Self {
        WorkspaceIndexError::Config(error)
    }
}

/// Compute a hash for the workspace config when present.
pub fn hash_workspace_config(
    workspace: &Workspace,
    fs: &dyn FileSystem,
) -> Result<Option<u64>, WorkspaceConfigError> {
    // collect unique package roots
    let mut package_roots = workspace.package_paths.clone();
    if !package_roots.contains(&workspace.root) {
        package_roots.push(workspace.root.clone());
    }
    package_roots.sort();
    package_roots.dedup();

    // seed the config worklist
    #[derive(Debug)]
    struct ConfigCandidate {
        path: PathBuf,
        required: bool,
    }

    let mut pending: Vec<ConfigCandidate> = package_roots
        .into_iter()
        .map(|root| ConfigCandidate {
            path: root.join("destack.json"),
            required: false,
        })
        .collect();

    // hash config contents for each package root
    let mut hasher = FxHasher::default();
    let mut found_any = false;
    let mut visited = HashSet::new();

    while let Some(candidate) = pending.pop() {
        // resolve config paths
        let Some(resolved) = resolve_destack_config_path(fs, &candidate.path) else {
            if candidate.required {
                return Err(WorkspaceConfigError::Missing {
                    path: candidate.path,
                });
            }
            continue;
        };

        // skip configs already hashed
        if !visited.insert(resolved.clone()) {
            continue;
        }

        // read config contents
        let content = fs
            .read_to_string(&resolved)
            .map_err(|error| WorkspaceConfigError::Read {
                path: resolved.clone(),
                error,
            })?;

        // strip comments for jsonc compatibility
        let stripped = strip_json(&content).map_err(|error| WorkspaceConfigError::Strip {
            path: resolved.clone(),
            error,
        })?;

        // parse config JSON
        let value = serde_json::from_str::<serde_json::Value>(&stripped).map_err(|error| {
            WorkspaceConfigError::Parse {
                path: resolved.clone(),
                error,
            }
        })?;

        // hash the config payload
        found_any = true;
        hasher.write(resolved.to_string_lossy().as_bytes());
        hash_json_value(&value).hash(&mut hasher);

        // extract extends entries
        let extends = match value.get("extends") {
            None | Some(serde_json::Value::Null) => Vec::new(),
            Some(serde_json::Value::String(value)) => vec![value.clone()],
            Some(serde_json::Value::Array(values)) => {
                let mut extends = Vec::with_capacity(values.len());
                for entry in values {
                    let Some(specifier) = entry.as_str() else {
                        return Err(WorkspaceConfigError::Extends {
                            path: resolved.clone(),
                            reason: "extends entries must be strings".to_string(),
                        });
                    };
                    extends.push(specifier.to_string());
                }
                extends
            }
            Some(_) => {
                return Err(WorkspaceConfigError::Extends {
                    path: resolved.clone(),
                    reason: "extends must be a string or array of strings".to_string(),
                });
            }
        };

        // enqueue extended configs
        if extends.is_empty() {
            continue;
        }

        let directory = resolved
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        for specifier in extends {
            pending.push(ConfigCandidate {
                path: resolve_extends_path(&directory, &specifier),
                required: true,
            });
        }
    }

    if !found_any {
        return Ok(None);
    }

    Ok(Some(hasher.finish()))
}

/// Resolve a config path when the config file exists.
fn resolve_destack_config_path(fs: &dyn FileSystem, path: &Path) -> Option<PathBuf> {
    if fs.exists(path).unwrap_or(false) {
        return Some(path.to_path_buf());
    }

    let jsonc = path.with_extension("jsonc");
    if fs.exists(&jsonc).unwrap_or(false) {
        return Some(jsonc);
    }

    None
}

/// Resolve a Destack config extends reference to a canonical path.
fn resolve_extends_path(directory: &Path, specifier: &str) -> PathBuf {
    if specifier.starts_with('.') || specifier.starts_with('/') {
        return directory.join(specifier);
    }

    directory.join("node_modules").join(specifier)
}

/// Resolve the cache root directory for a workspace.
pub fn resolve_workspace_cache_root(
    workspace_root: &Path,
    workspace_config: Option<&Destack>,
    cache_dir_override: Option<&Path>,
) -> PathBuf {
    // honor explicit overrides
    if let Some(cache_dir) = cache_dir_override {
        return resolve_cache_dir(cache_dir, workspace_root);
    }

    // honor config cache settings when present
    if let Some(config) = workspace_config {
        let cache_options = &config.options.cache;
        return resolve_cache_root_for_scope(
            &config.directory,
            cache_options.dir.as_deref(),
            cache_options.scope,
        );
    }

    resolve_cache_root_for_scope(workspace_root, None, CacheScope::Workspace)
}

/// Hash file contents for strict workspace index validation.
fn hash_file_content(
    program: &Program,
    file_id: FileId,
    path: &Path,
) -> Result<u64, WorkspaceIndexError> {
    // hash loaded file contents when available
    let file = program.files.get(file_id);
    match &file.content {
        FileContent::Text { content } => Ok(hash_bytes(content.as_bytes())),
        FileContent::Json { content, .. } => Ok(hash_bytes(content.as_bytes())),
        FileContent::Binary { content } => Ok(hash_bytes(content)),
        FileContent::Unloaded => {
            // fall back to reading from the file system
            let bytes =
                program
                    .fs
                    .read(path)
                    .map_err(|error| WorkspaceIndexError::ContentHash {
                        path: path.to_path_buf(),
                        reason: format!("failed to read file contents: {error}"),
                    })?;
            Ok(hash_bytes(&bytes))
        }
        FileContent::Missing => Err(WorkspaceIndexError::ContentHash {
            path: path.to_path_buf(),
            reason: "file contents missing".to_string(),
        }),
    }
}

/// Convert a system time into nanoseconds since unix epoch.
fn system_time_to_nanos(time: Option<SystemTime>) -> Option<u64> {
    // skip missing timestamps
    let time = time?;

    // compute nanoseconds since unix epoch
    let duration = time.duration_since(UNIX_EPOCH).ok()?;
    let seconds = duration.as_secs();
    let nanos = duration.subsec_nanos() as u64;
    seconds
        .checked_mul(1_000_000_000)
        .and_then(|base| base.checked_add(nanos))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::SystemTime;

    use destack_core::StringPool;
    use destack_source::{
        FileMetadata, FileRegistry, FileVersion, MemoryFileSystem, ModuleId, ModuleVersion,
        PhysicalFileSystem, TemporaryPhysicalFileSystem, strip_json,
    };
    use filetime::FileTime;
    use indexmap::IndexMap;

    use crate::{
        CacheValidate, DiskCacheStore, FormatterOptions, LinterOptions, ModuleRegistry,
        PackageRegistry, Program, Session, TsConfigRegistry, Workspace, WorkspaceConfigError,
        WorkspaceStore, hash_bytes,
    };

    use super::{
        WorkspaceFileEntry, WorkspaceIndexHeader, WorkspaceIndexSnapshot, WorkspaceModuleEntry,
        hash_workspace_config,
    };

    /// Roundtrip workspace index snapshots through disk.
    #[test]
    fn test_workspace_index_roundtrip() {
        // setup a temp workspace directory
        let root = TemporaryPhysicalFileSystem::new_with_prefix("workspace_index");
        let cache_root = root.path_for(".destack");
        let store = DiskCacheStore::new();
        let index_store = WorkspaceStore::new(&store, &cache_root);

        // build a minimal snapshot
        let header = WorkspaceIndexHeader::new(
            "0.0.0".to_string(),
            root.root().to_path_buf(),
            Some(42),
            1,
            2,
            CacheValidate::Strict,
        );
        let mut files = IndexMap::new();
        files.insert(
            root.path_for("a.ds"),
            WorkspaceFileEntry {
                version: FileVersion::new(1),
                exists: true,
                size_bytes: 12,
                modified_ns: Some(5),
                content_hash: Some(7),
            },
        );
        let mut modules = IndexMap::new();
        modules.insert(
            ModuleId::EPHEMERAL,
            WorkspaceModuleEntry::new(ModuleVersion::new(2), FileVersion::new(1)),
        );
        let snapshot = WorkspaceIndexSnapshot {
            header: header.clone(),
            strings: StringPool::new(),
            files,
            modules,
        };

        // write and read the snapshot
        index_store.save(&snapshot).unwrap();
        let loaded = index_store
            .load(&header)
            .unwrap()
            .expect("expected snapshot");

        // check that the snapshot is loaded
        assert_eq!(loaded.header.compiler_version, "0.0.0");
        assert_eq!(loaded.files.len(), 1);
        assert_eq!(loaded.modules.len(), 1);
    }

    /// Reject snapshots when the header does not match.
    #[test]
    fn test_workspace_index_header_mismatch() {
        // set up a temp workspace directory
        let root = TemporaryPhysicalFileSystem::new_with_prefix("workspace_index_mismatch");
        let cache_root = root.path_for(".destack");
        let store = DiskCacheStore::new();
        let index_store = WorkspaceStore::new(&store, &cache_root);

        // write a minimal snapshot
        let header = WorkspaceIndexHeader::new(
            "0.0.0".to_string(),
            root.root().to_path_buf(),
            Some(1),
            3,
            4,
            CacheValidate::Strict,
        );
        let snapshot = WorkspaceIndexSnapshot {
            header: header.clone(),
            strings: StringPool::new(),
            files: IndexMap::new(),
            modules: IndexMap::new(),
        };
        index_store.save(&snapshot).unwrap();

        // read with mismatched header
        let mismatch = WorkspaceIndexHeader::new(
            "0.0.1".to_string(),
            root.root().to_path_buf(),
            Some(1),
            3,
            4,
            CacheValidate::Strict,
        );
        let loaded = index_store.load(&mismatch).unwrap();

        // check that the snapshot is not loaded
        assert!(loaded.is_none());
    }

    /// Ignore comments when hashing workspace config.
    #[test]
    fn test_hash_workspace_config_ignores_comments() {
        // set up a workspace with a commented config
        let fs = MemoryFileSystem::new();
        let root = PathBuf::from("/workspace");
        let destack_config_path = root.join("destack.json");
        let commented = r#"{
  // comment
  "compiler": { "strict": true }
}
"#;
        let stripped =
            strip_json(commented).unwrap_or_else(|error| panic!("failed to strip json: {error}"));
        fs.add_file(&destack_config_path, commented.as_bytes())
            .unwrap();
        let workspace = Workspace::single_package(root);

        // compute hash with comments
        let hash_with_comments = hash_workspace_config(&workspace, &fs)
            .unwrap_or_else(|error| panic!("failed to hash config: {error}"))
            .unwrap_or_else(|| panic!("expected config hash"));

        // overwrite config without comments
        fs.add_file(&destack_config_path, stripped.as_bytes())
            .unwrap();
        let hash_without_comments = hash_workspace_config(&workspace, &fs)
            .unwrap_or_else(|error| panic!("failed to hash config: {error}"))
            .unwrap_or_else(|| panic!("expected config hash"));

        // check that the hashes are equal
        assert_eq!(
            hash_with_comments, hash_without_comments,
            "expected comment changes to be ignored"
        );
    }

    /// Hash workspace configs across extends.
    #[test]
    fn test_hash_workspace_config_tracks_extends() {
        let fs = MemoryFileSystem::new();
        let root = PathBuf::from("/workspace");
        let destack_config_path = root.join("destack.json");
        let base_path = root.join("base.json");
        fs.add_file(
            &destack_config_path,
            br#"{
  "extends": "./base.json",
  "compiler": { "strict": true }
}
"#,
        )
        .unwrap();
        fs.add_file(
            &base_path,
            br#"{
  "compiler": { "noImplicitAny": true }
}
"#,
        )
        .unwrap();
        let workspace = Workspace::single_package(root);

        let hash = hash_workspace_config(&workspace, &fs)
            .unwrap_or_else(|error| panic!("failed to hash config: {error}"))
            .unwrap_or_else(|| panic!("expected config hash"));

        // check that the hash is not zero
        assert_ne!(hash, 0);
    }

    /// Reject missing extends entries.
    #[test]
    fn test_hash_workspace_config_rejects_missing_extends() {
        let fs = MemoryFileSystem::new();
        let root = PathBuf::from("/workspace");
        let destack_config_path = root.join("destack.json");
        fs.add_file(
            &destack_config_path,
            br#"{
  "extends": "./missing.json"
}
"#,
        )
        .unwrap();
        let workspace = Workspace::single_package(root);

        let error =
            hash_workspace_config(&workspace, &fs).expect_err("expected missing extends error");

        // check that the error is a missing extends error
        assert!(
            matches!(error, WorkspaceConfigError::Missing { .. }),
            "expected missing error"
        );
    }

    /// Reject invalid json in workspace configs.
    #[test]
    fn test_hash_workspace_config_rejects_invalid_json() {
        let fs = MemoryFileSystem::new();
        let root = PathBuf::from("/workspace");
        let destack_config_path = root.join("destack.json");
        fs.add_file(&destack_config_path, br#"{ "broken": }"#)
            .unwrap();
        let workspace = Workspace::single_package(root);

        let error = hash_workspace_config(&workspace, &fs).expect_err("expected parse error");

        // check that the error is a parse error
        assert!(
            matches!(error, WorkspaceConfigError::Parse { .. }),
            "expected parse error"
        );
    }

    /// Resolve workspace file entry versions.
    #[test]
    fn test_workspace_file_entry_versions() {
        let file_metadata = FileMetadata::new(true, false, false, 10, Some(SystemTime::now()));
        let entry = WorkspaceFileEntry::from_metadata(FileVersion::new(1), &file_metadata, None);

        // check that the version is incremented
        assert_eq!(entry.version_for_missing(), FileVersion::new(2));

        let same = FileMetadata::new(true, false, false, 10, file_metadata.modified_at);
        let changed = FileMetadata::new(true, false, false, 11, file_metadata.modified_at);
        assert_eq!(entry.version_for_metadata(&same), FileVersion::new(1));
        assert_eq!(entry.version_for_metadata(&changed), FileVersion::new(2));
    }

    /// Roundtrip workspace index snapshots through disk.
    #[test]
    fn test_workspace_index_roundtrip_disk() {
        // setup a temp workspace directory
        let root = TemporaryPhysicalFileSystem::new_with_prefix("workspace_index_disk");
        let cache_root = root.path_for(".destack");
        let store = DiskCacheStore::new();
        let index_store = WorkspaceStore::new(&store, &cache_root);

        let files = FileRegistry::new();
        let program = Program::new(
            FormatterOptions::default(),
            LinterOptions::default(),
            root.root().to_path_buf(),
            Arc::new(PhysicalFileSystem),
            Arc::new(files),
            Arc::new(ModuleRegistry::new()),
            Arc::new(PackageRegistry::new()),
            Arc::new(TsConfigRegistry::new()),
            Arc::new(StringPool::new()),
            None,
        );

        let header = WorkspaceIndexHeader::new(
            "0.0.0".to_string(),
            root.root().to_path_buf(),
            Some(3),
            5,
            6,
            CacheValidate::Strict,
        );
        let snapshot = WorkspaceIndexSnapshot::from_program(&program, header)
            .unwrap_or_else(|error| panic!("failed to build snapshot: {error}"));

        index_store
            .save(&snapshot)
            .unwrap_or_else(|error| panic!("failed to write snapshot: {error}"));
        let loaded = index_store
            .load(&snapshot.header)
            .unwrap_or_else(|error| panic!("failed to read snapshot: {error}"))
            .unwrap_or_else(|| panic!("expected snapshot"));

        // check that the compiler version is loaded
        assert_eq!(loaded.header.compiler_version, "0.0.0");
    }

    /// Detect content changes even when metadata matches.
    #[test]
    fn test_workspace_index_detects_content_changes() {
        // set up a temp workspace and file
        let root = TemporaryPhysicalFileSystem::new_with_prefix("workspace_index_content");
        let file_path = root.write_text("main.ds", "abc").unwrap();

        let metadata = PhysicalFileSystem::metadata(&file_path).unwrap();
        let modified = metadata
            .modified_at
            .unwrap_or_else(|| panic!("missing modified time"));
        let entry = WorkspaceFileEntry::from_metadata(
            FileVersion::new(1),
            &metadata,
            Some(hash_bytes(b"abc")),
        );

        // build a snapshot with strict content hashes
        let header = WorkspaceIndexHeader::new(
            "0.0.0".to_string(),
            root.root().to_path_buf(),
            None,
            7,
            8,
            CacheValidate::Strict,
        );
        let mut files = IndexMap::new();
        files.insert(file_path.clone(), entry);
        let snapshot = WorkspaceIndexSnapshot {
            header,
            strings: StringPool::new(),
            files,
            modules: IndexMap::new(),
        };

        // write and reload
        let store = DiskCacheStore::new();
        let index_store = WorkspaceStore::new(&store, &root.path_for(".destack"));
        index_store
            .save(&snapshot)
            .unwrap_or_else(|error| panic!("failed to write snapshot: {error}"));
        let loaded = index_store
            .load(&snapshot.header)
            .unwrap_or_else(|error| panic!("failed to read snapshot: {error}"))
            .unwrap_or_else(|| panic!("expected snapshot"));

        // seed a program with the loaded snapshot
        let session = Session::new(root.root().to_path_buf())
            .with_fs(Arc::new(PhysicalFileSystem))
            .with_cache_store(Arc::new(DiskCacheStore::new()));
        session.apply_workspace_index(&loaded);

        // ensure version stays the same when hash matches
        assert_eq!(
            session.workspace_file_version_for_path(&file_path),
            FileVersion::new(1)
        );

        // mutate file contents but keep metadata same
        root.write_text("main.ds", "xyz").unwrap();
        filetime::set_file_mtime(&file_path, FileTime::from_system_time(modified))
            .unwrap_or_else(|error| panic!("failed to reset mtime: {error}"));

        assert_eq!(
            session.workspace_file_version_for_path(&file_path),
            FileVersion::new(2)
        );
    }

    /// Validate workspace index entries against filesystem metadata.
    #[test]
    fn test_workspace_index_entry_validation() {
        let file_metadata = FileMetadata::new(true, false, false, 12, Some(SystemTime::now()));
        let entry = WorkspaceFileEntry::from_metadata(FileVersion::new(1), &file_metadata, None);

        // check that the entry matches the metadata
        assert!(entry.matches_metadata(&file_metadata));
    }
}
