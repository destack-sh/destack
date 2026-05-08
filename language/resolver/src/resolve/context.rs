use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use destack_artifact::{ArtifactDependency, ArtifactPathState};
use destack_source::{FileContentId, FileId, FileMetadata};
use destack_workspace::{DestackConfig, Revision};

use crate::ResolverResult;

/// The request local scratch state for one resolve search chain.
#[derive(Debug)]
pub struct ResolverContext {
    /// The active repository revision for this request.
    revision: Revision,

    /// The exact resolver dependency facts observed in this request.
    dependencies: Vec<ArtifactDependency>,
    /// The exact resolver dependency facts already recorded in this request.
    dependency_set: HashSet<ArtifactDependency>,

    /// The memoized path metadata for this request.
    path_metadata_cache: HashMap<PathBuf, Option<FileMetadata>>,
    /// The parsed destack configs by path for this request.
    destack_configs_by_path: HashMap<PathBuf, DestackConfig>,
    /// The active destack extends stack for this request.
    extended_destack_configs: Vec<PathBuf>,
}

impl ResolverContext {
    /// Build one request local context for one revision.
    pub fn new(revision: Revision) -> Self {
        Self {
            revision,
            dependencies: Vec::new(),
            dependency_set: HashSet::new(),
            path_metadata_cache: HashMap::new(),
            destack_configs_by_path: HashMap::new(),
            extended_destack_configs: Vec::new(),
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

    /// Return one cached path metadata result when present.
    pub(crate) fn path_metadata(&self, path: &Path) -> Option<Option<FileMetadata>> {
        self.path_metadata_cache.get(path).copied()
    }

    /// Cache one path metadata result.
    pub(crate) fn cache_path_metadata(&mut self, path: &Path, metadata: Option<FileMetadata>) {
        self.path_metadata_cache
            .insert(path.to_path_buf(), metadata);
    }

    /// Return one cached config by path when present.
    pub(crate) fn destack_config(&self, path: &Path) -> Option<&DestackConfig> {
        self.destack_configs_by_path.get(path)
    }

    /// Cache one parsed destack config.
    pub(crate) fn cache_destack_config(&mut self, config: DestackConfig) {
        self.destack_configs_by_path
            .insert(config.path.clone(), config);
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

        assert_eq!(context.dependencies().len(), 1);
    }
}
