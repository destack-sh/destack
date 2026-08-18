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
            // read absent effect rows as unknown, which reports every effect
            let behavior = effects
                .function(function)
                .map(|effect| effect.behavior.clone())
                .unwrap_or_else(FunctionBehavior::unknown);
            self.check_hook(function, &behavior);
        }
    }

    /// Check one drop hook's closed behavior.
    fn check_hook(&mut self, function: LocalNodeId<Function>, behavior: &FunctionBehavior) {
        // collect the forbidden effects the hook may perform
        let mut effects = Vec::new();
        if behavior.allocates {
            effects.push("allocate");
        }
        if behavior.park.may_park() {
            effects.push("park");
        }
        if behavior.panic.may_panic() {
            effects.push("panic");
        }

        // report every forbidden effect in one diagnostic
        let effects = match effects.as_slice() {
            [] => return,
            [effect] => (*effect).to_string(),
            [first, second] => format!("{first} and {second}"),
            [first, second, third] => format!("{first}, {second}, and {third}"),
            [..] => unreachable!("drop hooks carry at most three forbidden effects"),
        };
        let anchor = self.verification.anchor(function.into_any());

        self.verification
            .emit_error(VerifyError::DropEffect { anchor, effects });
    }
}
