use std::mem;
use std::sync::Arc;

use destack_artifact::{
    ArtifactKey, DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded, DirExported,
    DirImported, DirParsed, DirResolved, DirView,
};
use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_pattern::{Matcher, ModuleContext, Pattern, PatternMatch, ProgramContext};
use destack_repository::{ArtifactReader, ProviderError, Revision};
use destack_source::{File, ModuleId, ProfileId};

use super::{CommandContext, CommandError, CommandResult};

/// Structural matches grouped by module and source file.
pub(super) struct PatternSelection {
    /// Selected modules in command input order.
    pub(super) modules: Vec<ModuleSelection>,
}

/// Structural Pattern matches in one module.
pub(super) struct ModuleSelection {
    /// The selected module.
    pub(super) module: ModuleId,
    /// The parsed DIR containing every selected node.
    pub(super) parsed: Arc<DirParsed>,
    /// Selected source files.
    pub(super) files: Vec<FileSelection>,
}

/// Structural Pattern matches in one source file.
pub(super) struct FileSelection {
    /// The selected source file.
    pub(super) file: Arc<File>,
    /// The selected roots and structural bindings.
    pub(super) matches: Vec<PatternMatch>,
}

impl PatternSelection {
    /// Find every structural match in the selected modules.
    pub(super) fn find(
        pattern: &Pattern,
        modules: &[ModuleId],
        revision: Revision,
        context: &CommandContext<'_>,
    ) -> CommandResult<Self> {
        let artifacts = ArtifactReader::new(context.repository.as_ref(), revision);
        let mut selected_modules = Vec::new();

        // match every physical source file independently
        for module in modules.iter().copied() {
            let parsed = artifacts
                .read::<DirParsed>(module)
                .map_err(|error| error.to_string())?;
            let view = dir::View::new(&parsed.tree);
            let matcher = Matcher::new(pattern, view);
            let mut selected_files = Vec::new();
            for parsed_file in &parsed.files {
                let file = context
                    .repository
                    .file(revision, parsed_file.file_id)
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| {
                        CommandError::internal(format!(
                            "missing Pattern source file {:?}",
                            parsed_file.file_id
                        ))
                    })?;
                let candidates = view.iter_node_ids_in_file(file.id);
                let matches = matcher
                    .find(candidates)
                    .map_err(|error| CommandError::internal(error.to_string()))?;
                if !matches.is_empty() {
                    selected_files.push(FileSelection { file, matches });
                }
            }
            if !selected_files.is_empty() {
                selected_modules.push(ModuleSelection {
                    module,
                    parsed,
                    files: selected_files,
                });
            }
        }

        Ok(Self {
            modules: selected_modules,
        })
    }

    /// Group selected modules by their active Profile.
    pub(super) fn group_by_profile(
        &self,
        revision: Revision,
        context: &CommandContext<'_>,
    ) -> CommandResult<FxIndexMap<ProfileId, Vec<ModuleId>>> {
        let mut profiles = FxIndexMap::<ProfileId, Vec<ModuleId>>::default();

        // retain source order within every Profile group
        for selection in &self.modules {
            let profile = context.selected_profile_id(revision, selection.module)?;
            profiles.entry(profile).or_default().push(selection.module);
        }

        Ok(profiles)
    }

    /// Retain matches accepted by every Predicate in one checked program.
    pub(super) fn retain(
        &mut self,
        pattern: &Pattern,
        roots: &[ModuleId],
        program: &ProgramContext,
    ) -> CommandResult<()> {
        let roots = roots.iter().copied().collect::<FxIndexSet<_>>();

        // evaluate only modules belonging to this checked program
        for selection in &mut self.modules {
            if !roots.contains(&selection.module) {
                continue;
            }
            let module = program
                .module(selection.module)
                .map_err(|error| CommandError::internal(error.to_string()))?;

            // remove rejected matches without disturbing source order
            for file in &mut selection.files {
                let matches = mem::take(&mut file.matches);
                let mut retained = Vec::with_capacity(matches.len());
                for pattern_match in matches {
                    let is_match = pattern
                        .matches_predicates(&pattern_match, module, program)
                        .map_err(|error| CommandError::internal(error.to_string()))?;
                    if is_match {
                        retained.push(pattern_match);
                    }
                }
                file.matches = retained;
            }
            selection.files.retain(|file| !file.matches.is_empty());
        }
        self.modules.retain(|selection| !selection.files.is_empty());

        Ok(())
    }
}

impl CommandContext<'_> {
    /// Provide checked DIR and build one ProgramContext from selected roots.
    pub(super) async fn provide_program_context(
        &self,
        profile: ProfileId,
        roots: &[ModuleId],
        revision: Revision,
    ) -> CommandResult<ProgramContext> {
        let root_keys = roots
            .iter()
            .map(|module| ArtifactKey::dir_checked(*module, profile))
            .collect::<Vec<_>>();
        self.provide(revision, &root_keys).await?;

        // walk the imports of the roots
        let (modules, _) = self.import_closure(revision, profile, roots).await?;
        let modules = modules.into_iter().collect::<FxIndexSet<_>>();

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
        self.provide(revision, &keys).await?;

        // assemble immutable checked module contexts
        let artifacts = ArtifactReader::new(self.repository.as_ref(), revision);
        let modules = modules
            .into_iter()
            .map(|module| {
                let key = (module, profile);
                let read = |error: ProviderError| error.to_string();
                let view = DirView::checked(
                    artifacts.read::<DirParsed>(module).map_err(read)?,
                    artifacts.read::<DirBound>(key).map_err(read)?,
                    artifacts.read::<DirImported>(key).map_err(read)?,
                    artifacts.read::<DirExpanded>(key).map_err(read)?,
                    artifacts.read::<DirResolved>(key).map_err(read)?,
                    artifacts.read::<DirDeclared>(key).map_err(read)?,
                    artifacts.read::<DirElaborated>(key).map_err(read)?,
                    artifacts.read::<DirChecked>(key).map_err(read)?,
                );
                let exported = artifacts.read::<DirExported>(key).map_err(read)?;

                ModuleContext::new(view, exported).map_err(|error| error.to_string().into())
            })
            .collect::<CommandResult<Vec<_>>>()?;

        ProgramContext::new(modules).map_err(|error| CommandError::internal(error.to_string()))
    }
}
