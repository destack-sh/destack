use std::sync::Arc;

use destack_artifact::DiagnosticAnchor;
use destack_repository::{LintLevel, LinterOptions};
use destack_source::{DiagnosticSeverity, PackageId};

use super::{DirModuleCheck, DirProgramCheck, Lint, LintCheck, MirModuleCheck, MirProgramCheck};
use crate::LinterError;

/// Lints for one package configuration.
#[derive(Debug)]
pub(crate) struct LintSet {
    /// The lints.
    lints: Arc<[Lint]>,
    /// The configured severities, or none when lint execution is disabled.
    severities: Option<Box<[Option<DiagnosticSeverity>]>>,
}

impl LintSet {
    /// Resolve one package configuration.
    pub(crate) fn resolve(
        package: PackageId,
        options: &LinterOptions,
        lints: Arc<[Lint]>,
    ) -> Result<Self, LinterError> {
        let mut levels = lints
            .iter()
            .map(|lint| lint.default_level)
            .collect::<Vec<_>>();
        let mut configured = vec![None; lints.len()];

        // resolve each sparse override by stable id or diagnostic code
        for (selector, level) in &options.rules {
            let index = lints
                .iter()
                .position(|lint| lint.id.as_ref() == selector || lint.code.as_ref() == selector)
                .ok_or_else(|| LinterError::UnknownConfiguredLint {
                    anchor: DiagnosticAnchor::Package(package),
                    selector: selector.clone(),
                })?;
            if let Some(first) = configured[index].replace(selector.as_str()) {
                return Err(LinterError::DuplicateConfiguredLint {
                    anchor: DiagnosticAnchor::Package(package),
                    lint: lints[index].id.to_string(),
                    first: first.to_string(),
                    second: selector.clone(),
                });
            }

            levels[index] = *level;
        }

        // validate disabled configurations without scheduling lint work
        if !options.enabled {
            return Ok(Self {
                lints,
                severities: None,
            });
        }

        // resolve each lint level
        let mut severities = Vec::with_capacity(lints.len());
        for level in levels {
            let severity = match level {
                LintLevel::Off => None,
                LintLevel::Warning => Some(DiagnosticSeverity::Warning),
                LintLevel::Error => Some(DiagnosticSeverity::Error),
            };
            severities.push(severity);
        }

        Ok(Self {
            lints,
            severities: Some(severities.into_boxed_slice()),
        })
    }

    /// Return the lints in this set.
    pub(crate) fn lints(&self) -> &[Lint] {
        &self.lints
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

    /// Return whether this configuration contains checked DIR module lints.
    pub(crate) fn has_dir_modules(&self) -> bool {
        self.dir_modules().next().is_some()
    }

    /// Return whether this configuration contains checked DIR program lints.
    pub(crate) fn has_dir_programs(&self) -> bool {
        self.dir_programs().next().is_some()
    }

    /// Return whether this configuration contains verified MIR module lints.
    pub(crate) fn has_mir_modules(&self) -> bool {
        self.mir_modules().next().is_some()
    }

    /// Return whether this configuration contains verified MIR program lints.
    pub(crate) fn has_mir_programs(&self) -> bool {
        self.mir_programs().next().is_some()
    }

    /// Return whether this configuration contains program lints.
    pub(crate) fn has_programs(&self) -> bool {
        self.has_dir_programs() || self.has_mir_programs()
    }

    /// Iterate lints and configured severities when execution is enabled.
    fn iter(&self) -> impl Iterator<Item = (&Lint, Option<DiagnosticSeverity>)> {
        self.severities
            .as_deref()
            .into_iter()
            .flat_map(|severities| self.lints.iter().zip(severities.iter().copied()))
    }
}
