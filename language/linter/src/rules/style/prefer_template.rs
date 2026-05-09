use crate::LintMeta;
use destack_ast::{self as ast, BinaryOperator, Expression, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::rules::common::span_has_comment;
use crate::{LintAstContext, LintFix, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Prefer template literals over string concatenation.
    ///
    /// Use `` `Hello ${name}` `` instead of `"Hello " + name`.
    /// Template literals are more readable for string interpolation.
    #[lint(
        id = "prefer-template",
        code = "LY057",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferTemplate,
    "Prefer template literals for string concatenation"
}

impl LintRule for PreferTemplate {
    fn meta(&self) -> &'static LintMeta {
        PreferTemplate::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // match add expressions only
            let Expression::Binary {
                operator: BinaryOperator::Add,
                ..
            } = expression
            else {
                continue;
            };

            // report only on top level concat chains
            if is_nested_add_expression(ctx, node_id) {
                continue;
            }

            // require one string like part and one non string part
            let has_string_part = concat_has_string_part(ctx, node_id);
            let has_non_string_part = concat_has_non_string_part(ctx, node_id);
            if !(has_string_part && has_non_string_part) {
                continue;
            }

            // honor effective severity
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let expression_span = ctx.tree.get_span(node_id);
            let mut diagnostic = LintReport::new(
                PREFER_TEMPLATE.id,
                PREFER_TEMPLATE.code,
                PREFER_TEMPLATE.category,
                severity,
                "prefer template literal for string concatenation",
                expression_span,
            )
            .label("use template literal: `` `...${x}...` ``");

            // keep fix generation conservative for comments and unsupported numeric escapes
            if !span_has_comment(ctx.tree, expression_span)
                && !concat_has_unsupported_numeric_escape(ctx, node_id)
            {
                let template_body = concat_to_template_body(ctx, node_id);
                let replacement = format!("`{template_body}`");
                let edits = ctx
                    .edit_builder()
                    .replace(expression_span, replacement)
                    .into_edits();
                let fix = LintFix::safe("Convert to template literal").with_edits(edits);
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return true when one expression is a string literal or template literal.
fn is_string_expression(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::ScalarLiteral(ScalarLiteral::String(_)) | Expression::TemplateExpression { .. }
    )
}

/// Return true when this add expression is nested under another add expression.
fn is_nested_add_expression(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let Some(parent_id) = ctx.parents.get(expression_id) else {
        return false;
    };
    if ctx.tree.get_node_type(parent_id) != ast::NodeType::Expression {
        return false;
    }

    let parent_expression_id = ast::LocalNodeId::<ast::Expression>::new(parent_id);
    let parent_expression = ctx.tree.get(parent_expression_id);
    matches!(
        parent_expression,
        Expression::Binary {
            left,
            operator: BinaryOperator::Add,
            right,
        } if *left == expression_id || *right == expression_id
    )
}

/// Return true when one concat chain contains any string-like part.
fn concat_has_string_part(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expression = ctx.tree.get(expression_id);
    match expression {
        Expression::Binary {
            left,
            operator: BinaryOperator::Add,
            right,
        } => concat_has_string_part(ctx, *left) || concat_has_string_part(ctx, *right),
        _ => is_string_expression(expression),
    }
}

/// Return true when one concat chain contains any non-string part.
fn concat_has_non_string_part(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expression = ctx.tree.get(expression_id);
    match expression {
        Expression::Binary {
            left,
            operator: BinaryOperator::Add,
            right,
        } => concat_has_non_string_part(ctx, *left) || concat_has_non_string_part(ctx, *right),
        _ => !is_string_expression(expression),
    }
}

/// Return true when a concat chain contains unsupported numeric string escapes.
fn concat_has_unsupported_numeric_escape(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expression = ctx.tree.get(expression_id);
    match expression {
        Expression::Binary {
            left,
            operator: BinaryOperator::Add,
            right,
        } => {
            concat_has_unsupported_numeric_escape(ctx, *left)
                || concat_has_unsupported_numeric_escape(ctx, *right)
        }
        Expression::ScalarLiteral(ScalarLiteral::String(_)) => {
            let span = ctx.tree.get_span(expression_id);
            let text = ctx.get_span_text(span);
            string_literal_has_unsupported_numeric_escape(text)
        }
        _ => false,
    }
}

/// Return true when one string literal text contains legacy numeric escapes.
fn string_literal_has_unsupported_numeric_escape(literal_text: &str) -> bool {
    let mut chars = literal_text.chars().peekable();
    let mut backslash_run = 0usize;

    while let Some(character) = chars.next() {
        if character == '\\' {
            backslash_run += 1;
            continue;
        }

        let had_odd_backslash = backslash_run % 2 == 1;
        backslash_run = 0;
        if !had_odd_backslash {
            continue;
        }

        if matches!(character, '1'..='9') {
            return true;
        }

        if character == '0' && chars.peek().is_some_and(|next| next.is_ascii_digit()) {
            return true;
        }
    }

    false
}

/// Build one template body string from a concat chain expression.
fn concat_to_template_body(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> String {
    let expression = ctx.tree.get(expression_id);
    match expression {
        Expression::Binary {
            left,
            operator: BinaryOperator::Add,
            right,
        } => {
            let left_text = concat_to_template_body(ctx, *left);
            let right_text = concat_to_template_body(ctx, *right);
            format!("{left_text}{right_text}")
        }

        // keep scalar string values as plain template content
        Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
            let span = ctx.tree.get_span(expression_id);
            let literal_text = ctx.get_span_text(span);
            if let Some(template_text) = template_text_from_string_literal_source(literal_text) {
                return template_text;
            }

            let content = ctx.strings.get(*string_id).to_string();
            escape_for_template(&content)
        }

        // preserve inner template placeholders when merging template operands
        Expression::TemplateExpression { .. } => template_expression_body(ctx, expression_id),

        // wrap non-string operands in `${...}`
        _ => {
            let expression_span = ctx.tree.get_span(expression_id);
            let expression_text = ctx.get_span_text(expression_span);
            format!("${{{expression_text}}}")
        }
    }
}

/// Escape special characters for use in a template literal.
fn escape_for_template(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace("${", "\\${")
}

/// Convert one source string literal token into template-literal-safe text.
fn template_text_from_string_literal_source(literal_text: &str) -> Option<String> {
    let mut chars = literal_text.chars();
    let quote = chars.next()?;
    if !matches!(quote, '\'' | '"') {
        return None;
    }
    if !literal_text.ends_with(quote) || literal_text.len() < 2 {
        return None;
    }

    let inner = &literal_text[1..literal_text.len() - 1];
    let inner_bytes = inner.as_bytes();
    let mut output = String::with_capacity(inner.len() + 8);
    let mut index = 0usize;

    while index < inner.len() {
        let tail = &inner[index..];
        let mut tail_chars = tail.chars();
        let character = tail_chars.next()?;
        let character_len = character.len_utf8();

        // unescape the source-quote escape that is no longer needed in templates
        if character == '\\'
            && let Some(next_character) = tail_chars.next()
            && next_character == quote
        {
            output.push(quote);
            index += character_len + next_character.len_utf8();
            continue;
        }

        // escape backticks unless they are already escaped by an odd backslash run
        if character == '`' {
            if count_backslashes_before(inner_bytes, index).is_multiple_of(2) {
                output.push('\\');
            }
            output.push('`');
            index += character_len;
            continue;
        }

        // escape `${` unless it is already escaped by an odd backslash run
        if character == '$' && tail.as_bytes().get(1) == Some(&b'{') {
            if count_backslashes_before(inner_bytes, index).is_multiple_of(2) {
                output.push('\\');
            }
            output.push('$');
            output.push('{');
            index += 2;
            continue;
        }

        // preserve non-special characters as-is
        output.push(character);
        index += character_len;
    }

    Some(output)
}

/// Count backslashes immediately before a byte index in one source string body.
fn count_backslashes_before(text: &[u8], mut index: usize) -> usize {
    let mut backslashes = 0usize;
    while index > 0 && text[index - 1] == b'\\' {
        backslashes += 1;
        index -= 1;
    }

    backslashes
}

/// Return the inner content of one template literal expression.
fn template_expression_body(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> String {
    let span = ctx.tree.get_span(expression_id);
    let text = ctx.get_span_text(span);
    if text.starts_with('`') && text.ends_with('`') && text.len() >= 2 {
        return text[1..text.len() - 1].to_string();
    }

    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_string_concat() {
        let test = TestProgram::for_rule_without_prelude(PreferTemplate);
        let result = test.lint_ast(
            "prefer_template/test_detects_string_concat.ds",
            r#"
const greeting = "Hello " + name
"#,
        );
        test.result(result).assert_lint("prefer-template");
    }

    #[test]
    fn test_detects_concat_with_string_on_right() {
        let test = TestProgram::for_rule_without_prelude(PreferTemplate);
        let result = test.lint_ast(
            "prefer_template/test_detects_concat_with_string_on_right.ds",
            r#"
const greeting = name + " says hi"
"#,
        );
        test.result(result).assert_lint("prefer-template");
    }

    #[test]
    fn test_allows_template_literal() {
        let test = TestProgram::for_rule_without_prelude(PreferTemplate);
        let result = test.lint_ast(
            "prefer_template/test_allows_template_literal.ds",
            r#"
const greeting = `Hello ${name}`
"#,
        );
        test.result(result).assert_no_lint("prefer-template");
    }

    #[test]
    fn test_allows_number_addition() {
        let test = TestProgram::for_rule_without_prelude(PreferTemplate);
        let result = test.lint_ast(
            "prefer_template/test_allows_number_addition.ds",
            r#"
const sum = a + b
"#,
        );
        test.result(result).assert_no_lint("prefer-template");
    }

    #[test]
    fn test_allows_two_strings() {
        let test = TestProgram::for_rule_without_prelude(PreferTemplate);
        let result = test.lint_ast(
            "prefer_template/test_allows_two_strings.ds",
            r#"
const x = "hello" + "world"
"#,
        );
        test.result(result).assert_no_lint("prefer-template");
    }

    #[test]
    fn test_fix_string_on_left() {
        let test = TestProgram::for_rule_without_prelude(PreferTemplate);
        let result = test.lint_ast(
            "prefer_template/test_fix_string_on_left.ds",
            r#"
const greeting = "Hello " + name
"#,
        );
        test.result(result)
            .assert_lint("prefer-template")
            .assert_safe_fixed(
                r#"
const greeting = `Hello ${name}`;
"#,
            );
    }

    #[test]
    fn test_fix_string_on_right() {
        let test = TestProgram::for_rule_without_prelude(PreferTemplate);
        let result = test.lint_ast(
            "prefer_template/test_fix_string_on_right.ds",
            r#"
const greeting = name + " says hi"
"#,
        );
        test.result(result)
            .assert_lint("prefer-template")
            .assert_safe_fixed(
                r#"
const greeting = `${name} says hi`;
"#,
            );
    }

    #[test]
    fn test_fix_concat_chain_with_string_ends() {
        let test = TestProgram::for_rule_without_prelude(PreferTemplate);
        let result = test.lint_ast(
            "prefer_template/test_fix_concat_chain_with_string_ends.ds",
            r#"
const greeting = "Hello " + name + "!"
"#,
        );
        test.result(result)
            .assert_lint("prefer-template")
            .assert_safe_fixed(
                r#"
const greeting = `Hello ${name}!`;
"#,
            );
    }

    #[test]
    fn test_fix_concat_chain_with_expression_ends() {
        let test = TestProgram::for_rule_without_prelude(PreferTemplate);
        let result = test.lint_ast(
            "prefer_template/test_fix_concat_chain_with_expression_ends.ds",
            r#"
const summary = left + ":" + right
"#,
        );
        test.result(result)
            .assert_lint("prefer-template")
            .assert_safe_fixed(
                r#"
const summary = `${left}:${right}`;
"#,
            );
    }

    #[test]
    fn test_fix_preserves_template_placeholders() {
        let test = TestProgram::for_rule_without_prelude(PreferTemplate);
        let result = test.lint_ast(
            "prefer_template/test_fix_preserves_template_placeholders.ds",
            r#"
const summary = prefix + `${value} units`
"#,
        );
        test.result(result)
            .assert_lint("prefer-template")
            .assert_safe_fixed(
                r#"
const summary = `${prefix}${value} units`;
"#,
            );
    }

    #[test]
    fn test_fix_preserves_template_placeholders_on_left() {
        let test = TestProgram::for_rule_without_prelude(PreferTemplate);
        let result = test.lint_ast(
            "prefer_template/test_fix_preserves_template_placeholders_on_left.ds",
            r#"
const summary = `${value} units` + suffix
"#,
        );
        test.result(result)
            .assert_lint("prefer-template")
            .assert_safe_fixed(
                r#"
const summary = `${value} units${suffix}`;
"#,
            );
    }

    #[test]
    fn test_detects_unsupported_numeric_escape_sequences() {
        assert!(string_literal_has_unsupported_numeric_escape("'\\033'"));
        assert!(string_literal_has_unsupported_numeric_escape("'\\8'"));
        assert!(!string_literal_has_unsupported_numeric_escape("'\\\\033'"));
    }

    #[test]
    fn test_fix_escapes_unescaped_template_placeholder_text() {
        let test = TestProgram::for_rule_without_prelude(PreferTemplate);
        let result = test.lint_ast(
            "prefer_template/test_fix_escapes_unescaped_template_placeholder_text.ds",
            r#"
const summary = '0 backslashes: ${bar}' + suffix
"#,
        );
        test.result(result)
            .assert_lint("prefer-template")
            .assert_safe_fixed(
                r#"
const summary = `0 backslashes: \${bar}${suffix}`;
"#,
            );
    }

    #[test]
    fn test_fix_preserves_escaped_template_placeholder_text() {
        let test = TestProgram::for_rule_without_prelude(PreferTemplate);
        let result = test.lint_ast(
            "prefer_template/test_fix_preserves_escaped_template_placeholder_text.ds",
            r#"
const summary = '1 backslash: \${bar}' + suffix
"#,
        );
        test.result(result)
            .assert_lint("prefer-template")
            .assert_safe_fixed(
                r#"
const summary = `1 backslash: \${bar}${suffix}`;
"#,
            );
    }

    #[test]
    fn test_no_fix_when_concat_contains_comments() {
        let test = TestProgram::for_rule_without_prelude(PreferTemplate);
        let result = test.lint_ast(
            "prefer_template/test_no_fix_when_concat_contains_comments.ds",
            r#"
const summary = "left" /* side */ + value
"#,
        );
        test.result(result)
            .assert_lint("prefer-template")
            .assert_has_no_fix("prefer-template");
    }
}
