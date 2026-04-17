use destack_artifact::{ArtifactKey, DirElaborated};
use destack_source::ModuleId;
use destack_workspace::ProfileId;

use crate::timing::tags;
use crate::{
    Compiler, CompilerContext, ElaborateError, ElaborateResult, RequirementError, ResolveError,
};
use destack_dir::AnchoredGlobalNodeId;

impl Compiler {
    /// Map one build requirement failure into an elaborate error.
    pub(crate) fn elaborate_error_from_requirement(
        &self,
        error: RequirementError,
    ) -> ElaborateError {
        match error {
            RequirementError::NotReady { requirement } => ElaborateError::Yield { requirement },
            RequirementError::Failed { requirement } => {
                ElaborateError::UnsatisfiedRequirement { requirement }
            }
        }
    }

    /// Map one resolve error into an elaborate error at one origin node.
    pub(crate) fn elaborate_error_from_resolve(
        &self,
        error: ResolveError,
        node: AnchoredGlobalNodeId,
    ) -> ElaborateError {
        match error {
            ResolveError::Yield { requirement } => ElaborateError::Yield { requirement },
            ResolveError::UnsatisfiedRequirement { requirement } => {
                ElaborateError::UnsatisfiedRequirement { requirement }
            }
            ResolveError::Skipped => ElaborateError::Skipped,
            _ => ElaborateError::UnsupportedConstruct { node },
        }
    }

    /// Build elaborated DIR for one module.
    pub fn process_dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> ElaborateResult<()> {
        let revision = context.revision();
        let artifact_key = ArtifactKey::dir_elaborated(module, profile);
        let artifact_stamp = context.artifact_stamp(&artifact_key);

        // reuse a persisted elaborated dir when it is still valid
        if context
            .restore_cached_artifact(
                artifact_key,
                |compiler| {
                    compiler.load_dir_elaborated_image(revision, module, artifact_stamp, profile)
                },
                |store, version, payload| store.publish_dir_elaborated(version, payload),
            )
            .is_some()
        {
            return Ok(());
        }

        let analyzed = context
            .require_artifact_dir_analyzed(module, profile)
            .map_err(|error| self.elaborate_error_from_requirement(error))?;
        let module_ref = context.module(module);
        let module_ref = module_ref.as_ref();
        let mut tree = analyzed.tree.as_ref().clone();
        let mut symbols = analyzed.symbols.as_ref().clone();
        let mut types = analyzed.types.as_ref().clone();

        let _timing = self.timing_scope(tags::ELABORATE_MODULE_TRANSFORM);
        let options = context.analyze_context_options_for_module(module_ref.id);
        self.elaborate_module_transform(
            module_ref,
            profile,
            options,
            context,
            &mut tree,
            &mut symbols,
            &mut types,
        )?;

        #[cfg(debug_assertions)]
        tree.debug_assert_valid_parents();

        let _timing = self.timing_scope(tags::ELABORATE_MODULE_REIFY);
        self.elaborate_module_reify(
            module_ref,
            profile,
            options,
            context,
            &mut tree,
            &mut symbols,
            &mut types,
        )?;

        #[cfg(debug_assertions)]
        tree.debug_assert_valid_parents();

        let is_code_module = context.is_code_module(module);

        if is_code_module {
            self.stats.record_elaborate();
        }

        // publish the elaborated artifact with transformed runtime tables
        let payload = DirElaborated::from_analyzed_with(analyzed.as_ref(), tree, symbols, types);

        context.publish_artifact(artifact_key, payload.clone(), |store, version, payload| {
            store.publish_dir_elaborated(version, payload)
        });
        context.store_artifact(
            &artifact_key,
            &payload,
            |compiler, artifact_stamp, payload| {
                compiler.store_dir_elaborated_image(
                    revision,
                    module,
                    profile,
                    artifact_stamp,
                    payload,
                )
            },
        );

        Ok(())
    }

    /// Ensure elaborated DIR exists for a module.
    pub fn require_dir_elaborated(
        &self,
        revision: destack_workspace::Revision,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), RequirementError> {
        self.require_artifact(revision, ArtifactKey::dir_elaborated(module, profile))
    }
}
