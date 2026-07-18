use std::borrow::Cow;
use std::fmt;

use destack_artifact::{DiagnosticAnchor, DiagnosticBuilder};
use destack_core::StringId;
use destack_repository::{LintLevel, ProviderError};

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

/// The availability of automatic fixes for one lint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fixability {
    /// Always provides a fix when it reports.
    Always,
    /// Can fix some cases but not all.
    Sometimes,
    /// Never provides fixes.
    Never,
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
pub type DirModuleCheck = fn(&DirModule, &Lint) -> LintResult;

/// One checked DIR program lint function.
pub type DirProgramCheck = fn(&DirProgram, &Lint) -> LintResult;

/// One verified MIR module lint function.
pub type MirModuleCheck = fn(&MirModule, &Lint) -> LintResult;

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

/// A lint.
#[derive(Debug, Clone)]
pub struct Lint {
    /// The lint id.
    pub id: Cow<'static, str>,
    /// The diagnostic code.
    pub code: Cow<'static, str>,
    /// The description.
    pub description: Cow<'static, str>,
    /// The diagnostic category.
    pub category: LintCategory,
    /// The default level.
    pub default_level: LintLevel,
    /// The availability of automatic fixes.
    pub fixability: Fixability,
    /// The check.
    pub check: LintCheck,
}

impl Lint {
    /// Return the accepted diagnostic selectors.
    pub(crate) fn selectors(&self) -> [StringId; 2] {
        [
            StringId::for_text(self.id.as_ref()),
            StringId::for_text(self.code.as_ref()),
        ]
    }

    /// Create one diagnostic for this lint.
    pub fn diagnostic(
        &self,
        message: impl Into<String>,
        primary: impl Into<DiagnosticAnchor>,
    ) -> DiagnosticBuilder<LinterDiagnostic> {
        let diagnostic = LinterDiagnostic::new(self, message, primary);

        DiagnosticBuilder::new(diagnostic)
    }

    /// Return whether this lint can provide fixes.
    pub const fn is_fixable(&self) -> bool {
        matches!(self.fixability, Fixability::Always | Fixability::Sometimes)
    }

    /// Return the IR tier inspected by this lint.
    pub const fn tier(&self) -> LintTier {
        self.check.tier()
    }

    /// Return the compilation scope inspected by this lint.
    pub const fn scope(&self) -> LintScope {
        self.check.scope()
    }
}
