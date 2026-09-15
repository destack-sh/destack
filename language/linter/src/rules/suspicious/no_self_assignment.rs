use destack_dir as dir;
use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow assigning a stable place to itself.
    pub NO_SELF_ASSIGNMENT {
        id: "no-self-assignment",
        summary: "Disallow assigning a stable place to itself",
        explanation: r#"
Assigning a stable storage place to itself leaves its value unchanged and performs a useless write.
Instead, you SHOULD remove the assignment or correct the unintended operand.
"#,
        example: {
            reported: r#"
function retain(value: int32): int32 {
    let result = value;
    result = result;
    return result;
}
"#,
            accepted: r#"
function retain(value: int32): int32 {
    const result = value;
    return result;
}
"#,
        },
        provenance: [Clippy("self_assignment"), Eslint("no-self-assign")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report direct assignments that read and write one stable place.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect assignments that can write without transforming their source value
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Assign {
            left,
            operator,
            right,
        } = node
        else {
            continue;
        };
        if !matches!(
            operator,
            dir::AssignOperator::Assign
                | dir::AssignOperator::AndAssign
                | dir::AssignOperator::OrAssign
                | dir::AssignOperator::CoalesceAssign
        ) {
            continue;
        }

        // require one target projection to select its corresponding source storage
        let pattern = left.into_global(module.id);
        let source = right.into_global_any(module.id);
        if !contains_self_assignment(module, pattern, source)? {
            continue;
        }

        let span = module.span(expression.into_any())?;
        output.report(lint.diagnostic("assignment writes a value back to the same place", span));
    }

    Ok(output)
}

/// Return whether one assignment pattern writes a source access back to itself.
fn contains_self_assignment(
    module: &DirModule<'_>,
    pattern: dir::GlobalNodeId<dir::AssignPattern>,
    source: dir::GlobalNodeIdAny,
) -> Result<bool, ProviderError> {
    let decision = module
        .decisions
        .assign_pattern_decision(pattern.into_any())
        .ok_or_else(|| {
            ProviderError::internal(format!("assignment pattern {pattern:?} has no decision"))
        })?;

    match decision {
        // compare one direct target with its source projection
        dir::AssignPatternDecision::Place => {
            let dir::AssignPattern::Place { expression } = module.view().get(pattern.local_id)
            else {
                return Err(ProviderError::internal(format!(
                    "place decision belongs to non-place assignment pattern {pattern:?}"
                )));
            };

            Ok(module.is_same_access(expression.into_any(), source.local_id))
        }
        // preserve default evaluation because it can replace an undefined source value
        dir::AssignPatternDecision::Default(_) => Ok(false),
        // compare fixed and rest sequence projections
        dir::AssignPatternDecision::Sequence(sequence) => {
            if contains_projections(module, &sequence.fields)? {
                return Ok(true);
            }

            match &sequence.rest {
                Some(rest) => contains_projection(module, rest.source, rest.pattern),
                None => Ok(false),
            }
        }
        // compare fixed tuple projections
        dir::AssignPatternDecision::Tuple(tuple) => contains_projections(module, &tuple.fields),
        // compare fixed and rest object projections
        dir::AssignPatternDecision::Object(object) => {
            if contains_projections(module, &object.fields)? {
                return Ok(true);
            }

            match &object.rest {
                Some(rest) => contains_projection(module, rest.source, rest.pattern),
                None => Ok(false),
            }
        }
    }
}

/// Return whether any field projection writes its source access back to itself.
fn contains_projections(
    module: &DirModule<'_>,
    fields: &[dir::AssignPatternFieldResolution],
) -> Result<bool, ProviderError> {
    for field in fields {
        if contains_projection(module, field.source, field.pattern)? {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Return whether one projection writes its source access back to itself.
fn contains_projection(
    module: &DirModule<'_>,
    source: dir::GlobalNodeIdAny,
    pattern: Option<dir::GlobalNodeIdAny>,
) -> Result<bool, ProviderError> {
    let Some(pattern) = pattern else {
        return Ok(false);
    };
    let pattern = pattern
        .try_into_typed::<dir::AssignPattern>()
        .map_err(|node| {
            ProviderError::internal(format!(
                "assignment projection {source:?} targets non-pattern node {node:?}"
            ))
        })?;

    contains_self_assignment(module, pattern, source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a stored field self-assignment.
    #[test]
    fn test_reports_field_self_assignment() {
        let session = TestSession::dir(
            &NO_SELF_ASSIGNMENT,
            r#"
struct Point {
    x: int32;
}
function retain(point: Point): void {
    point.x = point.x;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-self-assignment]: assignment writes a value back to the same place
 ──▶ main.ds:5:5
  │
3 │ }
4 │ function retain(point: Point): void {
5 │     point.x = point.x;
  │     ^^^^^^^^^^^^^^^^^
6 │ }
  │
"#,
        );
    }

    /// Report a destructured field written from its corresponding source projection.
    #[test]
    fn test_reports_destructured_field_self_assignment() {
        let session = TestSession::dir(
            &NO_SELF_ASSIGNMENT,
            r#"
struct Point {
    x: int32;
}
function retain(point: Point): void {
    ({ x: point.x } = point);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-self-assignment]: assignment writes a value back to the same place
 ──▶ main.ds:5:6
  │
3 │ }
4 │ function retain(point: Point): void {
5 │     ({ x: point.x } = point);
  │      ^^^^^^^^^^^^^^^^^^^^^^
6 │ }
  │
"#,
        );
    }

    /// Accept assignment from a distinct binding.
    #[test]
    fn test_accepts_distinct_binding_assignment() {
        let session = TestSession::dir(
            &NO_SELF_ASSIGNMENT,
            r#"
function replace(current: int32, next: int32): int32 {
    let result = current;
    result = next;
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept compound assignment because it computes a new value.
    #[test]
    fn test_accepts_compound_self_assignment() {
        let session = TestSession::dir(
            &NO_SELF_ASSIGNMENT,
            r#"
function double(value: int32): int32 {
    let result = value;
    result += result;
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report logical assignments that conditionally write the same value.
    #[test]
    fn test_reports_logical_self_assignments() {
        let session = TestSession::dir(
            &NO_SELF_ASSIGNMENT,
            r#"
function retain(value: boolean | undefined): boolean | undefined {
    let result = value;
    result &&= result;
    result ||= result;
    result ??= result;
    return result;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-self-assignment]: assignment writes a value back to the same place
 ──▶ main.ds:3:5
  │
1 │ function retain(value: boolean | undefined): boolean | undefined {
2 │     let result = value;
3 │     result &&= result;
  │     ^^^^^^^^^^^^^^^^^
4 │     result ||= result;
5 │     result ??= result;
  │

warning[no-self-assignment]: assignment writes a value back to the same place
 ──▶ main.ds:4:5
  │
2 │     let result = value;
3 │     result &&= result;
4 │     result ||= result;
  │     ^^^^^^^^^^^^^^^^^
5 │     result ??= result;
6 │     return result;
  │

warning[no-self-assignment]: assignment writes a value back to the same place
 ──▶ main.ds:5:5
  │
3 │     result &&= result;
4 │     result ||= result;
5 │     result ??= result;
  │     ^^^^^^^^^^^^^^^^^
6 │     return result;
7 │ }
  │
"#,
        );
    }
}
