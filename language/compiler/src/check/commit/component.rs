use destack_artifact::DirCheckedComponentEntry;
use destack_source::DiagnosticCollection;

use crate::CompilerResult;
use crate::check::CheckState;

impl CheckState<'_> {
    /// Commit solved check state into output DIR tables and diagnostics.
    pub(in crate::check) fn commit(
        mut self,
    ) -> CompilerResult<(Vec<DirCheckedComponentEntry>, DiagnosticCollection)> {
        let diagnostics = self.collect_diagnostics()?;
        let modules = self.commit_module_outputs()?;

        Ok((modules, diagnostics))
    }

    /// Commit output DIR tables for every loaded module.
    fn commit_module_outputs(&mut self) -> CompilerResult<Vec<DirCheckedComponentEntry>> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();
        let mut entries = Vec::with_capacity(modules.len());

        // commit modules in stable load order
        for module in modules {
            let checked = self.commit_module(module)?.finish();
            entries.push(DirCheckedComponentEntry { module, checked });
        }

        Ok(entries)
    }
}
