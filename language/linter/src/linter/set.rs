use tspp_artifact::{DiagnosticAnchor, DiagnosticControlIndex, IndexKind};
use tspp_core::{FxIndexSet, StringId};
use tspp_repository::{LintLevel, LinterOptions};
use tspp_source::{DiagnosticSeverity, PackageId};

use super::{
    DirModuleCheck, DirProgramCheck, Lint, LintCheck, LintScope, LintTier, MirModuleCheck,
    MirProgramCheck,
};
use crate::LinterError;

/// Lints selected for one module or program.
#[derive(Debug)]
pub(crate) struct LintSet {
    /// The selected lints and configured severities.
    selected: Vec<(&'static Lint, Option<DiagnosticSeverity>)>,
    /// Configuration errors found while resolving this set.
    errors: Box<[LinterError]>,
}

impl LintSet {
    /// Select one warning-level lint for an isolated test run.
    #[cfg(test)]
    pub(crate) fn single(lint: &'static Lint) -> Self {
        Self {
            selected: vec![(lint, Some(DiagnosticSeverity::Warning))],
            errors: Box::new([]),
        }
    }

    /// Resolve the lints selected by one package configuration.
    pub(crate) fn resolve(package: PackageId, options: &LinterOptions) -> Self {
        let lints = Lint::registry();
        let capacity = if options.only.is_empty() {
            lints.len()
        } else {
            options.only.len()
        };
        let mut selected = Vec::with_capacity(capacity);

        // select implementations independently of source controls
        for lint in lints {
            let is_selected =
                options.only.is_empty() || options.only.iter().any(|id| id == lint.id.as_ref());
            if !options.enabled || !is_selected {
                continue;
            }

            let level = options
                .rules
                .get(lint.id.as_ref())
                .copied()
                .unwrap_or(lint.default_level);
            let severity = match level {
                LintLevel::Off => None,
                LintLevel::Warning => Some(DiagnosticSeverity::Warning),
                LintLevel::Error => Some(DiagnosticSeverity::Error),
            };

            selected.push((*lint, severity));
        }

        // report each unknown id once in configuration order
        let errors = options
            .only
            .iter()
            .chain(options.rules.keys())
            .filter(|id| lints.iter().all(|lint| lint.id.as_ref() != id.as_str()))
            .cloned()
            .collect::<FxIndexSet<_>>()
            .into_iter()
            .map(|lint| LinterError::UnknownConfiguredLint {
                anchor: DiagnosticAnchor::Package(package),
                lint,
            })
            .collect::<Vec<_>>();

        Self {
            selected,
            errors: errors.into_boxed_slice(),
        }
    }

    /// Retain lints enabled by configuration or source controls.
    pub(crate) fn retain_active(&mut self, controls: &DiagnosticControlIndex<'_>) {
        self.selected.retain(|(lint, severity)| {
            let diagnostic = StringId::for_text(lint.id.as_ref());
            let is_activated = controls
                .iter()
                .any(|(_, table)| table.activates(diagnostic));

            severity.is_some() || is_activated
        });
    }

    /// Return configuration errors found while resolving this set.
    pub(crate) fn errors(&self) -> &[LinterError] {
        &self.errors
    }

    /// Iterate DIR module lints.
    pub(crate) fn dir_modules(
        &self,
    ) -> impl Iterator<Item = (&Lint, Option<DiagnosticSeverity>, DirModuleCheck)> {
        self.iter().filter_map(|(lint, severity)| match lint.check {
            LintCheck::DirModule(check) => Some((lint, severity, check)),
            _ => None,
        })
    }

    /// Iterate DIR program lints.
    pub(crate) fn dir_programs(
        &self,
    ) -> impl Iterator<Item = (&Lint, Option<DiagnosticSeverity>, DirProgramCheck)> {
        self.iter().filter_map(|(lint, severity)| match lint.check {
            LintCheck::DirProgram(check) => Some((lint, severity, check)),
            _ => None,
        })
    }

    /// Iterate verified MIR module lints.
    pub(crate) fn mir_modules(
        &self,
    ) -> impl Iterator<Item = (&Lint, Option<DiagnosticSeverity>, MirModuleCheck)> {
        self.iter().filter_map(|(lint, severity)| match lint.check {
            LintCheck::MirModule(check) => Some((lint, severity, check)),
            _ => None,
        })
    }

    /// Iterate verified MIR program lints.
    pub(crate) fn mir_programs(
        &self,
    ) -> impl Iterator<Item = (&Lint, Option<DiagnosticSeverity>, MirProgramCheck)> {
        self.iter().filter_map(|(lint, severity)| match lint.check {
            LintCheck::MirProgram(check) => Some((lint, severity, check)),
            _ => None,
        })
    }

    /// Return whether this set contains DIR module lints.
    pub(crate) fn has_dir_modules(&self) -> bool {
        self.dir_modules().next().is_some()
    }

    /// Return whether this set contains DIR program lints.
    pub(crate) fn has_dir_programs(&self) -> bool {
        self.dir_programs().next().is_some()
    }

    /// Iterate module indexes required by DIR lints at one scope.
    pub(crate) fn dir_indexes(&self, scope: LintScope) -> impl Iterator<Item = IndexKind> + '_ {
        IndexKind::ALL.into_iter().filter(move |kind| {
            self.iter().any(|(lint, _)| {
                lint.tier() == LintTier::Dir
                    && lint.scope() == scope
                    && lint.module_indexes.contains(kind)
            })
        })
    }

    /// Return whether this set contains verified MIR module lints.
    pub(crate) fn has_mir_modules(&self) -> bool {
        self.mir_modules().next().is_some()
    }

    /// Return whether this set contains verified MIR program lints.
    pub(crate) fn has_mir_programs(&self) -> bool {
        self.mir_programs().next().is_some()
    }

    /// Return whether this set contains module lints.
    pub(crate) fn has_modules(&self) -> bool {
        self.has_dir_modules() || self.has_mir_modules()
    }

    /// Return whether this set contains program lints.
    pub(crate) fn has_programs(&self) -> bool {
        self.has_dir_programs() || self.has_mir_programs()
    }

    /// Iterate selected lints and configured severities.
    fn iter(&self) -> impl Iterator<Item = (&Lint, Option<DiagnosticSeverity>)> {
        self.selected.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::NO_DEBUGGER;

    /// Apply exclusive selection and its explicit level.
    #[test]
    fn test_resolves_selection_and_level() {
        let mut options = LinterOptions {
            only: vec![NO_DEBUGGER.id.to_string()],
            ..LinterOptions::default()
        };
        options
            .rules
            .insert(NO_DEBUGGER.id.to_string(), LintLevel::Error);

        let lints = LintSet::resolve(PackageId::new(4), &options);
        let selected = lints
            .iter()
            .map(|(lint, severity)| (lint.id.as_ref(), severity))
            .collect::<Vec<_>>();

        assert_eq!(
            selected,
            vec![("no-debugger", Some(DiagnosticSeverity::Error))]
        );
        assert_eq!(lints.errors(), &[]);
    }

    /// Report unknown ids once in their first configured order.
    #[test]
    fn test_reports_unknown_ids_once() {
        let mut options = LinterOptions {
            only: vec!["missing-first".to_string(), "missing-second".to_string()],
            ..LinterOptions::default()
        };
        options
            .rules
            .insert("missing-first".to_string(), LintLevel::Off);
        let package = PackageId::new(7);

        let lints = LintSet::resolve(package, &options);

        assert_eq!(
            lints.errors(),
            &[
                LinterError::UnknownConfiguredLint {
                    anchor: DiagnosticAnchor::Package(package),
                    lint: "missing-first".to_string(),
                },
                LinterError::UnknownConfiguredLint {
                    anchor: DiagnosticAnchor::Package(package),
                    lint: "missing-second".to_string(),
                },
            ]
        );
    }
}
