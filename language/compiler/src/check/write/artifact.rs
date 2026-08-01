use destack_artifact::{DiagnosticRecord, DirChecked, DirDeclared};
use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::CheckState;

use super::CheckModuleSegments;

/// Proof that one module's solved state is closed for the checked write.
pub(in crate::check) struct ClosedModule(ModuleId);

impl CheckState<'_> {
    /// Write solved declaration state into one declared DIR artifact.
    pub(in crate::check) fn write_declared(
        mut self,
        module: ModuleId,
    ) -> CompilerResult<(DirDeclared, Vec<DiagnosticRecord>)> {
        // drop unresolved hole rows, they are not written forms
        self.prune_open_hole_symbols(module)?;

        // keep written forms in declared rows, defer bound failures to check
        self.close_modules(&[module], &FxIndexSet::default())?;
        let diagnostics = self.collect_diagnostics()?;
        let segments = CheckModuleSegments::from_state(self.module);

        Ok((segments.into_declared(module)?, diagnostics))
    }

    /// Close solved state before the checked write reads it.
    pub(in crate::check) fn close_checked(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<ClosedModule> {
        let failed_applications = self.failed_generic_applications()?;
        self.close_modules(&[module], &failed_applications)?;

        Ok(ClosedModule(module))
    }

    /// Write closed inference state into one checked DIR artifact.
    pub(in crate::check) fn write_checked(
        mut self,
        closed: ClosedModule,
    ) -> CompilerResult<(DirChecked, Vec<DiagnosticRecord>)> {
        let ClosedModule(module) = closed;
        let diagnostics = self.collect_diagnostics()?;

        // keep only resolutions the declared stage already carries
        if let Some(stage) = self.module.declared.clone() {
            self.module.resolutions.drop_carried(&stage.resolutions);
        }

        let segments = CheckModuleSegments::from_state(self.module);

        Ok((segments.into_checked(module)?, diagnostics))
    }

    /// Close solved state for selected modules.
    fn close_modules(
        &mut self,
        modules: &[ModuleId],
        failed_applications: &FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        for module in modules.iter().copied() {
            self.write_module(module, failed_applications)?;
        }

        Ok(())
    }
}
