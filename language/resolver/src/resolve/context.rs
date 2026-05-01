use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::{ArtifactDependency, ArtifactPathState};
use destack_source::{FileContentId, FileId, FileMetadata, PackageId};
use destack_workspace::{DestackDeclaration, Revision, TsConfigDeclaration};

#[cfg(not(target_arch = "wasm32"))]
use pnp::fs::{LruZipCache, open_zip_via_read_p};

use crate::resolve::TsConfigKey;
use crate::{PackageScope, ResolverBase, ResolverResult, TypeScriptOptionsReferences};

/// The maximum number of open Yarn PnP zip files per query.
#[cfg(not(target_arch = "wasm32"))]
const PNP_ZIP_CACHE_SIZE: u64 = 8;

/// The request local scratch state for one resolve search chain.
#[derive(Debug)]
pub struct ResolverContext {
    /// The active repository revision for this request.
    revision: Revision,

    /// The exact resolver dependency facts observed in this request.
    dependencies: Vec<ArtifactDependency>,
    /// The exact resolver dependency facts already recorded in this request.
    dependency_set: HashSet<ArtifactDependency>,

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

impl ResolverContext {
    /// Build one request local context for one revision.
    pub fn new(revision: Revision) -> Self {
        Self {
            revision,
            dependencies: Vec::new(),
            dependency_set: HashSet::new(),
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

    /// Return the active repository revision for this request.
    pub(crate) fn revision(&self) -> Revision {
        self.revision
    }

    /// Return the dependencies observed so far.
    pub fn dependencies(&self) -> &[ArtifactDependency] {
        &self.dependencies
    }

    /// Consume the context and return its observed dependencies.
    pub fn into_dependencies(self) -> Vec<ArtifactDependency> {
        self.dependencies
    }

    /// Track one exact dependency.
    pub(crate) fn track_dependency(&mut self, dependency: ArtifactDependency) {
        if self.dependency_set.insert(dependency.clone()) {
            self.dependencies.push(dependency);
        }
    }

    /// Track one path state dependency.
    pub(crate) fn track_path_state(&mut self, path: FileId, state: ArtifactPathState) {
        self.track_dependency(ArtifactDependency::path(path, state));
    }

    /// Track one path content dependency.
    pub(crate) fn track_file_content(&mut self, file: FileId, content: FileContentId) {
        self.track_dependency(ArtifactDependency::file_content(file, content));
    }

    /// Return one cached package scope result when present.
    pub(crate) fn package_scope(&self, path: &Path) -> Option<Option<PackageId>> {
        self.package_scope_cache.get(path).cloned()
    }

    /// Cache one package scope result.
    pub(crate) fn cache_package_scope(&mut self, path: &Path, package_id: Option<PackageId>) {
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

    /// Cache one package scope entry.
    pub(crate) fn cache_package(&mut self, entry: PackageScope) {
        let package_id = entry.package.id;
        let package_path = entry.package.path.clone();
        let entry = Arc::new(entry);

        // update the id lookup first
        self.packages_by_id.insert(package_id, entry);

        // update the path lookup when this package has one
        if let Some(package_path) = package_path {
            self.package_ids_by_path.insert(package_path, package_id);
        }
    }

    /// Return one cached path metadata result when present.
    pub(crate) fn path_metadata(&self, path: &Path) -> Option<Option<FileMetadata>> {
        self.path_metadata_cache.get(path).copied()
    }

    /// Cache one path metadata result.
    pub(crate) fn cache_path_metadata(&mut self, path: &Path, metadata: Option<FileMetadata>) {
        self.path_metadata_cache
            .insert(path.to_path_buf(), metadata);
    }

    /// Return one cached nearest tsconfig result when present.
    pub(crate) fn nearest_tsconfig(&self, path: &Path) -> Option<Option<TsConfigKey>> {
        self.nearest_tsconfig_cache.get(path).cloned()
    }

    /// Cache one nearest tsconfig result.
    pub(crate) fn cache_nearest_tsconfig(
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
        base: ResolverBase<'_>,
        path: &Path,
    ) -> Option<Option<TsConfigKey>> {
        match base {
            ResolverBase::File(_) => self.effective_file_tsconfig_cache.get(path).cloned(),
            ResolverBase::Directory(_) => {
                self.effective_directory_tsconfig_cache.get(path).cloned()
            }
        }
    }

    /// Cache one effective tsconfig result.
    pub(crate) fn cache_effective_tsconfig(
        &mut self,
        base: ResolverBase<'_>,
        path: &Path,
        tsconfig_key: Option<TsConfigKey>,
    ) {
        match base {
            ResolverBase::File(_) => {
                self.effective_file_tsconfig_cache
                    .insert(path.to_path_buf(), tsconfig_key);
            }
            ResolverBase::Directory(_) => {
                self.effective_directory_tsconfig_cache
                    .insert(path.to_path_buf(), tsconfig_key);
            }
        }
    }

    /// Return one cached declaration by path when present.
    pub(crate) fn destack_declaration(&self, path: &Path) -> Option<&DestackDeclaration> {
        self.destack_declarations_by_path.get(path)
    }

    /// Cache one parsed destack declaration.
    pub(crate) fn cache_destack_declaration(&mut self, declaration: DestackDeclaration) {
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

    /// Cache one tsconfig declaration.
    pub(crate) fn cache_tsconfig(&mut self, key: TsConfigKey, tsconfig: TsConfigDeclaration) {
        self.tsconfigs_by_key.insert(key, tsconfig);
    }

    /// Execute a closure with one extended destack config pushed on the stack.
    pub(crate) fn with_extended_destack_config<F, T>(
        &mut self,
        path: PathBuf,
        f: F,
    ) -> ResolverResult<T>
    where
        F: FnOnce(&mut Self) -> ResolverResult<T>,
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
    pub(crate) fn with_extended_tsconfig<F, T>(&mut self, path: PathBuf, f: F) -> ResolverResult<T>
    where
        F: FnOnce(&mut Self) -> ResolverResult<T>,
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

    /// Cache one Yarn PnP manifest for this request.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn cache_pnp_manifest(&mut self, manifest: Arc<pnp::Manifest>) {
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

    /// Dependencies should be recorded once in the request local buffer.
    #[test]
    fn test_track_dependency_deduplicates() {
        let mut context = ResolverContext::new(Revision::NULL);
        let file_id = FileId::from_logical_path(Path::new("/tmp/found"));

        context.track_path_state(file_id, ArtifactPathState::File);
        context.track_path_state(file_id, ArtifactPathState::File);

        assert_eq!(
            context.dependencies(),
            vec![ArtifactDependency::path(file_id, ArtifactPathState::File)]
        );
    }

    /// Package scope entries should round trip through the query cache.
    #[test]
    fn test_cache_package_scope_roundtrips() {
        let mut context = ResolverContext::new(Revision::NULL);

        context.cache_package_scope(Path::new("/tmp"), Some(PackageId::new(1)));

        assert_eq!(
            context.package_scope(Path::new("/tmp")),
            Some(Some(PackageId::new(1)))
        );
    }
}
