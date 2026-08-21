use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer repeat over concatenating one string in a counted loop.
    pub MANUAL_REPEAT_STRING {
        id: "manual-repeat-string",
        summary: "Prefer repeat over concatenating one string in a counted loop",
        explanation: r#"
Concatenating the same string in a counted loop repeatedly allocates and copies the growing result.
Instead, you SHOULD call `repeat` once and append the complete repeated string.
"#,
        example: {
            reported: r#"
function indent(depth: isize): string {
    let output = "";
    for (const _ of 0..depth) {
        output += "  ";
    }
    return output;
}
"#,
            accepted: r#"
function indent(depth: isize): string {
    let output = "";
    output += "  ".repeat(depth > 0 ? depth : 0);
    return output;
}
"#,
        },
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report zero-based counted loops that only append one stable string.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let binding_occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let access_occurrences = module.flows.access_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect zero-based counted loops
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let Some(iteration) = module.counted_iteration(expression)? else {
            continue;
        };
        if iteration.start != 0
            || iteration.end_kind != dir::RangeEnd::Open
            || !module.is_repeatable_expression(iteration.end)?
        {
            continue;
        }
        if module
            .integral_constant(iteration.end)?
            .is_some_and(|count| count <= 1)
        {
            continue;
        }

        // require exactly one body action independent of the counter
        let Some(action) = view.get(iteration.body).only_expression() else {
            continue;
        };
        if iteration.binding.is_some_and(|binding| {
            let body_uses = module.binding_uses_within(
                binding,
                iteration.body.into_any(),
                &binding_occurrences,
            );
            let end_uses =
                module.binding_uses_within(binding, iteration.end.into_any(), &binding_occurrences);

            !body_uses.is_empty() || !end_uses.is_empty()
        }) {
            continue;
        }

        // require one stable builtin string append
        let Some(assignment) = module.place_assignment(action) else {
            continue;
        };
        if assignment.operator != dir::AssignOperator::AddAssign
            || module
                .dir
                .representation_item(module.node_type_id(assignment.target.into_any())?)?
                != Some(dir::LanguageItem::String)
            || !module.is_repeatable_expression(assignment.target)?
            || !module.is_repeatable_expression(assignment.value)?
        {
            continue;
        }
        let Some(access) = module.access_resolution(assignment.target) else {
            continue;
        };
        let end_uses =
            module.access_uses_within(access.path(), iteration.end.into_any(), &access_occurrences);
        if iteration.is_end_rechecked && !end_uses.is_empty() {
            continue;
        }
        let target_uses = module.access_uses_within(
            access.path(),
            assignment.value.into_any(),
            &access_occurrences,
        );
        if !target_uses.is_empty() {
            continue;
        }

        // replace the loop with one repeated append
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("string is appended once per loop iteration", span);
        if let Some(suggestion) = suggestion(
            module,
            lint,
            expression,
            assignment.target,
            assignment.value,
            iteration.end,
        )? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one repeated string append from a counted concatenation loop.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    target: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Expression>,
    count: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let target_extent = module.source_extent(target.into_any())?;
    let value_extent = module.source_extent(value.into_any())?;
    let count_extent = module.source_extent(count.into_any())?;
    if module.has_unretained_comment(extent, &[target_extent, value_extent, count_extent])? {
        return Ok(None);
    }

    // preserve each authored expression and guard a dynamic negative count
    let target = module.source(target_extent)?;
    let value = module.expression_source(value, dir::OperatorPrecedence::Postfix)?;
    let constant_count = module.integral_constant(count)?;
    let count = module
        .expression_source(count, dir::OperatorPrecedence::Comparison)?
        .into_owned();
    let count = if constant_count.is_some() {
        count
    } else {
        format!("{count} > 0 ? {count} : 0")
    };
    let replacement = format!("{target} += {value}.repeat({count});");
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.suggestion("append one repeated string", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a dynamic counted concatenation loop with repeat.
    #[test]
    fn test_replaces_dynamic_count() {
        let session = TestSession::dir(
            &MANUAL_REPEAT_STRING,
            r#"
function indent(depth: isize): string {
    let output = "";
    for (const _ of 0..depth) {
        output += "  ";
    }
    return output;
}
"#,
        );

        session.assert_suggestions(
            r#"
function indent(depth: isize): string {
    let output = "";
    output += "  ".repeat(depth > 0 ? depth : 0);
    return output;
}
"#,
        );
    }

    /// Replace a classic counted concatenation loop with repeat.
    #[test]
    fn test_replaces_counter_loop() {
        let session = TestSession::dir(
            &MANUAL_REPEAT_STRING,
            r#"
function indent(depth: isize): string {
    let output = "";
    for (let index: isize = 0; index < depth; index++) {
        output += "  ";
    }
    return output;
}
"#,
        );

        session.assert_suggestions(
            r#"
function indent(depth: isize): string {
    let output = "";
    output += "  ".repeat(depth > 0 ? depth : 0);
    return output;
}
"#,
        );
    }

    /// Replace a fixed counted concatenation loop without a count guard.
    #[test]
    fn test_replaces_fixed_count() {
        let session = TestSession::dir(
            &MANUAL_REPEAT_STRING,
            r#"
function divider(): string {
    let output = "";
    for (const _ of 0..4) {
        output += "-";
    }
    return output;
}
"#,
        );

        session.assert_suggestions(
            r#"
function divider(): string {
    let output = "";
    output += "-".repeat(4);
    return output;
}
"#,
        );
    }

    /// Replace a loop whose named range binding is unused.
    #[test]
    fn test_replaces_unused_range_binding() {
        let session = TestSession::dir(
            &MANUAL_REPEAT_STRING,
            r#"
function divider(): string {
    let output = "";
    for (const index of 0..4) {
        output += "-";
    }
    return output;
}
"#,
        );

        session.assert_suggestions(
            r#"
function divider(): string {
    let output = "";
    output += "-".repeat(4);
    return output;
}
"#,
        );
    }

    /// Replace repeated appends to stable field storage.
    #[test]
    fn test_replaces_field_append() {
        let session = TestSession::dir(
            &MANUAL_REPEAT_STRING,
            r#"
class Output {
    value: string = "";
}

function indent(output: Output, depth: isize): void {
    for (const _ of 0..depth) {
        output.value += "  ";
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
class Output {
    value: string = "";
}

function indent(output: Output, depth: isize): void {
    output.value += "  ".repeat(depth > 0 ? depth : 0);
}
"#,
        );
    }

    /// Accept a loop whose range binding contributes to the appended string.
    #[test]
    fn test_accepts_used_range_binding() {
        let session = TestSession::dir(
            &MANUAL_REPEAT_STRING,
            r#"
function sequence(depth: isize): string {
    let output = "";
    for (const index of 0..depth) {
        output += index == 0 ? "a" : "b";
    }
    return output;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a range with at most one iteration.
    #[test]
    fn test_accepts_single_append() {
        let session = TestSession::dir(
            &MANUAL_REPEAT_STRING,
            r#"
function divider(): string {
    let output = "";
    for (const _ of 0..1) {
        output += "-";
    }
    return output;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an append value derived from the growing target.
    #[test]
    fn test_accepts_growing_value() {
        let session = TestSession::dir(
            &MANUAL_REPEAT_STRING,
            r#"
function double(depth: isize): string {
    let output = "x";
    for (const _ of 0..depth) {
        output += output;
    }
    return output;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a counter loop whose bound grows with the target string.
    #[test]
    fn test_accepts_growing_bound() {
        let session = TestSession::dir(
            &MANUAL_REPEAT_STRING,
            r#"
function extend(): string {
    let output = "x";
    for (let index: isize = 0; index < output.length; index++) {
        output += "x";
    }
    return output;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a loop whose upper bound depends on its counter.
    #[test]
    fn test_accepts_counter_dependent_bound() {
        let session = TestSession::dir(
            &MANUAL_REPEAT_STRING,
            r#"
function extend(): string {
    let output = "";
    for (let index: isize = 0; index < index + 4; index++) {
        output += "x";
    }
    return output;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a loop with another action.
    #[test]
    fn test_accepts_additional_action() {
        let session = TestSession::dir(
            &MANUAL_REPEAT_STRING,
            r#"
declare function trace(value: string): void;

function indent(depth: isize): string {
    let output = "";
    for (const _ of 0..depth) {
        output += "  ";
        trace(output);
    }
    return output;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Retain comments without offering a destructive suggestion.
    #[test]
    fn test_retains_loop_comment() {
        let session = TestSession::dir(
            &MANUAL_REPEAT_STRING,
            r#"
function indent(depth: isize): string {
    let output = "";
    for (const _ of 0..depth) {
        // append one indentation level
        output += "  ";
    }
    return output;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-repeat-string]: string is appended once per loop iteration
 ──▶ main.ds:3:5
  │
1 │ function indent(depth: isize): string {
2 │     let output = "";
3 │     for (const _ of 0..depth) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^
4 │         // append one indentation level
  │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
5 │         output += "  ";
  │         ^^^^^^^^^^^^^^^
6 │     }
  │     ^
7 │     return output;
8 │ }
  │
"#,
        );
    }
}
