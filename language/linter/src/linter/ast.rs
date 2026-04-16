use std::sync::Arc;

use destack_artifact::Ast;
use destack_ast::{self as ast, Argument, Decorator, Expression, ScalarLiteral, StringPool};
use destack_source::{EditBuilder, File, FileId, ModuleId, Span};
use destack_workspace::{LintSeverity, LinterOptions, Module, Repository, Revision};

use crate::rules::common::expression_path_segments;
use crate::{
    ConstValue, LintAstAnalysisCache, LintDiagnostic, LintMeta, LintRegexParse, LintRequirement,
    find_control_character, find_control_characters, find_misleading_character_class,
    find_useless_backreference,
};

/// Severity override from a `@allow`/`@warn`/`@deny`/`@forbid` decorator.
#[derive(Debug, Clone, Copy)]
struct LintSeverityOverride {
    severity: LintSeverity,
    /// `@forbid` prevents inner scopes from overriding.
    is_forbidden: bool,
}

/// The parsed shape of a decorator expression in the AST.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DecoratorCall<'a> {
    /// The decorator callee expression.
    pub callee: ast::LocalNodeId<ast::Expression>,
    /// The decorator arguments when the expression is a call.
    pub arguments: Option<&'a [ast::LocalNodeId<ast::Argument>]>,
}

/// Context for AST-level linting of a single module. Unfurls Ast.
pub struct LintAstContext<'a> {
    /// The repository containing this module.
    pub repository: Arc<Repository>,
    /// The module being linted.
    pub module: &'a Module,
    /// The source revision for this lint pass.
    pub revision: Revision,
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

    /// Cached analysis results.
    pub analysis: LintAstAnalysisCache,

    /// Collected diagnostics.
    diagnostics: Vec<LintDiagnostic>,
}

impl<'a> std::fmt::Debug for LintAstContext<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintAstContext")
            .field("module_id", &self.module.id)
            .finish()
    }
}

#[allow(clippy::too_many_arguments)]
impl<'a> LintAstContext<'a> {
    /// Create a new AST lint context for a module.
    pub fn new(
        repository: Arc<Repository>,
        module: &'a Module,
        revision: Revision,
        file: Arc<File>,
        tree: &'a ast::NodeTree,
        parents: &'a ast::NodeParentIndex,
        roots: &'a Vec<ast::LocalNodeId<ast::Expression>>,
        strings: &'a StringPool,
        options: &'a LinterOptions,
        compute_fixes: bool,
    ) -> Self {
        Self {
            repository,
            module,
            revision,
            file,
            tree,
            parents,
            roots,
            strings,
            options,
            compute_fixes,
            analysis: LintAstAnalysisCache::default(),
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

    /// Return one module snapshot for the active revision when present.
    pub fn repository_module(&self, module_id: ModuleId) -> Option<Arc<Module>> {
        self.repository
            .module(self.revision, module_id)
            .ok()
            .flatten()
    }

    /// Return one file snapshot for the active revision when present.
    pub fn repository_file(&self, file_id: FileId) -> Option<Arc<File>> {
        self.repository.file(self.revision, file_id).ok().flatten()
    }

    /// Return one AST artifact for one revision-scoped module.
    pub fn module_ast(&self, module_id: ModuleId) -> Option<Arc<Ast>> {
        self.repository.ast(self.revision, module_id)
    }

    /// Return the linter options.
    pub fn options(&self) -> &LinterOptions {
        self.options
    }

    /// Resolve severity for a rule.
    pub fn get_severity(&self, meta: &LintMeta) -> LintSeverity {
        self.options.resolve_severity(
            meta.id,
            meta.category,
            meta.category.default_severity(),
            meta.is_recommended(),
            meta.is_strict(),
        )
    }

    /// Check if a requirement is met.
    pub fn is_requirement_met(&self, _requirement: &LintRequirement) -> bool {
        false // AST does not have lib symbols or well-known symbols
    }

    /// Check if a rule is supported.
    pub fn is_rule_supported(&self, meta: &LintMeta) -> bool {
        if !self.options.include_declaration_files && !meta.supports_file_type(self.file.ty) {
            return false;
        }

        // requires all
        if !meta.requires_all.is_empty() {
            for requirement in meta.requires_all {
                if !self.is_requirement_met(requirement) {
                    return false;
                }
            }
        }
        // requires any
        if !meta.requires_any.is_empty() {
            for requirement in meta.requires_any {
                if self.is_requirement_met(requirement) {
                    return true;
                }
            }

            return false;
        }

        true
    }

    /// Check if a rule is enabled.
    pub fn is_rule_enabled(&self, meta: &LintMeta) -> bool {
        self.get_severity(meta).is_enabled()
    }

    /// Get effective severity for a rule at a specific node.
    ///
    /// Checks for `@allow`/`@deny`/`@warn`/`@forbid` decorators on the node.
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
            for decorator_id in self.tree.get_decorators(id) {
                if let Some(override_info) = self.eat_decorator(decorator_id, meta) {
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

    /// Unwrap parenthesized decorator expressions.
    fn unwrap_decorator_expression(
        &self,
        expression_id: ast::LocalNodeId<ast::Expression>,
    ) -> ast::LocalNodeId<ast::Expression> {
        let mut current = expression_id;
        loop {
            let expression = self.tree.get(current);
            let Expression::Parenthesized { expression } = expression else {
                return current;
            };
            current = *expression;
        }
    }

    /// Resolve decorator call information for a decorator node.
    pub(crate) fn decorator_call(
        &self,
        decorator_id: ast::LocalNodeId<ast::Decorator>,
    ) -> DecoratorCall<'_> {
        let decorator = self.tree.get(decorator_id);
        let expression_id = self.unwrap_decorator_expression(decorator.expression);
        match self.tree.get(expression_id) {
            Expression::Call {
                left,
                dynamic_arguments,
                ..
            } => DecoratorCall {
                callee: self.unwrap_decorator_expression(*left),
                arguments: Some(dynamic_arguments.as_slice()),
            },
            _ => DecoratorCall {
                callee: expression_id,
                arguments: None,
            },
        }
    }

    /// Resolve the decorator path if the decorator is a reference-like expression.
    pub(crate) fn decorator_path(
        &self,
        decorator_id: ast::LocalNodeId<ast::Decorator>,
    ) -> Option<Vec<ast::StringId>> {
        let call = self.decorator_call(decorator_id);
        expression_path_segments(self.tree, call.callee)
    }

    /// Resolve the decorator name as a dot separated string.
    pub(crate) fn decorator_name(
        &self,
        decorator_id: ast::LocalNodeId<ast::Decorator>,
    ) -> Option<String> {
        let path = self.decorator_path(decorator_id)?;
        if path.is_empty() {
            return None;
        }

        let mut segments = Vec::new();
        for segment in path {
            segments.push(self.strings.get(segment).to_string());
        }
        Some(segments.join("."))
    }

    /// Parse a decorator annotation and return the severity override if it matches this lint.
    fn eat_decorator(
        &self,
        decorator_id: ast::LocalNodeId<Decorator>,
        meta: &LintMeta,
    ) -> Option<LintSeverityOverride> {
        // check decorator name (must be single segment: allow, warn, deny, forbid)
        let call = self.decorator_call(decorator_id);
        let path = expression_path_segments(self.tree, call.callee)?;
        if path.len() != 1 {
            return None;
        }

        let name = self.strings.get(path[0]);
        let (severity, is_forbidden) = match name.as_ref() {
            "allow" => (LintSeverity::Off, false),
            "warn" => (LintSeverity::Warning, false),
            "deny" => (LintSeverity::Error, false),
            "forbid" => (LintSeverity::Error, true),
            _ => return None,
        };

        // extract the string argument (lint ID or code)
        let arguments = call.arguments?;
        let first_argument = self.tree.get(*arguments.first()?);
        let Argument::Positional { value, .. } = first_argument else {
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
    pub fn get_span_text(&self, span: Span) -> &str {
        &self.file.text()[span.start as usize..span.end as usize]
    }

    /// Create an EditBuilder with source text for text-aware operations.
    pub fn edit_builder(&self) -> EditBuilder<'_> {
        EditBuilder::from_file(self.module.file_id, self.file.text())
    }

    /// Return a constant value if the expression can be evaluated.
    pub fn const_value(&mut self, id: ast::LocalNodeId<ast::Expression>) -> Option<ConstValue> {
        self.analysis.const_value(self.tree, id)
    }

    /// Return a constant boolean value if the expression can be evaluated.
    pub fn const_bool(&mut self, id: ast::LocalNodeId<ast::Expression>) -> Option<bool> {
        self.const_value(id).map(ConstValue::to_bool)
    }

    /// Return cached regex parse info for a pattern string.
    pub fn regex_parse(&mut self, id: ast::StringId) -> LintRegexParse {
        self.analysis.regex_parse(self.strings, id)
    }

    /// Return cached regex parse info for a pattern and optional flags.
    pub fn regex_parse_with_flags(
        &mut self,
        pattern_id: ast::StringId,
        flags_id: Option<ast::StringId>,
    ) -> LintRegexParse {
        self.analysis
            .regex_parse_with_flags(self.strings, pattern_id, flags_id)
    }

    /// Return a control character found in the pattern string.
    pub fn regex_control_character(&self, id: ast::StringId) -> Option<char> {
        let pattern = self.strings.get(id);
        find_control_character(pattern.as_ref())
    }

    /// Return control characters found in the pattern string.
    pub fn regex_control_characters(
        &self,
        pattern_id: ast::StringId,
        flags_id: Option<ast::StringId>,
    ) -> Vec<String> {
        let pattern = self.strings.get(pattern_id);
        let flags = flags_id.map(|id| self.strings.get(id));
        let flags = flags.as_ref().map(|value| value.as_ref());

        find_control_characters(pattern.as_ref(), flags)
    }

    /// Return a misleading character class description for the pattern string.
    pub fn regex_misleading_character_class(&self, id: ast::StringId) -> Option<&'static str> {
        let pattern = self.strings.get(id);
        find_misleading_character_class(pattern.as_ref())
    }

    /// Return a useless backreference description for the pattern string.
    pub fn regex_useless_backreference(
        &mut self,
        pattern_id: ast::StringId,
        flags_id: Option<ast::StringId>,
    ) -> Option<String> {
        let parse = self.regex_parse_with_flags(pattern_id, flags_id);
        let error_kind = parse.error.as_ref().map(|error| &error.kind);
        let pattern = self.strings.get(pattern_id);
        let flags = flags_id.map(|id| self.strings.get(id));
        let flags = flags.as_deref();

        find_useless_backreference(pattern.as_ref(), flags, error_kind)
    }
}

#[cfg(test)]
mod tests {
    use crate::linter::TestProgram;
    use crate::rules::suspicious::{NO_EMPTY, NoEmpty};

    #[test]
    fn test_allow_suppresses_by_id() {
        let test = TestProgram::for_rule_with_prelude(NoEmpty);
        let result = test.lint_ast(
            "ast/test_allow_suppresses_by_id.ds",
            r#"
@allow("no-empty")
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("no-empty");
    }

    #[test]
    fn test_allow_suppresses_by_code() {
        let test = TestProgram::for_rule_with_prelude(NoEmpty);
        let source = format!(
            r#"
@allow("{code}")
function foo() {{}}
"#,
            code = NO_EMPTY.code
        );
        let result = test.lint_ast("ast/test_allow_suppresses_by_code.ds", source.as_str());
        test.result(result).assert_no_lint("no-empty");
    }

    #[test]
    fn test_allow_on_block() {
        let test = TestProgram::for_rule_with_prelude(NoEmpty);
        let result = test.lint_ast(
            "ast/test_allow_on_block.ds",
            r#"
@allow("no-empty")
{}
"#,
        );
        test.result(result).assert_no_lint("no-empty");
    }

    #[test]
    fn test_allow_does_not_affect_other_lints() {
        let test = TestProgram::for_rule_with_prelude(NoEmpty);
        let result = test.lint_ast(
            "ast/test_allow_does_not_affect_other_lints.ds",
            r#"
@allow("some-other-lint")
function foo() {
    if (ready) {}
}
"#,
        );
        test.result(result).assert_lint("no-empty");
    }

    #[test]
    fn test_forbid_prevents_inner_allow() {
        let test = TestProgram::for_rule_with_prelude(NoEmpty);
        let result = test.lint_ast(
            "ast/test_forbid_prevents_inner_allow.ds",
            r#"
@forbid("no-empty")
function outer() {
    @allow("no-empty")
    function inner() {
        if (ready) {}
    }
}
"#,
        );
        // inner @allow should be ignored due to outer @forbid
        test.result(result).assert_lint("no-empty");
    }

    #[test]
    fn test_warn_changes_severity() {
        let test = TestProgram::for_rule_with_prelude(NoEmpty);
        let result = test.lint_ast(
            "ast/test_warn_changes_severity.ds",
            r#"
@warn("no-empty")
function foo() {
    if (ready) {}
}
"#,
        );
        // should still lint but with warning severity
        test.result(result).assert_lint("no-empty");
    }

    #[test]
    fn test_deny_changes_severity() {
        let test = TestProgram::for_rule_with_prelude(NoEmpty);
        let result = test.lint_ast(
            "ast/test_deny_changes_severity.ds",
            r#"
@deny("no-empty")
function foo() {
    if (ready) {}
}
"#,
        );
        // should still lint with error severity
        test.result(result).assert_lint("no-empty");
    }
}
