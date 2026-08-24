use std::sync::Arc;

use destack_artifact::{DiagnosticAnchor, DiagnosticControlIndex, IndexKind};
use destack_core::{FxIndexSet, StringId};
use destack_repository::{LintLevel, LinterOptions};
use destack_source::{DiagnosticSeverity, PackageId};

use super::{
    DirModuleCheck, DirProgramCheck, Lint, LintCheck, LintScope, LintTier, MirModuleCheck,
    MirProgramCheck,
};
use crate::LinterError;

/// Lints selected for one module or program.
#[derive(Debug)]
pub(crate) struct LintSet {
    /// The lint registry.
    registry: Arc<[Lint]>,
    /// The selected lint indices and configured severities.
    selected: Vec<(usize, Option<DiagnosticSeverity>)>,
    /// Configuration errors found while resolving this set.
    errors: Box<[LinterError]>,
}

impl LintSet {
    /// Resolve the lints selected by one package configuration.
    pub(crate) fn resolve(package: PackageId, options: &LinterOptions, lints: Arc<[Lint]>) -> Self {
        let mut levels = lints
            .iter()
            .map(|lint| lint.default_level)
            .collect::<Vec<_>>();
        let mut selected = vec![options.only.is_empty(); lints.len()];
        let mut unknown = FxIndexSet::default();

        // resolve each exclusive selection by exact lint id
        for id in &options.only {
            let Some(index) = lints.iter().position(|lint| lint.id.as_ref() == id) else {
                unknown.insert(id.clone());

                continue;
            };

            selected[index] = true;
        }

        // resolve each sparse override by exact lint id
        for (id, level) in &options.rules {
            let Some(index) = lints.iter().position(|lint| lint.id.as_ref() == id) else {
                unknown.insert(id.clone());

                continue;
            };

            levels[index] = *level;
        }

        // select implementations independently of checked source controls
        let mut selected_lints = Vec::new();
        for (index, (level, is_selected)) in levels.into_iter().zip(selected).enumerate() {
            let severity = match level {
                LintLevel::Off => None,
                LintLevel::Warning => Some(DiagnosticSeverity::Warning),
                LintLevel::Error => Some(DiagnosticSeverity::Error),
            };
            if options.enabled && is_selected {
                selected_lints.push((index, severity));
            }
        }

        // report each unknown id once in configuration order
        let errors = unknown
            .into_iter()
            .map(|lint| LinterError::UnknownConfiguredLint {
                anchor: DiagnosticAnchor::Package(package),
                lint,
            })
            .collect::<Vec<_>>();

        Self {
            registry: lints,
            selected: selected_lints,
            errors: errors.into_boxed_slice(),
        }
    }

    /// Retain lints enabled by configuration or checked source controls.
    pub(crate) fn retain_active(&mut self, controls: &DiagnosticControlIndex<'_>) {
        self.selected.retain(|(index, severity)| {
            let diagnostic = StringId::for_text(self.registry[*index].id.as_ref());
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

    /// Iterate checked DIR module lints.
    pub(crate) fn dir_modules(
        &self,
    ) -> impl Iterator<Item = (&Lint, Option<DiagnosticSeverity>, DirModuleCheck)> {
        self.iter().filter_map(|(lint, severity)| match lint.check {
            LintCheck::DirModule(Some(check)) => Some((lint, severity, check)),
            _ => None,
        })
    }

    /// Iterate checked DIR program lints.
    pub(crate) fn dir_programs(
        &self,
    ) -> impl Iterator<Item = (&Lint, Option<DiagnosticSeverity>, DirProgramCheck)> {
        self.iter().filter_map(|(lint, severity)| match lint.check {
            LintCheck::DirProgram(Some(check)) => Some((lint, severity, check)),
            _ => None,
        })
    }

    /// Iterate verified MIR module lints.
    pub(crate) fn mir_modules(
        &self,
    ) -> impl Iterator<Item = (&Lint, Option<DiagnosticSeverity>, MirModuleCheck)> {
        self.iter().filter_map(|(lint, severity)| match lint.check {
            LintCheck::MirModule(Some(check)) => Some((lint, severity, check)),
            _ => None,
        })
    }

    /// Iterate verified MIR program lints.
    pub(crate) fn mir_programs(
        &self,
    ) -> impl Iterator<Item = (&Lint, Option<DiagnosticSeverity>, MirProgramCheck)> {
        self.iter().filter_map(|(lint, severity)| match lint.check {
            LintCheck::MirProgram(Some(check)) => Some((lint, severity, check)),
            _ => None,
        })
    }

    /// Return whether this set contains checked DIR module lints.
    pub(crate) fn has_dir_modules(&self) -> bool {
        self.dir_modules().next().is_some()
    }

    /// Return whether this set contains checked DIR program lints.
    pub(crate) fn has_dir_programs(&self) -> bool {
        self.dir_programs().next().is_some()
    }

    /// Iterate module indexes required by checked DIR lints at one scope.
    pub(crate) fn dir_indexes(&self, scope: LintScope) -> impl Iterator<Item = IndexKind> + '_ {
        IndexKind::ALL.into_iter().filter(move |kind| {
            self.iter().any(|(lint, _)| {
                lint.is_implemented()
                    && lint.tier() == LintTier::Dir
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
        self.selected
            .iter()
            .map(|(index, severity)| (&self.registry[*index], *severity))
    }
}

#[cfg(test)]
mod tests {
    use destack_repository::LintLevel;

    use super::*;
    use crate::rules::{FOR_DIRECTION, REDUNDANT_CLONE};

    /// Expose executable lints and omit stubs.
    #[test]
    fn test_iterates_executable_lints() {
        let is_executable_listed = Lint::all().any(|lint| lint.id == FOR_DIRECTION.id);
        let is_stub_listed = Lint::all().any(|lint| lint.id == REDUNDANT_CLONE.id);

        assert!(is_executable_listed);
        assert!(!is_stub_listed);
    }

    /// Reject a stub selected exclusively.
    #[test]
    fn test_rejects_selected_stub() {
        let package = PackageId::new(0);
        let options = LinterOptions {
            only: vec!["redundant-clone".to_string()],
            ..LinterOptions::default()
        };
        let registry = Lint::all().cloned().collect::<Vec<_>>().into();
        let lints = LintSet::resolve(package, &options, registry);

        assert!(lints.iter().next().is_none());
        assert_eq!(
            lints.errors(),
            [LinterError::UnknownConfiguredLint {
                anchor: DiagnosticAnchor::Package(package),
                lint: "redundant-clone".to_string(),
            }]
        );
    }

    /// Reject a configured stub.
    #[test]
    fn test_rejects_configured_stub() {
        let package = PackageId::new(0);
        let mut options = LinterOptions::default();
        options
            .rules
            .insert("redundant-clone".to_string(), LintLevel::Error);
        let registry = Lint::all().cloned().collect::<Vec<_>>().into();
        let lints = LintSet::resolve(package, &options, registry);

        assert_eq!(
            lints.errors(),
            [LinterError::UnknownConfiguredLint {
                anchor: DiagnosticAnchor::Package(package),
                lint: "redundant-clone".to_string(),
            }]
        );
    }
}
