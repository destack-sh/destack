use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{FileMetadata, PackageId};
use destack_workspace::{DestackDeclaration, Revision, TsConfigDeclaration};

#[cfg(not(target_arch = "wasm32"))]
use pnp::fs::{LruZipCache, open_zip_via_read_p};

use crate::resolve::TsConfigKey;
use crate::{PackageScope, ResolveError, ResolveOrigin, ResolveTrace, TypeScriptOptionsReferences};

/// The maximum number of open Yarn PnP zip files per query.
#[cfg(not(target_arch = "wasm32"))]
const PNP_ZIP_CACHE_SIZE: u64 = 8;

/// The request local scratch state for one resolve call chain.
#[derive(Debug)]
pub(crate) struct ResolveContext {
    /// The active repository revision for this request.
    revision: Revision,

    /// The files found while tracing this request.
    found_dependencies: Option<Vec<PathBuf>>,
    /// The files missed while tracing this request.
    missing_dependencies: Option<Vec<PathBuf>>,

    /// The memoized nearest package scope results for this request.
    package_scope_cache: HashMap<PathBuf, Option<PackageId>>,
    /// The memoized package ids by root path for this request.
    package_ids_by_path: HashMap<PathBuf, PackageId>,
    /// The memoized package entries by id for this request.
    packages_by_id: HashMap<PackageId, Arc<PackageScope>>,
    /// The memoized path metadata for this request.
    path_metadata_cache: HashMap<PathBuf, Option<FileMetadata>>,
    /// The memoized nearest tsconfig lookup results for this request.
    nearest_tsconfig_cache: HashMap<PathBuf, Option<TsConfigKey>>,
    /// The memoized effective file tsconfig lookup results for this request.
    effective_file_tsconfig_cache: HashMap<PathBuf, Option<TsConfigKey>>,
    /// The memoized effective directory tsconfig lookup results for this request.
    effective_directory_tsconfig_cache: HashMap<PathBuf, Option<TsConfigKey>>,

    /// The parsed destack declarations by path for this request.
    destack_declarations_by_path: HashMap<PathBuf, DestackDeclaration>,
    /// The active destack extends stack for this request.
    extended_destack_configs: Vec<PathBuf>,

    /// The tsconfig declarations by semantic key for this request.
    tsconfigs_by_key: HashMap<TsConfigKey, TsConfigDeclaration>,
    /// The active tsconfig extends stack for this request.
    extended_tsconfig_paths: Vec<PathBuf>,

    /// The active Yarn PnP manifest for this request.
    #[cfg(not(target_arch = "wasm32"))]
    pnp_manifest: Option<Arc<pnp::Manifest>>,
    /// The Yarn PnP zip cache for this request.
    #[cfg(not(target_arch = "wasm32"))]
    pnp_zip_cache: Option<LruZipCache<Vec<u8>>>,
}

impl ResolveContext {
    /// Build one request local context for one revision.
    pub(crate) fn new(revision: Revision) -> Self {
        Self {
            revision,
            found_dependencies: None,
            missing_dependencies: None,
            package_scope_cache: HashMap::new(),
            package_ids_by_path: HashMap::new(),
            packages_by_id: HashMap::new(),
            path_metadata_cache: HashMap::new(),
            nearest_tsconfig_cache: HashMap::new(),
            effective_file_tsconfig_cache: HashMap::new(),
            effective_directory_tsconfig_cache: HashMap::new(),
            destack_declarations_by_path: HashMap::new(),
            extended_destack_configs: Vec::new(),
            tsconfigs_by_key: HashMap::new(),
            extended_tsconfig_paths: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            pnp_manifest: None,
            #[cfg(not(target_arch = "wasm32"))]
            pnp_zip_cache: None,
        }
    }

    /// Build one tracing context for one revision.
    pub(crate) fn with_trace(revision: Revision) -> Self {
        Self {
            found_dependencies: Some(Vec::new()),
            missing_dependencies: Some(Vec::new()),
            ..Self::new(revision)
        }
    }

    /// Return the active repository revision for this request.
    pub(crate) fn revision(&self) -> Revision {
        self.revision
    }

    /// Append any recorded dependency tracing into the given public trace.
    pub(crate) fn append_trace_to(&mut self, trace: &mut ResolveTrace) {
        if let Some(found_dependencies) = &mut self.found_dependencies {
            trace.found_dependencies.append(found_dependencies);
        }

        if let Some(missing_dependencies) = &mut self.missing_dependencies {
            trace.missing_dependencies.append(missing_dependencies);
        }
    }

    /// Track one found dependency when dependency tracing is enabled.
    pub(crate) fn track_found_dependency(&mut self, path: &Path) {
        if let Some(dependencies) = &mut self.found_dependencies {
            dependencies.push(path.to_path_buf());
        }
    }

    /// Track one missing dependency when dependency tracing is enabled.
    pub(crate) fn track_missing_dependency(&mut self, path: &Path) {
        if let Some(dependencies) = &mut self.missing_dependencies {
            dependencies.push(path.to_path_buf());
        }
    }

    /// Return one cached package scope result when present.
    pub(crate) fn package_scope(&self, path: &Path) -> Option<Option<PackageId>> {
        self.package_scope_cache.get(path).cloned()
    }

    /// Remember one package scope result.
    pub(crate) fn remember_package_scope(&mut self, path: &Path, package_id: Option<PackageId>) {
        self.package_scope_cache
            .insert(path.to_path_buf(), package_id);
    }

    /// Return one cached package id by path when present.
    pub(crate) fn package_id_by_path(&self, path: &Path) -> Option<PackageId> {
        self.package_ids_by_path.get(path).copied()
    }

    /// Return one package scope by id when present.
    pub(crate) fn package(&self, id: PackageId) -> Option<Arc<PackageScope>> {
        self.packages_by_id.get(&id).cloned()
    }

    /// Remember one package scope in the request local cache.
    pub(crate) fn remember_package(&mut self, entry: PackageScope) {
        let package_id = entry.package.id;
        let package_path = entry.package.path.clone();
        let entry = Arc::new(entry);

        self.packages_by_id.insert(package_id, entry);

        if let Some(package_path) = package_path {
            self.package_ids_by_path.insert(package_path, package_id);
        }
    }

    /// Return one cached path metadata result when present.
    pub(crate) fn path_metadata(&self, path: &Path) -> Option<Option<FileMetadata>> {
        self.path_metadata_cache.get(path).copied()
    }

    /// Remember one path metadata result.
    pub(crate) fn remember_path_metadata(&mut self, path: &Path, metadata: Option<FileMetadata>) {
        self.path_metadata_cache
            .insert(path.to_path_buf(), metadata);
    }

    /// Return one cached nearest tsconfig result when present.
    pub(crate) fn nearest_tsconfig(&self, path: &Path) -> Option<Option<TsConfigKey>> {
        self.nearest_tsconfig_cache.get(path).cloned()
    }

    /// Remember one nearest tsconfig result.
    pub(crate) fn remember_nearest_tsconfig(
        &mut self,
        path: &Path,
        tsconfig_key: Option<TsConfigKey>,
    ) {
        self.nearest_tsconfig_cache
            .insert(path.to_path_buf(), tsconfig_key);
    }

    /// Return one effective tsconfig result when present.
    pub(crate) fn effective_tsconfig(
        &self,
        origin: ResolveOrigin,
        path: &Path,
    ) -> Option<Option<TsConfigKey>> {
        match origin {
            ResolveOrigin::File => self.effective_file_tsconfig_cache.get(path).cloned(),
            ResolveOrigin::Directory => self.effective_directory_tsconfig_cache.get(path).cloned(),
        }
    }

    /// Remember one effective tsconfig result.
    pub(crate) fn remember_effective_tsconfig(
        &mut self,
        origin: ResolveOrigin,
        path: &Path,
        tsconfig_key: Option<TsConfigKey>,
    ) {
        match origin {
            ResolveOrigin::File => {
                self.effective_file_tsconfig_cache
                    .insert(path.to_path_buf(), tsconfig_key);
            }
            ResolveOrigin::Directory => {
                self.effective_directory_tsconfig_cache
                    .insert(path.to_path_buf(), tsconfig_key);
            }
        }
    }

    /// Return one cached declaration by path when present.
    pub(crate) fn destack_declaration(&self, path: &Path) -> Option<DestackDeclaration> {
        self.destack_declarations_by_path.get(path).cloned()
    }

    /// Remember one parsed destack declaration.
    pub(crate) fn remember_destack_declaration(&mut self, declaration: DestackDeclaration) {
        self.destack_declarations_by_path
            .insert(declaration.path.clone(), declaration);
    }

    /// Return one tsconfig by semantic key when present.
    pub(crate) fn tsconfig(&self, key: &TsConfigKey) -> Option<TsConfigDeclaration> {
        self.tsconfigs_by_key.get(key).cloned()
    }

    /// Return one tsconfig by path and mode when present.
    pub(crate) fn tsconfig_for_path(
        &self,
        path: &Path,
        is_root: bool,
        references: &TypeScriptOptionsReferences,
    ) -> Option<TsConfigDeclaration> {
        let key = TsConfigKey::new(path, is_root, references);
        self.tsconfig(&key)
    }

    /// Remember one tsconfig declaration.
    pub(crate) fn remember_tsconfig(&mut self, key: TsConfigKey, tsconfig: TsConfigDeclaration) {
        self.tsconfigs_by_key.insert(key, tsconfig);
    }

    /// Execute a closure with one extended destack config pushed on the stack.
    pub(crate) fn with_extended_destack_config<F, T>(
        &mut self,
        path: PathBuf,
        f: F,
    ) -> Result<T, ResolveError>
    where
        F: FnOnce(&mut Self) -> Result<T, ResolveError>,
    {
        self.extended_destack_configs.push(path);
        let result = f(self);
        self.extended_destack_configs.pop();
        result
    }

    /// Return true when this request already visited the given destack config.
    pub(crate) fn is_extended_destack_config(&self, path: &Path) -> bool {
        self.extended_destack_configs
            .iter()
            .any(|extended| extended == path)
    }

    /// Return the active destack extends chain plus the given path.
    pub(crate) fn extended_destack_configs_with(&self, path: PathBuf) -> Vec<PathBuf> {
        let mut configs = self.extended_destack_configs.clone();
        configs.push(path);
        configs
    }

    /// Execute a closure with one extended tsconfig pushed on the stack.
    pub(crate) fn with_extended_tsconfig<F, T>(
        &mut self,
        path: PathBuf,
        f: F,
    ) -> Result<T, ResolveError>
    where
        F: FnOnce(&mut Self) -> Result<T, ResolveError>,
    {
        self.extended_tsconfig_paths.push(path);
        let result = f(self);
        self.extended_tsconfig_paths.pop();
        result
    }

    /// Return true when this request already visited the given tsconfig.
    pub(crate) fn is_extended_tsconfig(&self, path: &Path) -> bool {
        self.extended_tsconfig_paths
            .iter()
            .any(|extended| extended == path)
    }

    /// Return the active tsconfig extends chain plus the given path.
    pub(crate) fn extended_tsconfig_paths_with(&self, path: PathBuf) -> Vec<PathBuf> {
        let mut configs = self.extended_tsconfig_paths.clone();
        configs.push(path);
        configs
    }

    /// Return the cached Yarn PnP manifest when present.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn pnp_manifest(&self) -> Option<Arc<pnp::Manifest>> {
        self.pnp_manifest.clone()
    }

    /// Remember one Yarn PnP manifest for this request.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn remember_pnp_manifest(&mut self, manifest: Arc<pnp::Manifest>) {
        self.pnp_manifest = Some(manifest);
    }

    /// Return the query local Yarn PnP zip cache.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn pnp_zip_cache(&mut self) -> &mut LruZipCache<Vec<u8>> {
        self.pnp_zip_cache
            .get_or_insert_with(|| LruZipCache::new(PNP_ZIP_CACHE_SIZE, open_zip_via_read_p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Append trace should drain the request local buffers.
    #[test]
    fn test_append_trace_drains_dependencies() {
        let mut context = ResolveContext::with_trace(Revision::NULL);
        let mut trace = ResolveTrace::default();

        context.track_found_dependency(Path::new("/tmp/found"));
        context.track_missing_dependency(Path::new("/tmp/missing"));
        context.append_trace_to(&mut trace);

        assert_eq!(trace.found_dependencies, vec![PathBuf::from("/tmp/found")]);
        assert_eq!(
            trace.missing_dependencies,
            vec![PathBuf::from("/tmp/missing")]
        );
    }

    /// Package scope entries should round trip through the query cache.
    #[test]
    fn test_remember_package_scope_roundtrips() {
        let mut context = ResolveContext::new(Revision::NULL);

        context.remember_package_scope(Path::new("/tmp"), Some(PackageId::new(1)));

        assert_eq!(
            context.package_scope(Path::new("/tmp")),
            Some(Some(PackageId::new(1)))
        );
    }
}
