use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLibSymbol;
use crate::rules::common::{
    expression_is_symbol_or_global_qualified_member, expression_static_property_access,
    span_has_comment,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer `**` over `Math.pow()`.
    ///
    /// The exponentiation operator `**` is more concise and readable than
    /// `Math.pow()`. It also handles BigInt values correctly.
    #[lint(
        id = "prefer-exponentiation-operator",
        code = "LY036",
        category = Style,
        level = Dir,
        requires_all = [RequireLibSymbol("Math", &[])],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferExponentiationOperator,
    "Prefer ** over Math.pow()"
}

impl LintRule for PreferExponentiationOperator {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferExponentiationOperator::meta()
    }

    /// Check module DIR nodes for Math.pow usage.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = ExponentiationVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags Math.pow usage.
struct ExponentiationVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The Math symbol for this module.
    math_symbol: dir::GlobalSymbolId,
    /// The Math member name.
    math_name: StringId,
    /// The pow member name.
    pow_name: StringId,
    /// The global qualifier symbols.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> ExponentiationVisitor<'a, 'b> {
    /// Build a visitor for prefer-exponentiation-operator checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let math_name = ctx.string_id("Math");
        let pow_name = ctx.string_id("pow");
        let math_symbol = ctx.declared_library_symbol(math_name);
        let global_qualifiers = ctx.global_qualifier_symbols();

        Self {
            ctx,
            meta,
            math_symbol,
            math_name,
            pow_name,
            global_qualifiers,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a call expression for Math.pow usage.
    fn check_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // match Math.pow calls
        if !self.is_math_pow(left) {
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
            PREFER_EXPONENTIATION_OPERATOR.id,
            PREFER_EXPONENTIATION_OPERATOR.code,
            PREFER_EXPONENTIATION_OPERATOR.category,
            severity,
            "prefer ** operator over Math.pow()",
            self.ctx.module.file_id,
            span,
        )
        .with_label("use base ** exponent instead");
        if let Some(fix) = self.math_pow_fix(expression_id, generic_arguments, arguments) {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Build a safe fix from `Math.pow(base, exponent)` to `base ** exponent`.
    fn math_pow_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> Option<LintFix> {
        // skip static arguments until we support rendering them
        if !generic_arguments.is_empty() {
            return None;
        }

        // require exactly two positional arguments
        let [base_argument_id, exponent_argument_id] = arguments else {
            return None;
        };
        let base_argument = self.ctx.tree.get(*base_argument_id);
        let exponent_argument = self.ctx.tree.get(*exponent_argument_id);
        let dir::Argument::Positional { value: base_id, .. } = base_argument else {
            return None;
        };
        let dir::Argument::Positional {
            value: exponent_id, ..
        } = exponent_argument
        else {
            return None;
        };

        // preserve source text and parenthesize the whole replacement for precedence safety
        let base_span = self.ctx.get_span(*base_id);
        let base_text = self.ctx.get_span_text(base_span);
        let exponent_span = self.ctx.get_span(*exponent_id);
        let exponent_text = self.ctx.get_span_text(exponent_span);
        let replacement = format!("(({base_text}) ** ({exponent_text}))");

        // replace the full call expression
        let expression_span = self.ctx.get_span(expression_id);
        if span_has_comment(self.ctx.ast, expression_span) {
            return None;
        }

        let edits = self
            .ctx
            .edit_builder()
            .replace(expression_span, replacement)
            .into_edits();

        Some(LintFix::safe("Replace Math.pow() with **").with_edits(edits))
    }

    /// Return true when the expression is Math.pow.
    fn is_math_pow(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // match static property access with `pow`
        let Some((receiver_id, property_name)) =
            expression_static_property_access(self.ctx.tree, expression_id)
        else {
            return false;
        };
        if property_name != self.pow_name {
            return false;
        }

        self.is_math_object(receiver_id)
    }

    /// Return true when the expression is a Math object reference.
    fn is_math_object(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        expression_is_symbol_or_global_qualified_member(
            self.ctx.tree,
            expression_id,
            self.math_symbol,
            &self.global_qualifiers,
            self.math_name,
        )
    }
}

impl NodeVisitor for ExponentiationVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check call expressions for Math.pow
        if let dir::Expression::Call {
            left,
            generic_arguments,
            arguments,
        } = expression
        {
            self.check_call(id, *left, generic_arguments.as_slice(), arguments);
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
    fn test_flags_math_pow() {
        let test = TestProgram::for_rule_with_prelude(PreferExponentiationOperator);
        let result = test.lint_dir(
            "prefer_exponentiation_operator/test_flags_math_pow.ds",
            r#"
let value = Math.pow(2, 3);
"#,
        );
        test.result(result)
            .assert_lint("prefer-exponentiation-operator");
    }

    #[test]
    fn test_flags_global_math_pow() {
        let test = TestProgram::for_rule_with_prelude(PreferExponentiationOperator);
        let result = test.lint_dir(
            "prefer_exponentiation_operator/test_flags_global_math_pow.ds",
            r#"
let value = globalThis.Math.pow(2, 3);
"#,
        );
        test.result(result)
            .assert_lint("prefer-exponentiation-operator");
    }

    #[test]
    fn test_flags_computed_math_pow() {
        let test = TestProgram::for_rule_with_prelude(PreferExponentiationOperator);
        let result = test.lint_dir(
            "prefer_exponentiation_operator/test_flags_computed_math_pow.ds",
            r#"
let value = Math["pow"](2, 3);
"#,
        );
        test.result(result)
            .assert_lint("prefer-exponentiation-operator")
            .assert_has_fix("prefer-exponentiation-operator");
    }

    #[test]
    fn test_flags_computed_global_math_pow() {
        let test = TestProgram::for_rule_with_prelude(PreferExponentiationOperator);
        let result = test.lint_dir(
            "prefer_exponentiation_operator/test_flags_computed_global_math_pow.ds",
            r#"
let value = globalThis.Math[`pow`](2, 3);
"#,
        );
        test.result(result)
            .assert_lint("prefer-exponentiation-operator")
            .assert_has_fix("prefer-exponentiation-operator");
    }

    #[test]
    fn test_allows_shadowed_math_pow() {
        let test = TestProgram::for_rule_with_prelude(PreferExponentiationOperator);
        let result = test.lint_dir(
            "prefer_exponentiation_operator/test_allows_shadowed_math_pow.ds",
            r#"
let Math = { pow: (x: number, y: number): number => x + y };
let value = Math.pow(2, 3);
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-exponentiation-operator");
    }

    #[test]
    fn test_allows_exponentiation_operator() {
        let test = TestProgram::for_rule_with_prelude(PreferExponentiationOperator);
        let result = test.lint_dir(
            "prefer_exponentiation_operator/test_allows_exponentiation_operator.ds",
            r#"
let value = 2 ** 3;
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-exponentiation-operator");
    }

    #[test]
    fn test_allows_other_math_call() {
        let test = TestProgram::for_rule_with_prelude(PreferExponentiationOperator);
        let result = test.lint_dir(
            "prefer_exponentiation_operator/test_allows_other_math_call.ds",
            r#"
let value = Math.max(1, 2);
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-exponentiation-operator");
    }

    #[test]
    fn test_fix_math_pow() {
        let test = TestProgram::for_rule_with_prelude(PreferExponentiationOperator);
        let result = test.lint_dir(
            "prefer_exponentiation_operator/test_fix_math_pow.ds",
            r#"
let value = Math.pow(2, 3);
"#,
        );
        test.result(result)
            .assert_lint("prefer-exponentiation-operator")
            .assert_has_fix("prefer-exponentiation-operator")
            .assert_safe_fixed(
                r#"
let value = ((2) ** (3));
"#,
            );
    }

    #[test]
    fn test_no_fix_math_pow_single_argument() {
        let test = TestProgram::for_rule_with_prelude(PreferExponentiationOperator);
        let result = test.lint_dir(
            "prefer_exponentiation_operator/test_no_fix_math_pow_single_argument.ds",
            r#"
let value = Math.pow(2);
"#,
        );
        test.result(result)
            .assert_lint("prefer-exponentiation-operator")
            .assert_has_no_fix("prefer-exponentiation-operator");
    }

    #[test]
    fn test_no_fix_math_pow_spread_argument() {
        let test = TestProgram::for_rule_with_prelude(PreferExponentiationOperator);
        let result = test.lint_dir(
            "prefer_exponentiation_operator/test_no_fix_math_pow_spread_argument.ds",
            r#"
let values = [2, 3];
let value = Math.pow(...values);
"#,
        );
        test.result(result)
            .assert_lint("prefer-exponentiation-operator")
            .assert_has_no_fix("prefer-exponentiation-operator");
    }

    #[test]
    fn test_fix_wraps_unary_parent_context() {
        let test = TestProgram::for_rule_with_prelude(PreferExponentiationOperator);
        let result = test.lint_dir(
            "prefer_exponentiation_operator/test_fix_wraps_unary_parent_context.ds",
            r#"
let value = +Math.pow(a, b);
"#,
        );
        test.result(result)
            .assert_lint("prefer-exponentiation-operator")
            .assert_has_fix("prefer-exponentiation-operator")
            .assert_safe_fixed(
                r#"
let value = +((a) ** (b));
"#,
            );
    }

    #[test]
    fn test_no_fix_math_pow_with_comments() {
        let test = TestProgram::for_rule_with_prelude(PreferExponentiationOperator);
        let result = test.lint_dir(
            "prefer_exponentiation_operator/test_no_fix_math_pow_with_comments.ds",
            r#"
let value = Math.pow(
  /* base */ a,
  b,
);
"#,
        );
        test.result(result)
            .assert_lint("prefer-exponentiation-operator")
            .assert_has_no_fix("prefer-exponentiation-operator");
    }
}
