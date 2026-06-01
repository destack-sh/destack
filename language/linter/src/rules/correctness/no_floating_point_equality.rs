use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_method_call, expression_unwrap_parenthesized, is_float_type,
};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow direct `==` comparison of floats.
    ///
    /// Floating-point arithmetic can produce unexpected results due to
    /// representation errors. Direct equality comparisons are almost never
    /// correct. Use an epsilon-based comparison instead.
    #[lint(
        id = "no-floating-point-equality",
        code = "LC015",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoFloatingPointEquality,
    "Disallow direct == comparison of floats"
}

impl LintRule for NoFloatingPointEquality {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoFloatingPointEquality::meta()
    }

    /// Check module DIR nodes for float equality comparisons.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let mut visitor = FloatEqualityVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags direct equality comparisons of floats.
struct FloatEqualityVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The member name `signum`.
    signum_name: destack_dir::StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> FloatEqualityVisitor<'a, 'b> {
    /// Build a visitor for float equality checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        let signum_name = ctx.string_id("signum");
        Self {
            ctx,
            meta,
            signum_name,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.dir.tree();

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a binary expression for float equality comparison.
    fn check_float_equality(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) {
        // resolve the left operand type
        let left_is_float = self
            .ctx
            .expression_type_id(left)
            .is_some_and(|type_id| is_float_type(self.ctx, type_id));

        // resolve the right operand type
        let right_is_float = self
            .ctx
            .expression_type_id(right)
            .is_some_and(|type_id| is_float_type(self.ctx, type_id));

        // at least one operand must be a float
        if !left_is_float && !right_is_float {
            return;
        }

        // allow exact comparisons against stable constants like zero and infinity
        if self.expression_is_allowed_float_constant(left)
            || self.expression_is_allowed_float_constant(right)
        {
            return;
        }

        // allow signum result comparisons
        if self.expression_is_signum_result(left) && self.expression_is_signum_result(right) {
            return;
        }

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintReport::new(
                NO_FLOATING_POINT_EQUALITY.id,
                NO_FLOATING_POINT_EQUALITY.code,
                NO_FLOATING_POINT_EQUALITY.category,
                severity,
                "avoid direct equality comparison of floating-point numbers",
                span,
            )
            .label("use an epsilon-based comparison instead"),
        );
    }

    /// Return true when an expression is an allowed exact float constant.
    fn expression_is_allowed_float_constant(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let Some(constant_value) = self.ctx.const_value(expression_id) else {
            return false;
        };

        match constant_value {
            crate::ConstValue::Float(value) => value == 0.0 || value.is_infinite(),
            crate::ConstValue::Integer(value) | crate::ConstValue::Bigint(value) => value == 0,
            _ => false,
        }
    }

    /// Return true when an expression is the result of `signum()` on a float.
    fn expression_is_signum_result(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let expression_id = expression_unwrap_parenthesized(self.ctx.dir.tree(), expression_id);
        let expression = self.ctx.dir.get(expression_id);

        // unary negation preserves signum semantics
        if let dir::Expression::Unary {
            operator: dir::UnaryOperator::Negate,
            right,
        } = expression
        {
            return self.expression_is_signum_result(*right);
        }

        let Some(method_call) = expression_method_call(self.ctx.dir.tree(), expression_id) else {
            return false;
        };
        if method_call.method_name != self.signum_name {
            return false;
        }

        let Some(receiver_type_id) = self.ctx.expression_type_id(method_call.receiver_id) else {
            return false;
        };

        is_float_type(self.ctx, receiver_type_id)
    }
}

impl NodeVisitor for FloatEqualityVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check equality comparisons
        if let dir::Expression::Binary {
            operator: dir::BinaryOperator::Equal | dir::BinaryOperator::NotEqual,
            left,
            right,
        } = expression
        {
            self.check_float_equality(id, *left, *right);
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
    fn test_flags_float_equality() {
        let test = TestProgram::for_rule_without_prelude(NoFloatingPointEquality);
        let result = test.lint_dir(
            "no_floating_point_equality/test_flags_float_equality.ds",
            r#"
let a: float64 = 0.1 + 0.2;
let b: float64 = 0.3;
let equal = a == b;
"#,
        );
        test.result(result)
            .assert_lint("no-floating-point-equality");
    }

    #[test]
    fn test_flags_float_not_equal() {
        let test = TestProgram::for_rule_without_prelude(NoFloatingPointEquality);
        let result = test.lint_dir(
            "no_floating_point_equality/test_flags_float_not_equal.ds",
            r#"
let a: float64 = 1.0;
let b: float64 = 2.0;
let notEqual = a != b;
"#,
        );
        test.result(result)
            .assert_lint("no-floating-point-equality");
    }

    #[test]
    fn test_flags_number_equality() {
        let test = TestProgram::for_rule_without_prelude(NoFloatingPointEquality);
        let result = test.lint_dir(
            "no_floating_point_equality/test_flags_number_equality.ds",
            r#"
let a: number = 0.1 + 0.2;
let b: number = 0.3;
let equal = a == b;
"#,
        );
        test.result(result)
            .assert_lint("no-floating-point-equality");
    }

    #[test]
    fn test_allows_integer_equality() {
        let test = TestProgram::for_rule_without_prelude(NoFloatingPointEquality);
        let result = test.lint_dir(
            "no_floating_point_equality/test_allows_integer_equality.ds",
            r#"
let a: int32 = 1;
let b: int32 = 2;
let equal = a == b;
"#,
        );
        test.result(result)
            .assert_no_lint("no-floating-point-equality");
    }

    #[test]
    fn test_allows_string_equality() {
        let test = TestProgram::for_rule_without_prelude(NoFloatingPointEquality);
        let result = test.lint_dir(
            "no_floating_point_equality/test_allows_string_equality.ds",
            r#"
let a = "hello";
let b = "world";
let equal = a == b;
"#,
        );
        test.result(result)
            .assert_no_lint("no-floating-point-equality");
    }

    #[test]
    fn test_allows_float_less_than() {
        let test = TestProgram::for_rule_without_prelude(NoFloatingPointEquality);
        let result = test.lint_dir(
            "no_floating_point_equality/test_allows_float_less_than.ds",
            r#"
let a: float64 = 1.0;
let b: float64 = 2.0;
let less = a < b;
"#,
        );
        test.result(result)
            .assert_no_lint("no-floating-point-equality");
    }

    #[test]
    fn test_allows_float_equality_with_zero_constant() {
        let test = TestProgram::for_rule_without_prelude(NoFloatingPointEquality);
        let result = test.lint_dir(
            "no_floating_point_equality/test_allows_float_equality_with_zero_constant.ds",
            r#"
let value: float64 = read();
let isZero = value == 0.0;
"#,
        );
        test.result(result)
            .assert_no_lint("no-floating-point-equality");
    }

    #[test]
    fn test_allows_signum_comparison() {
        let test = TestProgram::for_rule_with_prelude(NoFloatingPointEquality);
        let result = test.lint_dir(
            "no_floating_point_equality/test_allows_signum_comparison.ds",
            r#"
let left: float64 = getLeft();
let right: float64 = getRight();
let sameSign = left.signum() == right.signum();
"#,
        );
        test.result(result)
            .assert_no_lint("no-floating-point-equality");
    }
}
