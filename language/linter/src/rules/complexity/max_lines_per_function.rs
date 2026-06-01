use crate::LintMeta;
use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    CallableOwnerId, callable_owner_span, count_file_span_lines, declaration_expression,
    expression_is_immediately_invoked, for_each_callable_signature,
};
use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of lines per function.
    ///
    /// Long functions are harder to understand, test, and maintain.
    /// Consider breaking them into smaller, focused helper functions.
    #[lint(
        id = "max-lines-per-function",
        code = "LX006",
        category = Complexity,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxLinesPerFunction,
    "Limit lines per function"
}

impl LintRule for MaxLinesPerFunction {
    fn meta(&self) -> &'static LintMeta {
        MaxLinesPerFunction::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        // resolve lint metadata, threshold, and source file
        let meta = self.meta();
        let max_lines = ctx.options().complexity.max_lines_per_function;
        let skip_comments = ctx
            .options()
            .complexity
            .max_lines_per_function_skip_comments;
        let skip_blank_lines = ctx
            .options()
            .complexity
            .max_lines_per_function_skip_blank_lines;
        let include_iifes = ctx.options().complexity.max_lines_per_function_iifes;
        let file = ctx.file.clone();

        // check all callable owners that have a body expression
        for_each_callable_signature(ctx.dir.tree(), |owner_id, _signature, body_id| {
            // skip signature-only callables without a body
            let Some(body_id) = body_id else {
                return;
            };

            // skip iifes unless they are explicitly included
            if !include_iifes && callable_owner_is_iife(ctx, owner_id) {
                return;
            }

            // resolve owner span for line counting
            let owner_span = callable_owner_span(ctx.dir.tree(), owner_id);
            let line_count =
                count_file_span_lines(file.as_ref(), owner_span, skip_comments, skip_blank_lines);
            if line_count <= max_lines {
                return;
            }

            // report declarations that exceed the line threshold
            if let CallableOwnerId::Declaration(declaration_id) = owner_id {
                report_line_limit_violation(
                    ctx,
                    meta,
                    declaration_id,
                    body_id,
                    line_count,
                    max_lines,
                );
                return;
            }

            // report members that exceed the line threshold
            if let CallableOwnerId::Member(member_id) = owner_id {
                report_line_limit_violation(ctx, meta, member_id, body_id, line_count, max_lines);
                return;
            }

            // report properties that exceed the line threshold
            if let CallableOwnerId::Property(property_id) = owner_id {
                report_line_limit_violation(ctx, meta, property_id, body_id, line_count, max_lines);
            }
        });
    }
}

/// Report one max-lines-per-function violation for a callable owner.
fn report_line_limit_violation<T: dir::Node>(
    ctx: &mut LintModuleContext<'_>,
    meta: &'static LintMeta,
    owner_id: dir::LocalNodeId<T>,
    body_id: dir::LocalNodeId<dir::Expression>,
    line_count: usize,
    max_lines: usize,
) {
    // resolve effective severity for this owner node
    let severity = ctx.get_effective_severity(meta, owner_id);
    if !severity.is_enabled() {
        return;
    }

    // report one line-count overflow diagnostic
    ctx.report(
        LintReport::new(
            MAX_LINES_PER_FUNCTION.id,
            MAX_LINES_PER_FUNCTION.code,
            MAX_LINES_PER_FUNCTION.category,
            severity,
            format!("function has {line_count} lines (max {max_lines})"),
            ctx.dir.get_span(body_id),
        )
        .label("consider breaking into smaller functions"),
    );
}

/// Return true when one callable owner is an immediately invoked function expression.
fn callable_owner_is_iife(ctx: &LintModuleContext<'_>, owner_id: CallableOwnerId) -> bool {
    let CallableOwnerId::Declaration(declaration_id) = owner_id else {
        return false;
    };
    let Some(expression_id) = declaration_expression(ctx.dir.tree(), declaration_id) else {
        return false;
    };

    expression_is_immediately_invoked(ctx.dir.tree(), expression_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_long_function() {
        let test = TestProgram::for_rule_without_prelude(MaxLinesPerFunction);
        // create a function with 51 lines (over default 50)
        let mut source = String::from("function foo() {\n");
        for i in 0..49 {
            source.push_str(&format!("    let x{i} = {i};\n"));
        }
        source.push_str("}\n");
        let result = test.lint(
            "max_lines_per_function/test_detects_long_function.ds",
            &source,
        );
        test.result(result).assert_lint("max-lines-per-function");
    }

    #[test]
    fn test_allows_short_function() {
        let test = TestProgram::for_rule_without_prelude(MaxLinesPerFunction);
        let result = test.lint(
            "max_lines_per_function/test_allows_short_function.ds",
            r#"
function foo() {
    let x = 1;
    let y = 2;
    return x + y;
}
"#,
        );
        test.result(result).assert_no_lint("max-lines-per-function");
    }

    #[test]
    fn test_allows_exactly_at_limit() {
        let test = TestProgram::for_rule_without_prelude(MaxLinesPerFunction);
        // create a function with exactly 50 lines
        let mut source = String::from("function foo() {\n");
        for i in 0..48 {
            source.push_str(&format!("    let x{i} = {i};\n"));
        }
        source.push_str("}\n");
        let result = test.lint(
            "max_lines_per_function/test_allows_exactly_at_limit.ds",
            &source,
        );
        test.result(result).assert_no_lint("max-lines-per-function");
    }

    #[test]
    fn test_checks_arrow_functions() {
        let test = TestProgram::for_rule_without_prelude(MaxLinesPerFunction);
        // create an arrow function with many lines
        let mut source = String::from("const foo = () => {\n");
        for i in 0..49 {
            source.push_str(&format!("    let x{i} = {i};\n"));
        }
        source.push_str("};\n");
        let result = test.lint(
            "max_lines_per_function/test_checks_arrow_functions.ds",
            &source,
        );
        test.result(result).assert_lint("max-lines-per-function");
    }

    #[test]
    fn test_checks_class_methods() {
        let test = TestProgram::for_rule_without_prelude(MaxLinesPerFunction);
        // create a method with many lines
        let mut source = String::from("class Example {\n    method() {\n");
        for i in 0..49 {
            source.push_str(&format!("        let x{i} = {i};\n"));
        }
        source.push_str("    }\n}\n");
        let result = test.lint(
            "max_lines_per_function/test_checks_class_methods.ds",
            &source,
        );
        test.result(result).assert_lint("max-lines-per-function");
    }

    #[test]
    fn test_checks_object_methods() {
        let test = TestProgram::for_rule_without_prelude(MaxLinesPerFunction);
        // create an object method with many lines
        let mut source = String::from("const object = {\n    method() {\n");
        for i in 0..49 {
            source.push_str(&format!("        let x{i} = {i};\n"));
        }
        source.push_str("    }\n};\n");
        let result = test.lint(
            "max_lines_per_function/test_checks_object_methods.ds",
            &source,
        );
        test.result(result).assert_lint("max-lines-per-function");
    }

    #[test]
    fn test_ignores_function_declarations() {
        let test = TestProgram::for_rule_without_prelude(MaxLinesPerFunction);
        let result = test.lint(
            "max_lines_per_function/test_ignores_function_declarations.ds",
            r#"
declare function foo(): void;
"#,
        );
        test.result(result).assert_no_lint("max-lines-per-function");
    }

    #[test]
    fn test_skips_comment_lines_when_enabled() {
        let test =
            TestProgram::for_rule_without_prelude(MaxLinesPerFunction).with_options(|options| {
                options.complexity.max_lines_per_function = 3;
                options.complexity.max_lines_per_function_skip_comments = true;
            });
        let result = test.lint(
            "max_lines_per_function/test_skips_comment_lines_when_enabled.ds",
            r#"
function foo() {
    // comment
    let x = 1;
}
"#,
        );
        test.result(result).assert_no_lint("max-lines-per-function");
    }

    #[test]
    fn test_skips_blank_lines_when_enabled() {
        let test =
            TestProgram::for_rule_without_prelude(MaxLinesPerFunction).with_options(|options| {
                options.complexity.max_lines_per_function = 3;
                options.complexity.max_lines_per_function_skip_blank_lines = true;
            });
        let result = test.lint(
            "max_lines_per_function/test_skips_blank_lines_when_enabled.ds",
            r#"
function foo() {

    let x = 1;
}
"#,
        );
        test.result(result).assert_no_lint("max-lines-per-function");
    }

    #[test]
    fn test_ignores_iife_by_default() {
        let test = TestProgram::for_rule_without_prelude(MaxLinesPerFunction)
            .with_options(|options| options.complexity.max_lines_per_function = 3);
        let result = test.lint(
            "max_lines_per_function/test_ignores_iife_by_default.ds",
            r#"
(function () {
    let a = 1;
    let b = 2;
})();
"#,
        );
        test.result(result).assert_no_lint("max-lines-per-function");
    }

    #[test]
    fn test_checks_iife_when_enabled() {
        let test =
            TestProgram::for_rule_without_prelude(MaxLinesPerFunction).with_options(|options| {
                options.complexity.max_lines_per_function = 3;
                options.complexity.max_lines_per_function_iifes = true;
            });
        let result = test.lint(
            "max_lines_per_function/test_checks_iife_when_enabled.ds",
            r#"
(function () {
    let a = 1;
    let b = 2;
})();
"#,
        );
        test.result(result).assert_lint("max-lines-per-function");
    }
}
