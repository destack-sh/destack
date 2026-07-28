use destack_artifact::ArtifactKey;
use destack_core::{FxIndexMap, FxIndexSet};
use destack_pattern::{ModuleContext, ProgramContext};
use destack_repository::ArtifactReader;
use destack_source::{ModuleId, ProfileId};

use super::super::{CommandContext, CommandError, CommandResult};

/// Checked programs grouped by their selected profile.
#[derive(Debug)]
pub(super) struct CheckedPrograms {
    /// The selected profile for each searched module.
    profiles: FxIndexMap<ModuleId, ProfileId>,
    /// Complete checked program state for each selected profile.
    programs: FxIndexMap<ProfileId, ProgramContext>,
    /// Checked artifact roots requested from the compiler.
    artifact_keys: Vec<ArtifactKey>,
}

impl CheckedPrograms {
    /// Build complete checked program contexts for searched modules.
    pub(super) fn new(context: &CommandContext<'_>, modules: &[ModuleId]) -> CommandResult<Self> {
        let revision = context.revision()?;
        let mut profiles = FxIndexMap::default();
        let mut roots_by_profile = FxIndexMap::<ProfileId, Vec<ModuleId>>::default();

        // group searched modules by the profile selected for each module
        for module in modules.iter().copied() {
            let profile = context.selected_profile_id(revision, module)?;
            profiles.insert(module, profile);
            roots_by_profile.entry(profile).or_default().push(module);
        }

        let mut programs = FxIndexMap::default();
        let mut artifact_keys = Vec::new();
        for (profile, roots) in roots_by_profile {
            let program = Self::build_program(context, profile, &roots, &mut artifact_keys)?;
            programs.insert(profile, program);
        }

        Ok(Self {
            profiles,
            programs,
            artifact_keys,
        })
    }

    /// Return the checked module and program for one searched module.
    pub(super) fn module(
        &self,
        module: ModuleId,
    ) -> CommandResult<(&ModuleContext, &ProgramContext)> {
        let profile = self
            .profiles
            .get(&module)
            .ok_or_else(|| CommandError::internal(format!("missing profile for {module:?}")))?;
        let program = self.programs.get(profile).ok_or_else(|| {
            CommandError::internal(format!("missing checked pattern program for {profile:?}"))
        })?;
        let module = program
            .module(module)
            .map_err(|error| CommandError::internal(error.to_string()))?;

        Ok((module, program))
    }

    /// Return checked artifact roots requested from the compiler.
    pub(super) fn artifact_keys(&self) -> &[ArtifactKey] {
        &self.artifact_keys
    }

    /// Return the number of selected profiles.
    pub(super) fn profile_count(&self) -> usize {
        self.programs.len()
    }

    /// Build one checked program from a profile's complete loaded module closure.
    fn build_program(
        context: &CommandContext<'_>,
        profile: ProfileId,
        roots: &[ModuleId],
        artifact_keys: &mut Vec<ArtifactKey>,
    ) -> CommandResult<ProgramContext> {
        let revision = context.revision()?;
        let root_keys = roots
            .iter()
            .map(|module| ArtifactKey::dir_checked(*module, profile))
            .collect::<Vec<_>>();
        context
            .session
            .provide(revision, &root_keys)
            .map_err(|error| error.to_string())?;

        // resolve the exact reference and inherent extension closure
        let revision = context.revision()?;
        let artifacts = ArtifactReader::new(context.repository.as_ref(), revision);
        let graph = artifacts
            .component_graph(profile)
            .map_err(|error| error.to_string())?;
        let global = artifacts
            .global_environment(profile)
            .map_err(|error| error.to_string())?;
        let implicit = global.implicit_modules().collect::<Vec<_>>();
        let mut components = FxIndexSet::default();
        for root in roots {
            let component = graph.reference_component(*root).ok_or_else(|| {
                CommandError::internal(format!("module {root:?} has no reference component"))
            })?;
            let external = graph.external_reference_components(component, implicit.iter().copied());
            components.insert(component);
            components.extend(external.components());
        }
        let modules = components
            .iter()
            .flat_map(|component| graph.reference_members(*component))
            .copied()
            .collect::<FxIndexSet<_>>();

        // provide every checked artifact consumed by ModuleContext
        let keys = modules
            .iter()
            .flat_map(|module| {
                [
                    ArtifactKey::dir_parsed(*module),
                    ArtifactKey::dir_bound(*module, profile),
                    ArtifactKey::dir_expanded(*module, profile),
                    ArtifactKey::dir_exported(*module, profile),
                    ArtifactKey::dir_resolved(*module, profile),
                    ArtifactKey::dir_checked(*module, profile),
                ]
            })
            .collect::<Vec<_>>();
        context
            .session
            .provide(revision, &keys)
            .map_err(|error| error.to_string())?;
        artifact_keys.extend(root_keys);

        // assemble immutable checked module contexts from the provided artifacts
        let revision = context.revision()?;
        let artifacts = ArtifactReader::new(context.repository.as_ref(), revision);
        let modules = modules
            .into_iter()
            .map(|module| {
                let parsed = artifacts
                    .dir_parsed(module)
                    .map_err(|error| error.to_string())?;
                let bound = artifacts
                    .dir_bound(module, profile)
                    .map_err(|error| error.to_string())?;
                let expanded = artifacts
                    .dir_expanded(module, profile)
                    .map_err(|error| error.to_string())?;
                let exported = artifacts
                    .dir_exported(module, profile)
                    .map_err(|error| error.to_string())?;
                let resolved = artifacts
                    .dir_resolved(module, profile)
                    .map_err(|error| error.to_string())?;
                let checked = artifacts
                    .dir_checked(module, profile)
                    .map_err(|error| error.to_string())?;

                ModuleContext::new(parsed, bound, expanded, exported, resolved, checked)
                    .map_err(|error| error.to_string().into())
            })
            .collect::<CommandResult<Vec<_>>>()?;

        ProgramContext::new(modules).map_err(|error| CommandError::internal(error.to_string()))
    }
}
