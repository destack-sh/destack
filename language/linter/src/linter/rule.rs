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

/// Static metadata about a lint rule.
#[derive(Debug, Clone, Copy)]
pub struct LintMeta {
    /// Rule ID like "no-floating-promise".
    pub id: &'static str,
    /// Code like "LC001".
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
    /// Whether this lint has an auto-fix.
    pub fixable: bool,
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

    /// Check if this lint is enabled by default (part of recommended set).
    pub fn is_recommended(&self) -> bool {
        self.category.is_recommended()
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
