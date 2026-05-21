use crate::check::{CheckModuleState, CheckResult, Obligation};

impl CheckModuleState {
    /// Validate solved check obligations and emit diagnostics.
    pub(in crate::check) fn validate(&mut self) -> CheckResult<()> {
        for obligation in self.obligations().to_vec() {
            self.validate_obligation(&obligation)?;
        }

        Ok(())
    }

    /// Validate one solved obligation.
    fn validate_obligation(&mut self, obligation: &Obligation) -> CheckResult<()> {
        match obligation {
            Obligation::Assignable {
                source,
                target,
                context,
            } => self.validate_assignable(*source, *target, context),
            Obligation::Satisfies { context, .. } => self.validate_satisfies(context),
            Obligation::Extends { context, .. } => self.validate_extends(context),
            Obligation::Implements { context, .. } => self.validate_implements(context),
            Obligation::ConcreteLayout { context, .. } => self.validate_concrete_layout(context),
            Obligation::Callable {
                callee,
                arguments,
                context,
                ..
            } => self.validate_callable(*callee, arguments, context),
        }
    }
}
