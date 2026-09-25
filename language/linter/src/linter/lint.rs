use std::borrow::Cow;
use std::fmt;

use tspp_artifact::{DiagnosticAnchor, DiagnosticBuilder, IndexKind};
use tspp_repository::{LintLevel, ProviderError};
use tspp_source::{Applicability, DiagnosticSuggestion, PatchSet};

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

/// A project whose lint informed this lint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintSource {
    /// Biome.
    Biome,
    /// Clippy.
    Clippy,
    /// ESLint.
    Eslint,
    /// eslint-plugin-jest.
    Jest,
    /// Oxc.
    Oxc,
    /// eslint-plugin-playwright.
    Playwright,
    /// eslint-plugin-react.
    React,
    /// Ruff.
    Ruff,
    /// rustc.
    Rustc,
    /// eslint-plugin-sonarjs.
    SonarJs,
    /// typescript-eslint.
    TypeScriptEslint,
    /// eslint-plugin-unicorn.
    Unicorn,
}

impl LintSource {
    /// Return the project name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Biome => "Biome",
            Self::Clippy => "Clippy",
            Self::Eslint => "ESLint",
            Self::Jest => "eslint-plugin-jest",
            Self::Oxc => "Oxc",
            Self::Playwright => "eslint-plugin-playwright",
            Self::React => "eslint-plugin-react",
            Self::Ruff => "Ruff",
            Self::Rustc => "rustc",
            Self::SonarJs => "eslint-plugin-sonarjs",
            Self::TypeScriptEslint => "typescript-eslint",
            Self::Unicorn => "eslint-plugin-unicorn",
        }
    }

    /// Return the documentation URL for one earlier rule.
    pub fn rule_url(self, rule: &str) -> String {
        match self {
            Self::Biome => format!("https://github.com/biomejs/biome/search?q={rule}&type=code"),
            Self::Clippy => {
                format!("https://rust-lang.github.io/rust-clippy/master/index.html#{rule}")
            }
            Self::Eslint => format!("https://eslint.org/docs/latest/rules/{rule}"),
            Self::Jest => format!(
                "https://github.com/jest-community/eslint-plugin-jest/blob/main/docs/rules/{rule}.md"
            ),
            Self::Oxc => {
                format!("https://github.com/oxc-project/oxc/search?q={rule}&type=code")
            }
            Self::Playwright => format!(
                "https://github.com/playwright-community/eslint-plugin-playwright/blob/main/docs/rules/{rule}.md"
            ),
            Self::React => format!(
                "https://github.com/jsx-eslint/eslint-plugin-react/blob/master/docs/rules/{rule}.md"
            ),
            Self::Ruff => format!("https://docs.astral.sh/ruff/rules/{rule}/"),
            Self::Rustc => format!(
                "https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#{rule}"
            ),
            Self::SonarJs => {
                format!("https://github.com/SonarSource/SonarJS/search?q={rule}&type=code")
            }
            Self::TypeScriptEslint => format!("https://typescript-eslint.io/rules/{rule}"),
            Self::Unicorn => format!(
                "https://github.com/sindresorhus/eslint-plugin-unicorn/blob/main/docs/rules/{rule}.md"
            ),
        }
    }
}

impl fmt::Display for LintSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// One earlier lint that informed this lint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LintProvenance {
    /// The project that defines the earlier lint.
    pub source: LintSource,
    /// The earlier rule name.
    pub rule: &'static str,
}

/// The IR tier inspected by one lint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintTier {
    /// DIR.
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

impl Fixability {
    /// Return the correction kind.
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Automatic => "automatic",
            Self::Suggestion => "suggestion",
        }
    }
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

/// One DIR module lint function.
pub type DirModuleCheck = for<'a> fn(&DirModule<'a>, &Lint) -> LintResult;

/// One DIR program lint function.
pub type DirProgramCheck = for<'a> fn(&DirProgram<'a>, &Lint) -> LintResult;

/// One verified MIR module lint function.
pub type MirModuleCheck = for<'a> fn(&mut MirModule<'a>, &Lint) -> LintResult;

/// One verified MIR program lint function.
pub type MirProgramCheck = for<'a> fn(&mut MirProgram<'a>, &Lint) -> LintResult;

/// The tier and scope of a lint.
#[derive(Clone, Copy)]
pub enum LintCheck {
    /// Check one module's DIR.
    DirModule(DirModuleCheck),
    /// Check one target program's DIR.
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

/// One source file in a canonical lint example.
#[derive(Debug, Clone)]
pub struct LintExampleSource {
    /// The source path.
    pub path: Cow<'static, str>,
    /// The source text.
    pub source: Cow<'static, str>,
}

impl LintExampleSource {
    /// Return the source without its framing newlines.
    pub fn source(&self) -> &str {
        trim_source_frame(&self.source)
    }

    /// Return the source path.
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// One canonical example of a lint violation and its accepted replacement.
#[derive(Debug, Clone)]
pub struct LintExample {
    /// The source that produces the lint.
    pub reported: LintExampleSource,
    /// The source that expresses the same intent without the lint.
    pub accepted: LintExampleSource,
}

/// Remove one framing newline from each edge of multiline source.
fn trim_source_frame(source: &str) -> &str {
    let source = source.strip_prefix('\n').unwrap_or(source);

    source.strip_suffix('\n').unwrap_or(source)
}

/// A lint.
#[derive(Debug)]
pub struct Lint {
    /// The lint id.
    pub id: Cow<'static, str>,
    /// The concise rule summary.
    pub summary: Cow<'static, str>,
    /// The rule rationale and reporting criteria.
    pub explanation: Cow<'static, str>,
    /// The canonical reported and accepted source pair.
    pub example: LintExample,
    /// The earlier lints that informed this lint, ordered by source and rule.
    pub provenance: &'static [LintProvenance],
    /// The diagnostic category.
    pub category: LintCategory,
    /// The default level.
    pub default_level: LintLevel,
    /// The safety of emitted corrections.
    pub fixability: Fixability,
    /// The module indexes required by the check.
    pub module_indexes: &'static [IndexKind],
    /// The check.
    pub check: LintCheck,
    /// The source file declaring the lint.
    pub source_path: &'static str,
    /// The one-based source line declaring the lint.
    pub source_line: u32,
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
        patches: impl Into<PatchSet>,
    ) -> Result<DiagnosticSuggestion, ProviderError> {
        self.correction(message, patches.into(), Applicability::Automatic)
    }

    /// Create a review correction for this lint.
    pub fn suggestion(
        &self,
        message: impl Into<String>,
        patches: impl Into<PatchSet>,
    ) -> Result<DiagnosticSuggestion, ProviderError> {
        self.correction(message, patches.into(), Applicability::Dangerous)
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
