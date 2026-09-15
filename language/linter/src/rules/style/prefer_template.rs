use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer template literals for string concatenation.
    pub PREFER_TEMPLATE {
        id: "prefer-template",
        summary: "Prefer template literals for string concatenation",
        explanation: r#"
String concatenation separates authored text from interpolated values with binary operators.
Instead, you SHOULD use one template literal containing the static and dynamic segments in their final order.
"#,
        example: {
            reported: r#"
function greet(name: string): string {
    return "hello, " + name;
}
"#,
            accepted: r#"
function greet(name: string): string {
    return `hello, ${name}`;
}
"#,
        },
        provenance: [Eslint("prefer-template")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report canonical string concatenation chains containing authored text.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect each outermost canonical string addition
    for expression in module.operator_expressions() {
        let expression = expression?;
        let node = view.get(expression);
        let dir::Expression::Binary {
            operator: dir::BinaryOperator::Add,
            ..
        } = node
        else {
            continue;
        };
        let string_add = dir::LanguageItem::String.member("add");
        if module.language_member(expression)? != Some(string_add) {
            continue;
        }

        // skip additions owned by a larger string-addition chain
        let parent = view.ancestor::<dir::Expression>(expression.into_any());
        let is_nested = match parent.map(|parent| (parent, view.get(parent))) {
            Some((
                parent,
                dir::Expression::Binary {
                    left,
                    operator: dir::BinaryOperator::Add,
                    right,
                },
            )) if *left == expression || *right == expression => {
                module.language_member(parent)? == Some(string_add)
            }
            _ => false,
        };
        if is_nested {
            continue;
        }

        // compose one template from the complete chain
        let span = module.source_extent(expression.into_any())?;
        let mut replacement = String::new();
        let mut retained = Vec::new();
        let (has_text, has_value) =
            append_template(module, expression, &mut replacement, &mut retained)?;
        if !has_text || !has_value {
            continue;
        }

        // replace the concatenation while retaining every dynamic expression
        let mut diagnostic = lint.diagnostic("string concatenation obscures a template", span);
        if let Some(suggestion) = suggestion(module, lint, span, retained, replacement)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        output.report(diagnostic);
    }

    Ok(output)
}

/// Append one string-addition subtree to a template body.
fn append_template(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
    template: &mut String,
    retained: &mut Vec<destack_source::Span>,
) -> Result<(bool, bool), ProviderError> {
    // flatten nested canonical string additions in evaluation order
    let node = module.view().get(expression);
    let string_add = dir::LanguageItem::String.member("add");
    if let dir::Expression::Binary {
        left,
        operator: dir::BinaryOperator::Add,
        right,
    } = node
        && module.language_member(expression)? == Some(string_add)
    {
        let (left, right) = (*left, *right);
        let left = append_template(module, left, template, retained)?;
        let right = append_template(module, right, template, retained)?;

        return Ok((left.0 || right.0, left.1 || right.1));
    }

    // append authored text as spelled, or retain one dynamic value
    let span = module.source_extent(expression.into_any())?;
    retained.push(span);
    if let Some(dir::Literal::String(_)) = module.view().get(expression).as_scalar() {
        let lexeme = module.source(span)?;
        let content = &lexeme[1..lexeme.len() - 1];
        append_template_text(content, template);

        Ok((true, false))
    } else {
        template.push_str("${");
        template.push_str(module.source(span)?);
        template.push('}');

        Ok((false, true))
    }
}

/// Append authored string content with template-only delimiters escaped.
fn append_template_text(source: &str, template: &mut String) {
    let mut preceding_backslashes = 0;
    let mut characters = source.chars().peekable();

    // retain authored escapes while protecting template delimiters
    while let Some(character) = characters.next() {
        let is_interpolation = character == '$' && characters.peek() == Some(&'{');
        let needs_escape = (character == '`' || is_interpolation) && preceding_backslashes % 2 == 0;
        if needs_escape {
            template.push('\\');
        }
        template.push(character);

        preceding_backslashes = if character == '\\' {
            preceding_backslashes + 1
        } else {
            0
        };
    }
}

/// Build one template literal from a complete concatenation chain.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: destack_source::Span,
    retained: Vec<destack_source::Span>,
    template: String,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    if module.has_unretained_comment(extent, &retained)? {
        return Ok(None);
    }

    // replace the complete concatenation
    let replacement = format!("`{template}`");
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.fix("use a template literal", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a dynamic value followed by literal text.
    #[test]
    fn test_replaces_trailing_literal() {
        let session = TestSession::dir(
            &PREFER_TEMPLATE,
            r#"
function greeting(name: string): string {
    return name + " says hello";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-template]: string concatenation obscures a template
 ──▶ main.ds:2:12
  │
1 │ function greeting(name: string): string {
2 │     return name + " says hello";
  │            ^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: use a template literal
--- a/main.ds
+++ b/main.ds

    1│ function greeting(name: string): string {
-   2│     return name + " says hello";
+   2│     return `${name} says hello`;
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function greeting(name: string): string {
    return `${name} says hello`;
}
"#,
        );
    }

    /// Escape interpolation syntax from the authored literal value.
    #[test]
    fn test_escapes_literal_interpolation_syntax() {
        let session = TestSession::dir(
            &PREFER_TEMPLATE,
            r#"
function placeholder(name: string): string {
    return "literal ${value}: " + name;
}
"#,
        );

        session.assert_fixes(
            r#"
function placeholder(name: string): string {
    return `literal \${value}: ${name}`;
}
"#,
        );
    }

    /// Preserve authored escapes already valid inside a template literal.
    #[test]
    fn test_preserves_literal_escapes() {
        let session = TestSession::dir(
            &PREFER_TEMPLATE,
            r#"
function describe(name: string): string {
    return "line\nliteral \${value}: " + name;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-template]: string concatenation obscures a template
 ──▶ main.ds:2:12
  │
1 │ function describe(name: string): string {
2 │     return "line\nliteral \${value}: " + name;
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: use a template literal
--- a/main.ds
+++ b/main.ds

    1│ function describe(name: string): string {
-   2│     return "line\nliteral \${value}: " + name;
+   2│     return `line\nliteral \${value}: ${name}`;
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function describe(name: string): string {
    return `line\nliteral \${value}: ${name}`;
}
"#,
        );
    }

    /// Accept concatenation between two dynamic strings.
    #[test]
    fn test_accepts_dynamic_concatenation() {
        let session = TestSession::dir(
            &PREFER_TEMPLATE,
            r#"
function join(left: string, right: string): string {
    return left + right;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve comments around the dynamic value by omitting the fix.
    #[test]
    fn test_reports_commented_concatenation_without_fix() {
        let session = TestSession::dir(
            &PREFER_TEMPLATE,
            r#"
function greet(name: string): string {
    return "hello, " + /* retain */ name;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-template]: string concatenation obscures a template
 ──▶ main.ds:2:12
  │
1 │ function greet(name: string): string {
2 │     return "hello, " + /* retain */ name;
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Replace a complete concatenation chain with one template.
    #[test]
    fn test_replaces_complete_concatenation_chain() {
        let session = TestSession::dir(
            &PREFER_TEMPLATE,
            r#"
function greeting(first: string, last: string): string {
    return "hello, " + first + " " + last + "!";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-template]: string concatenation obscures a template
 ──▶ main.ds:2:12
  │
1 │ function greeting(first: string, last: string): string {
2 │     return "hello, " + first + " " + last + "!";
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: use a template literal
--- a/main.ds
+++ b/main.ds

    1│ function greeting(first: string, last: string): string {
-   2│     return "hello, " + first + " " + last + "!";
+   2│     return `hello, ${first} ${last}!`;
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function greeting(first: string, last: string): string {
    return `hello, ${first} ${last}!`;
}
"#,
        );
    }
}
