use destack_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow repeatedly rebuilding a growing string in an iteration.
    pub REPEATED_STRING_GROWTH {
        id: "repeated-string-growth",
        summary: "Disallow repeatedly rebuilding a growing string in an iteration",
        explanation: r#"
Concatenating into storage that survives an iteration repeatedly copies the text accumulated so far.
Instead, you SHOULD append into a `StringBuilder` and convert it into a string after the iteration.
"#,
        example: {
            reported: r#"
function join(values: string[]): string {
    let output = "";
    for (const value of values) {
        output += value;
    }

    return output;
}
"#,
            accepted: r#"
import { StringBuilder } from "destack:string";

function join(values: string[]): string {
    const output = StringBuilder.new();
    for (const value of values) {
        output.append(value.borrow());
    }

    return output.toString();
}
"#,
        },
        category: Performance,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report repeated concatenation into string storage that survives an iteration.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect assignments within authored iteration bodies
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let Some(iteration) = module.enclosing_iteration(expression.into_any()) else {
            continue;
        };
        let Some(body) = module.iteration_body(iteration) else {
            continue;
        };
        if !view.is_inside(expression.into_any(), body.into_any()) {
            continue;
        }
        let Some(assignment) = module.place_assignment(expression) else {
            continue;
        };

        // require one stable string place created outside the iteration
        if module
            .dir
            .representation_item(module.node_type_id(assignment.target.into_any())?)?
            != Some(dir::LanguageItem::String)
        {
            continue;
        }
        let Some(access) = module.access_resolution(assignment.target) else {
            continue;
        };
        if let dir::AccessRoot::Symbol(symbol) = access.path().root()
            && symbol.module_id == module.id
        {
            let declaration = module.symbol_declaration(symbol)?.local_id;
            if view.is_inside(declaration, iteration.into_any()) {
                continue;
            }
        }

        // require concatenation with the previous value
        let is_repeated = match assignment.operator {
            dir::AssignOperator::AddAssign => true,
            dir::AssignOperator::Assign => {
                concatenates_target(module, assignment.value, assignment.target)?
            }
            _ => false,
        };
        if !is_repeated {
            continue;
        }

        let span = module.source_extent(expression.into_any())?;
        let diagnostic = lint
            .diagnostic("string is rebuilt on every iteration", span)
            .help("append into a StringBuilder and convert it after the iteration");
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return whether one checked string concatenation includes the assigned place.
fn concatenates_target(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
    target: dir::LocalNodeId<dir::Expression>,
) -> Result<bool, destack_repository::ProviderError> {
    let view = module.view();

    // descend through canonical string addition
    let string_add = dir::LanguageItem::String.member("add");
    if let dir::Expression::Binary {
        left,
        operator: dir::BinaryOperator::Add,
        right,
    } = view.get(expression)
        && module.language_member(expression)? == Some(string_add)
    {
        let contains_left = module.is_same_computation(*left, target)?
            || concatenates_target(module, *left, target)?;
        let contains_right = module.is_same_computation(*right, target)?
            || concatenates_target(module, *right, target)?;

        return Ok(contains_left || contains_right);
    }

    // descend through canonical String.concat calls
    let string_concat = dir::LanguageItem::String.member("concat");
    if let Some(call) = module.member_call(expression)
        && !call.is_optional()
        && module.language_member(expression)? == Some(string_concat)
    {
        if module.is_same_computation(call.receiver, target)?
            || concatenates_target(module, call.receiver, target)?
        {
            return Ok(true);
        }
        for argument in call.arguments {
            let Some(value) = view.get(*argument).value() else {
                continue;
            };
            if module.is_same_computation(value, target)?
                || concatenates_target(module, value, target)?
            {
                return Ok(true);
            }
        }

        return Ok(false);
    }

    // inspect interpolations in an untagged string template
    let dir::Expression::TemplateExpression {
        value: dir::TemplateLiteral::InterpolatedString { arguments, .. },
    } = view.get(expression)
    else {
        return Ok(false);
    };
    for argument in arguments {
        let Some(value) = view.get(*argument).value() else {
            continue;
        };
        if module.is_same_computation(value, target)? {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report compound concatenation into a string declared outside a for-of loop.
    #[test]
    fn test_reports_compound_assignment() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
            r#"
function join(values: string[]): string {
    let output = "";
    for (const value of values) {
        output += value;
    }

    return output;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[repeated-string-growth]: string is rebuilt on every iteration
 ──▶ main.ds:4:9
  │
2 │     let output = "";
3 │     for (const value of values) {
4 │         output += value;
  │         ^^^^^^^^^^^^^^^
5 │     }
6 │
  │

 = help: append into a StringBuilder and convert it after the iteration
"#,
        );
    }

    /// Report an expanded concatenation whose target is nested in the string operation.
    #[test]
    fn test_reports_expanded_assignment() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
            r#"
function join(values: string[]): string {
    let output = "";
    for (const value of values) {
        output = "[" + output + value;
    }

    return output;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[repeated-string-growth]: string is rebuilt on every iteration
 ──▶ main.ds:4:9
  │
2 │     let output = "";
3 │     for (const value of values) {
4 │         output = "[" + output + value;
  │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
5 │     }
6 │
  │

 = help: append into a StringBuilder and convert it after the iteration
"#,
        );
    }

    /// Report an interpolated replacement of a field that survives the loop.
    #[test]
    fn test_reports_template_field_assignment() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
            r#"
class Output {
    value: string = "";
}

function append(output: Output, values: string[]): void {
    for (const value of values) {
        output.value = `${output.value}${value}`;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[repeated-string-growth]: string is rebuilt on every iteration
 ──▶ main.ds:7:9
  │
5 │ function append(output: Output, values: string[]): void {
6 │     for (const value of values) {
7 │         output.value = `${output.value}${value}`;
  │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
8 │     }
9 │ }
  │

 = help: append into a StringBuilder and convert it after the iteration
"#,
        );
    }

    /// Report a canonical concat call that includes the preceding string.
    #[test]
    fn test_reports_concat_assignment() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
            r#"
function join(values: string[]): string {
    let output = "";
    for (const value of values) {
        output = output.concat(value);
    }

    return output;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[repeated-string-growth]: string is rebuilt on every iteration
 ──▶ main.ds:4:9
  │
2 │     let output = "";
3 │     for (const value of values) {
4 │         output = output.concat(value);
  │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
5 │     }
6 │
  │

 = help: append into a StringBuilder and convert it after the iteration
"#,
        );
    }

    /// Accept a string created anew within each iteration.
    #[test]
    fn test_accepts_iteration_local_string() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
            r#"
function write(values: string[]): void {
    for (const value of values) {
        let output = "";
        output += value;
        console.log(output);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept replacing a string without retaining its previous value.
    #[test]
    fn test_accepts_replacement() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
            r#"
function last(values: string[]): string {
    let output = "";
    for (const value of values) {
        output = value;
    }

    return output;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept assigning the existing string without concatenation.
    #[test]
    fn test_accepts_self_assignment() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
            r#"
function retain(values: string[]): string {
    let output = "";
    for (const _ of values) {
        output = output;
    }

    return output;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
