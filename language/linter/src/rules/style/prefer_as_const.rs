use crate::LintMeta;
use destack_ast::{self as ast, Declarator, Expression, Member, ScalarLiteral, TypeExpression};
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::span_has_comment;
use crate::{LintAstContext, LintFix, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Prefer `as const` over literal type assertions.
    ///
    /// Use `x as const` instead of `x as "literal"` for better type inference.
    #[lint(
        id = "prefer-as-const",
        code = "LY034",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferAsConst,
    "Prefer `as const` over literal type assertions"
}

impl LintRule for PreferAsConst {
    fn meta(&self) -> &'static LintMeta {
        PreferAsConst::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // prefer `as const` in literal cast assertions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            // look for type cast expressions
            let Expression::As {
                expression: left,
                target_type: right,
            } = expr
            else {
                continue;
            };

            // only report exact literal self assertions
            if !is_exact_literal_self_cast(ctx.tree, *left, *right) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let right_span = ctx.tree.get_span(*right);
            let edits = ctx.edit_builder().replace(right_span, "const").into_edits();
            let fix =
                LintFix::safe("Replace literal type assertion with `as const`").with_edits(edits);

            ctx.report(
                LintReport::new(
                    PREFER_AS_CONST.id,
                    PREFER_AS_CONST.code,
                    PREFER_AS_CONST.category,
                    severity,
                    "use `as const` instead of literal type assertion",
                    ctx.tree.get_span(node_id),
                )
                .label("prefer `as const`")
                .fix(fix),
            );
        }

        // prefer `as const` for literal type annotations on variable declarators
        for declarator_id in ctx.tree.iter_nodes::<Declarator>() {
            let declarator = ctx.tree.get(declarator_id);
            let Some(type_expression_id) = declarator.ty else {
                continue;
            };
            let Some(value_expression_id) = declarator.value else {
                continue;
            };
            if !is_exact_literal_self_cast(ctx.tree, value_expression_id, type_expression_id) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, declarator_id);
            if !severity.is_enabled() {
                continue;
            }

            let type_span = ctx.tree.get_span(type_expression_id);
            let mut diagnostic = LintReport::new(
                PREFER_AS_CONST.id,
                PREFER_AS_CONST.code,
                PREFER_AS_CONST.category,
                severity,
                "use `as const` instead of literal type annotation",
                type_span,
            )
            .label("prefer `as const`");
            if let Some(fix) = declarator_literal_annotation_fix(ctx, declarator_id) {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }

        // prefer `as const` for class field literal annotations
        for member_id in ctx.tree.iter_nodes::<Member>() {
            let member = ctx.tree.get(member_id);
            let Member::Field {
                declared_type: Some(type_expression_id),
                default: Some(default_expression_id),
                ..
            } = member
            else {
                continue;
            };
            if !is_exact_literal_self_cast(ctx.tree, *default_expression_id, *type_expression_id) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, member_id);
            if !severity.is_enabled() {
                continue;
            }

            let type_span = ctx.tree.get_span(*type_expression_id);
            let mut diagnostic = LintReport::new(
                PREFER_AS_CONST.id,
                PREFER_AS_CONST.code,
                PREFER_AS_CONST.category,
                severity,
                "use `as const` instead of literal type annotation",
                type_span,
            )
            .label("prefer `as const`");
            if let Some(fix) = member_field_literal_annotation_fix(ctx, member_id) {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Build a safe declarator fix from `name: 'x' = 'x'` to `name = 'x' as const`.
fn declarator_literal_annotation_fix(
    ctx: &LintAstContext<'_>,
    declarator_id: ast::LocalNodeId<Declarator>,
) -> Option<LintFix> {
    let declarator = ctx.tree.get(declarator_id);
    let type_expression_id = declarator.ty?;
    let value_expression_id = declarator.value?;

    // avoid rewriting commented declarators
    let declarator_span = ctx.tree.get_span(declarator_id);
    if span_has_comment(ctx.tree, declarator_span) {
        return None;
    }

    // derive source spans for annotation and initializer
    let value_span = ctx.tree.get_span(value_expression_id);
    let type_span = ctx.tree.get_span(type_expression_id);
    let before_type_span = Span::new(declarator_span.file, declarator_span.start, type_span.start);
    let after_value_span = Span::new(declarator_span.file, value_span.end, declarator_span.end);
    let before_type_text = ctx.get_span_text(before_type_span);
    let after_value_text = ctx.get_span_text(after_value_span);
    let value_text = ctx.get_span_text(value_span);

    // require `:<type> =` declarator shape
    let before_type_text = before_type_text.trim_end();
    let left_text = before_type_text.strip_suffix(':')?;
    let left_text = left_text.trim_end();

    // replace full declarator with const assertion form
    let replacement = format!("{left_text} = {value_text} as const{after_value_text}");
    let edits = ctx
        .edit_builder()
        .replace(declarator_span, replacement)
        .into_edits();

    Some(LintFix::safe("Replace literal annotation with `as const`").with_edits(edits))
}

/// Build a safe class field fix from `field: 'x' = 'x'` to `field = 'x' as const`.
fn member_field_literal_annotation_fix(
    ctx: &LintAstContext<'_>,
    member_id: ast::LocalNodeId<Member>,
) -> Option<LintFix> {
    let member = ctx.tree.get(member_id);
    let Member::Field {
        declared_type: Some(type_expression_id),
        default: Some(default_expression_id),
        ..
    } = member
    else {
        return None;
    };

    // avoid rewriting commented fields
    let member_span = ctx.tree.get_span(member_id);
    if span_has_comment(ctx.tree, member_span) {
        return None;
    }

    // derive source spans for annotation and initializer
    let default_span = ctx.tree.get_span(*default_expression_id);
    let type_span = ctx.tree.get_span(*type_expression_id);
    let before_type_span = Span::new(member_span.file, member_span.start, type_span.start);
    let after_default_span = Span::new(member_span.file, default_span.end, member_span.end);
    let before_type_text = ctx.get_span_text(before_type_span);
    let after_default_text = ctx.get_span_text(after_default_span);
    let default_text = ctx.get_span_text(default_span);

    // require `:<type> =` field shape
    let before_type_text = before_type_text.trim_end();
    let left_text = before_type_text.strip_suffix(':')?;
    let left_text = left_text.trim_end();

    // replace full field with const assertion form
    let replacement = format!("{left_text} = {default_text} as const{after_default_text}");
    let edits = ctx
        .edit_builder()
        .replace(member_span, replacement)
        .into_edits();

    Some(LintFix::safe("Replace literal annotation with `as const`").with_edits(edits))
}

/// Return true when a cast has the same literal value on both sides.
fn is_exact_literal_self_cast(
    tree: &ast::Tree,
    left_id: ast::LocalNodeId<Expression>,
    right_id: ast::LocalNodeId<ast::TypeExpression>,
) -> bool {
    let left = tree.get(left_id);
    let right = tree.get(right_id);
    match (left, right) {
        (
            Expression::ScalarLiteral(ScalarLiteral::String(left)),
            TypeExpression::ScalarLiteral {
                value: ScalarLiteral::String(right),
            },
        ) => left == right,
        (
            Expression::ScalarLiteral(ScalarLiteral::Integer(left)),
            TypeExpression::ScalarLiteral {
                value: ScalarLiteral::Integer(right),
            },
        ) => left == right,
        (
            Expression::ScalarLiteral(ScalarLiteral::Boolean(left)),
            TypeExpression::ScalarLiteral {
                value: ScalarLiteral::Boolean(right),
            },
        ) => left == right,
        (
            Expression::ScalarLiteral(ScalarLiteral::Bigint(left)),
            TypeExpression::ScalarLiteral {
                value: ScalarLiteral::Bigint(right),
            },
        ) => left == right,
        (
            Expression::ScalarLiteral(ScalarLiteral::Float(left)),
            TypeExpression::ScalarLiteral {
                value: ScalarLiteral::Float(right),
            },
        ) => left == right,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_string_literal_cast() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_detects_string_literal_cast.ds",
            r#"
const x = "hello" as "hello"
"#,
        );
        test.result(result).assert_lint("prefer-as-const");
    }

    #[test]
    fn test_fix_string_literal_self_cast() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_fix_string_literal_self_cast.ds",
            r#"
const x = "hello" as "hello"
"#,
        );
        test.result(result)
            .assert_lint("prefer-as-const")
            .assert_has_fix("prefer-as-const")
            .assert_safe_fixed(
                r#"
const x = "hello" as const;
"#,
            );
    }

    #[test]
    fn test_no_fix_for_non_literal_self_cast() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_no_fix_for_non_literal_self_cast.ds",
            r#"
const x = value as "hello"
"#,
        );
        test.result(result).assert_no_lint("prefer-as-const");
    }

    #[test]
    fn test_detects_number_literal_cast() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_detects_number_literal_cast.ds",
            r#"
const x = 42 as 42
"#,
        );
        test.result(result).assert_lint("prefer-as-const");
    }

    #[test]
    fn test_fix_number_literal_self_cast() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_fix_number_literal_self_cast.ds",
            r#"
const x = 42 as 42
"#,
        );
        test.result(result)
            .assert_lint("prefer-as-const")
            .assert_has_fix("prefer-as-const")
            .assert_safe_fixed(
                r#"
const x = 42 as const;
"#,
            );
    }

    /// Fix float literal self-casts to `as const`.
    #[test]
    fn test_fix_float_literal_self_cast() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_fix_float_literal_self_cast.ds",
            r#"
const x = 1.5 as 1.5
"#,
        );
        test.result(result)
            .assert_lint("prefer-as-const")
            .assert_has_fix("prefer-as-const")
            .assert_safe_fixed(
                r#"
const x = 1.5 as const;
"#,
            );
    }

    #[test]
    fn test_fix_boolean_literal_self_cast() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_fix_boolean_literal_self_cast.ds",
            r#"
const x = true as true
"#,
        );
        test.result(result)
            .assert_lint("prefer-as-const")
            .assert_has_fix("prefer-as-const")
            .assert_safe_fixed(
                r#"
const x = true as const;
"#,
            );
    }

    #[test]
    fn test_no_fix_for_string_literal_mismatch() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_no_fix_for_string_literal_mismatch.ds",
            r#"
const x = "hello" as "world"
"#,
        );
        test.result(result).assert_no_lint("prefer-as-const");
    }

    #[test]
    fn test_no_fix_for_number_literal_mismatch() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_no_fix_for_number_literal_mismatch.ds",
            r#"
const x = 41 as 42
"#,
        );
        test.result(result).assert_no_lint("prefer-as-const");
    }

    #[test]
    fn test_no_fix_for_boolean_literal_mismatch() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_no_fix_for_boolean_literal_mismatch.ds",
            r#"
const x = false as true
"#,
        );
        test.result(result).assert_no_lint("prefer-as-const");
    }

    #[test]
    fn test_fix_bigint_literal_self_cast() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_fix_bigint_literal_self_cast.ds",
            r#"
const x = 1n as 1n
"#,
        );
        test.result(result)
            .assert_lint("prefer-as-const")
            .assert_has_fix("prefer-as-const")
            .assert_safe_fixed(
                r#"
const x = 1n as const;
"#,
            );
    }

    #[test]
    fn test_allows_as_const() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_allows_as_const.ds",
            r#"
const x = "hello" as const
"#,
        );
        test.result(result).assert_no_lint("prefer-as-const");
    }

    #[test]
    fn test_allows_type_cast() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_allows_type_cast.ds",
            r#"
const x = value as string
"#,
        );
        test.result(result).assert_no_lint("prefer-as-const");
    }

    #[test]
    fn test_allows_object_cast() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_allows_object_cast.ds",
            r#"
const x = obj as { foo: string }
"#,
        );
        test.result(result).assert_no_lint("prefer-as-const");
    }

    #[test]
    fn test_fix_variable_literal_annotation() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_fix_variable_literal_annotation.ds",
            r#"
let foo: 'bar' = 'bar';
"#,
        );
        test.result(result)
            .assert_lint("prefer-as-const")
            .assert_has_fix("prefer-as-const")
            .assert_safe_fixed(
                r#"
let foo = 'bar' as const;
"#,
            );
    }

    #[test]
    fn test_fix_class_field_literal_annotation() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_fix_class_field_literal_annotation.ds",
            r#"
class Foo {
    bar: 'baz' = 'baz';
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-as-const")
            .assert_has_fix("prefer-as-const")
            .assert_safe_fixed(
                r#"
class Foo {
    bar = 'baz' as const;
}
"#,
            );
    }

    #[test]
    fn test_allows_variable_literal_annotation_with_non_literal_initializer() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_allows_variable_literal_annotation_with_non_literal_initializer.ds",
            r#"
let value = 'bar';
let foo: 'bar' = value;
"#,
        );
        test.result(result).assert_no_lint("prefer-as-const");
    }

    /// Keep a diagnostic when annotation rewrites are comment blocked.
    #[test]
    fn test_lint_without_fix_for_commented_variable_annotation() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_lint_without_fix_for_commented_variable_annotation.ds",
            r#"
let foo /* keep note */: 'bar' = 'bar';
"#,
        );
        test.result(result)
            .assert_lint("prefer-as-const")
            .assert_has_no_fix("prefer-as-const");
    }

    /// Keep a diagnostic when class-field annotation rewrites are comment blocked.
    #[test]
    fn test_lint_without_fix_for_commented_field_annotation() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_lint_without_fix_for_commented_field_annotation.ds",
            r#"
class Foo {
    bar /* keep note */: 'baz' = 'baz';
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-as-const")
            .assert_has_no_fix("prefer-as-const");
    }
}
