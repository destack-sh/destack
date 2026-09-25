use std::sync::Arc;

use dir::NodeVisitor as _;
use tspp_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, DirParsed, EnvironmentBound,
};
use tspp_dir as dir;
use tspp_repository::{ConditionSet, Module, ProviderContext};
use tspp_source::{ModuleId, ProfileId};

use super::state::BindState;

use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect inputs for bound DIR of one module profile.
    pub(crate) fn collect_dir_bound(
        &self,
        module: ModuleId,
        _profile: ProfileId,
        _context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::dir_parsed(module));

        Ok(dependencies)
    }

    /// Build bound DIR for one parsed module profile.
    pub(crate) fn provide_dir_bound(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // load provider inputs
        let artifacts = self.artifact_reader(context);
        let parsed = artifacts
            .read::<DirParsed>(module)
            .map_err(CompilerError::from)?;
        let module = self.module(context.revision(), module)?;
        let profile = self.profile(context.revision(), profile)?;

        // bind parsed dir
        let mut state = BindState::new(self, module.id, parsed.as_ref());
        self.bind_roots(
            &mut state,
            module.as_ref(),
            parsed.as_ref(),
            profile.conditions(),
        )?;
        let stats = state.stats;
        let dir_bound = state.finish();
        stats.record(context);

        Ok(ArtifactPayload::DirBound(Arc::new(dir_bound)))
    }

    /// Bind active parsed roots into a state.
    fn bind_roots(
        &self,
        state: &mut BindState<'_>,
        module: &Module,
        parsed: &DirParsed,
        conditions: &ConditionSet,
    ) -> CompilerResult<()> {
        // visit code roots
        if module.is_code() {
            let tree = &parsed.tree;
            for module_file in module.files_for_conditions(conditions) {
                state.stats.files += 1;

                let parsed_file =
                    parsed
                        .file(module_file.file_id)
                        .ok_or_else(|| CompilerError::Internal {
                            message: format!(
                                "active module file {:?} was not parsed",
                                module_file.file_id
                            ),
                        })?;

                for root in &parsed_file.roots {
                    state.push_root(*root);
                    let expression = tree.get(*root);
                    state.visit_expression(tree, *root, expression);
                }
            }
        }

        Ok(())
    }

    /// Collect inputs for the bound environment of one profile.
    pub(crate) fn collect_environment_bound(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();

        // language item modules back the language environment
        for module in self.repository.builtin_module_ids(context.revision())? {
            dependencies.require(ArtifactKey::dir_parsed(module));
            dependencies.require(ArtifactKey::dir_bound(module, profile));
        }

        // global module exports back the visible global resolutions
        let mut globals = self.load_global_module_ids(profile, context)?;
        if let Some((module, _)) = self.tree_builder_reference(profile, context)? {
            globals.push(module);
        }
        let artifacts = self.artifact_reader(context);
        self.collect_exported_modules(globals, profile, &artifacts, &mut dependencies)?;

        Ok(dependencies)
    }

    /// Build the bound environment for one profile.
    pub(crate) fn provide_environment_bound(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // discover environment inputs
        let globals = self.load_global_module_ids(profile, context)?;
        let language_modules = self.repository.builtin_module_ids(context.revision())?;

        // build language environment for profile
        let artifacts = self.artifact_reader(context);
        let language = self.build_language_environment(profile, &artifacts, &language_modules)?;
        let global_resolutions = self.build_global_resolutions(profile, &artifacts, &globals)?;
        let tree = self.resolve_tree_builder(profile, context, &artifacts)?;

        let environment = EnvironmentBound {
            language,
            globals,
            global_resolutions_by_key: global_resolutions,
            tree,
        };

        Ok(ArtifactPayload::EnvironmentBound(Arc::new(environment)))
    }
}
