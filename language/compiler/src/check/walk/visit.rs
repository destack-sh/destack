use crate::Compiler;
use crate::check::walk::state::WalkState;
use crate::check::{CheckResult, CheckState};

impl Compiler {
    /// Visit DIR and collect check constraints, obligations, and outputs.
    pub(in crate::check) fn visit_check(&self, _state: &mut CheckState<'_>) -> CheckResult<()> {
        let walk = WalkState::new();
        let _flow = &walk.flow;

        Ok(())
    }
}
