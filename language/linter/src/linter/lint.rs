use std::borrow::Cow;
use std::fmt;

use destack_artifact::{DiagnosticAnchor, DiagnosticBuilder};
use destack_repository::{LintLevel, ProviderError};
use destack_source::{Applicability, DiagnosticSuggestion, PatchSet};

use super::{DirModule, DirProgram, MirModule, MirProgram};
use crate::LinterDiagnostic;

/// One lint diagnostic category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintCategory {
    /// Likely bugs and logic errors.
    Correctness,
    /// Code that is likely unintentional.
    Suspicious,
    /// Inefficient code.
    Performance,
    /// Canonical language style.
    Style,
    /// Potential vulnerabilities.
    Security,
}

impl LintCategory {
    /// Return the category name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Correctness => "correctness",
            Self::Suspicious => "suspicious",
            Self::Performance => "performance",
            Self::Style => "style",
            Self::Security => "security",
        }
    }
}

impl fmt::Display for LintCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// The IR tier inspected by one lint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintTier {
    /// Checked DIR.
    Dir,
    /// Verified MIR.
    Mir,
}

impl LintTier {
    /// Return the name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Dir => "DIR",
            Self::Mir => "MIR",
        }
    }
}

/// The compilation scope inspected by one lint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintScope {
    /// One module.
    Module,
    /// One target program.
    Program,
}

impl LintScope {
    /// Return the name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Program => "program",
        }
    }
}

/// The safety of corrections emitted by one lint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fixability {
    /// Does not emit corrections.
    None,
    /// Emits corrections that are safe to apply automatically.
    Automatic,
    /// Emits corrections whose safety is decided for each report.
    Suggestion,
}

/// The result of one lint check.
pub type LintResult = Result<LintOutput, ProviderError>;

/// Diagnostics produced by one lint check.
#[derive(Debug, Default)]
pub struct LintOutput {
    /// The reported diagnostics.
    diagnostics: Vec<DiagnosticBuilder<LinterDiagnostic>>,
}

impl LintOutput {
    /// Report one diagnostic.
    pub fn report(&mut self, diagnostic: DiagnosticBuilder<LinterDiagnostic>) {
        self.diagnostics.push(diagnostic);
    }

    /// Return the reported diagnostics.
    pub(crate) fn into_diagnostics(self) -> Vec<DiagnosticBuilder<LinterDiagnostic>> {
        self.diagnostics
    }
}

/// One checked DIR module lint function.
pub type DirModuleCheck = for<'a> fn(&DirModule<'a>, &Lint) -> LintResult;

/// One checked DIR program lint function.
pub type DirProgramCheck = fn(&DirProgram, &Lint) -> LintResult;

/// One verified MIR module lint function.
pub type MirModuleCheck = for<'a> fn(&MirModule<'a>, &Lint) -> LintResult;

/// One verified MIR program lint function.
pub type MirProgramCheck = fn(&MirProgram, &Lint) -> LintResult;

/// The tier and scope of a lint.
#[derive(Clone, Copy)]
pub enum LintCheck {
    /// Check one module's checked DIR.
    DirModule(DirModuleCheck),
    /// Check one target program's checked DIR.
    DirProgram(DirProgramCheck),
    /// Check one module's verified MIR.
    MirModule(MirModuleCheck),
    /// Check one target program's verified MIR.
    MirProgram(MirProgramCheck),
}

impl fmt::Debug for LintCheck {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::DirModule(_) => "DirModule",
            Self::DirProgram(_) => "DirProgram",
            Self::MirModule(_) => "MirModule",
            Self::MirProgram(_) => "MirProgram",
        })
    }
}

impl LintCheck {
    /// Return the IR tier inspected by this check.
    pub const fn tier(self) -> LintTier {
        match self {
            Self::DirModule(_) | Self::DirProgram(_) => LintTier::Dir,
            Self::MirModule(_) | Self::MirProgram(_) => LintTier::Mir,
        }
    }

    /// Return the compilation scope inspected by this check.
    pub const fn scope(self) -> LintScope {
        match self {
            Self::DirModule(_) | Self::MirModule(_) => LintScope::Module,
            Self::DirProgram(_) | Self::MirProgram(_) => LintScope::Program,
        }
    }
}

/// One canonical example of a lint violation and its accepted replacement.
#[derive(Debug, Clone)]
pub struct LintExample {
    /// Source that produces the lint.
    pub reported: Cow<'static, str>,
    /// Source that expresses the same intent without the lint.
    pub accepted: Cow<'static, str>,
}

impl LintExample {
    /// Return the reported source without its framing newlines.
    pub fn reported(&self) -> &str {
        trim_source_frame(&self.reported)
    }

    /// Return the accepted source without its framing newlines.
    pub fn accepted(&self) -> &str {
        trim_source_frame(&self.accepted)
    }
}

/// Remove one framing newline from each edge of multiline source.
fn trim_source_frame(source: &str) -> &str {
    let source = source.strip_prefix('\n').unwrap_or(source);

    source.strip_suffix('\n').unwrap_or(source)
}

/// A lint.
#[derive(Debug, Clone)]
pub struct Lint {
    /// The lint id.
    pub id: Cow<'static, str>,
    /// The concise rule summary.
    pub summary: Cow<'static, str>,
    /// The rule rationale and reporting criteria.
    pub explanation: Cow<'static, str>,
    /// The canonical reported and accepted source pair.
    pub example: LintExample,
    /// The diagnostic category.
    pub category: LintCategory,
    /// The default level.
    pub default_level: LintLevel,
    /// The safety of emitted corrections.
    pub fixability: Fixability,
    /// The check.
    pub check: LintCheck,
}

impl Lint {
    /// Create one diagnostic for this lint.
    pub fn diagnostic(
        &self,
        message: impl Into<String>,
        primary: impl Into<DiagnosticAnchor>,
    ) -> DiagnosticBuilder<LinterDiagnostic> {
        let diagnostic = LinterDiagnostic::new(self, message, primary);

        DiagnosticBuilder::new(diagnostic)
    }

    /// Create an automatic correction for this lint.
    pub fn fix(
        &self,
        message: impl Into<String>,
        patches: PatchSet,
    ) -> Result<DiagnosticSuggestion, ProviderError> {
        self.correction(message, patches, Applicability::Automatic)
    }

    /// Create a review correction for this lint.
    pub fn suggestion(
        &self,
        message: impl Into<String>,
        patches: PatchSet,
    ) -> Result<DiagnosticSuggestion, ProviderError> {
        self.correction(message, patches, Applicability::Dangerous)
    }

    /// Return whether this lint can provide fixes.
    pub const fn is_fixable(&self) -> bool {
        !matches!(self.fixability, Fixability::None)
    }

    /// Return the IR tier inspected by this lint.
    pub const fn tier(&self) -> LintTier {
        self.check.tier()
    }

    /// Return the compilation scope inspected by this lint.
    pub const fn scope(&self) -> LintScope {
        self.check.scope()
    }

    /// Create one correction allowed by this lint.
    fn correction(
        &self,
        message: impl Into<String>,
        patches: PatchSet,
        applicability: Applicability,
    ) -> Result<DiagnosticSuggestion, ProviderError> {
        let is_allowed = match (self.fixability, applicability) {
            (Fixability::None, _) => false,
            (Fixability::Automatic, Applicability::Automatic) => true,
            (Fixability::Automatic, _) => false,
            (Fixability::Suggestion, _) => true,
        };
        if !is_allowed {
            return Err(ProviderError::internal(format!(
                "lint '{}' cannot emit a {applicability:?} correction",
                self.id
            )));
        }

        Ok(DiagnosticSuggestion::new(message, patches, applicability))
    }
}
