use destack_source::DiagnosticSeverity;
use destack_workspace::{LintCategory, LintSeverity};

use super::{LintModuleAstContext, LintModuleDirContext, LintProgramContext};

/// The IR level at which a lint operates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintLevel {
    /// Operates on AST (untyped, pre-binding).
    Ast,
    /// Operates on DIR (typed IR with symbols and types).
    Dir,
    /// Operates on MIR (low-level IR).
    Mir,
}

/// The scope at which a lint operates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintScope {
    /// Operates on a single module.
    Module,
    /// Operates on the entire program (cross-module analysis).
    Program,
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
}

/// Trait for lint rules.
pub trait LintRule: Send + Sync {
    /// Get the static metadata for this lint rule.
    fn meta(&self) -> &'static LintMeta;

    /// Check a module at AST level (syntax patterns, no type info).
    fn check_module_ast<'a>(&self, _severity: LintSeverity, _ctx: &mut LintModuleAstContext<'a>) {}

    /// Check a module at DIR level (typed IR with symbols and types).
    fn check_module_dir<'a>(&self, _severity: LintSeverity, _ctx: &mut LintModuleDirContext<'a>) {}

    /// Check the entire program (cross-module analysis).
    fn check_program(&self, _ctx: &mut LintProgramContext) {}
}

/// A boxed lint rule for dynamic dispatch.
pub type BoxedLintRule = Box<dyn LintRule>;

/// Create a boxed lint rule from a type implementing LintRule.
pub fn boxed<R: LintRule + 'static>(rule: R) -> BoxedLintRule {
    Box::new(rule)
}
