use std::sync::Arc;

use destack_artifact::{
    DiagnosticAnchor, DiagnosticBuilder, DiagnosticControlLevel, DiagnosticControlTable,
    DiagnosticDefinition,
};
use destack_core::{FxIndexMap, StringId, StringPool};
use destack_repository::ProviderError;
use destack_source::{DiagnosticSeverity, FileId, ModuleId};

use crate::{Lint, LinterDiagnostic, LinterError};

/// Diagnostic selector index.
#[derive(Debug)]
pub(crate) struct DiagnosticIndex {
    /// The sorted selector and diagnostic pairs.
    selectors: Box<[(StringId, StringId)]>,
}

/// Checked diagnostic controls.
pub(crate) struct DiagnosticControls {
    /// The checked control tables.
    tables: Box<[Arc<DiagnosticControlTable>]>,
    /// Control table indices by module.
    modules: FxIndexMap<ModuleId, usize>,
    /// Control table indices by physical file.
    files: FxIndexMap<FileId, usize>,
}

impl DiagnosticControls {
    /// Resolve checked control tables against the available diagnostics.
    pub(crate) fn resolve(
        control_tables: impl IntoIterator<Item = Arc<DiagnosticControlTable>>,
        diagnostics: &DiagnosticIndex,
        strings: &StringPool,
    ) -> Result<Self, Box<DiagnosticBuilder<LinterError>>> {
        let mut tables = Vec::new();

        // validate each table independently
        for table in control_tables {
            // resolve selectors in source order
            for (index, control) in table.controls.iter().enumerate() {
                let selector = strings.get(control.selector);
                let Some(diagnostic) = diagnostics.diagnostic(control.selector) else {
                    let error = LinterError::UnknownControlledDiagnostic {
                        anchor: DiagnosticAnchor::Span(control.source),
                        selector: selector.to_string(),
                    };

                    return Err(Box::new(error.builder()));
                };

                // reject aliases that override an enclosing forbid
                if !matches!(control.level, DiagnosticControlLevel::Forbid)
                    && let Some(forbidden) = table.controls[..index].iter().rev().find(|previous| {
                        matches!(previous.level, DiagnosticControlLevel::Forbid)
                            && diagnostics.diagnostic(previous.selector) == Some(diagnostic)
                            && previous.scope.contains(control.scope)
                    })
                {
                    let error = LinterError::ForbiddenDiagnosticOverride {
                        anchor: DiagnosticAnchor::Span(control.source),
                        selector: selector.to_string(),
                    };

                    return Err(Box::new(error.label(forbidden.source, "forbidden here")));
                }
            }

            tables.push(table);
        }

        // index exact module and file ownership
        let mut modules = FxIndexMap::default();
        let mut files = FxIndexMap::default();
        for (index, table) in tables.iter().enumerate() {
            modules.insert(table.module, index);
            for file in &table.files {
                files.insert(*file, index);
            }
        }

        Ok(Self {
            tables: tables.into_boxed_slice(),
            modules,
            files,
        })
    }

    /// Return whether controls activate a lint without a configured severity.
    pub(crate) fn enables(&self, lint: &Lint, severity: Option<DiagnosticSeverity>) -> bool {
        if severity.is_some() {
            return true;
        }

        let selectors = lint.selectors();

        self.tables.iter().any(|table| table.enables(&selectors))
    }

    /// Apply controls to diagnostics emitted by one lint.
    pub(crate) fn apply(
        &self,
        lint: &Lint,
        severity: Option<DiagnosticSeverity>,
        strings: &StringPool,
        mut diagnostics: Vec<DiagnosticBuilder<LinterDiagnostic>>,
    ) -> (
        Vec<DiagnosticBuilder<LinterDiagnostic>>,
        Vec<DiagnosticBuilder<LinterError>>,
    ) {
        let selectors = lint.selectors();
        let mut matched = self
            .tables
            .iter()
            .map(|table| vec![false; table.controls.len()])
            .collect::<Vec<_>>();

        // apply the effective control to every emitted diagnostic
        for pending in &mut diagnostics {
            let pending = pending.diagnostic_mut();
            pending.set_severity(severity);

            if let Some((table_index, table)) = self.table(&pending.primary)
                && let Some((control_index, control)) =
                    table.effective(&selectors, &pending.primary)
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
        for (table_index, table) in self.tables.iter().enumerate() {
            for (control_index, control) in table.expectations(&selectors) {
                if matched[table_index][control_index] {
                    continue;
                }

                let error = LinterError::UnmetExpectation {
                    anchor: DiagnosticAnchor::Span(control.source),
                    lint: lint.id.to_string(),
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

    /// Return the control table that owns one diagnostic anchor.
    fn table(&self, anchor: &DiagnosticAnchor) -> Option<(usize, &DiagnosticControlTable)> {
        let index = match anchor {
            DiagnosticAnchor::Span(span) => self.files.get(&span.file),
            DiagnosticAnchor::File(file) => self.files.get(file),
            DiagnosticAnchor::Module(module) => self.modules.get(module),
            DiagnosticAnchor::Package(_) => None,
        }?;

        Some((*index, self.tables[*index].as_ref()))
    }
}

impl DiagnosticIndex {
    /// Build the check warning and lint selector index.
    pub(crate) fn new(
        check_warnings: &[&DiagnosticDefinition],
        lints: &[Lint],
    ) -> Result<Self, ProviderError> {
        let mut selectors = check_warnings
            .iter()
            .map(|definition| {
                let selector = StringId::for_text(definition.code);

                (selector, selector, definition.code)
            })
            .collect::<Vec<_>>();

        // index every lint id and code
        for lint in lints {
            let diagnostic = StringId::for_text(lint.id.as_ref());
            for selector in [lint.id.as_ref(), lint.code.as_ref()] {
                let selector_id = StringId::for_text(selector);
                selectors.push((selector_id, diagnostic, selector));
            }
        }

        // reject duplicate identities before building the binary-search index
        selectors.sort_unstable_by_key(|(selector, _, _)| *selector);
        if let Some(duplicate) = selectors.windows(2).find(|pair| pair[0].0 == pair[1].0) {
            let selector = duplicate[0].2;

            return Err(ProviderError::internal(format!(
                "duplicate diagnostic selector '{selector}'"
            )));
        }
        let selectors = selectors
            .into_iter()
            .map(|(selector, diagnostic, _)| (selector, diagnostic))
            .collect::<Vec<_>>();

        Ok(Self {
            selectors: selectors.into_boxed_slice(),
        })
    }

    /// Return the diagnostic selected by one selector.
    fn diagnostic(&self, selector: StringId) -> Option<StringId> {
        let index = self
            .selectors
            .binary_search_by_key(&selector, |(selector, _)| *selector)
            .ok()?;

        Some(self.selectors[index].1)
    }
}
