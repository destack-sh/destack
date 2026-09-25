use std::cmp::Ordering;

use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch, PatchSet, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow for loops whose counter moves away from its bound.
    pub FOR_DIRECTION {
        id: "for-direction",
        summary: "Disallow for loops whose counter moves away from its bound",
        explanation: r#"
A counter that moves away from its relational bound cannot make a true loop condition false.
Instead, you MUST reverse the counter update or comparison direction.

An unconditional loop expresses intentional nontermination directly.
"#,
        example: {
            reported: r#"
for (let index: int32 = 0; index < 10; index--) {}
"#,
            accepted: r#"
for (let index: int32 = 0; index < 10; index++) {}
"#,
        },
        provenance: [Eslint("for-direction")],
        category: Correctness,
        level: Error,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One known counter movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    /// Move toward larger values.
    Increasing,
    /// Move toward smaller values.
    Decreasing,
}

impl Direction {
    /// Return the direction required by one relational condition.
    fn from_condition(operator: dir::BinaryOperator, is_counter_left: bool) -> Option<Self> {
        let direction = match (operator, is_counter_left) {
            (dir::BinaryOperator::LessThan | dir::BinaryOperator::LessThanOrEqual, true)
            | (dir::BinaryOperator::GreaterThan | dir::BinaryOperator::GreaterThanOrEqual, false) => {
                Self::Increasing
            }
            (dir::BinaryOperator::GreaterThan | dir::BinaryOperator::GreaterThanOrEqual, true)
            | (dir::BinaryOperator::LessThan | dir::BinaryOperator::LessThanOrEqual, false) => {
                Self::Decreasing
            }
            _ => return None,
        };

        Some(direction)
    }

    /// Return the direction of one nonzero scalar step.
    fn from_step(value: dir::Literal) -> Option<Self> {
        let ordering = match value {
            dir::Literal::Integer(value) | dir::Literal::Bigint(value) => value.cmp(&0),
            dir::Literal::Float(value) => value.partial_cmp(&0.0)?,
            _ => return None,
        };
        let direction = match ordering {
            Ordering::Greater => Self::Increasing,
            Ordering::Less => Self::Decreasing,
            Ordering::Equal => return None,
        };

        Some(direction)
    }

    /// Reverse this direction.
    fn reverse(self) -> Self {
        match self {
            Self::Increasing => Self::Decreasing,
            Self::Decreasing => Self::Increasing,
        }
    }
}

/// One counter update.
struct CounterUpdate<'a> {
    /// The updated storage path.
    access: &'a dir::AccessResolution,
    /// The known update direction.
    direction: Direction,
    /// The authored update operator.
    operator: Span,
    /// The operator that reverses this update.
    opposite_operator: &'static str,
}

impl<'a> CounterUpdate<'a> {
    /// Select one counter update.
    fn select(
        module: &'a DirModule<'_>,
        increment: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<Self>, ProviderError> {
        let view = module.view();
        let (target, direction, opposite_operator) = match view.get(increment) {
            dir::Expression::Unary { operator, right } => {
                let direction = match operator {
                    dir::UnaryOperator::PostIncrement | dir::UnaryOperator::PreIncrement => {
                        Direction::Increasing
                    }
                    dir::UnaryOperator::PostDecrement | dir::UnaryOperator::PreDecrement => {
                        Direction::Decreasing
                    }
                    _ => return Ok(None),
                };
                let opposite_operator = match direction {
                    Direction::Increasing => "--",
                    Direction::Decreasing => "++",
                };

                (*right, direction, opposite_operator)
            }
            dir::Expression::Assign { .. } => {
                let Some(assignment) = module.place_assignment(increment) else {
                    return Ok(None);
                };
                let Some(direction) = module
                    .scalar_constant(assignment.value)?
                    .and_then(Direction::from_step)
                else {
                    return Ok(None);
                };
                let (direction, opposite_operator) = match assignment.operator {
                    dir::AssignOperator::AddAssign => (direction, "-="),
                    dir::AssignOperator::SubtractAssign => (direction.reverse(), "+="),
                    _ => return Ok(None),
                };

                (assignment.target, direction, opposite_operator)
            }
            _ => return Ok(None),
        };

        // require builtin update behavior over one stable place
        let Some(resolution) = module.operator_decision(increment.into_any())? else {
            return Ok(None);
        };
        if !resolution.is_builtin() {
            return Ok(None);
        }
        let Some(access) = module.access_resolution(target) else {
            return Ok(None);
        };
        let operator = module.main_span(increment.into_any())?;

        Ok(Some(Self {
            access,
            direction,
            operator,
            opposite_operator,
        }))
    }

    /// Return the direction required by one stop condition.
    fn required_direction(
        &self,
        module: &DirModule<'_>,
        condition: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<Direction>, ProviderError> {
        let view = module.view();
        let dir::Expression::Binary {
            left,
            operator,
            right,
        } = view.get(condition)
        else {
            return Ok(None);
        };
        let Some(resolution) = module.operator_decision(condition.into_any())? else {
            return Ok(None);
        };
        if !resolution.is_builtin() {
            return Ok(None);
        }

        // identify one side as the updated storage path
        let is_left = module.access_resolution(*left) == Some(self.access);
        let is_right = module.access_resolution(*right) == Some(self.access);
        if is_left == is_right {
            return Ok(None);
        }
        let direction = Direction::from_condition(*operator, is_left);

        Ok(direction)
    }

    /// Build the review suggestion that reverses this update.
    fn suggestion(&self, lint: &Lint) -> Result<DiagnosticSuggestion, ProviderError> {
        let mut file = FilePatch::new(self.operator.file);
        file.replace(self.operator, self.opposite_operator);
        let patches = PatchSet::from_files(vec![file]);

        lint.suggestion("reverse the counter update", patches)
    }
}

/// Report for-loop counters whose known update opposes their stop condition.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect complete three-part for loops
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let dir::Expression::For {
            condition: Some(condition),
            increment: Some(increment),
            ..
        } = view.get(expression)
        else {
            continue;
        };
        let Some(update) = CounterUpdate::select(module, *increment)? else {
            continue;
        };
        let Some(required) = update.required_direction(module, *condition)? else {
            continue;
        };
        if update.direction == required {
            continue;
        }

        // report and offer the opposite update as one possible correction
        let diagnostic = lint
            .diagnostic(
                "loop counter moves away from its stop condition",
                update.operator,
            )
            .suggestion(update.suggestion(lint)?);
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Interpret the condition relative to a counter on the right.
    #[test]
    fn test_reports_reversed_counter_condition() {
        let session = TestSession::dir(
            &FOR_DIRECTION,
            r#"
for (let index: int32 = 0; 10 > index; index--) {}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[for-direction]: loop counter moves away from its stop condition
 ──▶ main.tspp:1:45
  │
1 │ for (let index: int32 = 0; 10 > index; index--) {}
  │                                             ^^
  │

 = suggestion: reverse the counter update (requires review)
--- a/main.tspp
+++ b/main.tspp

-   1│ for (let index: int32 = 0; 10 > index; index--) {}
+   1│ for (let index: int32 = 0; 10 > index; index++) {}
"#,
        );
        session.assert_suggestions(
            r#"
for (let index: int32 = 0; 10 > index; index++) {}
"#,
        );
    }

    /// Report an increment under a lower-bound condition.
    #[test]
    fn test_reports_increment_under_lower_bound() {
        let session = TestSession::dir(
            &FOR_DIRECTION,
            r#"
for (let index: int32 = 10; index >= 0; index++) {}
"#,
        );

        session.assert_suggestions(
            r#"
for (let index: int32 = 10; index >= 0; index--) {}
"#,
        );
    }

    /// Accept a decrement under a lower-bound condition.
    #[test]
    fn test_accepts_decrement_under_lower_bound() {
        let session = TestSession::dir(
            &FOR_DIRECTION,
            r#"
for (let index: int32 = 10; index >= 0; index--) {}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Read a named step whose type retains its exact value.
    #[test]
    fn test_reports_named_negative_step() {
        let session = TestSession::dir(
            &FOR_DIRECTION,
            r#"
const Step = -2;
for (let index: int32 = 0; index < 10; index += Step) {}
"#,
        );

        session.assert_suggestions(
            r#"
const Step = -2;
for (let index: int32 = 0; index < 10; index -= Step) {}
"#,
        );
    }

    /// Report subtraction that moves away from an upper bound.
    #[test]
    fn test_reports_subtraction_under_upper_bound() {
        let session = TestSession::dir(
            &FOR_DIRECTION,
            r#"
for (let index: int32 = 0; index < 10; index -= 1) {}
"#,
        );

        session.assert_suggestions(
            r#"
for (let index: int32 = 0; index < 10; index += 1) {}
"#,
        );
    }

    /// Accept subtraction that moves toward a lower bound.
    #[test]
    fn test_accepts_subtraction_under_lower_bound() {
        let session = TestSession::dir(
            &FOR_DIRECTION,
            r#"
for (let index: int32 = 10; index >= 0; index -= 1) {}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an update whose direction is unknown.
    #[test]
    fn test_accepts_unknown_step() {
        let session = TestSession::dir(
            &FOR_DIRECTION,
            r#"
function run(step: int32): void {
    for (let index: int32 = 10; index >= 0; index += step) {}
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a zero step because this rule compares directions only.
    #[test]
    fn test_accepts_zero_step() {
        let session = TestSession::dir(
            &FOR_DIRECTION,
            r#"
for (let index: int32 = 0; index < 10; index -= 0) {}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a condition that observes another storage path.
    #[test]
    fn test_accepts_condition_on_another_counter() {
        let session = TestSession::dir(
            &FOR_DIRECTION,
            r#"
let remaining: int32 = 10;
for (let index: int32 = 0; remaining > 0; index--) {}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept overloaded relational behavior.
    #[test]
    fn test_accepts_overloaded_comparison() {
        let session = TestSession::dir(
            &FOR_DIRECTION,
            r#"
import { Ordering, PartialCompare } from "tspp:ops";

struct Counter {
    value: int32;
}

extension of Counter implements PartialCompare<Counter> {
    equal(&readonly this, other: &readonly Counter): boolean {
        return this.value == other.value;
    }

    partialCompare(&readonly this, other: &readonly Counter): Ordering | null {
        return Ordering.Equal;
    }
}

let counter = Counter { value: 0 };
for (; counter < Counter { value: 10 }; counter.value++) {}
"#,
        );

        session.assert_no_diagnostics();
    }
}
