use crate::check::{CheckError, CheckModuleState, CheckResult, ObligationContext, TypeInferId};

impl CheckModuleState {
    /// Validate one assignability obligation.
    pub(in crate::check) fn validate_assignable(
        &mut self,
        source: TypeInferId,
        target: TypeInferId,
        context: &ObligationContext,
    ) -> CheckResult<()> {
        let Some(source) = self.infer_type_id(source) else {
            return Ok(());
        };
        let Some(target) = self.infer_type_id(target) else {
            return Ok(());
        };
        if !self.is_type_assignable(source, target) {
            self.push_diagnostic(CheckError::NotAssignable {
                anchor: context.anchor.clone(),
                module: self.module(),
            });
        }

        Ok(())
    }

    /// Validate one generic satisfaction obligation.
    pub(in crate::check) fn validate_satisfies(
        &mut self,
        _context: &ObligationContext,
    ) -> CheckResult<()> {
        Ok(())
    }
}
