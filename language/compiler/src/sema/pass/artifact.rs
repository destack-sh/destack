use std::sync::Arc;

use destack_artifact::{ArtifactProjectionFingerprint, DirChecked, DirDeclared, DirElaborated};
use destack_dir as dir;
use destack_repository::ArtifactAttemptRecorder;
use destack_source::ModuleId;

use crate::sema::{Answer, CheckModuleState, CheckState, Goal, Relation, Verdict};
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
            decisions,
            flows,
            ..
        } = self.module;

        // collect the foreign modules the stored entries recorded as they interned
        let references = self.module.references.iter().copied().collect::<Vec<_>>();

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
        mut self,
        module: ModuleId,
    ) -> CompilerResult<DirElaborated> {
        // persist the implementation winners this pass decided
        self.commit_selected_implementations(module)?;

        // mix the declared fingerprint so base changes reach this identity
        let inherited = self
            .module
            .declared
            .as_ref()
            .map(|declared| declared.fingerprint);
        let definitions = self.module.definitions_tail;
        let members = self.module.members_tail;
        let bindings = self.module.bindings_tail;
        let types = self.module.types_tail.finish();
        let auto = self.module.auto;
        let generics = self.module.generics_tail;
        let decorators = self.module.decorators_tail;
        let statics = self.module.statics_tail;
        let controls = self.module.controls;
        let resolutions = self.module.resolutions;
        let decisions = self.module.decisions;
        let flows = self.module.flows;

        // collect the foreign modules the stored entries recorded as they interned
        let references = self.module.references.iter().copied().collect::<Vec<_>>();

        let fingerprint = ArtifactProjectionFingerprint::from_serialized_payload(&(
            (module, &inherited),
            &references,
            &bindings,
            types.content_digest(),
            &members,
            &auto,
            &generics,
            &definitions,
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
            auto: Arc::new(auto),
            generics: Arc::new(generics),
            definitions: Arc::new(definitions),
            decorators: Arc::new(decorators),
            statics: Arc::new(statics),
            controls: Arc::new(controls),
            resolutions: Arc::new(resolutions),
            decisions: Arc::new(decisions),
            flows: Arc::new(flows),
        })
    }

    /// Commit the most general implementations this pass selected for its own owners.
    fn commit_selected_implementations(&mut self, module: ModuleId) -> CompilerResult<()> {
        // collect every implementation goal this pass answered
        let goals: Vec<_> = self
            .answers
            .iter()
            .filter_map(|(goal, answer)| match (goal.goal, answer) {
                (Goal::Implementation(relation), Answer::Implement(response)) => Some((
                    relation,
                    goal.operands,
                    response.value.verdict,
                    response.value.winner,
                )),
                _ => None,
            })
            .collect();

        for (relation, operands, verdict, winner) in goals {
            // keep the decided goals this module's own concrete owners answered
            if relation != Relation::Satisfies || verdict != Verdict::Holds {
                continue;
            }
            let [source, target] = *self.type_ids(module, operands)? else {
                return Err(CompilerError::Internal {
                    message: "an implementation goal lost its operand pair".to_string(),
                });
            };
            let Some(owner) = self.concrete_owner(module, source)? else {
                continue;
            };
            let dir::Type::Application(instance) = self.ty(target)? else {
                continue;
            };

            // commit the winner of the most general application, whose arguments all stay holes
            let arguments = self.type_ids(target.module_id, instance.arguments)?;
            let mut is_general = true;
            for argument in arguments {
                is_general &= matches!(self.ty(*argument)?, dir::Type::Hole(_));
            }
            if is_general {
                self.module
                    .auto
                    .set_selected(owner, instance.symbol, winner);
            }
        }

        Ok(())
    }

    /// Return the concrete declared owner one canonical type names in one module.
    fn concrete_owner(
        &mut self,
        module: ModuleId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        // owners name themselves through argument-free applications and references
        let symbol = match self.ty(ty)? {
            dir::Type::Application(instance) if instance.arguments.is_empty() => instance.symbol,
            dir::Type::Reference(reference) => reference.symbol,
            _ => return Ok(None),
        };

        // keep the owners this module declares
        if symbol.module_id != module {
            return Ok(None);
        }

        // generic owners decide under their parameters, which stay per-site
        if self.symbol_template(symbol)?.is_some() {
            return Ok(None);
        }

        Ok(Some(symbol))
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
            decisions,
            generics_tail: generics,
            definitions_tail: definitions,
            members_tail: members,
            coercions,
            captures,
            flows,
            auto,
            references,
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
                    &definitions,
                    &members,
                    &coercions,
                    &captures,
                    &flows,
                    &auto,
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
            definitions: Arc::new(definitions),
            members: Arc::new(members),
            coercions: Arc::new(coercions),
            captures: Arc::new(captures),
            flows: Arc::new(flows),
            auto: Arc::new(auto),
        })
    }
}
