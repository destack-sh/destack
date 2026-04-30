use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    const_i64, expression_is_symbol_or_global_qualified_member, expression_static_property_access,
    expression_static_string_literal, expression_unwrap_transparent, span_has_comment,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer numeric literals over `parseInt()`.
    ///
    /// Binary, octal, and hexadecimal literals are more concise and
    /// clearer about the radix being used.
    #[lint(
        id = "prefer-numeric-literals",
        code = "LY047",
        category = Style,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Number)],
        requires_any = [],
        fixable = Sometimes,
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
    /// The string id for the Number global name.
    number_name: StringId,
    /// The global qualifier symbols.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferNumericLiteralsVisitor<'a, 'b> {
    /// Build a visitor for prefer-numeric-literals checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let number_symbol = ctx.well_known_symbol(WellKnownSymbol::Number);
        let parse_int_name = ctx.string_id("parseInt");
        let number_name = ctx.string_id("Number");
        let global_qualifiers = ctx.global_qualifier_symbols();
        Self {
            ctx,
            meta,
            number_symbol,
            parse_int_name,
            number_name,
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

    /// Check a call expression for parseInt usage.
    fn check_call(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // match call expressions
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Call {
            left,
            generic_arguments,
            arguments,
            ..
        } = expression
        else {
            return;
        };

        // check if this is Number.parseInt or a global parseInt
        if !self.is_parse_int_callee(*left) {
            return;
        }

        // need exactly 2 arguments (string and radix)
        if arguments.len() != 2 {
            return;
        }

        // check if first argument is a static string
        let string_arg = self.ctx.tree.get(arguments[0]);
        let string_expr_id = string_arg.value();
        let Some(string_value) = expression_static_string_literal(self.ctx.tree, string_expr_id)
        else {
            return;
        };

        // check if radix is a constant 2, 8, or 16
        let radix_arg = self.ctx.tree.get(arguments[1]);
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

        // report the diagnostic and attach fix when safe
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintDiagnostic::new(
            PREFER_NUMERIC_LITERALS.id,
            PREFER_NUMERIC_LITERALS.code,
            PREFER_NUMERIC_LITERALS.category,
            severity,
            format!("prefer {prefix} literal over parseInt"),
            self.ctx.module.file_id,
            span,
        )
        .with_label(format!("use a {prefix} numeric literal instead"));
        if let Some(fix) = self.numeric_literal_fix(
            expression_id,
            generic_arguments.as_slice(),
            arguments.as_slice(),
            string_value,
            radix,
            prefix,
        ) {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Build a safe fix from Number.parseInt() to a numeric literal.
    fn numeric_literal_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        arguments: &[dir::LocalNodeId<dir::Argument>],
        string_value: StringId,
        radix: i64,
        prefix: &str,
    ) -> Option<LintFix> {
        // skip static call arguments until static argument rendering is supported
        if !generic_arguments.is_empty() {
            return None;
        }

        // require both call arguments to be positional
        if arguments.len() != 2 {
            return None;
        }
        for argument_id in arguments {
            let argument = self.ctx.tree.get(*argument_id);
            if !matches!(argument, dir::Argument::Positional { .. }) {
                return None;
            }
        }

        // only fix when the full string maps to one literal value with no partial parse behavior
        let value = self.ctx.strings.get(string_value);
        let replacement = preferred_numeric_literal(value.as_ref(), radix, prefix)?;

        // replace the full parseInt expression
        let expression_span = self.ctx.get_span(expression_id);
        if span_has_comment(self.ctx.ast, expression_span) {
            return None;
        }

        let edits = self
            .ctx
            .edit_builder()
            .replace(expression_span, replacement)
            .into_edits();

        Some(LintFix::safe("Replace parseInt call with numeric literal").with_edits(edits))
    }

    /// Return true when a callee expression is parseInt or Number.parseInt.
    fn is_parse_int_callee(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // match Number.parseInt and globalThis.Number.parseInt
        if let Some((base_id, property_name)) =
            expression_static_property_access(self.ctx.tree, expression_id)
            && property_name == self.parse_int_name
            && expression_is_symbol_or_global_qualified_member(
                self.ctx.tree,
                base_id,
                self.number_symbol,
                &self.global_qualifiers,
                self.number_name,
            )
        {
            return true;
        }

        // match bare global parseInt but skip local shadowed symbols
        let expression_id = expression_unwrap_transparent(self.ctx.tree, expression_id);
        let expression = self.ctx.tree.get(expression_id);
        match expression {
            dir::Expression::GlobalReference { path, .. }
            | dir::Expression::UnresolvedPath { path, .. } => {
                path.segments.len() == 1 && path.segments[0] == self.parse_int_name
            }
            dir::Expression::LocalReference { .. } | dir::Expression::ModuleReference { .. } => {
                false
            }
            _ => false,
        }
    }
}

impl NodeVisitor for PreferNumericLiteralsVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
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

/// Build a numeric literal replacement for a parseInt string when it is fully representable.
fn preferred_numeric_literal(value: &str, radix: i64, prefix: &str) -> Option<String> {
    // allow one optional sign at the front
    let (sign, digits) = if let Some(rest) = value.strip_prefix('+') {
        ('+', rest)
    } else if let Some(rest) = value.strip_prefix('-') {
        ('-', rest)
    } else {
        ('\0', value)
    };
    if digits.is_empty() {
        return None;
    }

    // avoid radix-prefixed strings that parseInt may treat specially
    if has_radix_prefix(digits, radix) {
        return None;
    }

    // require all digits to match the declared radix
    if !digits
        .chars()
        .all(|character| is_valid_radix_digit(character, radix))
    {
        return None;
    }

    // build the replacement literal with preserved sign
    let literal = format!("{prefix}{digits}");
    if sign == '-' {
        return Some(format!("-{literal}"));
    }
    if sign == '+' {
        return Some(format!("+{literal}"));
    }

    Some(literal)
}

/// Return true when a string starts with a radix prefix that could alter parseInt behavior.
fn has_radix_prefix(value: &str, radix: i64) -> bool {
    match radix {
        2 => value.starts_with("0b") || value.starts_with("0B"),
        8 => value.starts_with("0o") || value.starts_with("0O"),
        16 => value.starts_with("0x") || value.starts_with("0X"),
        _ => false,
    }
}

/// Return true when a character is a valid digit for the radix.
fn is_valid_radix_digit(character: char, radix: i64) -> bool {
    match radix {
        2 => matches!(character, '0' | '1'),
        8 => matches!(character, '0'..='7'),
        16 => character.is_ascii_hexdigit(),
        _ => false,
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
            "prefer_numeric_literals/test_flags_parseint_hex.ds",
            r#"
let hex = Number.parseInt("FF", 16);
"#,
        );
        test.result(result)
            .assert_lint("prefer-numeric-literals")
            .assert_has_fix("prefer-numeric-literals")
            .assert_safe_fixed(
                r#"
let hex = 0xFF;
"#,
            );
    }

    /// Flag global parseInt with hex radix.
    #[test]
    fn test_flags_global_parseint_hex() {
        let test = TestProgram::for_rule_with_prelude(PreferNumericLiterals);
        let result = test.lint_dir(
            "prefer_numeric_literals/test_flags_global_parseint_hex.ds",
            r#"
let hex = parseInt("FF", 16);
"#,
        );
        test.result(result)
            .assert_lint("prefer-numeric-literals")
            .assert_has_fix("prefer-numeric-literals")
            .assert_safe_fixed(
                r#"
let hex = 0xFF;
"#,
            );
    }

    /// Flag globalThis.Number.parseInt with binary radix.
    #[test]
    fn test_flags_global_qualified_number_parseint_binary() {
        let test = TestProgram::for_rule_with_prelude(PreferNumericLiterals);
        let result = test.lint_dir(
            "prefer_numeric_literals/test_flags_global_qualified_number_parseint_binary.ds",
            r#"
let bin = globalThis.Number.parseInt("1010", 2);
"#,
        );
        test.result(result)
            .assert_lint("prefer-numeric-literals")
            .assert_has_fix("prefer-numeric-literals")
            .assert_safe_fixed(
                r#"
let bin = 0b1010;
"#,
            );
    }

    /// Flag Number.parseInt with binary radix.
    #[test]
    fn test_flags_parseint_binary() {
        let test = TestProgram::for_rule_with_prelude(PreferNumericLiterals);
        let result = test.lint_dir(
            "prefer_numeric_literals/test_flags_parseint_binary.ds",
            r#"
let bin = Number.parseInt("1010", 2);
"#,
        );
        test.result(result)
            .assert_lint("prefer-numeric-literals")
            .assert_has_fix("prefer-numeric-literals")
            .assert_safe_fixed(
                r#"
let bin = 0b1010;
"#,
            );
    }

    /// Flag Number.parseInt with octal radix.
    #[test]
    fn test_flags_parseint_octal() {
        let test = TestProgram::for_rule_with_prelude(PreferNumericLiterals);
        let result = test.lint_dir(
            "prefer_numeric_literals/test_flags_parseint_octal.ds",
            r#"
let oct = Number.parseInt("777", 8);
"#,
        );
        test.result(result)
            .assert_lint("prefer-numeric-literals")
            .assert_has_fix("prefer-numeric-literals")
            .assert_safe_fixed(
                r#"
let oct = 0o777;
"#,
            );
    }

    /// Fix Number.parseInt with a signed literal.
    #[test]
    fn test_fix_parseint_negative_hex() {
        let test = TestProgram::for_rule_with_prelude(PreferNumericLiterals);
        let result = test.lint_dir(
            "prefer_numeric_literals/test_fix_parseint_negative_hex.ds",
            r#"
let value = Number.parseInt("-ff", 16);
"#,
        );
        test.result(result)
            .assert_lint("prefer-numeric-literals")
            .assert_has_fix("prefer-numeric-literals")
            .assert_safe_fixed(
                r#"
let value = -0xFF;
"#,
            );
    }

    /// Keep lint without fix when parseInt would rely on partial parsing.
    #[test]
    fn test_no_fix_for_invalid_radix_digit() {
        let test = TestProgram::for_rule_with_prelude(PreferNumericLiterals);
        let result = test.lint_dir(
            "prefer_numeric_literals/test_no_fix_for_invalid_radix_digit.ds",
            r#"
let value = Number.parseInt("102", 2);
"#,
        );
        test.result(result)
            .assert_lint("prefer-numeric-literals")
            .assert_has_no_fix("prefer-numeric-literals");
    }

    /// Keep lint without fix when the literal string includes a radix prefix.
    #[test]
    fn test_no_fix_for_prefixed_hex_string() {
        let test = TestProgram::for_rule_with_prelude(PreferNumericLiterals);
        let result = test.lint_dir(
            "prefer_numeric_literals/test_no_fix_for_prefixed_hex_string.ds",
            r#"
let value = Number.parseInt("0xFF", 16);
"#,
        );
        test.result(result)
            .assert_lint("prefer-numeric-literals")
            .assert_has_no_fix("prefer-numeric-literals");
    }

    /// Keep lint without fix when call contains comments.
    #[test]
    fn test_no_fix_when_parseint_contains_comments() {
        let test = TestProgram::for_rule_with_prelude(PreferNumericLiterals);
        let result = test.lint_dir(
            "prefer_numeric_literals/test_no_fix_when_parseint_contains_comments.ds",
            r#"
let value = parseInt(
  /* radix string */ "FF",
  16,
);
"#,
        );
        test.result(result)
            .assert_lint("prefer-numeric-literals")
            .assert_has_no_fix("prefer-numeric-literals");
    }

    /// Allow Number.parseInt with radix 10.
    #[test]
    fn test_allows_parseint_decimal() {
        let test = TestProgram::for_rule_with_prelude(PreferNumericLiterals);
        let result = test.lint_dir(
            "prefer_numeric_literals/test_allows_parseint_decimal.ds",
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
            "prefer_numeric_literals/test_allows_parseint_variable_string.ds",
            r#"
let hex = "FF";
let value = Number.parseInt(hex, 16);
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-numeric-literals");
    }

    /// Allow shadowed parseInt symbols.
    #[test]
    fn test_allows_shadowed_parseint() {
        let test = TestProgram::for_rule_with_prelude(PreferNumericLiterals);
        let result = test.lint_dir(
            "prefer_numeric_literals/test_allows_shadowed_parseint.ds",
            r#"
let parseInt = (value: string, radix: number): number => 0;
let value = parseInt("FF", 16);
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-numeric-literals");
    }

    /// Flag template string parseInt with static content.
    #[test]
    fn test_flags_template_parseint_hex() {
        let test = TestProgram::for_rule_with_prelude(PreferNumericLiterals);
        let result = test.lint_dir(
            "prefer_numeric_literals/test_flags_template_parseint_hex.ds",
            r#"
let value = parseInt(`FF`, 16);
"#,
        );
        test.result(result)
            .assert_lint("prefer-numeric-literals")
            .assert_has_fix("prefer-numeric-literals")
            .assert_safe_fixed(
                r#"
let value = 0xFF;
"#,
            );
    }

    /// Allow Number.parseInt with variable radix.
    #[test]
    fn test_allows_parseint_variable_radix() {
        let test = TestProgram::for_rule_with_prelude(PreferNumericLiterals);
        let result = test.lint_dir(
            "prefer_numeric_literals/test_allows_parseint_variable_radix.ds",
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
            "prefer_numeric_literals/test_allows_numeric_literals.ds",
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
