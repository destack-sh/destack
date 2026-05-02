use destack_ast::{
    self as ast, Expression, LocalNodeId, NodeVisitor, NodeVisitorOptions, Tree, walk_expression,
};
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintMeta, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow control flow statements in finally blocks.
    ///
    /// Using `return`, `throw`, `break`, or `continue` in a `finally` block can cause
    /// unexpected behavior by overriding the return value or exception from the try/catch.
    #[lint(
        id = "no-unsafe-finally",
        code = "LC033",
        category = Correctness,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoUnsafeFinally,
    "Disallow control flow in finally blocks"
}

impl LintRule for NoUnsafeFinally {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoUnsafeFinally::meta()
    }

    /// Check module AST nodes for unsafe control flow inside finally blocks.
    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect try expressions that include finally blocks
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Try {
                finally_expression: Some(finally_id),
                ..
            } = ctx.tree.get(node_id)
            else {
                continue;
            };

            // resolve effective lint severity
            let severity = ctx.get_effective_severity(meta, node_id);

            // skip disabled diagnostics
            if !severity.is_enabled() {
                continue;
            }

            // check for unsafe control flow in the finally block
            let mut visitor = FinallyVisitor {
                options: NodeVisitorOptions::default(),
                severity,
                diagnostics: Vec::new(),
                breakable_scope_depth: 0,
                continuable_scope_depth: 0,
                labels: Vec::new(),
            };

            // resolve finally expression
            let finally_expression = ctx.tree.get(*finally_id);
            visitor.visit_expression(ctx.tree, *finally_id, finally_expression);

            // report all unsafe control-flow diagnostics from this finally traversal
            for diagnostic in visitor.diagnostics {
                ctx.report(diagnostic);
            }
        }
    }
}

/// Visitor that detects unsafe control-flow exits within one finally traversal.
struct FinallyVisitor {
    /// The visitor options.
    options: NodeVisitorOptions,
    /// The lint severity.
    severity: LintSeverity,
    /// Collected diagnostics.
    diagnostics: Vec<LintReport>,
    /// The count of breakable scopes entered within the finally traversal.
    breakable_scope_depth: usize,
    /// The count of continuable loop scopes entered within the finally traversal.
    continuable_scope_depth: usize,
    /// Label targets declared within the finally traversal.
    labels: Vec<LabelScope>,
}

/// One labeled scope entered while walking a finally block.
#[derive(Clone, Copy)]
struct LabelScope {
    /// The declared label.
    name: ast::StringId,
    /// Whether `continue <label>` is valid for this label.
    can_continue: bool,
}

impl FinallyVisitor {
    /// Return true when one break target is inside the current finally traversal.
    fn break_is_local_target(&self, label: Option<ast::StringId>) -> bool {
        // unlabeled breaks target the nearest breakable scope
        let Some(label) = label else {
            return self.breakable_scope_depth > 0;
        };

        // labeled breaks are local only when label exists in this finally traversal
        self.labels.iter().rev().any(|scope| scope.name == label)
    }

    /// Return true when one continue target is inside the current finally traversal.
    fn continue_is_local_target(&self, label: Option<ast::StringId>) -> bool {
        // unlabeled continues target the nearest loop scope
        let Some(label) = label else {
            return self.continuable_scope_depth > 0;
        };

        // labeled continues are local only for in finally loop labels
        self.labels
            .iter()
            .rev()
            .any(|scope| scope.name == label && scope.can_continue)
    }

    /// Report one unsafe control-flow expression in a finally block.
    fn report_unsafe(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Expression>,
        message: &'static str,
        label: &'static str,
    ) {
        self.diagnostics.push(
            LintReport::new(
                NO_UNSAFE_FINALLY.id,
                NO_UNSAFE_FINALLY.code,
                NO_UNSAFE_FINALLY.category,
                self.severity,
                message,
                tree.get_span(id),
            )
            .label(label),
        );
    }

    /// Return true when one expression is a loop that can be a continue target.
    fn expression_is_loop_target(expression: &Expression) -> bool {
        matches!(
            expression,
            Expression::While { .. }
                | Expression::ForEach { .. }
                | Expression::For { .. }
                | Expression::Loop { .. }
        )
    }
}

impl NodeVisitor for FinallyVisitor {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // don't traverse declaration bodies, as their control flow is local
        if matches!(expression, Expression::Declaration(..)) {
            return;
        }

        // enter local breakable and continuable scopes
        let mut entered_break_scope = false;
        let mut entered_continuable_scope = false;
        let mut entered_label = false;

        // label scopes are local break targets and may be local continue targets for loops
        if let Expression::Labelled { label, body } = expression {
            let body_expression = tree.get(*body);
            let can_continue = Self::expression_is_loop_target(body_expression);

            self.labels.push(LabelScope {
                name: *label,
                can_continue,
            });
            entered_label = true;
        }

        // any loop is both breakable and continuable
        if matches!(
            expression,
            Expression::While { .. }
                | Expression::ForEach { .. }
                | Expression::For { .. }
                | Expression::Loop { .. }
        ) {
            self.breakable_scope_depth += 1;
            entered_break_scope = true;

            self.continuable_scope_depth += 1;
            entered_continuable_scope = true;
        }
        // match is breakable like switch in JavaScript
        else if matches!(expression, Expression::Match { .. }) {
            self.breakable_scope_depth += 1;
            entered_break_scope = true;
        }

        // return in finally is unsafe
        if matches!(expression, Expression::Return { .. }) {
            self.report_unsafe(
                tree,
                id,
                "`return` in finally block",
                "this may override a thrown exception",
            );
        }

        // throw in finally is unsafe
        if matches!(expression, Expression::Throw { .. }) {
            self.report_unsafe(
                tree,
                id,
                "`throw` in finally block",
                "this may override a thrown exception",
            );
        }

        // break is unsafe only when the target is outside this finally
        if let Expression::Break { label, .. } = expression
            && !self.break_is_local_target(*label)
        {
            self.report_unsafe(
                tree,
                id,
                "`break` in finally block",
                "this may disrupt expected control flow",
            );
        }

        // continue is unsafe only when the target is outside this finally
        if let Expression::Continue { label } = expression
            && !self.continue_is_local_target(*label)
        {
            self.report_unsafe(
                tree,
                id,
                "`continue` in finally block",
                "this may disrupt expected control flow",
            );
        }

        // walk child expressions in this subtree
        walk_expression(self, tree, id, expression);

        // restore traversal scopes after children
        // leave one label scope
        if entered_label {
            self.labels.pop();
        }

        // leave one continuable scope
        if entered_continuable_scope {
            self.continuable_scope_depth -= 1;
        }

        // leave one breakable scope
        if entered_break_scope {
            self.breakable_scope_depth -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_return_in_finally() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeFinally);
        let result = test.lint_ast(
            "no_unsafe_finally/test_detects_return_in_finally.ds",
            r#"
function foo() {
    try {
        throw "error";
    } finally {
        return 1;
    }
}
"#,
        );
        test.result(result).assert_lint("no-unsafe-finally");
    }

    #[test]
    fn test_detects_throw_in_finally() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeFinally);
        let result = test.lint_ast(
            "no_unsafe_finally/test_detects_throw_in_finally.ds",
            r#"
function foo() {
    try {
        throw "error1";
    } finally {
        throw "error2";
    }
}
"#,
        );
        test.result(result).assert_lint("no-unsafe-finally");
    }

    #[test]
    fn test_detects_break_in_finally() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeFinally);
        let result = test.lint_ast(
            "no_unsafe_finally/test_detects_break_in_finally.ds",
            r#"
while (true) {
    try {
        throw "error";
    } finally {
        break;
    }
}
"#,
        );
        test.result(result).assert_lint("no-unsafe-finally");
    }

    #[test]
    fn test_detects_continue_in_finally() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeFinally);
        let result = test.lint_ast(
            "no_unsafe_finally/test_detects_continue_in_finally.ds",
            r#"
while (true) {
    try {
        throw "error";
    } finally {
        continue;
    }
}
"#,
        );
        test.result(result).assert_lint("no-unsafe-finally");
    }

    #[test]
    fn test_allows_return_in_try() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeFinally);
        let result = test.lint_ast(
            "no_unsafe_finally/test_allows_return_in_try.ds",
            r#"
function foo() {
    try {
        return 1;
    } finally {
        console.log("cleanup");
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-unsafe-finally");
    }

    #[test]
    fn test_allows_return_in_nested_function() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeFinally);
        let result = test.lint_ast(
            "no_unsafe_finally/test_allows_return_in_nested_function.ds",
            r#"
function foo() {
    try {
        throw "error";
    } finally {
        const inner = () => {
            return 1;
        };
        inner();
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-unsafe-finally");
    }

    #[test]
    fn test_allows_return_in_nested_class_method() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeFinally);
        let result = test.lint_ast(
            "no_unsafe_finally/test_allows_return_in_nested_class_method.ds",
            r#"
function foo() {
    try {
        throw "error";
    } finally {
        class Helper {
            run() {
                return 1;
            }
        }
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-unsafe-finally");
    }

    #[test]
    fn test_allows_break_inside_finally_loop() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeFinally);
        let result = test.lint_ast(
            "no_unsafe_finally/test_allows_break_inside_finally_loop.ds",
            r#"
function foo() {
    try {
        throw "error";
    } finally {
        while (true) {
            break;
        }
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-unsafe-finally");
    }

    #[test]
    fn test_allows_continue_inside_finally_loop() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeFinally);
        let result = test.lint_ast(
            "no_unsafe_finally/test_allows_continue_inside_finally_loop.ds",
            r#"
function foo() {
    try {
        throw "error";
    } finally {
        for (;;) {
            continue;
        }
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-unsafe-finally");
    }

    #[test]
    fn test_flags_break_to_outer_label_from_finally() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeFinally);
        let result = test.lint_ast(
            "no_unsafe_finally/test_flags_break_to_outer_label_from_finally.ds",
            r#"
outer: while (true) {
    try {
        throw "error";
    } finally {
        break outer;
    }
}
"#,
        );
        test.result(result).assert_lint("no-unsafe-finally");
    }

    #[test]
    fn test_allows_break_to_inner_label_in_finally() {
        let test = TestProgram::for_rule_without_prelude(NoUnsafeFinally);
        let result = test.lint_ast(
            "no_unsafe_finally/test_allows_break_to_inner_label_in_finally.ds",
            r#"
function foo() {
    try {
        throw "error";
    } finally {
        inner: while (true) {
            break inner;
        }
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-unsafe-finally");
    }
}
