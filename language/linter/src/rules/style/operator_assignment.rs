use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require compound assignment where equivalent.
    pub OPERATOR_ASSIGNMENT {
        id: "operator-assignment",
        summary: "Require compound assignment where equivalent",
        explanation: r#"
`place = place + value` reads and writes the same stable place and is equivalent to `place += value`.
Instead, you SHOULD use the corresponding compound assignment.
"#,
        example: {
            reported: r#"
function advance(value: int32): int32 {
    let result = value;
    result = result + 1;
    return result;
}
"#,
            accepted: r#"
function advance(value: int32): int32 {
    let result = value;
    result += 1;
    return result;
}
"#,
        },
        provenance: [Eslint("operator-assignment")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report assignments that repeat one stable place as a selected binary operand.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect plain assignments whose value is a compound operation
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let Some(assignment) = module.place_assignment(expression) else {
            continue;
        };
        if assignment.operator != dir::AssignOperator::Assign {
            continue;
        }
        let dir::Expression::Binary {
            left: repeated,
            operator,
            right: value,
        } = view.get(assignment.value)
        else {
            continue;
        };
        let Ok(operator) = dir::AssignOperator::try_from(*operator) else {
            continue;
        };

        // require one selected operation and one duplicable target
        if module
            .operator_decision(assignment.value.into_any())?
            .is_none()
        {
            continue;
        }
        if !module.is_same_computation(assignment.target, *repeated)? {
            continue;
        }

        // report the expanded assignment and preserve all retained source
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("assignment repeats its target", span);
        if let Some(suggestion) = suggestion(
            module,
            lint,
            expression,
            assignment.target,
            *value,
            operator.text(),
        )? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build the exact compound assignment replacement.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    target: dir::LocalNodeId<dir::Expression>,
    value: dir::LocalNodeId<dir::Expression>,
    operator: &str,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let target = module.source_extent(target.into_any())?;
    let value = module.source_extent(value.into_any())?;

    // do not discard comments outside the retained target and value
    if module.has_unretained_comment(extent, &[target, value])? {
        return Ok(None);
    }

    // retain the exact authored operands
    let replacement = format!(
        "{} {operator} {}",
        module.source(target)?,
        module.source(value)?
    );
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.fix("use compound assignment", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a repeated field update over one stable receiver.
    #[test]
    fn test_replaces_repeated_field_update() {
        let session = TestSession::dir(
            &OPERATOR_ASSIGNMENT,
            r#"
struct Counter {
    value: int32;
}
function advance(counter: Counter): void {
    counter.value = counter.value * 2;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[operator-assignment]: assignment repeats its target
 ──▶ main.tspp:5:5
  │
3 │ }
4 │ function advance(counter: Counter): void {
5 │     counter.value = counter.value * 2;
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
6 │ }
  │

 = fix: use compound assignment
--- a/main.tspp
+++ b/main.tspp

    4│ function advance(counter: Counter): void {
-   5│     counter.value = counter.value * 2;
+   5│     counter.value *= 2;
    6│ }
"#,
        );
        session.assert_fixes(
            r#"
struct Counter {
    value: int32;
}
function advance(counter: Counter): void {
    counter.value *= 2;
}
"#,
        );
    }

    /// Accept an operation whose first operand is a distinct place.
    #[test]
    fn test_accepts_distinct_binary_operand() {
        let session = TestSession::dir(
            &OPERATOR_ASSIGNMENT,
            r#"
function replace(current: int32, next: int32): int32 {
    let result = current;
    result = next + 1;
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Do not commute operands to manufacture compound assignment.
    #[test]
    fn test_accepts_repeated_second_operand() {
        let session = TestSession::dir(
            &OPERATOR_ASSIGNMENT,
            r#"
function advance(value: int32): int32 {
    let result = value;
    result = 1 + result;
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace an update through an overloaded operator.
    #[test]
    fn test_replaces_overloaded_operator_update() {
        let session = TestSession::dir(
            &OPERATOR_ASSIGNMENT,
            r#"
import { Add } from "tspp:ops";

struct Counter {
    value: int32;
}

extension of Counter implements Add<Counter> {
    type Output = Counter;
    add(other: Counter): Counter {
        return other;
    }
}

function advance(current: Counter, next: Counter): Counter {
    let result = current;
    result = result + next;
    return result;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[operator-assignment]: assignment repeats its target
  ──▶ main.tspp:16:5
   │
14 │ function advance(current: Counter, next: Counter): Counter {
15 │     let result = current;
16 │     result = result + next;
   │     ^^^^^^^^^^^^^^^^^^^^^^
17 │     return result;
18 │ }
   │

 = fix: use compound assignment
--- a/main.tspp
+++ b/main.tspp

   15│     let result = current;
-  16│     result = result + next;
+  16│     result += next;
   17│     return result;
"#,
        );
        session.assert_fixes(
            r#"
import { Add } from "tspp:ops";

struct Counter {
    value: int32;
}

extension of Counter implements Add<Counter> {
    type Output = Counter;
    add(other: Counter): Counter {
        return other;
    }
}

function advance(current: Counter, next: Counter): Counter {
    let result = current;
    result += next;
    return result;
}
"#,
        );
    }
}
