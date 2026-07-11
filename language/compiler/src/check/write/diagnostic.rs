use destack_artifact::{DiagnosticBuilder, ToDiagnostic};
use destack_core::FxIndexSet;
use destack_source::DiagnosticCollection;

use crate::CompilerResult;
use crate::check::{CheckError, CheckState, CheckWarning};

impl CheckState<'_> {
    /// Collect final diagnostics for one checked component.
    pub(in crate::check) fn collect_diagnostics(&mut self) -> CompilerResult<DiagnosticCollection> {
        let mut errors = Vec::<DiagnosticBuilder<CheckError>>::new();
        let mut warnings = Vec::<DiagnosticBuilder<CheckWarning>>::new();

        // drain walked and solved diagnostics in stable module order
        let modules = self.modules.keys().copied().collect::<Vec<_>>();
        for module in modules {
            errors.append(&mut self.module_mut(module).diagnostics);
            warnings.append(&mut self.module_mut(module).warnings);
        }

        // unfinished constraints stayed parked on dependencies forever
        let mut unresolved_origins = Vec::new();
        for (id, constraint) in self.solver.constraints.iter() {
            if self.solver.constraints.is_complete(id) {
                continue;
            }

            unresolved_origins.push(constraint.cause());
        }

        // report each unsolved anchor once
        let mut reported = FxIndexSet::default();
        for cause in unresolved_origins {
            let origin = self.solver.cause(cause).origin;
            let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
            if reported.insert((module, anchor.clone())) {
                errors.push(CheckError::CannotInferType { anchor, module }.into());
            }
        }

        // render diagnostics in production order
        let mut collection = DiagnosticCollection::new();
        for diagnostic in errors {
            collection.insert(diagnostic.to_diagnostic(self.context)?);
        }
        for warning in warnings {
            collection.insert(warning.to_diagnostic(self.context)?);
        }

        Ok(collection)
    }
}
