use destack_artifact::{DirChecked, DirDeclared};
use destack_source::{DiagnosticCollection, ModuleId};

use crate::CompilerResult;
use crate::check::CheckState;

use super::CheckModuleSegments;

impl CheckState<'_> {
    /// Write solved declaration state into one declared DIR artifact.
    pub(in crate::check) fn write_declared(
        mut self,
        module: ModuleId,
    ) -> CompilerResult<DirDeclared> {
        self.close_modules(&[module])?;
        let segments = CheckModuleSegments::from_state(self.module);

        segments.into_declared(module)
    }

    /// Write solved inference state into one checked DIR artifact.
    pub(in crate::check) fn write_checked(
        mut self,
        module: ModuleId,
    ) -> CompilerResult<(DirChecked, DiagnosticCollection)> {
        self.close_modules(&[module])?;
        let diagnostics = self.collect_diagnostics()?;
        let segments = CheckModuleSegments::from_state(self.module);

        Ok((segments.into_checked(module)?, diagnostics))
    }

    /// Close solved state for selected modules.
    fn close_modules(&mut self, modules: &[ModuleId]) -> CompilerResult<()> {
        let failed_applications = self.failed_generic_applications()?;
        for module in modules.iter().copied() {
            self.write_module(module, &failed_applications)?;
        }

        Ok(())
    }
}
