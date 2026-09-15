use destack_dir as dir;
use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow repeated accumulator spreads within iterations.
    pub NO_ACCUMULATING_SPREAD {
        id: "no-accumulating-spread",
        summary: "Disallow repeated accumulator spreads within iterations",
        explanation: r#"
Spreading an accumulator into its replacement copies every value accumulated so far.
Repeating the replacement in a loop or reduction callback can make total copying grow quadratically.
Instead, you SHOULD use an operation that extends storage without rebuilding the preceding values.

Aliasing can make in-place mutation observably different, so the diagnostic requires a manual rewrite.
"#,
        example: {
            reported: r#"
function copy(source: int32[]): int32[] {
    let output: int32[] = [];
    for (const value of source) {
        output = [...output, value];
    }

    return output;
}
"#,
            accepted: r#"
function copy(source: int32[]): int32[] {
    let output: int32[] = [];
    for (const value of source) {
        output.push(value);
    }

    return output;
}
"#,
        },
        provenance: [
            Biome("noAccumulatingSpread"),
            Oxc("no-accumulating-spread"),
        ],
        category: Performance,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report accumulator spreads in loops and reduction callbacks.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    report_loop_spreads(module, lint, &mut output)?;
    report_reduction_spreads(module, lint, &mut output)?;

    Ok(output)
}

/// Report loop assignments that spread their target into a replacement.
fn report_loop_spreads(
    module: &DirModule<'_>,
    lint: &Lint,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let view = module.view();

    // inspect direct assignments within authored iteration bodies
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
        if module.enclosing_callable(expression.into_any())
            != module.enclosing_callable(iteration.into_any())
        {
            continue;
        }
        let Some(assignment) = module.place_assignment(expression) else {
            continue;
        };
        if assignment.operator != dir::AssignOperator::Assign {
            continue;
        }

        // find the assigned place spread into its replacement
        let spread = accumulator_spread(module, assignment.value, |value| {
            module.is_same_computation(assignment.target, value)
        })?;
        let Some(spread) = spread else {
            continue;
        };

        let span = module.source_extent(spread)?;
        let diagnostic = lint
            .diagnostic("accumulator is copied on every iteration", span)
            .help("extend the accumulator without rebuilding its preceding values");
        output.report(diagnostic);
    }

    Ok(())
}

/// Report reduction callbacks that spread their accumulator into each result.
fn report_reduction_spreads(
    module: &DirModule<'_>,
    lint: &Lint,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let view = module.view();

    // inspect canonical Array and Iterator reduce callbacks
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        let member = module.language_member(expression)?;
        let is_reduction = member == Some(dir::LanguageItem::Array.member("reduce"))
            || member == Some(dir::LanguageItem::Iterator.member("reduce"));
        if call.is_optional() || !is_reduction {
            continue;
        }

        // require a direct synchronous callback
        let Some(callback) = call
            .arguments
            .first()
            .and_then(|argument| view.get(*argument).value())
        else {
            continue;
        };
        let Some(lambda) = module.lambda(callback) else {
            continue;
        };
        if lambda.signature.asynchrony != dir::Asynchrony::Sync || lambda.signature.is_generator {
            continue;
        }

        // select the accumulator binding and callback value
        let Some(parameter) = lambda.signature.parameters.first().copied() else {
            continue;
        };
        if !matches!(
            view.get(parameter),
            dir::Parameter::Named {
                default: None,
                is_optional: false,
                ..
            }
        ) {
            continue;
        }
        let Some(body) = lambda
            .body
            .and_then(|body| module.sole_value_expression(body))
        else {
            continue;
        };
        let accumulator = module.declaration_symbol(parameter)?;

        // find the accumulator spread into the callback result
        let spread = accumulator_spread(module, body, |value| {
            module
                .selected_symbol(value)
                .map(|symbol| symbol == Some(accumulator))
        })?;
        let Some(spread) = spread else {
            continue;
        };

        let span = module.source_extent(spread)?;
        let diagnostic = lint
            .diagnostic("reduction accumulator is copied on every iteration", span)
            .help("extend the accumulator without rebuilding its preceding values");
        output.report(diagnostic);
    }

    Ok(())
}

/// Return the matching value spread directly into one array or object expression.
fn accumulator_spread(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
    mut is_accumulator: impl FnMut(dir::LocalNodeId<dir::Expression>) -> Result<bool, ProviderError>,
) -> Result<Option<dir::LocalNodeIdAny>, ProviderError> {
    let view = module.view();

    match view.get(expression) {
        // inspect spread arguments of one array expression
        dir::Expression::ArrayExpression { elements } => {
            for element in elements {
                if let dir::Argument::Spread { value } = view.get(*element)
                    && is_accumulator(*value)?
                {
                    return Ok(Some(element.into_any()));
                }
            }
        }
        // inspect spread properties of one object expression
        dir::Expression::ObjectExpression { properties } => {
            for property in properties {
                if let dir::Property::Spread { value } = view.get(*property)
                    && is_accumulator(*value)?
                {
                    return Ok(Some(property.into_any()));
                }
            }
        }
        _ => {}
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a loop that spreads its array target into each replacement.
    #[test]
    fn test_reports_loop_accumulator() {
        let session = TestSession::dir(
            &NO_ACCUMULATING_SPREAD,
            r#"
function copy(source: int32[]): int32[] {
    let output: int32[] = [];
    for (const value of source) {
        output = [...output, value];
    }

    return output;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-accumulating-spread]: accumulator is copied on every iteration
 ──▶ main.ds:4:19
  │
2 │     let output: int32[] = [];
3 │     for (const value of source) {
4 │         output = [...output, value];
  │                   ^^^^^^^^^
5 │     }
6 │
  │

 = help: extend the accumulator without rebuilding its preceding values
"#,
        );
    }

    /// Report a reduction callback that spreads its accumulator.
    #[test]
    fn test_reports_reduction_accumulator() {
        let session = TestSession::dir(
            &NO_ACCUMULATING_SPREAD,
            r#"
function copy(source: int32[]): int32[] {
    return source.reduce<int32[]>((output, value) => [...output, value], []);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-accumulating-spread]: reduction accumulator is copied on every iteration
 ──▶ main.ds:2:55
  │
1 │ function copy(source: int32[]): int32[] {
2 │     return source.reduce<int32[]>((output, value) => [...output, value], []);
  │                                                       ^^^^^^^^^
3 │ }
  │

 = help: extend the accumulator without rebuilding its preceding values
"#,
        );
    }

    /// Report a loop that spreads its object target into each replacement.
    #[test]
    fn test_reports_object_loop_accumulator() {
        let session = TestSession::dir(
            &NO_ACCUMULATING_SPREAD,
            r#"
function retain(source: boolean[]): { active: boolean } {
    let output: { active: boolean } = { active: false };
    for (const value of source) {
        output = { ...output, active: value };
    }

    return output;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-accumulating-spread]: accumulator is copied on every iteration
 ──▶ main.ds:4:20
  │
2 │     let output: { active: boolean } = { active: false };
3 │     for (const value of source) {
4 │         output = { ...output, active: value };
  │                    ^^^^^^^^^
5 │     }
6 │
  │

 = help: extend the accumulator without rebuilding its preceding values
"#,
        );
    }

    /// Report a reduction callback that spreads its object accumulator.
    #[test]
    fn test_reports_object_reduction_accumulator() {
        let session = TestSession::dir(
            &NO_ACCUMULATING_SPREAD,
            r#"
function retain(source: boolean[]): { active: boolean } {
    return source.reduce<{ active: boolean }>(
        (output, value) => ({ ...output, active: value }),
        { active: false },
    );
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-accumulating-spread]: reduction accumulator is copied on every iteration
 ──▶ main.ds:3:31
  │
1 │ function retain(source: boolean[]): { active: boolean } {
2 │     return source.reduce<{ active: boolean }>(
3 │         (output, value) => ({ ...output, active: value }),
  │                               ^^^^^^^^^
4 │         { active: false },
5 │     );
  │

 = help: extend the accumulator without rebuilding its preceding values
"#,
        );
    }

    /// Accept a spread that does not contain the assigned target.
    #[test]
    fn test_accepts_other_spread() {
        let session = TestSession::dir(
            &NO_ACCUMULATING_SPREAD,
            r#"
function copy(source: int32[]): int32[] {
    let output: int32[] = [];
    for (const value of source) {
        output = [...source, value];
    }

    return output;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept one replacement outside an iteration.
    #[test]
    fn test_accepts_single_replacement() {
        let session = TestSession::dir(
            &NO_ACCUMULATING_SPREAD,
            r#"
function append(output: int32[], value: int32): int32[] {
    let result = output;
    result = [...result, value];

    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a spread in the initializer of a later iteration.
    #[test]
    fn test_accepts_loop_initializer() {
        let session = TestSession::dir(
            &NO_ACCUMULATING_SPREAD,
            r#"
function copy(input: int32[], condition: boolean): int32[] {
    let output = input;
    for (output = [...output]; condition;) {
        break;
    }

    return output;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report an Iterator reduction that spreads its accumulator.
    #[test]
    fn test_reports_iterator_reduction_accumulator() {
        let session = TestSession::dir(
            &NO_ACCUMULATING_SPREAD,
            r#"
import { Iterator } from "destack:iter";

function copy(source: Iterator<int32>): int32[] {
    return source.reduce<int32[]>((output, value) => [...output, value], []);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-accumulating-spread]: reduction accumulator is copied on every iteration
 ──▶ main.ds:4:55
  │
2 │
3 │ function copy(source: Iterator<int32>): int32[] {
4 │     return source.reduce<int32[]>((output, value) => [...output, value], []);
  │                                                       ^^^^^^^^^
5 │ }
  │

 = help: extend the accumulator without rebuilding its preceding values
"#,
        );
    }

    /// Accept a deferred spread nested within a loop.
    #[test]
    fn test_accepts_nested_callback_spread() {
        let session = TestSession::dir(
            &NO_ACCUMULATING_SPREAD,
            r#"
function callbacks(source: int32[]): (() => int32[])[] {
    let output: int32[] = [];
    let callbacks: (() => int32[])[] = [];
    for (const value of source) {
        callbacks.push(() => {
            output = [...output, value];

            return output;
        });
    }

    return callbacks;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
