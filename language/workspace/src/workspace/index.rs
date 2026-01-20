use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use indexmap::IndexMap;
use postcard::Error as PostcardError;
use rustc_hash::FxHasher;
use serde::{Deserialize, Serialize};

use destack_source::{
    CACHE_FORMAT_VERSION, CACHE_MAGIC, FileContent, FileId, FileMetadata, FileSystem, FileVersion,
    ModuleId, ModuleVersion, PathExt, strip_json,
};

use crate::{
    CacheScope, CacheValidate, DEFAULT_COMPILER_CACHE_NAMESPACE, DsConfig, ModuleGraph,
    ModuleGraphKey, ModuleSignatureDigest, ModuleSignatureKey, Program, WORKSPACE_INDEX_FILE_NAME,
    Workspace, resolve_cache_dir, resolve_cache_root_for_scope,
};

/// Maximum size allowed for workspace index payloads.
pub const WORKSPACE_INDEX_LIMIT_BYTES: u64 = 256 * 1024 * 1024;

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
    /// Cache validation strategy for the snapshot.
    pub cache_validate: CacheValidate,
}

impl WorkspaceIndexHeader {
    /// Create a new workspace index header.
    pub fn new(
        compiler_version: String,
        workspace_root: PathBuf,
        config_hash: Option<u64>,
        cache_validate: CacheValidate,
    ) -> Self {
        Self {
            magic: CACHE_MAGIC,
            format_version: CACHE_FORMAT_VERSION,
            compiler_version,
            workspace_root,
            config_hash,
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

        self.config_hash == expected.config_hash && self.cache_validate == expected.cache_validate
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
    /// File entries keyed by path.
    pub files: IndexMap<PathBuf, WorkspaceFileEntry>,
    /// Module entries keyed by module id.
    pub modules: IndexMap<ModuleId, WorkspaceModuleEntry>,
    /// Module graphs keyed by profile.
    pub module_graphs: IndexMap<ModuleGraphKey, ModuleGraph>,
    /// Module signature digests keyed by module and profile.
    pub module_signature_digests: IndexMap<ModuleSignatureKey, ModuleSignatureDigest>,
}

impl WorkspaceIndexSnapshot {
    /// Build a snapshot from the current program state.
    pub fn from_program(
        program: &Program,
        header: WorkspaceIndexHeader,
    ) -> Result<Self, WorkspaceIndexError> {
        // collect file and module entries from program modules
        let cache_validate = header.cache_validate;
        let mut files = IndexMap::new();
        let mut modules = IndexMap::new();

        // walk modules and snapshot file/module metadata
        for module in program.modules.iter() {
            // snapshot module metadata
            let module = module.read();
            modules.insert(
                module.id,
                WorkspaceModuleEntry::new(module.version, module.source_version),
            );

            // skip modules without file paths
            let Some(path) = module.path.as_ref() else {
                continue;
            };

            // skip duplicate file paths
            if files.contains_key(path) {
                continue;
            }

            // collect file metadata and optional content hashes
            let file = program.files.get(module.file_id);
            let entry = match program.fs.metadata(path) {
                Ok(metadata) => {
                    let content_hash =
                        if cache_validate == CacheValidate::Strict && metadata.is_file {
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

        // snapshot module graphs
        let mut module_graphs = IndexMap::new();
        let mut graph_entries: Vec<_> = program
            .index
            .module_graphs
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect();
        graph_entries.sort_by_key(|(key, _)| *key);
        for (key, graph) in graph_entries {
            module_graphs.insert(key, graph);
        }

        // snapshot signature digests
        let mut module_signature_digests = IndexMap::new();
        let mut digest_entries: Vec<_> = program
            .index
            .module_signature_digests
            .iter()
            .map(|entry| (*entry.key(), *entry.value()))
            .collect();
        digest_entries.sort_by_key(|(key, _)| *key);
        for (key, digest) in digest_entries {
            module_signature_digests.insert(key, digest);
        }

        Ok(Self {
            header,
            files,
            modules,
            module_graphs,
            module_signature_digests,
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

/// Compute a hash for the workspace dsconfig when present.
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
            path: root.join("dsconfig.json"),
            required: false,
        })
        .collect();

    // hash dsconfig contents for each package root
    let mut hasher = FxHasher::default();
    let mut found_any = false;
    let mut visited = HashSet::new();

    while let Some(candidate) = pending.pop() {
        // resolve dsconfig paths
        let Some(resolved) = resolve_dsconfig_path(fs, &candidate.path) else {
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

    // return the hash when configs were found
    if found_any {
        return Ok(Some(hasher.finish()));
    }

    Ok(None)
}

/// Resolve a dsconfig path to the on disk file location.
fn resolve_dsconfig_path(fs: &dyn FileSystem, path: &Path) -> Option<PathBuf> {
    // resolve path to the actual dsconfig file
    let meta = fs.metadata(path).ok();

    // use the path when it already points at a file
    if meta.is_some_and(|meta| meta.is_file) {
        return Some(path.to_path_buf());
    }

    // resolve directories to dsconfig.json when present
    if meta.is_some_and(|meta| meta.is_directory) {
        let candidate = path.join("dsconfig.json");
        let candidate_meta = fs.metadata(&candidate).ok();
        if candidate_meta.is_some_and(|meta| meta.is_file) {
            return Some(candidate);
        }
        return None;
    }

    // skip paths that already include an extension
    if path.extension().is_some() {
        return None;
    }

    // try a json extension when missing
    let mut candidate = path.to_path_buf();
    candidate.set_extension("json");
    let candidate_meta = fs.metadata(&candidate).ok();
    if candidate_meta.is_some_and(|meta| meta.is_file) {
        return Some(candidate);
    }

    // no config found
    None
}

/// Resolve an extends specifier relative to a base directory.
fn resolve_extends_path(directory: &Path, specifier: &str) -> PathBuf {
    match specifier.as_bytes().first() {
        None => directory.join(specifier),
        Some(b'/') => PathBuf::from(specifier),
        Some(b'.') => directory.normalize_with(specifier),
        _ => directory.normalize_with(specifier),
    }
}

/// Hash a JSON value with stable ordering for cache purposes.
fn hash_json_value(value: &serde_json::Value) -> u64 {
    let mut hasher = FxHasher::default();
    hash_json_value_inner(value, &mut hasher);
    hasher.finish()
}

/// Hash a JSON object with a stable key ordering.
fn hash_json_object(map: &serde_json::Map<String, serde_json::Value>, hasher: &mut FxHasher) {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();

    for key in keys {
        key.hash(hasher);
        if let Some(value) = map.get(key) {
            hash_json_value_inner(value, hasher);
        }
    }
}

/// Hash a JSON value with canonical ordering for object keys.
fn hash_json_value_inner(value: &serde_json::Value, hasher: &mut FxHasher) {
    match value {
        serde_json::Value::Null => {
            0_u8.hash(hasher);
        }
        serde_json::Value::Bool(value) => {
            1_u8.hash(hasher);
            value.hash(hasher);
        }
        serde_json::Value::Number(value) => {
            2_u8.hash(hasher);
            value.to_string().hash(hasher);
        }
        serde_json::Value::String(value) => {
            3_u8.hash(hasher);
            value.hash(hasher);
        }
        serde_json::Value::Array(values) => {
            4_u8.hash(hasher);
            values.len().hash(hasher);
            for entry in values {
                hash_json_value_inner(entry, hasher);
            }
        }
        serde_json::Value::Object(map) => {
            5_u8.hash(hasher);
            hash_json_object(map, hasher);
        }
    }
}

/// Resolve the cache root directory for a workspace.
pub fn resolve_workspace_cache_root(
    workspace_root: &Path,
    workspace_config: Option<&DsConfig>,
    cache_dir_override: Option<&Path>,
) -> PathBuf {
    // honor explicit overrides
    if let Some(cache_dir) = cache_dir_override {
        return resolve_cache_dir(cache_dir, workspace_root);
    }

    // honor dsconfig cache settings when present
    if let Some(dsconfig) = workspace_config {
        let cache_options = &dsconfig.options.cache;
        return resolve_cache_root_for_scope(
            &dsconfig.directory,
            cache_options.dir.as_deref(),
            cache_options.scope,
        );
    }

    resolve_cache_root_for_scope(workspace_root, None, CacheScope::Workspace)
}

/// Resolve the workspace index path for a cache root.
pub fn workspace_index_path(cache_root: &Path) -> PathBuf {
    cache_root
        .join(DEFAULT_COMPILER_CACHE_NAMESPACE)
        .join(WORKSPACE_INDEX_FILE_NAME)
}

/// Read a workspace index snapshot if it matches the expected header.
pub fn read_workspace_index(
    path: &Path,
    expected: &WorkspaceIndexHeader,
) -> Result<Option<WorkspaceIndexSnapshot>, WorkspaceIndexError> {
    // short circuit when the index is missing
    if !path.exists() {
        return Ok(None);
    }

    // read the index bytes
    let bytes = std::fs::read(path).map_err(WorkspaceIndexError::Io)?;
    let size = bytes.len() as u64;
    if size > WORKSPACE_INDEX_LIMIT_BYTES {
        return Err(WorkspaceIndexError::SizeLimitExceeded {
            limit: WORKSPACE_INDEX_LIMIT_BYTES,
            actual: size,
        });
    }

    // decode the snapshot
    let snapshot: WorkspaceIndexSnapshot =
        postcard::from_bytes(&bytes).map_err(WorkspaceIndexError::Deserialize)?;

    // validate the header
    if !snapshot.header.matches(expected) {
        return Ok(None);
    }

    Ok(Some(snapshot))
}

/// Write a workspace index snapshot to disk.
pub fn write_workspace_index(
    path: &Path,
    snapshot: &WorkspaceIndexSnapshot,
) -> Result<(), WorkspaceIndexError> {
    // serialize the snapshot
    let bytes = postcard::to_allocvec(snapshot).map_err(WorkspaceIndexError::Serialize)?;

    // guard against oversized payloads
    let size = bytes.len() as u64;
    if size > WORKSPACE_INDEX_LIMIT_BYTES {
        return Err(WorkspaceIndexError::SizeLimitExceeded {
            limit: WORKSPACE_INDEX_LIMIT_BYTES,
            actual: size,
        });
    }

    // ensure the parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(WorkspaceIndexError::Io)?;
    }

    // write atomically
    write_workspace_bytes_atomic(path, &bytes)?;

    Ok(())
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

/// Hash bytes with a stable hasher.
fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = FxHasher::default();
    bytes.hash(&mut hasher);
    hasher.finish()
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

/// Write workspace index bytes using an atomic replace.
fn write_workspace_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), WorkspaceIndexError> {
    // resolve target directory and file name
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workspace");

    // build a temp path for atomic replace
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp_name = format!("{file_name}.tmp-{}-{timestamp}", std::process::id());
    let temp_path = dir.join(temp_name);

    // write and sync temp file
    let mut file = std::fs::File::create(&temp_path).map_err(WorkspaceIndexError::Io)?;
    std::io::Write::write_all(&mut file, bytes).map_err(WorkspaceIndexError::Io)?;
    file.sync_all().map_err(WorkspaceIndexError::Io)?;

    // best effort directory sync
    if let Ok(dir_file) = std::fs::File::open(dir) {
        let _ = dir_file.sync_all();
    }

    // rename into place with fallback for existing targets
    if let Err(error) = std::fs::rename(&temp_path, path) {
        if path.exists() {
            let _ = std::fs::remove_file(path);
            std::fs::rename(&temp_path, path).map_err(WorkspaceIndexError::Io)?;
        } else {
            return Err(WorkspaceIndexError::Io(error));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    use destack_source::{
        FileMetadata, FileVersion, MemoryFileSystem, ModuleId, ModuleVersion, strip_json,
    };
    use indexmap::IndexMap;

    use crate::{CacheValidate, Workspace};

    use super::{
        WorkspaceFileEntry, WorkspaceIndexHeader, WorkspaceIndexSnapshot, WorkspaceModuleEntry,
        hash_workspace_config, read_workspace_index, workspace_index_path, write_workspace_index,
    };

    /// Roundtrip workspace index snapshots through disk.
    #[test]
    fn test_workspace_index_roundtrip() {
        // setup a temp workspace directory
        let root =
            std::env::temp_dir().join(format!("destack-workspace-index-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let cache_root = root.join(".destack");
        let index_path = workspace_index_path(&cache_root);

        // build a minimal snapshot
        let header = WorkspaceIndexHeader::new(
            "0.0.0".to_string(),
            root.clone(),
            Some(42),
            CacheValidate::Strict,
        );
        let mut files = IndexMap::new();
        files.insert(
            root.join("a.ds"),
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
            files,
            modules,
            module_graphs: IndexMap::new(),
            module_signature_digests: IndexMap::new(),
        };

        // write and read the snapshot
        write_workspace_index(&index_path, &snapshot).unwrap();
        let loaded = read_workspace_index(&index_path, &header)
            .unwrap()
            .expect("expected snapshot");

        // assertion block
        assert_eq!(loaded.header.compiler_version, "0.0.0");
        assert_eq!(loaded.files.len(), 1);
        assert_eq!(loaded.modules.len(), 1);

        // cleanup
        let _ = fs::remove_dir_all(root);
    }

    /// Reject snapshots when the header does not match.
    #[test]
    fn test_workspace_index_header_mismatch() {
        // set up a temp workspace directory
        let root = std::env::temp_dir().join(format!(
            "destack-workspace-index-mismatch-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let cache_root = root.join(".destack");
        let index_path = workspace_index_path(&cache_root);

        // write a minimal snapshot
        let header = WorkspaceIndexHeader::new(
            "0.0.0".to_string(),
            root.clone(),
            Some(1),
            CacheValidate::Strict,
        );
        let snapshot = WorkspaceIndexSnapshot {
            header: header.clone(),
            files: IndexMap::new(),
            modules: IndexMap::new(),
            module_graphs: IndexMap::new(),
            module_signature_digests: IndexMap::new(),
        };
        write_workspace_index(&index_path, &snapshot).unwrap();

        // read with mismatched header
        let mismatch = WorkspaceIndexHeader::new(
            "0.0.1".to_string(),
            root.clone(),
            Some(1),
            CacheValidate::Strict,
        );
        let loaded = read_workspace_index(&index_path, &mismatch).unwrap();

        // assertion block
        assert!(loaded.is_none());

        // cleanup
        let _ = fs::remove_dir_all(root);
    }

    /// Ignore comments when hashing workspace config.
    #[test]
    fn test_hash_workspace_config_ignores_comments() {
        // set up a workspace with a commented config
        let fs = MemoryFileSystem::new();
        let root = PathBuf::from("/workspace");
        let dsconfig_path = root.join("dsconfig.json");
        let commented = r#"{
  // comment
  "compilerOptions": { "strict": true }
}
"#;
        let stripped =
            strip_json(commented).unwrap_or_else(|error| panic!("failed to strip json: {error}"));
        fs.add_file(&dsconfig_path, commented.as_bytes()).unwrap();
        let workspace = Workspace::single_package(root);

        // compute hash with comments
        let hash_with_comments = hash_workspace_config(&workspace, &fs)
            .unwrap_or_else(|error| panic!("failed to hash config: {error}"))
            .unwrap_or_else(|| panic!("expected config hash"));

        // overwrite config without comments
        fs.add_file(&dsconfig_path, stripped.as_bytes()).unwrap();
        let hash_without_comments = hash_workspace_config(&workspace, &fs)
            .unwrap_or_else(|error| panic!("failed to hash config: {error}"))
            .unwrap_or_else(|| panic!("expected config hash"));

        // assertion block
        assert_eq!(hash_with_comments, hash_without_comments);
    }

    /// Track changes in extended workspace configs.
    #[test]
    fn test_hash_workspace_config_tracks_extends() {
        // set up a workspace with an extended config
        let fs = MemoryFileSystem::new();
        let root = PathBuf::from("/workspace");
        let dsconfig_path = root.join("dsconfig.json");
        let base_path = root.join("base.json");
        fs.add_file(
            &dsconfig_path,
            br#"{ "extends": "./base", "compilerOptions": { "strict": true } }"#,
        )
        .unwrap();
        fs.add_file(&base_path, br#"{ "linter": { "preset": "recommended" } }"#)
            .unwrap();
        let workspace = Workspace::single_package(root);

        // compute the initial hash
        let initial_hash = hash_workspace_config(&workspace, &fs)
            .unwrap_or_else(|error| panic!("failed to hash config: {error}"))
            .unwrap_or_else(|| panic!("expected config hash"));

        // update the extended config
        fs.add_file(&base_path, br#"{ "linter": { "preset": "strict" } }"#)
            .unwrap();
        let updated_hash = hash_workspace_config(&workspace, &fs)
            .unwrap_or_else(|error| panic!("failed to hash config: {error}"))
            .unwrap_or_else(|| panic!("expected config hash"));

        // assertion block
        assert_ne!(initial_hash, updated_hash);
    }

    /// Reject invalid workspace config JSON.
    #[test]
    fn test_hash_workspace_config_rejects_invalid_json() {
        // set up a workspace with invalid config
        let fs = MemoryFileSystem::new();
        let root = PathBuf::from("/workspace");
        let dsconfig_path = root.join("dsconfig.json");
        fs.add_file(&dsconfig_path, br#"{ "compilerOptions": "#)
            .unwrap();
        let workspace = Workspace::single_package(root);

        // compute hash
        let result = hash_workspace_config(&workspace, &fs);

        // assertion block
        assert!(result.is_err(), "expected invalid config to error");
    }

    /// Reject missing extended workspace configs.
    #[test]
    fn test_hash_workspace_config_rejects_missing_extends() {
        // set up a workspace with missing extends
        let fs = MemoryFileSystem::new();
        let root = PathBuf::from("/workspace");
        let dsconfig_path = root.join("dsconfig.json");
        fs.add_file(&dsconfig_path, br#"{ "extends": "./missing" }"#)
            .unwrap();
        let workspace = Workspace::single_package(root);

        // compute hash
        let result = hash_workspace_config(&workspace, &fs);

        // assertion block
        assert!(result.is_err(), "expected missing extends to error");
    }

    /// Track version changes for workspace file entries.
    #[test]
    fn test_workspace_file_entry_versions() {
        // set up file metadata samples
        let version = FileVersion::new(3);
        let timestamp = SystemTime::UNIX_EPOCH + Duration::new(5, 0);
        let metadata = FileMetadata::new(true, false, false, 12, Some(timestamp));
        let updated_metadata = FileMetadata::new(true, false, false, 14, Some(timestamp));

        // build entry from metadata
        let entry = WorkspaceFileEntry::from_metadata(version, &metadata, None);

        // assertion block
        assert!(entry.matches_metadata(&metadata));
        assert_eq!(entry.version_for_metadata(&metadata), version);
        assert_eq!(
            entry.version_for_metadata(&updated_metadata),
            version.next()
        );
        assert_eq!(entry.version_for_missing(), version.next());
        assert_eq!(
            WorkspaceFileEntry::missing(version).version_for_missing(),
            version
        );
    }
}
