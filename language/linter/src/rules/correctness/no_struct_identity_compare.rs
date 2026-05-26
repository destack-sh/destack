use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, SymbolKind, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{expression_unwrap_transparent, is_reference_symbol_kind};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow identity comparison on value types.
    ///
    /// Structs are value types in Destack and have no identity. Using `===`
    /// or `!==` (identity comparison) on structs is incorrect since they
    /// cannot be compared by reference. Use `==` or `!=` for value comparison.
    #[lint(
        id = "no-struct-identity-compare",
        code = "LC027",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoStructIdentityCompare,
    "Disallow identity comparison on value types"
}

impl LintRule for NoStructIdentityCompare {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoStructIdentityCompare::meta()
    }

    /// Check module DIR nodes for struct identity comparisons.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let mut visitor = StructCompareVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags identity comparisons on struct types.
struct StructCompareVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> StructCompareVisitor<'a, 'b> {
    /// Build a visitor for struct comparison checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        Self {
            ctx,
            meta,
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

    /// Check a binary expression for struct identity comparison.
    fn check_struct_compare(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        mut left: dir::LocalNodeId<dir::Expression>,
        mut right: dir::LocalNodeId<dir::Expression>,
    ) {
        // normalize transparent wrappers for nullish and type checks
        left = expression_unwrap_transparent(self.ctx.dir.tree(), left);
        right = expression_unwrap_transparent(self.ctx.dir.tree(), right);

        // allow explicit nullish sentinel checks on optional values
        if is_nullish_literal_expression(self.ctx.dir.tree(), left)
            || is_nullish_literal_expression(self.ctx.dir.tree(), right)
        {
            return;
        }

        // resolve the left operand type
        let left_is_struct = self.ctx.expression_type_id(left).is_some_and(|type_id| {
            is_reference_symbol_kind(
                self.ctx.types,
                &self.ctx.symbols,
                type_id,
                SymbolKind::Struct,
            )
        });

        // resolve the right operand type
        let right_is_struct = self.ctx.expression_type_id(right).is_some_and(|type_id| {
            is_reference_symbol_kind(
                self.ctx.types,
                &self.ctx.symbols,
                type_id,
                SymbolKind::Struct,
            )
        });

        // at least one operand must be a struct
        if !left_is_struct && !right_is_struct {
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
                NO_STRUCT_IDENTITY_COMPARE.id,
                NO_STRUCT_IDENTITY_COMPARE.code,
                NO_STRUCT_IDENTITY_COMPARE.category,
                severity,
                "identity comparison on struct type",
                span,
            )
            .label("structs have no identity; use == or != instead"),
        );
    }
}

/// Return true when one expression is a nullish literal.
fn is_nullish_literal_expression(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression = tree.get(expression_id);
    let dir::Expression::Type { value } = expression else {
        return false;
    };

    matches!(
        tree.get(*value),
        dir::TypeExpression::Literal {
            value: dir::TypeLiteral::Null | dir::TypeLiteral::Undefined,
        }
    )
}

impl NodeVisitor for StructCompareVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check identity comparisons (=== and !==)
        if let dir::Expression::Binary {
            operator,
            left,
            right,
        } = expression
            && matches!(
                operator,
                dir::BinaryOperator::EqualStrict | dir::BinaryOperator::NotEqualStrict
            )
        {
            self.check_struct_compare(id, *left, *right);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag struct strict equality (===).
    #[test]
    fn test_flags_struct_strict_equality() {
        let test = TestProgram::for_rule_without_prelude(NoStructIdentityCompare);
        let result = test.lint_dir(
            "no_struct_identity_compare/test_flags_struct_strict_equality.ds",
            r#"
struct Point { x: int32; y: int32 }
let a = Point { x: 1, y: 2 };
let b = Point { x: 1, y: 2 };
let equal = a === b;
"#,
        );
        test.result(result)
            .assert_lint("no-struct-identity-compare");
    }

    /// Flag struct strict not-equal (!==).
    #[test]
    fn test_flags_struct_strict_not_equal() {
        let test = TestProgram::for_rule_without_prelude(NoStructIdentityCompare);
        let result = test.lint_dir(
            "no_struct_identity_compare/test_flags_struct_strict_not_equal.ds",
            r#"
struct Point { x: int32; y: int32 }
let a = Point { x: 1, y: 2 };
let b = Point { x: 3, y: 4 };
let notEqual = a !== b;
"#,
        );
        test.result(result)
            .assert_lint("no-struct-identity-compare");
    }

    /// Allow struct value equality (==).
    #[test]
    fn test_allows_struct_value_equality() {
        let test = TestProgram::for_rule_without_prelude(NoStructIdentityCompare);
        let result = test.lint_dir(
            "no_struct_identity_compare/test_allows_struct_value_equality.ds",
            r#"
struct Point { x: int32; y: int32 }
let a = Point { x: 1, y: 2 };
let b = Point { x: 1, y: 2 };
let equal = a == b;
"#,
        );
        test.result(result)
            .assert_no_lint("no-struct-identity-compare");
    }

    /// Allow class identity comparison.
    #[test]
    fn test_allows_class_identity() {
        let test = TestProgram::for_rule_without_prelude(NoStructIdentityCompare);
        let result = test.lint_dir(
            "no_struct_identity_compare/test_allows_class_identity.ds",
            r#"
class Point { x: int32; y: int32 }
let a = new Point();
let b = new Point();
let equal = a === b;
"#,
        );
        test.result(result)
            .assert_no_lint("no-struct-identity-compare");
    }

    /// Allow primitive identity comparison.
    #[test]
    fn test_allows_primitive_identity() {
        let test = TestProgram::for_rule_without_prelude(NoStructIdentityCompare);
        let result = test.lint_dir(
            "no_struct_identity_compare/test_allows_primitive_identity.ds",
            r#"
let a: int32 = 1;
let b: int32 = 2;
let equal = a === b;
"#,
        );
        test.result(result)
            .assert_no_lint("no-struct-identity-compare");
    }

    /// Allow nullish checks on optional struct values.
    #[test]
    fn test_allows_struct_nullish_check() {
        let test = TestProgram::for_rule_without_prelude(NoStructIdentityCompare);
        let result = test.lint_dir(
            "no_struct_identity_compare/test_allows_struct_nullish_check.ds",
            r#"
struct Point { x: int32; y: int32 }
let value: Point | null = null;
let isNull = value === null;
"#,
        );
        test.result(result)
            .assert_no_lint("no-struct-identity-compare");
    }

    /// Allow wrapped nullish checks on optional struct values.
    #[test]
    fn test_allows_wrapped_struct_nullish_check() {
        let test = TestProgram::for_rule_without_prelude(NoStructIdentityCompare);
        let result = test.lint_dir(
            "no_struct_identity_compare/test_allows_wrapped_struct_nullish_check.ds",
            r#"
struct Point { x: int32; y: int32 }
let value: Point | null = null;
let isNull = (value) === (null);
"#,
        );
        test.result(result)
            .assert_no_lint("no-struct-identity-compare");
    }
}
