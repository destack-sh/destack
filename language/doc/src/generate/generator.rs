use rustc_hash::FxHashMap;
use tspp_dir as dir;
use tspp_repository::{Repository, Revision};
use tspp_source::{ModuleId, ProfileId};

use crate::{DocError, DocResult};

use super::Module;

/// Generator for one checked program revision.
pub struct Generator<'a> {
    /// The repository containing the checked program.
    repository: &'a Repository,
    /// The exact source revision.
    revision: Revision,
    /// The profile used for checked artifacts.
    profile: ProfileId,
    /// The checked module closure.
    modules: FxHashMap<ModuleId, Module<'a>>,
}

impl std::fmt::Debug for Generator<'_> {
    /// Format the documented revision and profile.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Generator")
            .field("revision", &self.revision)
            .field("profile", &self.profile)
            .finish_non_exhaustive()
    }
}

impl<'a> Generator<'a> {
    /// Read a generator over completed compiler artifacts.
    pub fn read(
        repository: &'a Repository,
        revision: Revision,
        profile: ProfileId,
        module_ids: impl IntoIterator<Item = ModuleId>,
    ) -> DocResult<Self> {
        let mut modules = FxHashMap::default();

        // compose the complete checked module closure before rendering
        for module_id in module_ids {
            let module = Module::read(repository, revision, module_id, profile)?;
            if modules.insert(module_id, module).is_some() {
                return Err(DocError::invalid(format!(
                    "duplicate documentation module: {module_id:?}"
                )));
            }
        }

        Ok(Self {
            repository,
            revision,
            profile,
            modules,
        })
    }

    /// Return the documented repository.
    pub(crate) fn repository(&self) -> &'a Repository {
        self.repository
    }

    /// Return the documented revision.
    pub(crate) fn revision(&self) -> Revision {
        self.revision
    }

    /// Return the checked profile.
    pub(crate) fn profile(&self) -> ProfileId {
        self.profile
    }

    /// Return one checked module.
    pub(crate) fn module(&self, module_id: ModuleId) -> DocResult<&Module<'a>> {
        self.modules
            .get(&module_id)
            .ok_or_else(|| DocError::missing(format!("documentation module: {module_id:?}")))
    }

    /// Read one global type with its owning module.
    pub(crate) fn read_type<R>(
        &self,
        type_id: dir::GlobalTypeId,
        read: impl FnOnce(&dir::Type, &Module<'_>) -> DocResult<R>,
    ) -> DocResult<R> {
        let module = self.module(type_id.module_id)?;
        let type_value = module.types().get_type(type_id.local_id);

        read(&type_value, module)
    }
}
