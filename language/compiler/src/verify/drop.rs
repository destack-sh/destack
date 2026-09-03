use destack_core::FxIndexSet;
use destack_mir::{Function, FunctionBehavior, LocalNodeId};

use crate::verify::{VerifyError, VerifyState};

/// Drop effect checker for one MIR module.
pub(in crate::verify) struct DropChecker<'a, 'b> {
    /// Module verification state.
    verification: &'a mut VerifyState<'b>,
}

impl<'a, 'b> DropChecker<'a, 'b> {
    /// Create one module drop checker.
    pub(in crate::verify) fn new(verification: &'a mut VerifyState<'b>) -> Self {
        Self { verification }
    }

    /// Check every authored drop hook for forbidden effects.
    pub(in crate::verify) fn check(mut self) {
        // collect each hook once across its registered storages
        let hooks: FxIndexSet<_> = self
            .verification
            .drops
            .hooks()
            .map(|(_, _, function)| function)
            .collect();
        let effects = self.verification.effects.clone();

        for function in hooks {
            // check each hook where its body was analyzed, an imported hook checking in its module
            let Some(effect) = effects.function(function) else {
                continue;
            };
            let behavior = effect.behavior.clone();

            self.check_hook(function, &behavior);
        }
    }

    /// Check one drop hook's closed behavior.
    fn check_hook(&mut self, function: LocalNodeId<Function>, behavior: &FunctionBehavior) {
        if !behavior.park.may_park() {
            return;
        }

        let anchor = self.verification.anchor(function.into_any());

        self.verification
            .emit_error(VerifyError::DropEffect { anchor });
    }
}
