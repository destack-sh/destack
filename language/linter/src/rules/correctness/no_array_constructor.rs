use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    CallLikeExpressionInfo, expression_call_like, expression_target_symbol,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow using the Array constructor.
    ///
    /// Array constructors are confusing because single argument calls
    /// create sparse arrays instead of arrays with values.
    #[lint(
        id = "no-array-constructor",
        code = "LC003",
        category = Correctness,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoArrayConstructor,
    "Disallow Array constructor usage"
}

impl LintRule for NoArrayConstructor {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoArrayConstructor::meta()
    }

    /// Check module DIR nodes for Array constructor calls.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        // resolve lint metadata
        let meta = self.meta();

        // walk the module for array constructor calls
        let mut visitor = ArrayConstructorVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags Array constructor calls.
struct ArrayConstructorVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Array symbol for this module.
    array_symbol: dir::GlobalSymbolId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> ArrayConstructorVisitor<'a, 'b> {
    /// Build a visitor for Array constructor checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.well_known_symbol(WellKnownSymbol::Array);
        Self {
            ctx,
            meta,
            array_symbol,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        // capture roots and tree references
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        // walk the module expression tree
        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a call or constructor for Array usage.
    fn check_array_constructor(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        call_like: CallLikeExpressionInfo<'_>,
    ) {
        // ignore non array references
        let Some(target_symbol) = expression_target_symbol(self.ctx.tree, call_like.left) else {
            return;
        };
        if target_symbol != self.array_symbol {
            return;
        }

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // build diagnostic and attach fix when safe
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintDiagnostic::new(
            NO_ARRAY_CONSTRUCTOR.id,
            NO_ARRAY_CONSTRUCTOR.code,
            NO_ARRAY_CONSTRUCTOR.category,
            severity,
            "avoid using the Array constructor",
            self.ctx.module.file_id,
            span,
        )
        .with_label(format!(
            "replace this {} with an array literal",
            if call_like.is_new {
                "constructor"
            } else {
                "call"
            }
        ));
        if self.ctx.include_fixes
            && let Some(fix) = self.array_constructor_fix(expression_id, call_like)
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Build a safe fix from an Array constructor to a literal.
    fn array_constructor_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        call_like: CallLikeExpressionInfo<'_>,
    ) -> Option<LintFix> {
        // skip static arguments until we support rendering them
        if !call_like.generic_arguments.is_empty() {
            return None;
        }

        // skip single non spread constructors: `Array(3)` is not `[3]`
        if call_like.dynamic_arguments.len() == 1 {
            let argument = self.ctx.tree.get(call_like.dynamic_arguments[0]);
            if !matches!(argument, dir::Argument::Spread { .. }) {
                return None;
            }
        }

        // collect positional and spread arguments in order
        let mut elements = Vec::new();
        for argument_id in call_like.dynamic_arguments {
            let argument = self.ctx.tree.get(*argument_id);
            let element = match argument {
                dir::Argument::Positional { value, .. } => {
                    let value_span = self.ctx.get_span(*value);
                    let value_text = self.ctx.get_span_text(value_span);
                    value_text.to_string()
                }
                dir::Argument::Spread { value, .. } => {
                    let value_span = self.ctx.get_span(*value);
                    let value_text = self.ctx.get_span_text(value_span);
                    format!("...{value_text}")
                }
                _ => return None,
            };
            elements.push(element);
        }

        // build literal replacement
        let replacement = if elements.is_empty() {
            "[]".to_string()
        } else {
            format!("[{}]", elements.join(", "))
        };

        // replace the full constructor expression
        let expression_span = self.ctx.get_span(expression_id);
        let edits = self
            .ctx
            .edit_builder()
            .replace(expression_span, replacement)
            .into_edits();

        Some(LintFix::safe("Replace Array constructor with array literal").with_edits(edits))
    }
}

impl NodeVisitor for ArrayConstructorVisitor<'_, '_> {
    /// Return visitor options.
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    /// Visit an expression node.
    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check for array constructor calls
        if let Some(call_like) = expression_call_like(expression) {
            self.check_array_constructor(id, call_like);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_array_constructor_call() {
        let test = TestProgram::for_rule_with_prelude(NoArrayConstructor);
        let result = test.lint_dir(
            "no_array_constructor/test_flags_array_constructor_call.ds",
            r#"
let items = Array(1, 2);
"#,
        );
        test.result(result).assert_lint("no-array-constructor");
    }

    #[test]
    fn test_flags_array_constructor_new() {
        let test = TestProgram::for_rule_with_prelude(NoArrayConstructor);
        let result = test.lint_dir(
            "no_array_constructor/test_flags_array_constructor_new.ds",
            r#"
let items = new Array(1);
"#,
        );
        test.result(result)
            .assert_lint("no-array-constructor")
            .assert_has_no_fix("no-array-constructor");
    }

    #[test]
    fn test_allows_array_literal() {
        let test = TestProgram::for_rule_with_prelude(NoArrayConstructor);
        let result = test.lint_dir(
            "no_array_constructor/test_allows_array_literal.ds",
            r#"
let items = [1, 2];
"#,
        );
        test.result(result).assert_no_lint("no-array-constructor");
    }

    #[test]
    fn test_allows_shadowed_array() {
        let test = TestProgram::for_rule_with_prelude(NoArrayConstructor);
        let result = test.lint_dir(
            "no_array_constructor/test_allows_shadowed_array.ds",
            r#"
let Array = (value: number): number => value;
let item = Array(1);
"#,
        );
        test.result(result).assert_no_lint("no-array-constructor");
    }

    #[test]
    fn test_allows_shadowed_array_in_function() {
        let test = TestProgram::for_rule_with_prelude(NoArrayConstructor);
        let result = test.lint_dir(
            "no_array_constructor/test_allows_shadowed_array_in_function.ds",
            r#"
let build = (Array: (value: number) => number): number => {
    return Array(1);
};
"#,
        );
        test.result(result).assert_no_lint("no-array-constructor");
    }

    #[test]
    fn test_fix_array_constructor_call_with_multiple_arguments() {
        let test = TestProgram::for_rule_with_prelude(NoArrayConstructor);
        let result = test.lint_dir(
            "no_array_constructor/test_fix_array_constructor_call_with_multiple_arguments.ds",
            r#"
let items = Array(1, 2, 3);
"#,
        );
        test.result(result)
            .assert_lint("no-array-constructor")
            .assert_has_fix("no-array-constructor")
            .assert_safe_fixed(
                r#"
let items = [1, 2, 3];
"#,
            );
    }

    #[test]
    fn test_fix_array_constructor_call_with_no_arguments() {
        let test = TestProgram::for_rule_with_prelude(NoArrayConstructor);
        let result = test.lint_dir(
            "no_array_constructor/test_fix_array_constructor_call_with_no_arguments.ds",
            r#"
let items = Array();
"#,
        );
        test.result(result)
            .assert_lint("no-array-constructor")
            .assert_has_fix("no-array-constructor")
            .assert_safe_fixed(
                r#"
let items = [];
"#,
            );
    }

    #[test]
    fn test_fix_array_constructor_new_with_no_arguments() {
        let test = TestProgram::for_rule_with_prelude(NoArrayConstructor);
        let result = test.lint_dir(
            "no_array_constructor/test_fix_array_constructor_new_with_no_arguments.ds",
            r#"
let items = new Array();
"#,
        );
        test.result(result)
            .assert_lint("no-array-constructor")
            .assert_has_fix("no-array-constructor")
            .assert_safe_fixed(
                r#"
let items = [];
"#,
            );
    }

    #[test]
    fn test_fix_array_constructor_new_without_parentheses() {
        let test = TestProgram::for_rule_with_prelude(NoArrayConstructor);
        let result = test.lint_dir(
            "no_array_constructor/test_fix_array_constructor_new_without_parentheses.ds",
            r#"
let items = new Array;
"#,
        );
        test.result(result)
            .assert_lint("no-array-constructor")
            .assert_has_fix("no-array-constructor")
            .assert_safe_fixed(
                r#"
let items = [];
"#,
            );
    }

    #[test]
    fn test_flags_array_constructor_single_argument() {
        let test = TestProgram::for_rule_with_prelude(NoArrayConstructor);
        let result = test.lint_dir(
            "no_array_constructor/test_flags_array_constructor_single_argument.ds",
            r#"
let items = Array(3);
"#,
        );
        test.result(result)
            .assert_lint("no-array-constructor")
            .assert_has_no_fix("no-array-constructor");
    }

    #[test]
    fn test_fix_array_constructor_single_spread_argument() {
        let test = TestProgram::for_rule_with_prelude(NoArrayConstructor);
        let result = test.lint_dir(
            "no_array_constructor/test_fix_array_constructor_single_spread_argument.ds",
            r#"
let args = [1, 2, 3];
let items = Array(...args);
"#,
        );
        test.result(result)
            .assert_lint("no-array-constructor")
            .assert_has_fix("no-array-constructor")
            .assert_safe_fixed(
                r#"
let args = [1, 2, 3];
let items = [...args];
"#,
            );
    }

    #[test]
    fn test_fix_array_constructor_mixed_arguments_with_spread() {
        let test = TestProgram::for_rule_with_prelude(NoArrayConstructor);
        let result = test.lint_dir(
            "no_array_constructor/test_fix_array_constructor_mixed_arguments_with_spread.ds",
            r#"
let args = [2, 3];
let items = new Array(1, ...args);
"#,
        );
        test.result(result)
            .assert_lint("no-array-constructor")
            .assert_has_fix("no-array-constructor")
            .assert_safe_fixed(
                r#"
let args = [2, 3];
let items = [1, ...args];
"#,
            );
    }
}
