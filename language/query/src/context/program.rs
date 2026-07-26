use std::sync::{Arc, OnceLock};

use destack_artifact::{ArtifactKey, ModuleIndex, PackageGraph, ProgramIndex};
use destack_repository::{
    ArtifactReader, PackageKind, ProviderError, Repository, RepositoryError, Revision,
};
use destack_source::{ModuleId, ProfileId};

use crate::{Module, ModuleQueryContext, QueryError, QueryResult};

/// Query context anchored to a program revision.
#[derive(Debug)]
pub struct ProgramQueryContext<'a> {
    /// The repository used for this query.
    repository: &'a Repository,
    /// The revision used for this query.
    revision: Revision,
    /// The profile covered by this program.
    profile_id: ProfileId,
    /// The program postings index.
    index: Arc<ProgramIndex>,
    /// Active package graph for this program profile.
    package_graph: Arc<PackageGraph>,
    /// Lazily realized module contexts in program index order.
    modules: Box<[OnceLock<Result<ModuleQueryContext<'a>, Box<ProviderError>>>]>,
    /// Lazily read module indexes in program index order.
    module_indexes: Box<[OnceLock<Result<Arc<ModuleIndex>, ProviderError>>]>,
}

impl<'a> ProgramQueryContext<'a> {
    /// Return artifacts required to build one complete program query context.
    pub fn artifact_keys(
        repository: &Repository,
        revision: Revision,
        profile_id: ProfileId,
    ) -> Result<Vec<ArtifactKey>, ProviderError> {
        let module_ids = repository.module_ids(revision).map_err(|error| {
            ProviderError::internal(format!("failed to read program modules: {error}"))
        })?;
        let mut artifacts = vec![ArtifactKey::package_graph(profile_id)];

        // request every module context and module index consumed by the program index
        for module_id in module_ids {
            artifacts.extend(ModuleQueryContext::artifact_keys(module_id, profile_id));
            artifacts.push(ArtifactKey::module_index(module_id, profile_id));
        }

        artifacts.push(ArtifactKey::program_index(profile_id));
        artifacts.sort_unstable();
        artifacts.dedup();

        Ok(artifacts)
    }

    /// Iterate over the exact modules covered by this program context.
    pub fn modules(&self) -> impl Iterator<Item = Module> + '_ {
        self.index.modules.iter().map(move |module_id| Module {
            module_id: *module_id,
            profile_id: self.profile_id,
        })
    }

    /// Return whether a module belongs to authored workspace source.
    pub(crate) fn is_authored_module(&self, module_id: ModuleId) -> QueryResult<bool> {
        let package_id = module_id.package_id;
        let package = self.repository.package(self.revision, package_id)?.ok_or(
            RepositoryError::MissingPackage {
                package: package_id,
            },
        )?;
        let is_authored = matches!(package.kind, PackageKind::Declared | PackageKind::Implicit);

        Ok(is_authored)
    }

    /// Return the authored modules covered by this program context.
    pub fn authored_modules(&self) -> QueryResult<Vec<Module>> {
        let mut modules = Vec::new();

        // retain only modules owned by configured workspace packages
        for module in self.modules() {
            if self.is_authored_module(module.module_id)? {
                modules.push(module);
            }
        }

        Ok(modules)
    }

    /// Return the repository for this program context.
    pub fn repository(&self) -> &'a Repository {
        self.repository
    }

    /// Return the revision for this program context.
    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// Return the profile covered by this program context.
    pub fn profile_id(&self) -> ProfileId {
        self.profile_id
    }

    /// Return the program postings index.
    pub(crate) fn index(&self) -> &ProgramIndex {
        self.index.as_ref()
    }

    /// Return the active package graph for this program.
    pub(crate) fn package_graph(&self) -> &PackageGraph {
        self.package_graph.as_ref()
    }

    /// Return one module context covered by this program.
    pub fn module(&self, module_id: ModuleId) -> QueryResult<&ModuleQueryContext<'a>> {
        let ordinal = self.module_ordinal(module_id)?;
        let context = self.modules[ordinal].get_or_init(|| {
            ModuleQueryContext::new(self.repository, self.revision, module_id, self.profile_id)
        });

        match context {
            Ok(context) => Ok(context),
            Err(error) => Err(QueryError::from(error.clone())),
        }
    }

    /// Return one module index selected by a program postings ordinal.
    pub(crate) fn module_index_at(&self, ordinal: u32) -> QueryResult<(ModuleId, &ModuleIndex)> {
        let ordinal = ordinal as usize;
        let module_id = *self
            .index
            .modules
            .get(ordinal)
            .ok_or(QueryError::invalid(format!("program ordinal: {ordinal:?}")))?;
        let index = self.module_index(module_id)?;

        Ok((module_id, index))
    }

    /// Return one module index covered by this program.
    pub(crate) fn module_index(&self, module_id: ModuleId) -> QueryResult<&ModuleIndex> {
        let ordinal = self.module_ordinal(module_id)?;
        let index = self.module_indexes[ordinal].get_or_init(|| {
            let artifacts = ArtifactReader::new(self.repository, self.revision);

            artifacts.module_index(module_id, self.profile_id)
        });

        match index {
            Ok(index) => Ok(index.as_ref()),
            Err(error) => Err(QueryError::from(error.clone())),
        }
    }

    /// Build one program query context from one exact profile index.
    pub fn new(
        repository: &'a Repository,
        revision: Revision,
        profile_id: ProfileId,
        index: Arc<ProgramIndex>,
        package_graph: Arc<PackageGraph>,
    ) -> QueryResult<Self> {
        // require the persisted module order used by postings lookups
        if index.modules.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(QueryError::invalid("program index"));
        }
        if package_graph.profile != profile_id {
            return Err(QueryError::invalid("package graph profile"));
        }

        let module_count = index.modules.len();
        let modules = (0..module_count)
            .map(|_| OnceLock::new())
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let module_indexes = (0..module_count)
            .map(|_| OnceLock::new())
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Ok(Self {
            repository,
            revision,
            profile_id,
            index,
            package_graph,
            modules,
            module_indexes,
        })
    }

    /// Return the ordinal for one indexed module.
    fn module_ordinal(&self, module_id: ModuleId) -> QueryResult<usize> {
        self.index
            .modules
            .binary_search(&module_id)
            .map_err(|_| QueryError::missing(format!("program module: {module_id:?}")))
    }
}
