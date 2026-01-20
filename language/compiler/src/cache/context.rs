use destack_source::{
    CacheHeader, CacheKind, FileContent, FileId, FileVersion, ModuleId, ProfileId, ProfileVersion,
    TargetId,
};
use destack_workspace::{
    CacheError, CacheMode, DSCONFIG_CACHE_IGNORED_KEYS, ModuleGraphKey, ModuleSignatureDigest,
    ModuleSignatureKey, WorkspaceIndexError, WorkspaceIndexHeader, WorkspaceIndexSnapshot,
    WorkspaceIndexStore, hash_bytes, hash_workspace_config, resolve_cache_dir,
    resolve_cache_root_for_scope, trim_json_object,
};

use crate::compile::Compiler;

use super::hasher::CacheHasher;
use super::{CacheHandle, CacheOptions};

/// Bytes per megabyte.
pub(super) const BYTES_PER_MB: u64 = 1024 * 1024;

/// Cache context for a module.
#[derive(Debug, Clone)]
pub struct CacheContext {
    /// The compiler version that produced the cache.
    pub compiler_version: String,
    /// The file version used when producing the cache.
    pub file_version: FileVersion,
    /// The profile id for the cached payload when applicable.
    pub profile_id: Option<ProfileId>,
    /// The profile version used when producing the cache when applicable.
    pub profile_version: Option<ProfileVersion>,
    /// Hash of the normalized source contents.
    pub source_hash: u64,
    /// Hash of the effective compiler configuration.
    pub config_hash: u64,
    /// Hash of the target configuration and triple.
    pub target_hash: u64,
    /// Hash of dependency signatures for cache invalidation.
    pub dependency_hash: u64,
}

impl CacheContext {
    /// Build a cache header for this context.
    pub fn header(&self, cache_kind: CacheKind, module_id: ModuleId) -> CacheHeader {
        CacheHeader::new(
            cache_kind,
            self.compiler_version.clone(),
            module_id,
            self.file_version,
            self.profile_id,
            self.profile_version,
            self.source_hash,
            self.config_hash,
            self.target_hash,
            self.dependency_hash,
            0,
        )
    }
}

impl Compiler {
    /// Resolve cache options for a module.
    pub(crate) fn cache_options_for_module(&self, module_id: ModuleId) -> CacheOptions {
        // load module and package
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let package = self.program.packages.get(module.package_id);
        let package = package.read();

        // derive cache options from dsconfig
        let cache_options = package
            .dsconfig
            .as_ref()
            .map(|dsconfig| dsconfig.options.cache.clone())
            .unwrap_or_default();

        // resolve cache root directory
        let workspace_root = self.session.workspace.root.clone();
        let base_dir = package
            .dsconfig
            .as_ref()
            .map(|dsconfig| dsconfig.directory.clone())
            .unwrap_or_else(|| workspace_root.clone());
        let mut dir = resolve_cache_root_for_scope(
            &base_dir,
            cache_options.dir.as_deref(),
            cache_options.scope,
        );

        // override from session cache dir when provided
        if let Some(cache_dir) = self.session.options.cache_dir_override.as_ref() {
            dir = resolve_cache_dir(cache_dir, &workspace_root);
        }

        // build resolved cache options
        CacheOptions {
            mode: cache_options.mode,
            dir,
            policy: cache_options.policy,
            validate: cache_options.validate,
            scope: cache_options.scope,
            max_size_mb: cache_options.max_size_mb,
        }
    }

    /// Resolve cache options for the workspace index.
    pub(crate) fn workspace_cache_options(&self) -> CacheOptions {
        // derive cache options from workspace config
        let cache_options = self
            .session
            .workspace
            .config
            .as_ref()
            .map(|config| config.options.cache.clone())
            .unwrap_or_default();

        // resolve workspace cache root
        let dir = self.session.workspace_cache_dir();

        // build resolved cache options
        CacheOptions {
            mode: cache_options.mode,
            dir,
            policy: cache_options.policy,
            validate: cache_options.validate,
            scope: cache_options.scope,
            max_size_mb: cache_options.max_size_mb,
        }
    }

    /// Build a cache context for a module.
    pub(crate) fn cache_context_for_module(
        &self,
        module_id: ModuleId,
        profile_id: Option<ProfileId>,
        target_id: Option<&TargetId>,
        cache_kind: CacheKind,
    ) -> Result<CacheContext, CacheError> {
        // load module file id
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let file_id = module.file_id;

        // derive profile info
        let (resolved_profile_id, profile_version) = if cache_kind.requires_profile() {
            let profile_id = profile_id.ok_or(CacheError::MissingDependencyData {
                module_id,
                profile_id: None,
                reason: "missing profile id for cache context".to_string(),
            })?;
            let profile = self.program.profile(profile_id);
            (Some(profile_id), Some(profile.version))
        } else {
            (None, None)
        };

        // compute file and config hashes
        let file_version = self.cache_file_version(file_id)?;
        let source_hash = self.cache_source_hash(file_id)?;
        let config_hash = self.cache_config_hash(module_id, profile_id);
        let target_hash = self.cache_target_hash(module_id, target_id);
        let dependency_hash = self.cache_dependency_hash(module_id, profile_id, cache_kind)?;

        // build cache context
        Ok(CacheContext {
            compiler_version: env!("CARGO_PKG_VERSION").to_string(),
            file_version,
            profile_id: resolved_profile_id,
            profile_version,
            source_hash,
            config_hash,
            target_hash,
            dependency_hash,
        })
    }

    /// Build cache handle for a module when available.
    pub(crate) fn cache_handle_for_module(
        &self,
        module_id: ModuleId,
        profile_id: Option<ProfileId>,
        target_id: Option<&TargetId>,
        cache_kind: CacheKind,
    ) -> Option<CacheHandle<'_>> {
        // resolve cache options
        let options = self.cache_options_for_module(module_id);

        // skip when cache is disabled
        if !options.is_enabled() {
            return None;
        }

        // resolve cache context
        let context = self
            .cache_context_for_module(module_id, profile_id, target_id, cache_kind)
            .ok()?;

        Some(CacheHandle {
            registry: &self.cache,
            cache_store: self.session.cache_store.as_ref(),
            stats: self.stats.as_ref(),
            options,
            context,
            module_id,
        })
    }

    /// Compute the file version for a cache context.
    fn cache_file_version(&self, file_id: FileId) -> Result<FileVersion, CacheError> {
        let file = self.program.files.get(file_id);
        Ok(file.version)
    }

    /// Compute the source hash for a cache context.
    fn cache_source_hash(&self, file_id: FileId) -> Result<u64, CacheError> {
        // hash file content
        let file = self.program.files.get(file_id);
        match &file.content {
            FileContent::Text { content } => Ok(hash_bytes(content.as_bytes())),
            FileContent::Json { content, .. } => Ok(hash_bytes(content.as_bytes())),
            FileContent::Binary { content } => Ok(hash_bytes(content)),
            FileContent::Missing => Err(CacheError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "file content missing",
            ))),
            FileContent::Unloaded => Err(CacheError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "file content not loaded",
            ))),
        }
    }

    /// Compute the config hash for a cache context.
    fn cache_config_hash(&self, module_id: ModuleId, profile_id: Option<ProfileId>) -> u64 {
        // seed the config hash
        let mut hasher = CacheHasher::new();

        // hash the dsconfig content if available
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let package = self.program.packages.get(module.package_id);
        let package = package.read();
        if let Some(dsconfig) = package.dsconfig.as_ref() {
            let file = self.program.files.get(dsconfig.file_id);
            match &file.content {
                FileContent::Json { value, .. } => {
                    let trimmed = trim_json_object(value, &DSCONFIG_CACHE_IGNORED_KEYS);
                    hasher.hash_json_value(&trimmed);
                }
                FileContent::Text { content } => {
                    hasher.hash_bytes(content.as_bytes());
                }
                FileContent::Binary { content } => {
                    hasher.hash_bytes(content);
                }
                FileContent::Missing => {}
                FileContent::Unloaded => {}
            }
        }

        // hash tsconfig content when present
        if let Some(tsconfig_id) = module.tsconfig_id {
            let tsconfig = self.program.tsconfigs.get(tsconfig_id);
            let tsconfig = tsconfig.read();
            let file = self.program.files.get(tsconfig.file_id);
            match &file.content {
                FileContent::Json { value, .. } => {
                    hasher.hash_json_value(value);
                }
                FileContent::Text { content } => {
                    hasher.hash_bytes(content.as_bytes());
                }
                FileContent::Binary { content } => {
                    hasher.hash_bytes(content);
                }
                FileContent::Missing => {}
                FileContent::Unloaded => {}
            }
        }

        // hash compiler options that affect compilation
        hasher.hash_compiler_options(&self.options);

        // hash the profile key if provided
        if let Some(profile_id) = profile_id {
            let profile = self.program.profile(profile_id);
            hasher.hash_value(&profile.key);
        }

        // finish the config hash
        hasher.finish()
    }

    /// Compute the target hash for a cache context.
    fn cache_target_hash(&self, module_id: ModuleId, target_id: Option<&TargetId>) -> u64 {
        // hash the target id when present
        let mut hasher = CacheHasher::new();
        if let Some(target_id) = target_id {
            hasher.hash_value(target_id);

            // hash the resolved target config when available
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let package = self.program.packages.get(module.package_id);
            let package = package.read();
            if let Some(target) = package.targets.get(target_id) {
                hasher.hash_value(target);
            }
        }

        hasher.finish()
    }

    /// Compute dependency signature hash for a cache context.
    fn cache_dependency_hash(
        &self,
        module_id: ModuleId,
        profile_id: Option<ProfileId>,
        cache_kind: CacheKind,
    ) -> Result<u64, CacheError> {
        // only include dependency signatures for profile scoped caches
        if !cache_kind.requires_profile() {
            return Ok(0);
        }

        // resolve the profile id for dependency tracking
        let profile_id = profile_id.ok_or(CacheError::MissingDependencyData {
            module_id,
            profile_id: None,
            reason: "missing profile id for dependency tracking".to_string(),
        })?;

        // load the module graph for this profile
        let graph_key = ModuleGraphKey::new(profile_id);
        let Some(graph) = self.program.index.module_graphs.get(&graph_key) else {
            return Err(CacheError::MissingDependencyData {
                module_id,
                profile_id: Some(profile_id),
                reason: "module graph missing".to_string(),
            });
        };

        // snapshot dependency list and module versions before releasing the graph guard
        let dependencies = graph.dependencies_for(module_id);
        let graph_versions = graph.module_versions.clone();
        drop(graph);

        // resolve profile version for dependency signatures
        let Some(profile) = self.program.profiles.get(profile_id) else {
            return Err(CacheError::MissingDependencyData {
                module_id,
                profile_id: Some(profile_id),
                reason: "missing profile data for dependency tracking".to_string(),
            });
        };
        let profile_version = profile.version;

        // hash dependency ids and signature hashes
        let mut hasher = CacheHasher::new();
        for dependency in dependencies {
            // verify graph metadata is consistent with module versions
            let graph_version = graph_versions.get(&dependency).copied().ok_or(
                CacheError::MissingDependencyData {
                    module_id,
                    profile_id: Some(profile_id),
                    reason: format!("missing graph version for dependency {dependency:?}"),
                },
            )?;

            let module = self.program.modules.get(dependency);
            let module = module.read();
            let module_version = module.version;
            if graph_version != module_version {
                return Err(CacheError::MissingDependencyData {
                    module_id,
                    profile_id: Some(profile_id),
                    reason: format!("stale graph version for dependency {dependency:?}"),
                });
            }
            drop(module);

            let signature_key = ModuleSignatureKey::new(dependency, profile_id);
            let digest = if let Some(digest) = self
                .program
                .index
                .module_signature_digests
                .get(&signature_key)
            {
                *digest.value()
            } else if let Some(signature) = self.program.index.module_signatures.get(&signature_key)
            {
                ModuleSignatureDigest::new(
                    signature.module_id,
                    signature.profile_id,
                    signature.module_version,
                    signature.profile_version,
                    signature.hash,
                )
            } else {
                return Err(CacheError::MissingDependencyData {
                    module_id,
                    profile_id: Some(profile_id),
                    reason: format!("missing signature for dependency {dependency:?}"),
                });
            };

            if digest.module_version != module_version {
                return Err(CacheError::MissingDependencyData {
                    module_id,
                    profile_id: Some(profile_id),
                    reason: format!("stale signature for dependency {dependency:?}"),
                });
            }
            if digest.profile_version != profile_version {
                return Err(CacheError::MissingDependencyData {
                    module_id,
                    profile_id: Some(profile_id),
                    reason: format!("stale signature profile for dependency {dependency:?}"),
                });
            }
            hasher.hash_value(&dependency);
            hasher.hash_value(&digest.hash);
        }

        Ok(hasher.finish())
    }

    /// Load the workspace index snapshot into program state.
    pub(crate) fn load_workspace_index(&self) -> Result<(), WorkspaceIndexError> {
        // skip loading when disk cache is disabled
        let options = self.workspace_cache_options();
        if options.mode != CacheMode::Disk {
            return Ok(());
        }

        // resolve workspace index path
        let cache_root = options.dir.clone();
        let cache_store = self.session.cache_store.as_ref();
        let index_store = WorkspaceIndexStore::new(cache_store, &cache_root);

        let header = self.workspace_index_header()?;
        let snapshot = index_store.load(&header)?;
        let Some(snapshot) = snapshot else {
            return Ok(());
        };

        // apply snapshot to program state
        self.program.apply_workspace_index(snapshot);

        Ok(())
    }

    /// Flush the current program state into the workspace index.
    pub(crate) fn flush_workspace_index(&self) -> Result<(), WorkspaceIndexError> {
        // skip writing when disk cache is disabled
        let options = self.workspace_cache_options();
        if options.mode != CacheMode::Disk {
            return Ok(());
        }

        // resolve workspace index path
        let cache_root = options.dir.clone();
        let cache_store = self.session.cache_store.as_ref();
        let index_store = WorkspaceIndexStore::new(cache_store, &cache_root);

        let header = self.workspace_index_header()?;
        let snapshot = WorkspaceIndexSnapshot::from_program(&self.program, header)?;
        index_store.save(&snapshot)?;

        Ok(())
    }

    /// Build a workspace index header from the current session config.
    fn workspace_index_header(&self) -> Result<WorkspaceIndexHeader, WorkspaceIndexError> {
        let options = self.workspace_cache_options();
        let config_hash = hash_workspace_config(&self.session.workspace, self.session.fs.as_ref())?;
        let mut compiler_hasher = CacheHasher::new();
        compiler_hasher.hash_compiler_options(&self.options);
        let compiler_options_hash = compiler_hasher.finish();

        let mut resolve_hasher = CacheHasher::new();
        resolve_hasher.hash_resolve_options(&self.options.import_resolve);
        let resolve_options_hash = resolve_hasher.finish();
        Ok(WorkspaceIndexHeader::new(
            env!("CARGO_PKG_VERSION").to_string(),
            self.session.workspace.root.clone(),
            config_hash,
            compiler_options_hash,
            resolve_options_hash,
            options.validate,
        ))
    }
}
