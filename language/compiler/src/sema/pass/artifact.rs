use std::sync::Arc;

use destack_artifact::{ArtifactProjectionFingerprint, DirChecked, DirDeclared, DirElaborated};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::sema::{CheckModuleState, CheckState};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Convert solved state into one declared DIR module.
    pub(in crate::sema) fn into_declared(
        mut self,
        module: ModuleId,
    ) -> CompilerResult<DirDeclared> {
        // persist walked decorator uses for elaborate to select and evaluate
        for application in std::mem::take(&mut self.decorators) {
            self.module.decorators_tail.insert_use(dir::DecoratorUse {
                source: application.expression.decorator.into_global(module),
                owner: application.owner,
                target: application.expression.target.into_global(module),
                symbol: application.symbol,
                generic_arguments: application.expression.generic_arguments,
                arguments: application.expression.arguments,
            });
        }

        let definitions = self.module.merged_definitions();
        let members = self.module.merged_members();
        let types = self.module.types_tail.finish();
        let CheckModuleState {
            bindings_tail: bindings,
            decorators_tail: decorators,
            statics_tail: statics,
            generics_tail: generics,
            resolutions,
            decisions,
            ..
        } = self.module;

        // collect the foreign modules the stored entries recorded as they interned
        let references = self.module.references.iter().copied().collect::<Vec<_>>();

        let fingerprint = ArtifactProjectionFingerprint::from_serialized_payload(&(
            module,
            &references,
            &bindings,
            &decorators,
            &types,
            &statics,
            &generics,
            &definitions,
            &decisions,
            &resolutions,
            &members,
        ))
        .map_err(|error| CompilerError::Internal {
            message: format!(
                "failed to fingerprint declared DIR payload for module {module:?}: {error}"
            ),
        })?;

        Ok(DirDeclared {
            fingerprint,
            references,
            bindings: Arc::new(bindings),
            decorators: Arc::new(decorators),
            types: Arc::new(types),
            statics: Arc::new(statics),
            generics: Arc::new(generics),
            definitions: Arc::new(definitions),
            resolutions: Arc::new(resolutions),
            decisions: Arc::new(decisions),
            members: Arc::new(members),
        })
    }

    /// Convert flattened state into one elaborated DIR module.
    pub(in crate::sema) fn into_elaborated(
        mut self,
        module: ModuleId,
    ) -> CompilerResult<DirElaborated> {
        let definitions = self.module.merged_definitions();
        let members = self.module.merged_members();
        let bindings = self.module.bindings_tail;
        let types = self.module.types_tail.finish();
        let auto = self.module.auto;
        let generics = self.module.generics_tail;
        let decorators = self.module.decorators_tail;
        let statics = self.module.statics_tail;
        let controls = self.module.controls;
        let resolutions = self.module.resolutions;
        let decisions = self.module.decisions;

        // collect the foreign modules the stored entries recorded as they interned
        let references = self.module.references.iter().copied().collect::<Vec<_>>();

        let fingerprint = ArtifactProjectionFingerprint::from_serialized_payload(&(
            module,
            &references,
            &bindings,
            &types,
            &members,
            &auto,
            &generics,
            &definitions,
            &decorators,
            &statics,
            &controls,
            &resolutions,
            &decisions,
        ))
        .map_err(|error| CompilerError::Internal {
            message: format!(
                "failed to fingerprint elaborated DIR payload for module {module:?}: {error}"
            ),
        })?;

        Ok(DirElaborated {
            fingerprint,
            references,
            bindings: Arc::new(bindings),
            types: Arc::new(types),
            members: Arc::new(members),
            auto: Arc::new(auto),
            generics: Arc::new(generics),
            definitions: Arc::new(definitions),
            decorators: Arc::new(decorators),
            statics: Arc::new(statics),
            controls: Arc::new(controls),
            resolutions: Arc::new(resolutions),
            decisions: Arc::new(decisions),
        })
    }

    /// Convert solved state into one checked DIR module.
    pub(in crate::sema) fn into_checked(mut self, module: ModuleId) -> CompilerResult<DirChecked> {
        let definitions = self.module.merged_definitions();
        let members = self.module.merged_members();
        let types = self.module.types_tail.finish();
        let CheckModuleState {
            bindings_tail: bindings,
            decorators_tail: decorators,
            controls,
            statics_tail: statics,
            resolutions,
            decisions,
            generics_tail: generics,
            coercions,
            captures,
            flows,
            ..
        } = self.module;

        let fingerprint = ArtifactProjectionFingerprint::from_serialized_payload(&(
            module,
            &bindings,
            &decorators,
            &controls,
            &types,
            &statics,
            &resolutions,
            &decisions,
            &generics,
            &definitions,
            &members,
            &coercions,
            &captures,
            &flows,
        ))
        .map_err(|error| CompilerError::Internal {
            message: format!("failed to fingerprint DIR payload for module {module:?}: {error}"),
        })?;

        Ok(DirChecked {
            fingerprint,
            bindings: Arc::new(bindings),
            decorators: Arc::new(decorators),
            controls: Arc::new(controls),
            types: Arc::new(types),
            statics: Arc::new(statics),
            resolutions: Arc::new(resolutions),
            decisions: Arc::new(decisions),
            generics: Arc::new(generics),
            definitions: Arc::new(definitions),
            members: Arc::new(members),
            coercions: Arc::new(coercions),
            captures: Arc::new(captures),
            flows: Arc::new(flows),
        })
    }
}
