use std::sync::Arc;

use destack_artifact::{DiagnosticAnchor, DiagnosticControlIndex};
use destack_core::StringId;
use destack_repository::{LintLevel, LinterOptions};
use destack_source::{DiagnosticSeverity, PackageId};

use super::{DirModuleCheck, DirProgramCheck, Lint, LintCheck, MirModuleCheck, MirProgramCheck};
use crate::LinterError;

/// Lints scheduled for one module or program.
#[derive(Debug)]
pub(crate) struct LintSet {
    /// The lint registry.
    registry: Arc<[Lint]>,
    /// The scheduled lint indices and configured severities.
    scheduled: Box<[(usize, Option<DiagnosticSeverity>)]>,
}

impl LintSet {
    /// Resolve the lints scheduled by one package configuration and its source controls.
    pub(crate) fn resolve(
        package: PackageId,
        options: &LinterOptions,
        lints: Arc<[Lint]>,
        controls: &DiagnosticControlIndex<'_>,
    ) -> Result<Self, LinterError> {
        let mut levels = lints
            .iter()
            .map(|lint| lint.default_level)
            .collect::<Vec<_>>();

        // resolve each sparse override by exact lint id
        for (id, level) in &options.rules {
            let index = lints
                .iter()
                .position(|lint| lint.id.as_ref() == id)
                .ok_or_else(|| LinterError::UnknownConfiguredLint {
                    anchor: DiagnosticAnchor::Package(package),
                    lint: id.clone(),
                })?;

            levels[index] = *level;
        }

        // schedule configured or source-activated lints
        let mut scheduled = Vec::new();
        for (index, level) in levels.into_iter().enumerate() {
            let severity = match level {
                LintLevel::Off => None,
                LintLevel::Warning => Some(DiagnosticSeverity::Warning),
                LintLevel::Error => Some(DiagnosticSeverity::Error),
            };
            let diagnostic = StringId::for_text(lints[index].id.as_ref());
            let is_activated = controls
                .iter()
                .any(|(_, table)| table.activates(diagnostic));
            let is_scheduled = severity.is_some() || is_activated;
            if options.enabled && is_scheduled {
                scheduled.push((index, severity));
            }
        }

        Ok(Self {
            registry: lints,
            scheduled: scheduled.into_boxed_slice(),
        })
    }

    /// Iterate checked DIR module lints.
    pub(crate) fn dir_modules(
        &self,
    ) -> impl Iterator<Item = (&Lint, Option<DiagnosticSeverity>, DirModuleCheck)> {
        self.iter().filter_map(|(lint, severity)| match lint.check {
            LintCheck::DirModule(check) => Some((lint, severity, check)),
            _ => None,
        })
    }

    /// Iterate checked DIR program lints.
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

    /// Return whether this set contains checked DIR module lints.
    pub(crate) fn has_dir_modules(&self) -> bool {
        self.dir_modules().next().is_some()
    }

    /// Return whether this set contains checked DIR program lints.
    pub(crate) fn has_dir_programs(&self) -> bool {
        self.dir_programs().next().is_some()
    }

    /// Return whether this set contains verified MIR module lints.
    pub(crate) fn has_mir_modules(&self) -> bool {
        self.mir_modules().next().is_some()
    }

    /// Return whether this set contains verified MIR program lints.
    pub(crate) fn has_mir_programs(&self) -> bool {
        self.mir_programs().next().is_some()
    }

    /// Return whether this set contains program lints.
    pub(crate) fn has_programs(&self) -> bool {
        self.has_dir_programs() || self.has_mir_programs()
    }

    /// Iterate scheduled lints and configured severities.
    fn iter(&self) -> impl Iterator<Item = (&Lint, Option<DiagnosticSeverity>)> {
        self.scheduled
            .iter()
            .map(|(index, severity)| (&self.registry[*index], *severity))
    }
}
