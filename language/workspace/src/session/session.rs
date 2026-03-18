use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use destack_core::StringPool;
use destack_source::{
    File, FileRegistry, FileSystem, FileType, FileVersion, ModuleId, ModuleVersion,
    PhysicalFileSystem,
};
use parking_lot::RwLock;

use crate::{
    Builtins, CacheStore, Destack, DiskCacheStore, FormatterOptions, LinterOptions,
    ModuleRegistry, PackageRegistry, ProfileKey, Program, SessionOptions, TsConfigRegistry,
    Workspace, WorkspaceFileEntry, WorkspaceIndexHeader, WorkspaceIndexSnapshot,
    WorkspaceModuleEntry, file_content_hash_for_path, resolve_workspace_cache_root,
};

/// A session is the persistent state for a workspace.
#[derive(Debug)]
pub struct Session {
    /// The workspace this session is for.
    pub workspace: RwLock<Workspace>,
    /// The current working directory.
    pub cwd: PathBuf,
    /// Session options for default tooling behavior.
    pub options: SessionOptions,
    /// The file system.
    pub fs: Arc<dyn FileSystem>,
    /// The cache store.
    pub cache_store: Arc<dyn CacheStore>,
    /// The files in the session.
    pub files: Arc<FileRegistry>,

    /// The programs in the session, keyed by root path.
    pub programs: DashMap<PathBuf, Arc<Program>>,
    /// The package registry (for builtins and cross-program sharing).
    pub packages: Arc<PackageRegistry>,
    /// The module registry (for builtins).
    pub modules: Arc<ModuleRegistry>,
    /// The tsconfig registry (separate registry as tsconfigs can be nested within packages).
    pub tsconfigs: Arc<TsConfigRegistry>,
    /// The combined string pool.
    pub strings: Arc<StringPool>,
    /// Loaded workspace-index header for this session.
    workspace_index_header: RwLock<Option<WorkspaceIndexHeader>>,
    /// Loaded workspace-index file version seeds for this session.
    workspace_index_files: DashMap<PathBuf, WorkspaceFileEntry>,
    /// Loaded workspace-index module version seeds for this session.
    workspace_index_modules: DashMap<ModuleId, WorkspaceModuleEntry>,
    /// Compiled builtins (always loaded).
    pub builtins: Arc<Builtins>,
}

impl Session {
    /// Create a new session with builtins loaded.
    pub fn new(cwd: PathBuf) -> Self {
        let workspace = Arc::new(Workspace::single_package(cwd.clone()));
        Self::workspace(cwd, workspace)
    }

    /// Create a session for a workspace with builtins loaded.
    pub fn workspace(cwd: PathBuf, workspace: Arc<Workspace>) -> Self {
        // create session options with defaults
        let options = SessionOptions::default();

        let files = Arc::new(FileRegistry::new());
        let modules = Arc::new(ModuleRegistry::new());
        let packages = Arc::new(PackageRegistry::new());
        let tsconfigs = Arc::new(TsConfigRegistry::new());
        let strings = Arc::new(StringPool::new());
        let builtins = Arc::new(Builtins::embedded(
            files.clone(),
            modules.clone(),
            packages.clone(),
        ));

        Self {
            workspace: RwLock::new((*workspace).clone()),
            cwd,
            options,
            fs: Arc::new(PhysicalFileSystem::new()),
            cache_store: Arc::new(DiskCacheStore::new()),
            files,

            programs: DashMap::new(),
            packages,
            modules,
            tsconfigs,
            strings,
            workspace_index_header: RwLock::new(None),
            workspace_index_files: DashMap::new(),
            workspace_index_modules: DashMap::new(),
            builtins,
        }
    }

    /// Set the file system (builder pattern).
    pub fn with_fs(mut self, fs: Arc<dyn FileSystem>) -> Self {
        self.fs = fs;
        self
    }

    /// Set the cache store (builder pattern).
    pub fn with_cache_store(mut self, cache_store: Arc<dyn CacheStore>) -> Self {
        self.cache_store = cache_store;
        self
    }

    /// Set session options (builder pattern).
    pub fn with_options(mut self, options: SessionOptions) -> Self {
        self.options = options;
        self
    }

    /// Set the workspace for this session (builder pattern).
    pub fn with_workspace(mut self, workspace: Workspace) -> Self {
        self.workspace = RwLock::new(workspace);
        self
    }

    /// Set the cache dir override (builder pattern).
    pub fn with_cache_dir(mut self, cache_dir: PathBuf) -> Self {
        self.options.cache_dir_override = Some(cache_dir);
        self
    }

    /// Set formatter options (builder pattern).
    pub fn with_formatter(mut self, formatter: FormatterOptions) -> Self {
        self.options.formatter = formatter;
        self
    }

    /// Set linter options (builder pattern).
    pub fn with_linter(mut self, linter: LinterOptions) -> Self {
        self.options.linter = linter;
        self
    }

    /// Resolve the cache root directory for this workspace.
    pub fn workspace_cache_dir(&self) -> PathBuf {
        // derive the workspace cache root
        let workspace = self.workspace.read();
        resolve_workspace_cache_root(
            &workspace.root,
            workspace.config.as_deref(),
            self.options.cache_dir_override.as_deref(),
        )
    }

    /// Snapshot the current workspace state.
    pub fn workspace_snapshot(&self) -> Workspace {
        self.workspace.read().clone()
    }

    /// Return the current workspace root.
    pub fn workspace_root(&self) -> PathBuf {
        self.workspace.read().root.clone()
    }

    /// Return the current workspace config.
    pub fn workspace_config(&self) -> Option<Arc<Destack>> {
        self.workspace.read().config.clone()
    }

    /// Update the workspace configuration.
    pub fn update_workspace_config(&self, config: Option<Destack>) {
        // update the workspace config in place
        let mut workspace = self.workspace.write();
        workspace.config = config.map(Arc::new);
    }

    /// Resolve a cached file version for a path using the workspace index.
    pub fn workspace_file_version_for_path(&self, path: &Path) -> FileVersion {
        // return the initial version when no snapshot exists
        let Some(entry) = self
            .workspace_index_files
            .get(path)
            .map(|entry| *entry.value())
        else {
            return FileVersion::INITIAL;
        };

        // compare against filesystem metadata when available
        let metadata = self.fs.metadata(path).ok();
        let Some(metadata) = metadata else {
            return entry.version_for_missing();
        };

        // return early when metadata changes
        let version = entry.version_for_metadata(&metadata);
        if version != entry.version {
            return version;
        }

        // skip content hashing when no hash is recorded
        let Some(expected_hash) = entry.content_hash else {
            return entry.version;
        };

        // compare against the current content hash
        let Some(current_hash) =
            file_content_hash_for_path(self.fs.as_ref(), self.files.as_ref(), path)
        else {
            return entry.version.next();
        };

        if current_hash == expected_hash {
            entry.version
        } else {
            entry.version.next()
        }
    }

    /// Resolve a cached module version for one module id and file version.
    pub fn workspace_module_version_for_id(
        &self,
        module_id: ModuleId,
        file_version: FileVersion,
    ) -> ModuleVersion {
        // return the initial version when no snapshot exists
        let Some(entry) = self
            .workspace_index_modules
            .get(&module_id)
            .map(|entry| *entry.value())
        else {
            return ModuleVersion::INITIAL;
        };

        // use the stored version when the source version matches
        if entry.source_version == file_version {
            entry.version
        } else {
            entry.version.next()
        }
    }

    /// Apply one loaded workspace index snapshot to the session state.
    pub fn apply_workspace_index_snapshot(&self, snapshot: &WorkspaceIndexSnapshot) {
        // update the loaded workspace-index header
        *self.workspace_index_header.write() = Some(snapshot.header.clone());

        // replace the loaded workspace-index entries
        self.workspace_index_files.clear();
        self.workspace_index_modules.clear();

        for (path, entry) in &snapshot.files {
            self.workspace_index_files.insert(path.clone(), *entry);
        }

        for (module_id, entry) in &snapshot.modules {
            self.workspace_index_modules.insert(*module_id, *entry);
        }
    }

    /// Serialize the loaded workspace index when present.
    pub fn serialize_loaded_workspace_index(&self) -> Result<Option<Vec<u8>>, postcard::Error> {
        let Some(header) = self.workspace_index_header.read().clone() else {
            return Ok(None);
        };

        // collect loaded file entries in stable order
        let mut files = indexmap::IndexMap::new();
        let mut file_entries: Vec<_> = self
            .workspace_index_files
            .iter()
            .map(|entry| (entry.key().clone(), *entry.value()))
            .collect();
        file_entries.sort_by(|(left, _), (right, _)| left.cmp(right));
        for (path, entry) in file_entries {
            files.insert(path, entry);
        }

        // collect loaded module entries in stable order
        let mut modules = indexmap::IndexMap::new();
        let mut module_entries: Vec<_> = self
            .workspace_index_modules
            .iter()
            .map(|entry| (*entry.key(), *entry.value()))
            .collect();
        module_entries.sort_by_key(|(module_id, _)| *module_id);
        for (module_id, entry) in module_entries {
            modules.insert(module_id, entry);
        }

        let snapshot = WorkspaceIndexSnapshot {
            header,
            files,
            modules,
        };

        postcard::to_allocvec(&snapshot).map(Some)
    }

    /// Return true when the loaded workspace index has one file entry.
    pub fn has_workspace_index_file_entry(&self, path: &Path) -> bool {
        self.workspace_index_files.contains_key(path)
    }

    /// Return true when the loaded workspace index has one module entry.
    pub fn has_workspace_index_module_entry(&self, module_id: ModuleId) -> bool {
        self.workspace_index_modules.contains_key(&module_id)
    }

    /// Load one tracked destack config file from the workspace.
    pub fn load_destack_for_path(&self, path: &Path) -> Option<Destack> {
        // read raw config content
        let content = self.fs.read_to_string(path).ok()?;

        // build a tracked jsonc file entry
        let file_id = self
            .files
            .get_id_by_path(path)
            .unwrap_or_else(|| self.files.next_id());
        let file_version = self.workspace_file_version_for_path(path);
        let name = path.file_name()?.to_string_lossy().to_string();
        let uri = destack_source::Uri::from_path(path);
        let file = File::from_text_as_jsonc(
            file_id,
            name,
            uri,
            Some(path.to_path_buf()),
            FileType::Json,
            content,
        )
        .ok()?
        .with_version(file_version);

        // insert or refresh the tracked file entry
        if self.files.contains_path(path) {
            self.files.replace(file);
        } else {
            self.files.insert(file);
        }

        // parse the tracked config payload
        let file = self.files.get(file_id);
        Destack::parse(&file).ok()
    }

    /// Load a lib module set (e.g., "dom", "es2024").
    /// Returns None if the lib name is not registered.
    pub fn load_lib(&self, name: &str, profile_key: &ProfileKey) -> Option<Vec<ModuleId>> {
        self.builtins
            .load_lib(name, self.files.clone(), self.modules.clone(), profile_key)
    }

    /// Add a root to the session.
    /// Creates a new Program for the given root path.
    pub fn add_root(&self, root: PathBuf) -> Arc<Program> {
        let program = Arc::new(Program::new(
            self.options.formatter,
            self.options.linter.clone(),
            root.clone(),
            self.fs.clone(),
            self.files.clone(),
            self.modules.clone(),
            self.packages.clone(),
            self.tsconfigs.clone(),
            self.strings.clone(),
            Some(self.builtins.clone()),
        ));
        self.programs.insert(root.clone(), program.clone());
        program
    }

    /// Remove a root from the session.
    pub fn remove_root(&self, root: &Path) -> Option<Arc<Program>> {
        self.programs.remove(root).map(|(_, program)| program)
    }

    /// Get a program by root path.
    pub fn get_program(&self, root: &Path) -> Option<Arc<Program>> {
        self.programs.get(root).map(|p| p.clone())
    }

    /// Get or create a program for the given root path.
    pub fn get_or_create_program(&self, root: PathBuf) -> Arc<Program> {
        if let Some(program) = self.programs.get(&root) {
            return program.clone();
        }
        self.add_root(root)
    }

    /// Find the program that contains the given path.
    pub fn find_program_for_path_maybe(&self, path: &Path) -> Option<Arc<Program>> {
        // track the best matching root by depth
        let mut best_match: Option<(usize, Arc<Program>)> = None;
        for entry in self.programs.iter() {
            if !path.starts_with(entry.key()) {
                continue;
            }

            let depth = entry.key().components().count();
            let replace = best_match
                .as_ref()
                .map(|(best_depth, _)| depth > *best_depth)
                .unwrap_or(true);
            if replace {
                best_match = Some((depth, entry.value().clone()));
            }
        }

        // return the deepest matching root
        best_match.map(|(_, program)| program)
    }

    /// Find the program that contains the given path.
    /// Falls back to the cwd-based program if no match is found.
    pub fn find_program_for_path(&self, path: &Path) -> Arc<Program> {
        // return the best matching program when available
        if let Some(program) = self.find_program_for_path_maybe(path) {
            return program;
        }

        // fallback: create/get a program for the cwd
        self.get_or_create_program(self.cwd.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use destack_source::MemoryFileSystem;

    use super::Session;
    use crate::MemoryCacheStore;

    /// Removes roots from the session program map.
    #[test]
    fn test_session_remove_root() {
        let fs = Arc::new(MemoryFileSystem::new());
        let root = PathBuf::from("/workspace");
        let session = Session::new(root.clone())
            .with_fs(fs)
            .with_cache_store(Arc::new(MemoryCacheStore::new()));

        let program = session.add_root(root.clone());
        let removed = session.remove_root(&root);

        assert!(removed.is_some());
        assert!(Arc::ptr_eq(&removed.unwrap(), &program));
        assert!(session.get_program(&root).is_none());
    }

    /// Returns None when removing an unknown root.
    #[test]
    fn test_session_remove_root_missing() {
        let fs = Arc::new(MemoryFileSystem::new());
        let root = PathBuf::from("/workspace");
        let session = Session::new(root)
            .with_fs(fs)
            .with_cache_store(Arc::new(MemoryCacheStore::new()));

        let missing = session.remove_root(PathBuf::from("/missing").as_path());
        assert!(missing.is_none());
    }

    /// Selects the deepest matching program root for path routing.
    #[test]
    fn test_session_find_program_for_path_prefers_deepest_root() {
        let fs = Arc::new(MemoryFileSystem::new());
        let workspace_root = PathBuf::from("/workspace");
        let package_root = PathBuf::from("/workspace/packages/app");
        let session = Session::new(workspace_root.clone())
            .with_fs(fs)
            .with_cache_store(Arc::new(MemoryCacheStore::new()));

        let workspace_program = session.add_root(workspace_root.clone());
        let package_program = session.add_root(package_root.clone());

        let path = package_root.join("src/main.ds");
        let resolved = session
            .find_program_for_path_maybe(path.as_path())
            .expect("expected matching root");

        assert!(!Arc::ptr_eq(&resolved, &workspace_program));
        assert!(Arc::ptr_eq(&resolved, &package_program));
    }
}
