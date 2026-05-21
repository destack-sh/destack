use crate::check::{CheckModuleState, CheckResult, ObligationContext};

impl CheckModuleState {
    /// Validate one extends obligation.
    pub(in crate::check) fn validate_extends(
        &mut self,
        _context: &ObligationContext,
    ) -> CheckResult<()> {
        Ok(())
    }

    /// Validate one implements obligation.
    pub(in crate::check) fn validate_implements(
        &mut self,
        _context: &ObligationContext,
    ) -> CheckResult<()> {
        Ok(())
    }
}
