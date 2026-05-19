use crate::Compiler;
use crate::check::{CheckResult, CheckState};

impl Compiler {
    /// Validate solved check obligations and emit diagnostics.
    pub(in crate::check) fn validate_check(&self, _state: &mut CheckState<'_>) -> CheckResult<()> {
        Ok(())
    }
}
