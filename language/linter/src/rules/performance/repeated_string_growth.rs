use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

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

Counted loops that append one invariant string can append a single `repeat` result.
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
        provenance: [],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report repeated concatenation into string storage that survives an iteration.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let binding_occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let access_occurrences = module.flows.access_occurrences().collect::<Vec<_>>();
    let mut repeated = FxIndexSet::default();
    let mut output = LintOutput::default();

    // replace counted loops that append one invariant string
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let Some(iteration) = module.counted_iteration(expression)? else {
            continue;
        };
        if iteration.start != 0
            || iteration.end_kind != dir::RangeEnd::Open
            || !module.is_duplicable_expression(iteration.end)?
        {
            continue;
        }
        if module
            .integral_constant(iteration.end)?
            .is_some_and(|end| iteration.count(end) <= 1)
        {
            continue;
        }

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
            || !module.is_speculatable_expression(assignment.target)?
            || !module.is_speculatable_expression(assignment.value)?
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

        // replace the complete loop and record its action
        repeated.insert(action);
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

    // inspect assignments within authored iteration bodies
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        if repeated.contains(&expression) {
            continue;
        }
        let Some(iteration) = module.enclosing_iteration(expression.into_any()) else {
            continue;
        };
        if let Some(iteration) = module.counted_iteration(iteration)?
            && let Some(end) = module.integral_constant(iteration.end)?
            && iteration.count(end) <= 1
        {
            continue;
        }
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
        if !module.is_stable_access(assignment.target, body.into_any(), &access_occurrences) {
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

/// Return whether one string concatenation includes the assigned place.
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

    /// Replace a dynamic counted append with one repeat call.
    #[test]
    fn test_replaces_dynamic_repeat() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
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

    /// Replace a classic counted append with one repeat call.
    #[test]
    fn test_replaces_counter_repeat() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
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

    /// Use a direct constant count for a fixed append.
    #[test]
    fn test_replaces_fixed_repeat() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
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
            &REPEATED_STRING_GROWTH,
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
    fn test_replaces_field_repeat() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
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

    /// Report growth whose appended value depends on the range binding.
    #[test]
    fn test_reports_counter_dependent_value() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
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

        session.assert_diagnostics(
            r#"
warning[repeated-string-growth]: string is rebuilt on every iteration
 ──▶ main.ds:4:9
  │
2 │     let output = "";
3 │     for (const index of 0..depth) {
4 │         output += index == 0 ? "a" : "b";
  │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
5 │     }
6 │
  │

 = help: append into a StringBuilder and convert it after the iteration
"#,
        );
    }

    /// Accept a counted loop that appends at most once.
    #[test]
    fn test_accepts_single_append() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
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

    /// Retain a loop whose appended value reads the growing string.
    #[test]
    fn test_reports_growing_value() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
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

        session.assert_diagnostics(
            r#"
warning[repeated-string-growth]: string is rebuilt on every iteration
 ──▶ main.ds:4:9
  │
2 │     let output = "x";
3 │     for (const _ of 0..depth) {
4 │         output += output;
  │         ^^^^^^^^^^^^^^^^
5 │     }
6 │
  │

 = help: append into a StringBuilder and convert it after the iteration
"#,
        );
    }

    /// Retain a counter loop whose bound reads the growing string.
    #[test]
    fn test_reports_growing_bound() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
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

        session.assert_diagnostics(
            r#"
warning[repeated-string-growth]: string is rebuilt on every iteration
 ──▶ main.ds:4:9
  │
2 │     let output = "x";
3 │     for (let index: isize = 0; index < output.length; index++) {
4 │         output += "x";
  │         ^^^^^^^^^^^^^
5 │     }
6 │
  │

 = help: append into a StringBuilder and convert it after the iteration
"#,
        );
    }

    /// Report growth when the loop counter contributes to its upper bound.
    #[test]
    fn test_reports_counter_dependent_bound() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
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

        session.assert_diagnostics(
            r#"
warning[repeated-string-growth]: string is rebuilt on every iteration
 ──▶ main.ds:4:9
  │
2 │     let output = "";
3 │     for (let index: isize = 0; index < index + 4; index++) {
4 │         output += "x";
  │         ^^^^^^^^^^^^^
5 │     }
6 │
  │

 = help: append into a StringBuilder and convert it after the iteration
"#,
        );
    }

    /// Retain a counted loop that performs another action.
    #[test]
    fn test_reports_additional_action() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
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

        session.assert_diagnostics(
            r#"
warning[repeated-string-growth]: string is rebuilt on every iteration
 ──▶ main.ds:6:9
  │
4 │     let output = "";
5 │     for (const _ of 0..depth) {
6 │         output += "  ";
  │         ^^^^^^^^^^^^^^
7 │         trace(output);
8 │     }
  │

 = help: append into a StringBuilder and convert it after the iteration
"#,
        );
    }

    /// Retain a loop comment and report only the diagnostic.
    #[test]
    fn test_retains_repeat_comment() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
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
warning[repeated-string-growth]: string is appended once per loop iteration
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
7 │
8 │     return output;
  │
"#,
        );
    }

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

    /// Accept concatenation into a different string on each iteration.
    #[test]
    fn test_accepts_varying_target() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
            r#"
function append(outputs: string[], values: string[]): void {
    for (const [index, value] of values.entries()) {
        outputs[index] += value;
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

    /// Accept an append after the target is reset on every iteration.
    #[test]
    fn test_accepts_reset_target() {
        let session = TestSession::dir(
            &REPEATED_STRING_GROWTH,
            r#"
function last(values: string[]): string {
    let output = "";
    for (const value of values) {
        output = "";
        output += value;
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
