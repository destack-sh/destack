use std::sync::Arc;

use destack_artifact::DirCheckedComponentEntry;
use destack_source::DiagnosticCollection;

use crate::CompilerResult;
use crate::check::CheckState;

impl CheckState<'_> {
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
        let modules = self.component_modules.clone();
        let mut entries = Vec::with_capacity(modules.len());

        // commit modules in stable load order
        for module in modules {
            self.commit_module(module);
            let output = self
                .outputs
                .shift_remove(&module)
                .expect("check output was not loaded");
            let checked = destack_artifact::DirCheckedModule {
                types: Arc::new(output.types),
                statics: Arc::new(output.statics),
                resolutions: Arc::new(output.resolutions),
                generics: Arc::new(output.generics),
                relations: Arc::new(output.relations),
                coercions: Arc::new(output.coercions),
                extensions: Arc::new(output.extensions),
                layouts: Arc::new(output.layouts),
                captures: Arc::new(output.captures),
            };

            entries.push(DirCheckedComponentEntry { module, checked });
        }

        Ok(entries)
    }
}
