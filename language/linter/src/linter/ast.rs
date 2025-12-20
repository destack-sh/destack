use std::sync::Arc;

use destack_ast::{self as ast, Annotation, Argument, Expression, ScalarLiteral, StringPool};
use destack_source::{EditBuilder, File};
use destack_workspace::{LintSeverity, LinterOptions, Module, Program};

use crate::{LintDiagnostic, LintMeta};

/// Severity override from a `@allow`/`@warn`/`@deny`/`@forbid` decorator.
#[derive(Debug, Clone, Copy)]
struct LintSeverityOverride {
    severity: LintSeverity,
    /// `@forbid` prevents inner scopes from overriding.
    is_forbidden: bool,
}

/// Context for AST-level linting of a single module. Unfurls ModuleAst.
pub struct LintModuleAstContext<'a> {
    /// The program containing this module.
    pub program: Arc<Program>,
    /// The module being linted.
    pub module: &'a Module,
    /// The source file.
    pub file: Arc<File>,

    /// The AST tree.
    pub tree: &'a ast::NodeTree,
    /// The parent index.
    pub parents: &'a ast::NodeParentIndex,
    /// The roots.
    pub roots: &'a Vec<ast::LocalNodeId<ast::Expression>>,
    /// The string pool.
    pub strings: &'a StringPool,

    /// Linter configuration.
    pub options: &'a LinterOptions,

    /// Whether to compute fixes for diagnostics.
    pub compute_fixes: bool,

    /// Collected diagnostics.
    diagnostics: Vec<LintDiagnostic>,
}

impl<'a> std::fmt::Debug for LintModuleAstContext<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintModuleAstContext")
            .field("module_id", &self.module.id)
            .finish()
    }
}

#[allow(clippy::too_many_arguments)]
impl<'a> LintModuleAstContext<'a> {
    /// Create a new AST lint context for a module.
    pub fn new(
        program: Arc<Program>,
        module: &'a Module,
        file: Arc<File>,
        tree: &'a ast::NodeTree,
        parents: &'a ast::NodeParentIndex,
        roots: &'a Vec<ast::LocalNodeId<ast::Expression>>,
        strings: &'a StringPool,
        options: &'a LinterOptions,
        compute_fixes: bool,
    ) -> Self {
        Self {
            program,
            module,
            file,
            tree,
            parents,
            roots,
            strings,
            options,
            compute_fixes,
            diagnostics: Vec::new(),
        }
    }

    /// Return the file id.
    pub fn file_id(&self) -> destack_source::FileId {
        self.module.file_id
    }

    /// Return the module id.
    pub fn module_id(&self) -> destack_source::ModuleId {
        self.module.id
    }

    /// Return the linter options.
    pub fn options(&self) -> &LinterOptions {
        self.options
    }

    /// Resolve severity for a rule.
    pub fn get_severity(&self, meta: &LintMeta) -> LintSeverity {
        self.options
            .resolve_severity(meta.id, meta.category, meta.category.default_severity())
    }

    /// Check if a rule is enabled.
    pub fn is_rule_enabled(&self, meta: &LintMeta) -> bool {
        self.get_severity(meta).is_enabled()
    }

    /// Get effective severity for a rule at a specific node.
    ///
    /// Checks for `@allow`/`@deny`/`@warn`/`@forbid` decorators on the node
    /// and its ancestors, returning the effective severity at that location.
    /// Rules should call this before reporting to respect per-node suppressions.
    pub fn get_effective_severity<T: ast::Node>(
        &self,
        meta: &LintMeta,
        node_id: ast::LocalNodeId<T>,
    ) -> LintSeverity {
        // walk up parent chain, collecting decorator overrides (innermost first)
        let mut overrides: Vec<LintSeverityOverride> = Vec::new();
        let mut current = Some(node_id.id);

        while let Some(id) = current {
            for annotation_id in self.tree.get_annotations(id) {
                if let Some(override_info) = self.parse_decorator(annotation_id, meta) {
                    overrides.push(override_info);
                }
            }
            current = self.parents.get_by_id(id);
        }

        // apply from outermost to innermost (reverse since we collected innermost first)
        let mut effective = self.get_severity(meta);
        let mut is_forbidden = false;

        for item in overrides.into_iter().rev() {
            if is_forbidden {
                continue;
            }
            effective = item.severity;
            is_forbidden = item.is_forbidden;
        }

        effective
    }

    /// Parse a decorator annotation and return the severity override if it matches this lint.
    fn parse_decorator(
        &self,
        annotation_id: ast::LocalNodeId<Annotation>,
        meta: &LintMeta,
    ) -> Option<LintSeverityOverride> {
        let annotation = self.tree.get(annotation_id);
        let Annotation::Decorator { node, .. } = annotation else {
            return None;
        };

        // check decorator name (must be single segment: allow, warn, deny, forbid)
        let decorator = self.tree.get(*node);
        if decorator.left.segments.len() != 1 {
            return None;
        }

        let name = self.strings.get(decorator.left.segments[0]);
        let (severity, is_forbidden) = match name.as_ref() {
            "allow" => (LintSeverity::Off, false),
            "warn" => (LintSeverity::Warning, false),
            "deny" => (LintSeverity::Error, false),
            "forbid" => (LintSeverity::Error, true),
            _ => return None,
        };

        // extract the string argument (lint ID or code)
        let arguments = decorator.arguments.as_ref()?;
        let first_argument = self.tree.get(*arguments.first()?);
        let Argument::Positional { value } = first_argument else {
            return None;
        };

        let argument_expression = self.tree.get(*value);
        let Expression::ScalarLiteral(ScalarLiteral::String(string_id)) = argument_expression
        else {
            return None;
        };

        // match against lint ID or code
        let specifier = self.strings.get(*string_id);
        if specifier.as_ref() == meta.id || specifier.as_ref() == meta.code {
            Some(LintSeverityOverride {
                severity,
                is_forbidden,
            })
        } else {
            None
        }
    }

    /// Report a lint diagnostic.
    pub fn report(&mut self, diagnostic: LintDiagnostic) {
        if diagnostic.is_enabled() {
            self.diagnostics.push(diagnostic);
        }
    }

    /// Take the collected diagnostics.
    pub fn take_diagnostics(&mut self) -> Vec<LintDiagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Return a reference to collected diagnostics.
    pub fn diagnostics(&self) -> &[LintDiagnostic] {
        &self.diagnostics
    }

    /// Get the full source text.
    pub fn source_text(&self) -> &str {
        self.file.text()
    }

    /// Get the source text for a span.
    pub fn get_span_text(&self, span: destack_source::Span) -> &str {
        &self.file.text()[span.start as usize..span.end as usize]
    }

    /// Create an EditBuilder with source text for text-aware operations.
    pub fn edit_builder(&self) -> EditBuilder<'_> {
        EditBuilder::from_file(self.module.file_id, self.file.text())
    }
}

#[cfg(test)]
mod tests {
    use crate::linter::TestProgram;
    use crate::rules::suspicious::NoEmpty;

    #[test]
    fn test_allow_suppresses_by_id() {
        let test = TestProgram::for_rule_with_builtins(NoEmpty);
        let result = test.lint_ast(
            "test.ds",
            r#"
@allow("no-empty")
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("no-empty");
    }

    #[test]
    fn test_allow_suppresses_by_code() {
        let test = TestProgram::for_rule_with_builtins(NoEmpty);
        let result = test.lint_ast(
            "test.ds",
            r#"
@allow("LC002")
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("no-empty");
    }

    #[test]
    fn test_allow_on_block() {
        let test = TestProgram::for_rule_with_builtins(NoEmpty);
        let result = test.lint_ast(
            "test.ds",
            r#"
@allow("no-empty")
{}
"#,
        );
        test.result(result).assert_no_lint("no-empty");
    }

    #[test]
    fn test_allow_does_not_affect_other_lints() {
        let test = TestProgram::for_rule_with_builtins(NoEmpty);
        let result = test.lint_ast(
            "test.ds",
            r#"
@allow("some-other-lint")
function foo() {}
"#,
        );
        test.result(result).assert_lint("no-empty");
    }

    #[test]
    fn test_forbid_prevents_inner_allow() {
        let test = TestProgram::for_rule_with_builtins(NoEmpty);
        let result = test.lint_ast(
            "test.ds",
            r#"
@forbid("no-empty")
function outer() {
    @allow("no-empty")
    function inner() {}
}
"#,
        );
        // inner @allow should be ignored due to outer @forbid
        test.result(result).assert_lint("no-empty");
    }

    #[test]
    fn test_warn_changes_severity() {
        let test = TestProgram::for_rule_with_builtins(NoEmpty);
        let result = test.lint_ast(
            "test.ds",
            r#"
@warn("no-empty")
function foo() {}
"#,
        );
        // should still lint but with warning severity
        test.result(result).assert_lint("no-empty");
    }

    #[test]
    fn test_deny_changes_severity() {
        let test = TestProgram::for_rule_with_builtins(NoEmpty);
        let result = test.lint_ast(
            "test.ds",
            r#"
@deny("no-empty")
function foo() {}
"#,
        );
        // should still lint with error severity
        test.result(result).assert_lint("no-empty");
    }
}
