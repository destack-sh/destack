use destack_artifact::{DiagnosticBuilder, ToDiagnostic};
use destack_source::DiagnosticCollection;
use indexmap::IndexSet;

use crate::CompilerResult;
use crate::check::{CheckError, CheckState, CheckWarning, Origin};

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
        for (id, constraint) in self.constraints.iter() {
            if self.constraints.is_complete(id) {
                continue;
            }

            unresolved_origins.push(constraint.origin);
        }

        // unfinished obligations never saw their inputs solve
        unresolved_origins.extend(
            self.obligations
                .unfinished()
                .map(|obligation| Origin::Node(obligation.source())),
        );

        // report each unsolved anchor once
        let mut reported = IndexSet::new();
        for origin in unresolved_origins {
            let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
            if reported.insert((module, anchor.clone())) {
                errors.push(CheckError::CannotSolve { anchor, module }.into());
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
