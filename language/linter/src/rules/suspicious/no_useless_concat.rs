use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow concatenating adjacent string literals.
    pub NO_USELESS_CONCAT {
        id: "no-useless-concat",
        summary: "Disallow concatenating adjacent string literals",
        explanation: r#"
Concatenating adjacent string literals produces a value that is already known in the source.
Instead, you SHOULD write the value as one string or template literal.
"#,
        example: {
            reported: r#"
function message(): string {
    return "hello, " + "world";
}
"#,
            accepted: r#"
function message(): string {
    return "hello, world";
}
"#,
        },
        provenance: [Eslint("no-useless-concat")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report canonical string concatenation between two authored literals.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect builtin addition between two literal strings
    for expression in module.operator_expressions() {
        let expression = expression?;
        let node = view.get(expression);
        let dir::Expression::Binary {
            left,
            operator: dir::BinaryOperator::Add,
            right,
        } = node
        else {
            continue;
        };
        let is_left_string = matches!(view.get(*left).as_scalar(), Some(dir::Literal::String(_)));
        let is_right_string = matches!(view.get(*right).as_scalar(), Some(dir::Literal::String(_)));
        if !is_left_string || !is_right_string {
            continue;
        }
        let string_add = dir::LanguageItem::String.member("add");
        if module.language_member(expression)? != Some(string_add) {
            continue;
        }

        let span = module.source_extent(expression.into_any())?;
        let diagnostic = lint
            .diagnostic("adjacent string literals are concatenated", span)
            .help("write the constant text as one string or template literal");
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report concatenation between a string and a plain template literal.
    #[test]
    fn test_reports_string_and_template_literals() {
        let session = TestSession::dir(
            &NO_USELESS_CONCAT,
            r#"
function message(): string {
    return "hello, " + `world`;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-useless-concat]: adjacent string literals are concatenated
 ──▶ main.tspp:2:12
  │
1 │ function message(): string {
2 │     return "hello, " + `world`;
  │            ^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = help: write the constant text as one string or template literal
"#,
        );
    }

    /// Accept concatenation with a dynamic string value.
    #[test]
    fn test_accepts_dynamic_string_operand() {
        let session = TestSession::dir(
            &NO_USELESS_CONCAT,
            r#"
function greet(name: string): string {
    return "hello, " + name;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an interpolated template literal without an added literal operand.
    #[test]
    fn test_accepts_interpolated_template() {
        let session = TestSession::dir(
            &NO_USELESS_CONCAT,
            r#"
function greet(name: string): string {
    return `hello, ${name}`;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
