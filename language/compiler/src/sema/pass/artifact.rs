use std::sync::Arc;

use tspp_artifact::{ArtifactProjectionFingerprint, DirChecked, DirDeclared, DirElaborated};
use tspp_dir as dir;
use tspp_repository::ArtifactAttemptRecorder;
use tspp_source::ModuleId;

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

        let types = self.module.types_tail.finish();
        let CheckModuleState {
            bindings_tail: bindings,
            decorators_tail: decorators,
            statics_tail: statics,
            generics_tail: generics,
            definitions_tail: definitions,
            members_tail: members,
            resolutions,
            decisions_tail: decisions,
            flows,
            references,
            ..
        } = self.module;

        // collect the foreign modules the stored entries recorded as they interned
        let references = references.iter().copied().collect::<Vec<_>>();

        let fingerprint = ArtifactProjectionFingerprint::from_serialized_payload(&(
            module,
            &references,
            &bindings,
            &decorators,
            types.content_digest(),
            &statics,
            &generics,
            &definitions,
            &decisions,
            &resolutions,
            &members,
            &flows,
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
            flows: Arc::new(flows),
        })
    }

    /// Convert flattened state into one elaborated DIR module.
    pub(in crate::sema) fn into_elaborated(
        self,
        module: ModuleId,
    ) -> CompilerResult<DirElaborated> {
        // mix the declared fingerprint so base changes reach this identity
        let inherited = self
            .module
            .declared
            .as_ref()
            .map(|declared| declared.fingerprint);
        let definitions = self.module.definitions_tail;
        let representations = self.module.representations_tail;
        let members = self.module.members_tail;
        let bindings = self.module.bindings_tail;
        let types = self.module.types_tail.finish();
        let generics = self.module.generics_tail;
        let decorators = self.module.decorators_tail;
        let statics = self.module.statics_tail;
        let controls = self.module.controls;
        let resolutions = self.module.resolutions;
        let decisions = self.module.decisions_tail;
        let flows = self.module.flows;

        // collect the foreign modules the stored entries recorded as they interned
        let references = self.module.references.iter().copied().collect::<Vec<_>>();

        let fingerprint = ArtifactProjectionFingerprint::from_serialized_payload(&(
            (module, &inherited),
            &references,
            &bindings,
            types.content_digest(),
            &members,
            &generics,
            &definitions,
            &representations,
            &decorators,
            &statics,
            &controls,
            &resolutions,
            &decisions,
            &flows,
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
            generics: Arc::new(generics),
            definitions: Arc::new(definitions),
            representations: Arc::new(representations),
            decorators: Arc::new(decorators),
            statics: Arc::new(statics),
            controls: Arc::new(controls),
            resolutions: Arc::new(resolutions),
            decisions: Arc::new(decisions),
            flows: Arc::new(flows),
        })
    }

    /// Convert solved state into one checked DIR module.
    pub(in crate::sema) fn into_checked(self, module: ModuleId) -> CompilerResult<DirChecked> {
        // mix the elaborated fingerprint so base changes reach this identity
        let inherited = self
            .module
            .elaborated
            .as_ref()
            .map(|elaborated| elaborated.fingerprint);
        let types = self.module.types_tail.finish();
        let CheckModuleState {
            bindings_tail: bindings,
            decorators_tail: decorators,
            controls,
            statics_tail: statics,
            resolutions,
            decisions_tail: decisions,
            generics_tail: generics,
            members_tail: members,
            coercions_tail: coercions,
            captures,
            flows,
            references,
            representations_tail: representations,
            ..
        } = self.module;

        // store the foreign modules the pass observed
        let references = references.iter().copied().collect::<Vec<_>>();

        let recorder = self.recorder;
        let fingerprint =
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "fingerprint", || {
                ArtifactProjectionFingerprint::from_serialized_payload(&(
                    (module, &inherited),
                    &references,
                    &bindings,
                    &decorators,
                    &controls,
                    types.content_digest(),
                    &statics,
                    &resolutions,
                    &decisions,
                    &generics,
                    &members,
                    &coercions,
                    &captures,
                    &flows,
                    &representations,
                ))
                .map_err(|error| CompilerError::Internal {
                    message: format!(
                        "failed to fingerprint DIR payload for module {module:?}: {error}"
                    ),
                })
            })?;

        Ok(DirChecked {
            fingerprint,
            references,
            bindings: Arc::new(bindings),
            decorators: Arc::new(decorators),
            controls: Arc::new(controls),
            types: Arc::new(types),
            statics: Arc::new(statics),
            resolutions: Arc::new(resolutions),
            decisions: Arc::new(decisions),
            generics: Arc::new(generics),
            members: Arc::new(members),
            coercions: Arc::new(coercions),
            captures: Arc::new(captures),
            flows: Arc::new(flows),
            representations: Arc::new(representations),
        })
    }
}
