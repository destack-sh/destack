use tspp_artifact::{
    DiagnosticAnchor, DiagnosticBuilder, DiagnosticControlIndex, DiagnosticControlLevel,
};
use tspp_core::{StringId, StringPool};
use tspp_source::DiagnosticSeverity;

use crate::{Lint, LinterDiagnostic, LinterError};

impl Lint {
    /// Apply source controls to this lint's diagnostics.
    pub(crate) fn apply_controls(
        &self,
        controls: &DiagnosticControlIndex<'_>,
        severity: Option<DiagnosticSeverity>,
        strings: &StringPool,
        mut diagnostics: Vec<DiagnosticBuilder<LinterDiagnostic>>,
    ) -> (
        Vec<DiagnosticBuilder<LinterDiagnostic>>,
        Vec<DiagnosticBuilder<LinterError>>,
    ) {
        let diagnostic = StringId::for_text(self.id.as_ref());
        let mut matched = controls
            .iter()
            .map(|(_, table)| vec![false; table.len()])
            .collect::<Vec<_>>();

        // apply the effective control to every emitted diagnostic
        for pending in &mut diagnostics {
            let pending = pending.diagnostic_mut();
            pending.set_severity(severity);

            if let Some((table_index, control_index, control)) =
                controls.effective(diagnostic, &pending.primary)
            {
                if matches!(control.level, DiagnosticControlLevel::Expect) {
                    matched[table_index][control_index] = true;
                }
                pending.set_severity(control.level.severity());
            }
        }

        // remove diagnostics suppressed by the effective control
        diagnostics.retain(|diagnostic| diagnostic.diagnostic().severity.is_some());
        let mut errors = Vec::new();

        // report expectations not matched by this lint
        for (table_index, table) in controls.iter() {
            for (control_index, control) in table.expectations(diagnostic) {
                if matched[table_index][control_index] {
                    continue;
                }

                let error = LinterError::UnmetExpectation {
                    anchor: DiagnosticAnchor::Span(control.source),
                    lint: self.id.to_string(),
                };
                let error = error.builder();
                let error = match control.reason {
                    Some(reason) => error.note(strings.get(reason)),
                    None => error,
                };
                errors.push(error);
            }
        }

        (diagnostics, errors)
    }
}
