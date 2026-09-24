use std::sync::Arc;

use destack_artifact::{DiagnosticRecord, DirAnalyzed};

use crate::CompilerResult;
use crate::sema::CheckState;

impl CheckState<'_> {
    /// Run the analyze pass: project the memberships tooling reads over the settled module.
    pub(in crate::sema) fn run_analyze(&mut self) -> CompilerResult<()> {
        self.import_external_modules()?;

        self.settle_member_bindings(self.module_id)
    }

    /// Convert analyzed state into one analyzed DIR module.
    pub(in crate::sema) fn into_analyzed(
        mut self,
    ) -> CompilerResult<(DirAnalyzed, Vec<DiagnosticRecord>)> {
        let diagnostics = self.collect_diagnostics()?;
        let types = self.module.types_tail.finish();

        Ok((
            DirAnalyzed {
                types: Arc::new(types),
                members: Arc::new(self.module.members_tail),
            },
            diagnostics,
        ))
    }
}
