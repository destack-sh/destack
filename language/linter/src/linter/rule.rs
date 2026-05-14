use destack_dir::LanguageItem;
use destack_source::{DiagnosticSeverity, FileType};
use destack_workspace::{LintCategory, LintSeverity};

use super::{LintModuleContext, LintPackageContext, LintWorkspaceContext};

/// The IR level at which a lint operates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintLevel {
    /// Operates on DIR.
    Dir,
    /// Operates on MIR.
    Mir,
}

/// The scope at which a lint operates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintScope {
    /// Operates on a single module.
    Module,
    /// Operates on a single package.
    Package,
    /// Operates on the active workspace.
    Workspace,
}

/// A symbol requirement for running a lint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintRequirement {
    /// A symbol provided by a builtin lib, resolved by name.
    /// If libs are provided (second argument), at least one of them must be available.
    RequireLibSymbol(&'static str, &'static [&'static str]),
    /// A language item provided by the standard library.
    RequireLanguageItem(LanguageItem),
}

/// Whether a lint is part of the recommended set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Recommended {
    /// Always recommended (core recommended set).
    Always,
    /// Only recommended in strict/pedantic mode.
    Strict,
    /// Not recommended by default.
    Off,
}

/// Whether a lint can provide automatic fixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fixable {
    /// Always provides a fix when it reports.
    Always,
    /// Can fix some cases but not all.
    Sometimes,
    /// Never provides fixes.
    No,
}

/// The stability/maturity of a lint rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stability {
    /// Mature and well-tested.
    Stable,
    /// New rule, may have rough edges or false positives.
    Experimental,
}

/// How a lint should run on declaration files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeclarationMode {
    /// Run on both declaration and non declaration files.
    Include,
    /// Skip declaration files.
    Exclude,
    /// Run only on declaration files.
    Only,
}

/// Static metadata about a lint rule.
#[derive(Debug, Clone, Copy)]
pub struct LintMeta {
    /// Rule ID like "no-floating-promise".
    pub id: &'static str,
    /// Code like "LC002".
    pub code: &'static str,
    /// Name like "NoFloatingPromise".
    pub name: &'static str,
    /// Human-readable description.
    pub description: &'static str,
    /// Category.
    pub category: LintCategory,
    /// Default severity when not configured.
    pub default_severity: DiagnosticSeverity,
    /// URL to documentation.
    pub docs_url: Option<&'static str>,
    /// Whether this lint can provide automatic fixes.
    pub fixable: Fixable,
    /// Whether this lint is part of the recommended set.
    pub recommended: Recommended,
    /// The stability/maturity of this lint.
    pub stability: Stability,
    /// IR level this lint operates on.
    pub level: LintLevel,
    /// Scope this lint operates on.
    pub scope: LintScope,
    /// Symbols that must all be available for this lint to run.
    pub requires_all: &'static [LintRequirement],
    /// Symbols where at least one must be available for this lint to run.
    pub requires_any: &'static [LintRequirement],
    /// Declaration file policy for this lint.
    pub declarations: DeclarationMode,
}

impl LintMeta {
    /// Get the full lint identifier (category/id).
    pub fn full_id(&self) -> String {
        format!("{}/{}", self.category.name(), self.id)
    }

    /// Check if this lint is part of the core recommended set.
    pub fn is_recommended(&self) -> bool {
        matches!(self.recommended, Recommended::Always)
    }

    /// Check if this lint is recommended in strict mode.
    pub fn is_strict(&self) -> bool {
        matches!(self.recommended, Recommended::Always | Recommended::Strict)
    }

    /// Check if this lint can provide fixes.
    pub fn is_fixable(&self) -> bool {
        matches!(self.fixable, Fixable::Always | Fixable::Sometimes)
    }

    /// Check if this lint is stable.
    pub fn is_stable(&self) -> bool {
        matches!(self.stability, Stability::Stable)
    }

    /// Check whether this lint should run on the given file type.
    pub fn supports_file_type(&self, file_type: FileType) -> bool {
        let is_declaration = matches!(
            file_type,
            FileType::TypeScriptDeclaration | FileType::DestackDeclaration
        );

        match self.declarations {
            DeclarationMode::Include => true,
            DeclarationMode::Exclude => !is_declaration,
            DeclarationMode::Only => is_declaration,
        }
    }
}

/// Trait for lint rules.
pub trait LintRule: Send + Sync {
    /// Get the static metadata for this lint rule.
    fn meta(&self) -> &'static LintMeta;

    /// Check one module.
    fn check_module<'a>(&self, _severity: LintSeverity, _ctx: &mut LintModuleContext<'a>) {}

    /// Check one package.
    fn check_package(&self, _ctx: &mut LintPackageContext) {}

    /// Check one workspace.
    fn check_workspace(&self, _ctx: &mut LintWorkspaceContext) {}
}

/// A boxed lint rule for dynamic dispatch.
pub type BoxedLintRule = Box<dyn LintRule>;

/// Create a boxed lint rule from a type implementing LintRule.
pub fn boxed<R: LintRule + 'static>(rule: R) -> BoxedLintRule {
    Box::new(rule)
}
