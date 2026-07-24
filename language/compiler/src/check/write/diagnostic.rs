use destack_artifact::{
    DiagnosticAnchor, DiagnosticBuilder, DiagnosticControlIndex, DiagnosticControlLevel,
    DiagnosticControlTable, ToDiagnostic,
};
use destack_core::{FxIndexSet, StringId, StringPool};
use destack_source::DiagnosticCollection;

use crate::check::{CheckError, CheckState, CheckWarning};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Collect final diagnostics for one checked component.
    pub(in crate::check) fn collect_diagnostics(&mut self) -> CompilerResult<DiagnosticCollection> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();
        let mut errors = Vec::<DiagnosticBuilder<CheckError>>::new();
        let mut warnings = Vec::<DiagnosticBuilder<CheckWarning>>::new();

        // drain walked and solved diagnostics in stable module order
        for module in modules.iter().copied() {
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
            if self.is_component_module(module) && reported.insert((module, anchor.clone())) {
                errors.push(CheckError::CannotInferType { anchor, module }.into());
            }
        }

        // index each module and physical file into the control tables
        let control_tables = modules
            .iter()
            .map(|module| &self.module(*module).controls)
            .collect::<Vec<_>>();
        let controls = DiagnosticControlIndex::new(control_tables).map_err(|error| {
            CompilerError::Internal {
                message: error.to_string(),
            }
        })?;
        let mut matched_controls = controls
            .iter()
            .map(|(_, table)| vec![false; table.len()])
            .collect::<Vec<_>>();

        // resolve warning controls before checking expectations
        let mut controlled_warnings = Vec::with_capacity(warnings.len());
        for warning in warnings {
            let mut diagnostic = warning.to_diagnostic(self.context)?;
            let anchor = warning.diagnostic().anchor();
            let diagnostic_id = StringId::for_text(warning.diagnostic().id());
            let mut is_enabled = true;

            // apply the effective control from that module
            if let Some((table_index, control_index, control)) =
                controls.effective(diagnostic_id, &anchor)
            {
                if matches!(control.level, DiagnosticControlLevel::Expect) {
                    matched_controls[table_index][control_index] = true;
                }
                let severity = control.level.severity();
                if let Some(severity) = severity {
                    diagnostic.severity = severity;
                }
                is_enabled = severity.is_some();
            }
            if is_enabled {
                controlled_warnings.push(diagnostic);
            }
        }

        // append unmet expectations owned by compiler warnings
        for ((_, table), matched) in controls.iter().zip(&matched_controls) {
            append_unmet_expectations(table, matched, &mut errors, self.strings());
        }

        // render diagnostics in production order
        let mut collection = DiagnosticCollection::new();
        for diagnostic in errors {
            collection.insert(diagnostic.to_diagnostic(self.context)?);
        }
        for warning in controlled_warnings {
            collection.insert(warning);
        }

        Ok(collection)
    }
}

/// Append unmet expectations that select check warnings.
fn append_unmet_expectations(
    table: &DiagnosticControlTable,
    matched: &[bool],
    diagnostics: &mut Vec<DiagnosticBuilder<CheckError>>,
    strings: &StringPool,
) {
    for definition in CheckWarning::ALL {
        let diagnostic = StringId::for_text(definition.id);
        for (index, control) in table.expectations(diagnostic) {
            if matched[index] {
                continue;
            }

            let diagnostic = CheckError::UnmetDiagnosticExpectation {
                anchor: DiagnosticAnchor::Span(control.source),
                module: table.module,
                diagnostic: definition.id.to_string(),
            };
            let diagnostic = match control.reason {
                Some(reason) => DiagnosticBuilder::new(diagnostic).note(strings.get(reason)),
                None => DiagnosticBuilder::new(diagnostic),
            };
            diagnostics.push(diagnostic);
        }
    }
}
