use destack_mir as mir;

use crate::verify::{VerifyError, VerifyState};

impl VerifyState<'_> {
    /// Check the execution constraints of user-authored Drop hooks.
    pub(in crate::verify) fn check_drop_hooks(&mut self) {
        let analyses = mir::TreeAnalysisCache::new(self.dispatch, self.memory, self.effects);
        let effects = analyses.get::<mir::FunctionEffectAnalysis>(self.tree);
        let hooks = self
            .drops
            .hooks
            .iter()
            .map(|(ty, hook)| (*ty, *hook))
            .collect::<Vec<_>>();

        for (ty, hook) in hooks {
            let anchor = self.anchor(hook.into());

            // require the one canonical nonconsuming receiver signature
            if !self.is_drop_hook_signature(ty, hook) {
                self.emit_error(VerifyError::InvalidDropSignature { anchor });

                continue;
            }

            let effect = effects
                .function(hook)
                .cloned()
                .unwrap_or_else(mir::FunctionEffect::unknown);
            let anchor = self.anchor(hook.into());

            // reject suspension
            if effect.behavior.suspend.may_suspend() {
                self.emit_error(VerifyError::DropMaySuspend {
                    anchor: anchor.clone(),
                });
            }

            // reject panic unwinding
            if effect.behavior.panic.may_panic() {
                self.emit_error(VerifyError::DropMayPanic {
                    anchor: anchor.clone(),
                });
            }

            // reject allocation reentrancy
            if effect.behavior.allocates {
                self.emit_error(VerifyError::DropMayAllocate {
                    anchor: anchor.clone(),
                });
            }

            // reject entropy and host-dependent behavior
            if !effect.behavior.determinism.is_deterministic() {
                self.emit_error(VerifyError::DropMayObserveEntropy { anchor });
            }
        }
    }

    /// Return whether one function has the canonical Drop hook signature.
    fn is_drop_hook_signature(&self, ty: mir::TypeId, hook: mir::FunctionId) -> bool {
        let function = self.tree.get(hook);
        if function.parameters.len() != 1
            || !matches!(self.tree.get(function.return_type), mir::Type::Void)
        {
            return false;
        }

        let receiver = self.tree.get(function.parameters[0].ty);

        matches!(
            receiver,
            mir::Type::Reference {
                kind: mir::ReferenceKind::Borrowed,
                access: mir::Access::Exclusive,
                pointee,
                nullability: mir::Nullability::None,
                ..
            } if *pointee == ty
        )
    }
}
