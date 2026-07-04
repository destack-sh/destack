use std::sync::Arc;

use destack_artifact::{ArtifactKey, ModuleIndex, ProgramIndex};
use destack_repository::{Repository, Revision};
use destack_source::{ModuleId, ProfileId};

use super::ModuleQueryContext;
use super::module::{ready_artifact_version, ready_module_query_context};

/// Query context anchored to a program revision.
#[derive(Debug)]
pub struct ProgramQueryContext<'a> {
    /// The repository used for this query.
    repository: &'a Repository,
    /// The revision used for this query.
    revision: Revision,
    /// The indexed profiles included in this program query.
    profiles: Vec<ProgramQueryProfile>,
}

/// Index payloads for one profile.
#[derive(Debug)]
pub(crate) struct ProgramQueryProfile {
    /// The profile covered by these indexes.
    profile_id: ProfileId,
    /// The program postings index.
    index: Arc<ProgramIndex>,
    /// The module indexes referenced by the program index.
    modules: Vec<Arc<ModuleIndex>>,
}

impl ProgramQueryProfile {
    /// Return the profile covered by these indexes.
    pub(crate) fn profile_id(&self) -> ProfileId {
        self.profile_id
    }

    /// Return the module indexes for this profile.
    pub(crate) fn modules(&self) -> &[Arc<ModuleIndex>] {
        &self.modules
    }

    /// Return the program postings index for this profile.
    pub(crate) fn index(&self) -> &ProgramIndex {
        self.index.as_ref()
    }
}

impl<'a> ProgramQueryContext<'a> {
    /// Return the repository for this program context.
    pub fn repository(&self) -> &'a Repository {
        self.repository
    }

    /// Return the revision for this program context.
    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// Return the profiles included in this program context.
    pub fn profile_ids(&self) -> impl Iterator<Item = ProfileId> + '_ {
        self.profiles.iter().map(ProgramQueryProfile::profile_id)
    }

    /// Return the program query profiles.
    pub(crate) fn profiles(&self) -> &[ProgramQueryProfile] {
        &self.profiles
    }

    /// Return a module context for one module profile.
    pub(crate) fn module_context(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ModuleQueryContext<'a> {
        ready_module_query_context(self.repository, self.revision, module_id, profile_id)
    }
}

/// Return a program query context for explicit profiles.
pub fn program_query_context<'a>(
    repository: &'a Repository,
    revision: Revision,
    profile_indexes: impl IntoIterator<Item = (ProfileId, Arc<ProgramIndex>)>,
) -> ProgramQueryContext<'a> {
    let mut profiles = Vec::new();

    // load the module indexes referenced by each profile index
    for (profile_id, index) in profile_indexes {
        let modules = module_indexes(repository, revision, profile_id, &index);

        profiles.push(ProgramQueryProfile {
            profile_id,
            index,
            modules,
        });
    }

    ProgramQueryContext {
        repository,
        revision,
        profiles,
    }
}

/// Return all module indexes referenced by one program index.
fn module_indexes(
    repository: &Repository,
    revision: Revision,
    profile_id: ProfileId,
    program: &ProgramIndex,
) -> Vec<Arc<ModuleIndex>> {
    let mut indexes = Vec::with_capacity(program.modules.len());

    // read each indexed module payload from its artifact binding
    for module_id in &program.modules {
        let key = ArtifactKey::module_index(*module_id, profile_id);
        let version = ready_artifact_version(repository, revision, key);
        let index = repository
            .artifact_table()
            .module_index(&version)
            .unwrap_or_else(|| panic!("missing program module index payload: {version:?}"));

        indexes.push(index);
    }

    indexes
}
