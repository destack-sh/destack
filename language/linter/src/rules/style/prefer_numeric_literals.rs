use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{const_i64, expression_is_global_qualified_member};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer numeric literals over `parseInt()`.
    ///
    /// Binary, octal, and hexadecimal literals are more concise and
    /// clearer about the radix being used.
    #[lint(
        id = "prefer-numeric-literals",
        code = "LY053",
        category = Style,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Number)],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferNumericLiterals,
    "Prefer numeric literals over parseInt()"
}

impl LintRule for PreferNumericLiterals {
    fn meta(&self) -> &'static LintMeta {
        PreferNumericLiterals::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = PreferNumericLiteralsVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags parseInt with binary/octal/hex radix.
struct PreferNumericLiteralsVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Number symbol for this module.
    number_symbol: dir::GlobalSymbolId,
    /// The string id for the parseInt method name.
    parse_int_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferNumericLiteralsVisitor<'a, 'b> {
    /// Build a visitor for prefer-numeric-literals checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let number_symbol = ctx.well_known_symbol(WellKnownSymbol::Number);
        let parse_int_name = ctx.program.strings.intern("parseInt");
        Self {
            ctx,
            meta,
            number_symbol,
            parse_int_name,
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

    /// Check a call expression for parseInt usage.
    fn check_call(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // match call expressions
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Call {
            left,
            dynamic_arguments,
            ..
        } = expression
        else {
            return;
        };

        // check if this is Number.parseInt
        let is_number_parse_int = expression_is_global_qualified_member(
            self.ctx.tree,
            *left,
            &[self.number_symbol],
            self.parse_int_name,
        );
        if !is_number_parse_int {
            return;
        }

        // need exactly 2 arguments (string and radix)
        if dynamic_arguments.len() != 2 {
            return;
        }

        // check if first argument is a string literal
        let string_arg = self.ctx.tree.get(dynamic_arguments[0]);
        let string_expr_id = string_arg.value();
        let string_expr = self.ctx.tree.get(string_expr_id);
        let is_string_literal = matches!(
            string_expr,
            dir::Expression::ScalarLiteral {
                value: dir::ScalarLiteral::String(_)
            }
        );
        if !is_string_literal {
            return;
        }

        // check if radix is a constant 2, 8, or 16
        let radix_arg = self.ctx.tree.get(dynamic_arguments[1]);
        let radix_expr_id = radix_arg.value();
        let Some(const_value) = self.ctx.const_value(radix_expr_id) else {
            return;
        };
        let Some(radix) = const_i64(&const_value) else {
            return;
        };

        // map radix to literal prefix
        let prefix = match radix {
            2 => "0b",
            8 => "0o",
            16 => "0x",
            _ => return,
        };

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                PREFER_NUMERIC_LITERALS.id,
                PREFER_NUMERIC_LITERALS.code,
                PREFER_NUMERIC_LITERALS.category,
                severity,
                format!("prefer {prefix} literal over parseInt"),
                self.ctx.module.file_id,
                span,
            )
            .with_label(format!("use a {prefix} numeric literal instead")),
        );
    }
}

impl NodeVisitor for PreferNumericLiteralsVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check call expressions
        if matches!(expression, dir::Expression::Call { .. }) {
            self.check_call(id);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag Number.parseInt with hex radix.
    #[test]
    fn test_flags_parseint_hex() {
        let test = TestProgram::for_rule_with_prelude(PreferNumericLiterals);
        let result = test.lint_dir(
            "test.ds",
            r#"
let hex = Number.parseInt("FF", 16);
"#,
        );
        test.result(result).assert_lint("prefer-numeric-literals");
    }

    /// Flag Number.parseInt with binary radix.
    #[test]
    fn test_flags_parseint_binary() {
        let test = TestProgram::for_rule_with_prelude(PreferNumericLiterals);
        let result = test.lint_dir(
            "test.ds",
            r#"
let bin = Number.parseInt("1010", 2);
"#,
        );
        test.result(result).assert_lint("prefer-numeric-literals");
    }

    /// Flag Number.parseInt with octal radix.
    #[test]
    fn test_flags_parseint_octal() {
        let test = TestProgram::for_rule_with_prelude(PreferNumericLiterals);
        let result = test.lint_dir(
            "test.ds",
            r#"
let oct = Number.parseInt("777", 8);
"#,
        );
        test.result(result).assert_lint("prefer-numeric-literals");
    }

    /// Allow Number.parseInt with radix 10.
    #[test]
    fn test_allows_parseint_decimal() {
        let test = TestProgram::for_rule_with_prelude(PreferNumericLiterals);
        let result = test.lint_dir(
            "test.ds",
            r#"
let dec = Number.parseInt("42", 10);
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-numeric-literals");
    }

    /// Allow Number.parseInt with variable string.
    #[test]
    fn test_allows_parseint_variable_string() {
        let test = TestProgram::for_rule_with_prelude(PreferNumericLiterals);
        let result = test.lint_dir(
            "test.ds",
            r#"
let hex = "FF";
let value = Number.parseInt(hex, 16);
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-numeric-literals");
    }

    /// Allow Number.parseInt with variable radix.
    #[test]
    fn test_allows_parseint_variable_radix() {
        let test = TestProgram::for_rule_with_prelude(PreferNumericLiterals);
        let result = test.lint_dir(
            "test.ds",
            r#"
let radix = 16;
let value = Number.parseInt("FF", radix);
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-numeric-literals");
    }

    /// Allow numeric literals directly.
    #[test]
    fn test_allows_numeric_literals() {
        let test = TestProgram::for_rule_with_prelude(PreferNumericLiterals);
        let result = test.lint_dir(
            "test.ds",
            r#"
let hex = 0xFF;
let bin = 0b1010;
let oct = 0o777;
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-numeric-literals");
    }
}
