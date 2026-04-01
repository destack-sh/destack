use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use destack_artifact::{ArtifactStore, CacheStore, DiskCacheStore, ProfileKey};
use destack_core::StringPool;
use destack_source::{
    DiagnosticCollection, DiagnosticCollector, File, FileId, FileStore, FileSystem, FileType,
    ModuleId, PackageId, PhysicalFileSystem, PrintOptions, ProfileId, TargetId, Uri,
};
use parking_lot::RwLock;

use crate::repository::{
    Builtins, ContentId, ContentStore, FileOrigin, QueryIndex, RepositoryError, RepositoryImage,
    RepositoryImageHeader, RepositoryImageKey, RepositoryImageStore, RepositoryOptions,
    RepositorySnapshot, discover_workspace_root,
};
use crate::revision::{Ref, Revision, RevisionData, SourceMap};
use crate::{
    DestackDeclaration, FormatterOptions, LinterOptions, TsConfigOptions, Workspace, WorkspaceKind,
};
use destack_source::DiagnosticStore;

/// Tsconfig context for one module profile decision.
#[derive(Debug, Clone)]
pub(crate) struct ModuleTsConfigContext {
    /// The normalized tsconfig options.
    pub(crate) options: TsConfigOptions,
    /// The tsconfig directory for resolving relative paths.
    pub(crate) directory: PathBuf,
}

/// One repository with lineage, sources, artifacts, diagnostics, and shared inputs.
#[derive(Debug)]
pub struct Repository {
    /// Default formatter options.
    pub formatter: FormatterOptions,
    /// Default linter options.
    pub linter: LinterOptions,
    /// The workspace root directory.
    pub(crate) root: PathBuf,
    /// The working directory for repository-relative operations.
    pub cwd: PathBuf,
    /// Repository defaults for downstream tools.
    pub(crate) options: RepositoryOptions,
    /// The file system backing repository discovery and loads.
    pub(crate) fs: Arc<dyn FileSystem>,
    /// The logical path for each file id.
    pub(crate) logical_path_by_file_id: DashMap<FileId, Arc<str>>,
    /// The origin metadata for each file id.
    pub(crate) file_origin_by_file_id: DashMap<FileId, FileOrigin>,
    /// The package id for each target id.
    pub(crate) package_id_by_target_id: DashMap<TargetId, PackageId>,
    /// The target name for each target id.
    pub(crate) target_name_by_target_id: DashMap<TargetId, Arc<str>>,
    /// Shared string pool.
    pub strings: Arc<StringPool>,
    /// The published source revision graph.
    pub(crate) revisions: DashMap<Revision, Arc<RevisionData>>,
    /// The named movable refs.
    pub(crate) refs: DashMap<Ref, Revision>,
    /// Shared immutable source contents.
    pub(crate) contents: ContentStore,
    /// Shared immutable derived artifacts.
    pub(crate) artifacts: Arc<ArtifactStore>,
    /// Shared transient diagnostics.
    pub diagnostics: DiagnosticCollector,
    /// Shared persisted diagnostics.
    pub(crate) diagnostic_store: DiagnosticStore,
    /// Shared builtin selection metadata.
    pub builtins: Arc<Builtins>,
    /// Shared query index storage.
    pub(crate) query_index: RwLock<QueryIndex>,
    /// Shared persistent cache backend.
    pub(crate) cache_store: Arc<dyn CacheStore>,
}

impl Repository {
    /// Open one repository rooted at one directory with default options and disk cache.
    pub fn open_root(root: PathBuf) -> Self {
        let repository = Self::new(
            root.clone(),
            RepositoryOptions::default(),
            Arc::new(DiskCacheStore::new()),
            Arc::new(PhysicalFileSystem::new()),
        );
        let reference = Ref::for_workspace_root(&root);

        repository
            .seed_root_source(&reference)
            .unwrap_or_else(|error| panic!("failed to seed root source: {error}"));
        repository
            .builtins
            .install_in_repository(&repository, &reference)
            .unwrap_or_else(|error| panic!("failed to seed builtin source: {error}"));

        repository
    }

    /// Open one repository rooted at one directory and import that root from one file system.
    pub fn open_root_from_fs(
        root: PathBuf,
        fs: Arc<dyn FileSystem>,
    ) -> Result<Self, RepositoryError> {
        let repository = Self::new(
            root.clone(),
            RepositoryOptions::default(),
            Arc::new(DiskCacheStore::new()),
            fs,
        );
        let reference = Ref::for_workspace_root(&root);
        let _ = repository.seed_root_source(&reference)?;
        let _ = repository
            .builtins
            .install_in_repository(&repository, &reference)?;
        let _ = repository.import_from_fs(&reference, &root)?;

        Ok(repository)
    }

    /// Open one repository after discovering workspace metadata from one path.
    pub fn open_detected_from_fs(
        path: PathBuf,
        fs: Arc<dyn FileSystem>,
    ) -> Result<Self, RepositoryError> {
        let root = discover_workspace_root(&fs, &path)?;

        Self::open_root_from_fs(root, fs)
    }

    /// Create one repository from explicit parts.
    pub fn new(
        root: PathBuf,
        options: RepositoryOptions,
        cache: Arc<dyn CacheStore>,
        fs: Arc<dyn FileSystem>,
    ) -> Self {
        let strings = Arc::new(StringPool::new());
        let contents = ContentStore::new();
        let mut builtins = Builtins::empty();

        let revisions = DashMap::new();
        let refs = DashMap::new();
        let cwd = root.clone();
        let workspace_reference = Ref::for_workspace_root(&root);

        // builtin metadata
        builtins.load_intrinsics();

        let repository = Self {
            formatter: FormatterOptions::default(),
            linter: LinterOptions::default(),
            root,
            cwd,
            options,
            fs,
            logical_path_by_file_id: DashMap::new(),
            file_origin_by_file_id: DashMap::new(),
            package_id_by_target_id: DashMap::new(),
            target_name_by_target_id: DashMap::new(),
            strings,
            revisions,
            refs,
            contents,
            artifacts: Arc::new(ArtifactStore::default()),
            diagnostics: DiagnosticCollector::new(),
            diagnostic_store: DiagnosticStore::new(),
            builtins: Arc::new(builtins),
            query_index: RwLock::new(QueryIndex::default()),
            cache_store: cache,
        };

        // initial repository revision
        let initial_revision = Arc::new(RevisionData::new(
            smallvec::SmallVec::new(),
            Arc::new(SourceMap::new()),
        ));
        let initial_revision_id = initial_revision.revision();
        repository
            .revisions
            .insert(initial_revision_id, initial_revision);
        repository
            .refs
            .insert(workspace_reference, initial_revision_id);

        repository
    }

    /// Override the default formatter options.
    pub fn with_formatter(mut self, formatter: FormatterOptions) -> Self {
        self.formatter = formatter;
        self
    }

    /// Override the default linter options.
    pub fn with_linter(mut self, linter: LinterOptions) -> Self {
        self.linter = linter;
        self
    }

    /// Override the backing cache store.
    pub fn with_cache_store(mut self, cache_store: Arc<dyn CacheStore>) -> Self {
        self.cache_store = cache_store;
        self
    }

    /// Override the cache directory root.
    pub fn with_cache_dir(mut self, cache_dir: PathBuf) -> Self {
        self.options.cache_dir_override = Some(cache_dir);
        self
    }

    /// Return the repository file system.
    pub fn file_system(&self) -> &Arc<dyn FileSystem> {
        &self.fs
    }

    /// Return the repository string pool.
    pub fn string_pool(&self) -> &Arc<StringPool> {
        &self.strings
    }

    /// Return the repository artifact store.
    pub fn artifact_store(&self) -> &Arc<ArtifactStore> {
        &self.artifacts
    }

    /// Return the repository builtin inputs.
    pub fn builtins(&self) -> &Arc<Builtins> {
        &self.builtins
    }

    /// Load one builtin library module set for one profile.
    pub fn load_builtin_library(
        &self,
        name: &str,
        profile_key: &ProfileKey,
    ) -> Option<Vec<ModuleId>> {
        self.builtins.load_library(name, profile_key)
    }

    /// Return the repository diagnostic store.
    pub fn diagnostic_store(&self) -> &DiagnosticStore {
        &self.diagnostic_store
    }

    /// Return the repository cache store.
    pub fn cache_store(&self) -> &Arc<dyn CacheStore> {
        &self.cache_store
    }

    /// Return the repository formatter options.
    pub fn formatter_options(&self) -> &FormatterOptions {
        &self.formatter
    }

    /// Print one diagnostic collection with revision scoped source context.
    pub fn print_diagnostics(&self, revision: Revision, diagnostics: &DiagnosticCollection) {
        let options = PrintOptions::new()
            .with_line_width(self.formatter.line_width as u32)
            .with_module_count(
                self.workspace_module_ids(revision)
                    .unwrap_or_default()
                    .len(),
            );

        let files = FileStore::new();
        let mut file_ids = diagnostics
            .iter()
            .into_iter()
            .map(|diagnostic| diagnostic.file_id)
            .collect::<Vec<_>>();

        for diagnostic in diagnostics.iter() {
            if let Some(secondary_spans) = diagnostic.secondary_spans.as_ref() {
                file_ids.extend(secondary_spans.iter().map(|span| span.span.file));
            }
        }

        file_ids.sort_unstable();
        file_ids.dedup();

        for file_id in file_ids {
            let Ok(Some(file)) = self.file(revision, file_id) else {
                continue;
            };

            files.insert(file.as_ref().clone());
        }

        destack_source::print_diagnostics(&files, diagnostics, options);
    }

    /// Return the repository linter options.
    pub fn linter_options(&self) -> &LinterOptions {
        &self.linter
    }

    /// Return the repository workspace root.
    pub fn workspace_root(&self) -> &Path {
        &self.root
    }

    /// Return one repository-derived workspace view for one revision.
    pub fn workspace(&self, revision: Revision) -> Result<Workspace, RepositoryError> {
        let workspace_declaration = self.workspace_destack_declaration(revision)?;
        let package_ids = self.workspace_package_ids(revision)?;
        let kind = if package_ids.len() > 1 {
            WorkspaceKind::Monorepo
        } else {
            WorkspaceKind::SinglePackage
        };

        Ok(Workspace {
            destack_file_id: workspace_declaration
                .as_ref()
                .map(|declaration| declaration.file_id),
            root: self.root.clone(),
            kind,
            package_ids,
        })
    }

    /// Return the repository default options.
    pub fn options(&self) -> &RepositoryOptions {
        &self.options
    }

    /// Resolve the repository cache directory.
    pub fn cache_dir(&self) -> PathBuf {
        if let Some(cache_dir) = self.options.cache_dir_override.as_ref() {
            if cache_dir.is_absolute() {
                return cache_dir.clone();
            }

            return self.root.join(cache_dir);
        }

        self.root.join(".destack")
    }

    /// Load one `destack.json` declaration from disk when present.
    pub fn load_destack_declaration_for_path(&self, path: &Path) -> Option<DestackDeclaration> {
        let content = self.fs.read_to_string(path).ok()?;
        let file = File::from_text_as_jsonc(
            FileId::new(0),
            path.file_name()?.to_string_lossy().to_string(),
            Uri::from_path(path),
            Some(path.to_path_buf()),
            FileType::Json,
            content,
        )
        .ok()?;
        let file = Arc::new(file);

        DestackDeclaration::parse(&file).ok()
    }

    /// Serialize the current repository image.
    pub fn serialize_repository_image(&self) -> Result<Vec<u8>, RepositoryError> {
        let image = self.build_repository_image()?;

        image
            .serialize()
            .map_err(|error| RepositoryError::RepositoryImage {
                operation: "serialize",
                message: error.to_string(),
            })
    }

    /// Load one persisted repository image when present.
    pub fn load_repository_image(&self) -> Result<bool, RepositoryError> {
        let image_store = self.repository_image_store();
        let Some(image) = image_store
            .load::<RepositorySnapshot>(&RepositoryImageKey::Snapshot)
            .map_err(|error| RepositoryError::RepositoryImage {
                operation: "load",
                message: error.to_string(),
            })?
        else {
            return Ok(false);
        };

        image
            .header
            .validate(
                &RepositoryImageKey::Snapshot,
                Self::repository_image_compiler_version(),
            )
            .map_err(|error| RepositoryError::RepositoryImage {
                operation: "validate",
                message: error.to_string(),
            })?;

        self.apply_repository_snapshot(&image.payload);

        Ok(true)
    }

    /// Load one repository image from serialized bytes.
    pub fn load_repository_image_bytes(&self, bytes: &[u8]) -> Result<(), RepositoryError> {
        let image = RepositoryImage::<RepositorySnapshot>::deserialize(bytes).map_err(|error| {
            RepositoryError::RepositoryImage {
                operation: "deserialize",
                message: error.to_string(),
            }
        })?;

        image
            .header
            .validate(
                &RepositoryImageKey::Snapshot,
                Self::repository_image_compiler_version(),
            )
            .map_err(|error| RepositoryError::RepositoryImage {
                operation: "validate",
                message: error.to_string(),
            })?;

        self.apply_repository_snapshot(&image.payload);

        Ok(())
    }

    /// Persist the current repository image into the cache store.
    pub fn store_repository_image(&self) -> Result<(), RepositoryError> {
        let image_store = self.repository_image_store();
        let image = self.build_repository_image()?;

        image_store
            .save(&image)
            .map_err(|error| RepositoryError::RepositoryImage {
                operation: "store",
                message: error.to_string(),
            })
    }

    /// Build one repository image store rooted at this repository cache directory.
    fn repository_image_store(&self) -> RepositoryImageStore<'_> {
        RepositoryImageStore::new(self.cache_store.as_ref(), &self.cache_dir())
    }

    /// Return the compiler version embedded in repository image headers.
    fn repository_image_compiler_version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// Build one repository image from the current repository state.
    fn build_repository_image(
        &self,
    ) -> Result<RepositoryImage<RepositorySnapshot>, RepositoryError> {
        let snapshot = RepositorySnapshot::from_strings(self.strings.as_ref());
        let header = RepositoryImageHeader::new(
            RepositoryImageKey::Snapshot,
            Self::repository_image_compiler_version().to_string(),
        );

        RepositoryImage::new(header, snapshot).map_err(|error| RepositoryError::RepositoryImage {
            operation: "build",
            message: error.to_string(),
        })
    }

    /// Apply one repository snapshot to the live repository state.
    fn apply_repository_snapshot(&self, snapshot: &RepositorySnapshot) {
        self.strings.replace_from(&snapshot.strings);
    }

    /// Return the stable profile id for one semantic profile key.
    pub fn profile_id(&self, key: ProfileKey) -> ProfileId {
        ProfileId::new(key.stable_hash())
    }

    /// Read the repository query index.
    pub fn with_query_index<R>(&self, read: impl FnOnce(&QueryIndex) -> R) -> R {
        let query_index = self.query_index.read();
        read(&query_index)
    }

    /// Mutate the repository query index.
    pub fn with_query_index_mut<R>(&self, write: impl FnOnce(&mut QueryIndex) -> R) -> R {
        let mut query_index = self.query_index.write();
        write(&mut query_index)
    }

    /// Return the current revision for one ref.
    pub fn current(&self, reference: &Ref) -> Result<Revision, RepositoryError> {
        self.refs
            .get(reference)
            .map(|entry| *entry.value())
            .ok_or_else(|| RepositoryError::MissingRef {
                reference: reference.clone(),
            })
    }

    /// Return one published revision.
    pub fn revision(&self, revision: Revision) -> Result<Arc<RevisionData>, RepositoryError> {
        self.revisions
            .get(&revision)
            .map(|entry| Arc::clone(entry.value()))
            .ok_or(RepositoryError::MissingRevision { revision })
    }

    /// Return one shared content payload.
    pub fn content(
        &self,
        content: ContentId,
    ) -> Result<Arc<destack_source::FileContent>, RepositoryError> {
        self.contents
            .get(content)
            .ok_or(RepositoryError::MissingContent { content })
    }
}
