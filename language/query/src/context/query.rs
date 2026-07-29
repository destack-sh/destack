use std::sync::OnceLock;

use destack_dir as dir;
use destack_repository::{ProviderError, Repository, RepositoryError, Revision};
use destack_source::{ModuleId, ProfileId};

use crate::{ModuleQueryContext, QueryError, QueryResult};

/// Shared semantic access for queries at one program revision.
#[derive(Debug)]
pub struct QueryContext<'a> {
    /// The repository used for this query.
    repository: &'a Repository,
    /// The revision used for this query.
    revision: Revision,
    /// The profile used for module artifacts.
    profile_id: ProfileId,
    /// Module ids addressable by this context.
    module_ids: Box<[ModuleId]>,
    /// Lazily realized module contexts.
    modules: Box<[OnceLock<Result<ModuleQueryContext<'a>, Box<ProviderError>>>]>,
}

impl<'a> QueryContext<'a> {
    /// Build semantic access for one exact profile revision.
    pub fn new(
        repository: &'a Repository,
        revision: Revision,
        profile_id: ProfileId,
    ) -> QueryResult<Self> {
        let mut module_ids = repository.module_ids(revision)?;
        module_ids.sort_unstable();
        module_ids.dedup();
        let modules = (0..module_ids.len())
            .map(|_| OnceLock::new())
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Ok(Self {
            repository,
            revision,
            profile_id,
            module_ids: module_ids.into_boxed_slice(),
            modules,
        })
    }

    /// Return the repository for this query.
    pub fn repository(&self) -> &'a Repository {
        self.repository
    }

    /// Return the exact revision for this query.
    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// Return the profile used for module artifacts.
    pub fn profile_id(&self) -> ProfileId {
        self.profile_id
    }

    /// Return the addressable modules in stable ordinal order.
    pub(crate) fn module_ids(&self) -> &[ModuleId] {
        &self.module_ids
    }

    /// Return one module context at this revision.
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

    /// Read one global type with its owning module.
    pub(crate) fn read_type<R>(
        &self,
        type_id: dir::GlobalTypeId,
        read: impl FnOnce(&dir::Type, &ModuleQueryContext<'_>) -> QueryResult<R>,
    ) -> QueryResult<R> {
        let module = self.module(type_id.module_id)?;
        let type_value = module.types().get_type(type_id.local_id);

        read(&type_value, module)
    }

    /// Return whether one module belongs to authored workspace source.
    pub(crate) fn is_authored_module(&self, module_id: ModuleId) -> QueryResult<bool> {
        let package_id = module_id.package_id;
        let package = self.repository.package(self.revision, package_id)?.ok_or(
            RepositoryError::MissingPackage {
                package: package_id,
            },
        )?;
        let is_authored = package.kind.is_authored();

        Ok(is_authored)
    }

    /// Return the ordinal of one addressable module.
    fn module_ordinal(&self, module_id: ModuleId) -> QueryResult<usize> {
        self.module_ids
            .binary_search(&module_id)
            .map_err(|_| QueryError::missing(format!("query module: {module_id:?}")))
    }
}
