use tspp_artifact::{
    DiagnosticAnchor, DiagnosticBuilder, DiagnosticControlIndex, DiagnosticControlLevel,
    DiagnosticControlTable, DiagnosticRecord,
};
use tspp_core::{StringId, StringPool};

use crate::sema::{Check, CheckError, CheckState, CheckWarning};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Collect final diagnostics for one checked module.
    pub(in crate::sema) fn collect_diagnostics(&mut self) -> CompilerResult<Vec<DiagnosticRecord>> {
        let modules = [self.module_id];
        let mut errors = Vec::<DiagnosticBuilder<CheckError>>::new();
        let mut warnings = Vec::<DiagnosticBuilder<CheckWarning>>::new();

        // drain walked and solved diagnostics in stable module order
        for module in modules.iter().copied() {
            errors.append(&mut self.module_mut(module).diagnostics);
            warnings.append(&mut self.module_mut(module).warnings);
        }

        // refuse undecided relation checks outside declarations
        if !self.is_declaring() {
            for (id, check) in self.fulfill.checks.iter() {
                let Check::Relation(relation) = check else {
                    continue;
                };
                if !self.fulfill.checks.is_complete(id) {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "checked write found the undecided relation check {:?} at {:?}",
                            id, relation.origin,
                        ),
                    });
                }
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
            let mut diagnostic = warning.to_record(self.context)?;
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
                    diagnostic.diagnostic.severity = severity;
                }
                is_enabled = severity.is_some();
            }
            if is_enabled {
                controlled_warnings.push(diagnostic);
            }
        }

        // append unmet expectations once bodies produced every warning
        if self.is_checking() {
            for ((_, table), matched) in controls.iter().zip(&matched_controls) {
                append_unmet_expectations(table, matched, &mut errors, self.strings());
            }
        }

        // collect diagnostics in source order
        let mut records = Vec::with_capacity(errors.len() + controlled_warnings.len());
        for diagnostic in errors {
            records.push(diagnostic.to_record(self.context)?);
        }
        records.extend(controlled_warnings);
        records.sort_by_key(|record| {
            let span = record.diagnostic.primary.target.span();

            (
                span.map_or(0, |span| span.start),
                span.map_or(0, |span| span.end),
            )
        });

        Ok(records)
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
