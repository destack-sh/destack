use destack_artifact::{DirCheckedModule, DirDeclaredModule};
use destack_source::{DiagnosticCollection, ModuleId};

use crate::CompilerResult;
use crate::check::CheckState;

use super::CheckModuleSegments;

impl CheckState<'_> {
    /// Write solved declaration state into declared DIR artifacts.
    pub(in crate::check) fn write_declared(mut self) -> CompilerResult<Vec<DirDeclaredModule>> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();
        self.close_modules(&modules)?;
        let modules = self.take_modules(modules);
        let mut declared = Vec::with_capacity(modules.len());

        // fingerprint each declared module projection
        for (module, segments) in modules {
            declared.push(segments.into_declared(module)?);
        }

        Ok(declared)
    }

    /// Write solved inference state into checked DIR artifacts.
    pub(in crate::check) fn write_checked(
        mut self,
    ) -> CompilerResult<(Vec<DirCheckedModule>, DiagnosticCollection)> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();
        self.close_modules(&modules)?;
        let diagnostics = self.collect_diagnostics()?;
        let modules = self.take_modules(modules);

        let mut checked = Vec::with_capacity(modules.len());

        // fingerprint each checked module projection
        for (module, segments) in modules {
            checked.push(segments.into_checked(module)?);
        }

        Ok((checked, diagnostics))
    }

    /// Close solved state for selected modules.
    fn close_modules(&mut self, modules: &[ModuleId]) -> CompilerResult<()> {
        let failed_applications = self.failed_generic_applications()?;
        for module in modules.iter().copied() {
            self.write_module(module, &failed_applications)?;
        }

        Ok(())
    }

    /// Take closed segments for selected modules.
    fn take_modules(&mut self, modules: Vec<ModuleId>) -> Vec<(ModuleId, CheckModuleSegments)> {
        modules
            .into_iter()
            .map(|module| {
                let state = self.take_module(module);

                (module, CheckModuleSegments::from_state(state))
            })
            .collect()
    }
}
