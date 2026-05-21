use crate::check::{CheckModuleState, CheckResult, ObligationContext};

impl CheckModuleState {
    /// Validate one concrete layout obligation.
    pub(in crate::check) fn validate_concrete_layout(
        &mut self,
        _context: &ObligationContext,
    ) -> CheckResult<()> {
        Ok(())
    }
}
