use destack_artifact::DirCheckedComponentEntry;
use destack_source::DiagnosticCollection;

use crate::CompilerResult;
use crate::check::CheckComponentState;

impl CheckComponentState<'_> {
    /// Commit solved check state into checked DIR tables and diagnostics.
    pub(in crate::check) fn commit(
        mut self,
    ) -> CompilerResult<(Vec<DirCheckedComponentEntry>, DiagnosticCollection)> {
        let diagnostics = self.collect_diagnostics()?;

        self.commit_coercion_table()?;
        self.commit_generic_instance_table()?;
        self.commit_call_resolution_table()?;
        self.commit_construct_resolution_table()?;
        self.commit_operator_resolution_table()?;

        let modules = self.commit_checked_modules()?;

        Ok((modules, diagnostics))
    }

    /// Commit checked DIR tables for every loaded module.
    fn commit_checked_modules(&mut self) -> CompilerResult<Vec<DirCheckedComponentEntry>> {
        let modules = std::mem::take(&mut self.modules);
        let mut entries = Vec::with_capacity(modules.len());

        // commit modules in stable load order
        for (module, check_module) in modules {
            let checked = check_module.commit(self.environment.as_ref());

            entries.push(DirCheckedComponentEntry { module, checked });
        }

        Ok(entries)
    }
}
