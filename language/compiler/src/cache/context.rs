use std::hash::{Hash, Hasher};

use rustc_hash::FxHasher;

use destack_source::{
    CacheHeader, CacheKind, FileContent, FileId, FileVersion, ModuleId, ProfileId, ProfileVersion,
    TargetId,
};
use destack_workspace::{
    CacheError, CacheScope, ModuleGraphKey, ModuleSignatureKey, resolve_global_cache_root,
};

use crate::compile::Compiler;

use super::hash::{hash_bytes, hash_dsconfig_value};
use super::{CacheHandle, CacheOptions};

/// Default cache directory name for workspace scoped caches.
pub const DEFAULT_CACHE_DIR: &str = ".destack";
/// Default cache directory name for global caches.
pub const DEFAULT_GLOBAL_CACHE_DIR: &str = "destack";
/// Namespace for compiler cache entries.
pub const DEFAULT_CACHE_NAMESPACE: &str = "compiler";
/// Bytes per megabyte.
pub(super) const BYTES_PER_MB: u64 = 1024 * 1024;

/// Cache context for a module.
#[derive(Debug, Clone)]
pub struct CacheContext {
    /// The compiler version that produced the cache.
    pub compiler_version: String,
    /// The file version used when producing the cache.
    pub file_version: FileVersion,
    /// The profile id for the cached payload.
    pub profile_id: ProfileId,
    /// The profile version used when producing the cache.
    pub profile_version: ProfileVersion,
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
        let mut dir = if let Some(dsconfig) = package.dsconfig.as_ref()
            && let Some(cache_dir) = cache_options.dir.as_ref()
        {
            if cache_dir.is_absolute() {
                cache_dir.clone()
            } else {
                dsconfig.directory.join(cache_dir)
            }
        } else {
            workspace_root.join(DEFAULT_CACHE_DIR)
        };

        // apply global cache scope
        if cache_options.scope == CacheScope::Global
            && let Some(global_root) = resolve_global_cache_root(DEFAULT_GLOBAL_CACHE_DIR)
        {
            dir = global_root;
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
        let (resolved_profile_id, profile_version) = if let Some(profile_id) = profile_id {
            let profile = self.program.profile(profile_id);
            (profile_id, profile.version)
        } else {
            (ProfileId::new(0), ProfileVersion::INITIAL)
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
            FileContent::Unloaded => Err(CacheError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "file content not loaded",
            ))),
        }
    }

    /// Compute the config hash for a cache context.
    fn cache_config_hash(&self, module_id: ModuleId, profile_id: Option<ProfileId>) -> u64 {
        // seed the config hash
        let mut hasher = FxHasher::default();

        // hash the dsconfig content if available
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let package = self.program.packages.get(module.package_id);
        let package = package.read();
        if let Some(dsconfig) = package.dsconfig.as_ref() {
            let file = self.program.files.get(dsconfig.file_id);
            match &file.content {
                FileContent::Json { value, .. } => {
                    hash_dsconfig_value(value).hash(&mut hasher);
                }
                FileContent::Text { content } => {
                    hash_bytes(content.as_bytes()).hash(&mut hasher);
                }
                FileContent::Binary { content } => {
                    hash_bytes(content).hash(&mut hasher);
                }
                FileContent::Unloaded => {}
            }
        }

        // hash the profile key if provided
        if let Some(profile_id) = profile_id {
            let profile = self.program.profile(profile_id);
            profile.key.hash(&mut hasher);
        }

        // finish the config hash
        hasher.finish()
    }

    /// Compute the target hash for a cache context.
    fn cache_target_hash(&self, module_id: ModuleId, target_id: Option<&TargetId>) -> u64 {
        // hash the target id when present
        let mut hasher = FxHasher::default();
        if let Some(target_id) = target_id {
            target_id.hash(&mut hasher);

            // hash the resolved target config when available
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let package = self.program.packages.get(module.package_id);
            let package = package.read();
            if let Some(target) = package.targets.get(target_id) {
                target.hash(&mut hasher);
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
        // only include dependency signatures for mir caches
        if cache_kind != CacheKind::Mir {
            return Ok(0);
        }

        // resolve the profile id for dependency tracking
        let profile_id = profile_id.unwrap_or(ProfileId::new(0));

        // load the module graph for this profile
        let graph_key = ModuleGraphKey::new(profile_id);
        let Some(graph) = self.program.index.module_graphs.get(&graph_key) else {
            return Err(CacheError::MissingDependencyData {
                module_id,
                profile_id,
                reason: "module graph missing".to_string(),
            });
        };

        // snapshot dependency list before releasing the graph guard
        let dependencies = graph.dependencies_for(module_id);
        drop(graph);

        // hash dependency ids and signature hashes
        let mut hasher = FxHasher::default();
        for dependency in dependencies {
            let module = self.program.modules.get(dependency);
            if module.read().dir_maybe(profile_id).is_none() {
                return Err(CacheError::MissingDependencyData {
                    module_id,
                    profile_id,
                    reason: format!("missing dir for dependency {dependency:?}"),
                });
            }

            let signature_key = ModuleSignatureKey::new(dependency, profile_id);
            let Some(signature) = self.program.index.module_signatures.get(&signature_key) else {
                return Err(CacheError::MissingDependencyData {
                    module_id,
                    profile_id,
                    reason: format!("missing signature for dependency {dependency:?}"),
                });
            };
            dependency.hash(&mut hasher);
            signature.value().hash.hash(&mut hasher);
        }

        Ok(hasher.finish())
    }
}
