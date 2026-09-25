use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer Array.popIf over conditionally popping the last element.
    pub MANUAL_POP_IF {
        id: "manual-pop-if",
        summary: "Prefer Array.popIf over conditionally popping the last element",
        explanation: r#"
Testing an array's last element before popping it performs two separate accesses to the same element.
Instead, you SHOULD use `popIf` to test and remove the last element together.
"#,
        example: {
            reported: r#"
function popExpected(values: int32[], expected: int32): int32 | undefined {
    return values.last() === expected ? values.pop() : undefined;
}
"#,
            accepted: r#"
function popExpected(values: int32[], expected: int32): int32 | undefined {
    return values.popIf((value) => value === expected);
}
"#,
        },
        provenance: [Clippy("manual_pop_if")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report conditional array pops guarded by the same array's last value.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect value conditionals with an undefined fallback
    for (expression, node) in module.view().iter_nodes::<dir::Expression>() {
        let dir::Expression::If {
            condition,
            then_expression,
            else_expression: Some(else_expression),
            ..
        } = node
        else {
            continue;
        };
        let Some(condition) = condition.as_expression() else {
            continue;
        };
        let Some(popped) = module.sole_value_expression(*then_expression) else {
            continue;
        };
        let Some(fallback) = module.sole_value_expression(*else_expression) else {
            continue;
        };
        if module.view().get(fallback).as_scalar() != Some(dir::Literal::Undefined) {
            continue;
        }
        let Some((receiver, predicate)) = pop_if(module, condition, popped)? else {
            continue;
        };

        // replace the complete conditional when every comment survives
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic =
            lint.diagnostic("array pop repeats a preceding last-element test", span);
        if let Some(suggestion) = suggestion(module, lint, span, receiver, predicate)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select the receiver and predicate of a manual popIf operation.
fn pop_if(
    module: &DirModule<'_>,
    condition: dir::LocalNodeId<dir::Expression>,
    popped: dir::LocalNodeId<dir::Expression>,
) -> Result<
    Option<(
        dir::LocalNodeId<dir::Expression>,
        dir::LocalNodeId<dir::Expression>,
    )>,
    ProviderError,
> {
    let Some((operator, [left, right])) = module.builtin_binary(condition)? else {
        return Ok(None);
    };
    if !operator.is_equality() || operator.is_negative_equality() {
        return Ok(None);
    }

    // select the canonical last call from either operand
    let left = left.source.local_id;
    let right = right.source.local_id;
    let (last, predicate_value) = if is_array_last(module, left)? {
        (left, right)
    } else if is_array_last(module, right)? {
        (right, left)
    } else {
        return Ok(None);
    };
    if !module.is_speculatable_expression(predicate_value)? {
        return Ok(None);
    }
    let Some(last) = module.member_call(last) else {
        return Ok(None);
    };

    // require a canonical pop on the same stable array
    let Some(pop) = module.member_call(popped) else {
        return Ok(None);
    };
    if pop.is_optional()
        || !pop.generic_arguments.is_empty()
        || !pop.arguments.is_empty()
        || module.language_member(popped)? != Some(dir::LanguageItem::Array.member("pop"))
        || !module.is_duplicable_expression(last.receiver)?
        || !module.is_same_computation(last.receiver, pop.receiver)?
    {
        return Ok(None);
    }

    Ok(Some((pop.receiver, predicate_value)))
}

/// Return whether one expression is a canonical zero-argument Array.last call.
fn is_array_last(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<bool, ProviderError> {
    let Some(call) = module.member_call(expression) else {
        return Ok(false);
    };
    let is_last = !call.is_optional()
        && call.generic_arguments.is_empty()
        && call.arguments.is_empty()
        && module.language_member(expression)? == Some(dir::LanguageItem::Array.member("last"));

    Ok(is_last)
}

/// Build one popIf replacement.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: tspp_source::Span,
    receiver: dir::LocalNodeId<dir::Expression>,
    predicate: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    // retain the receiver and compared value exactly as authored
    let receiver_extent = module.source_extent(receiver.into_any())?;
    let predicate_extent = module.source_extent(predicate.into_any())?;
    if module.has_unretained_comment(extent, &[receiver_extent, predicate_extent])? {
        return Ok(None);
    }

    // build the canonical operation with a collision free parameter
    let name = module.fresh_binding_name("value");
    let receiver = module.expression_source(receiver, dir::OperatorPrecedence::Postfix)?;
    let predicate = module.expression_source(predicate, dir::OperatorPrecedence::Equality)?;
    let replacement = format!("{receiver}.popIf(({name}) => {name} === {predicate})");
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.suggestion("use popIf", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a conditional pop guarded by the last element.
    #[test]
    fn test_replaces_conditional_pop() {
        let session = TestSession::dir(
            &MANUAL_POP_IF,
            r#"
function popExpected(values: int32[], expected: int32): int32 | undefined {
    return values.last() === expected ? values.pop() : undefined;
}
"#,
        );

        session.assert_suggestions(
            r#"
function popExpected(values: int32[], expected: int32): int32 | undefined {
    return values.popIf((value) => value === expected);
}
"#,
        );
    }

    /// Replace reversed equality with the same canonical predicate.
    #[test]
    fn test_replaces_reversed_equality() {
        let session = TestSession::dir(
            &MANUAL_POP_IF,
            r#"
function popExpected(values: int32[], expected: int32): int32 | undefined {
    return expected === values.last() ? values.pop() : undefined;
}
"#,
        );

        session.assert_suggestions(
            r#"
function popExpected(values: int32[], expected: int32): int32 | undefined {
    return values.popIf((value) => value === expected);
}
"#,
        );
    }

    /// Avoid shadowing a binding used by the predicate.
    #[test]
    fn test_avoids_binding_collision() {
        let session = TestSession::dir(
            &MANUAL_POP_IF,
            r#"
function popExpected(values: int32[], value: int32): int32 | undefined {
    return values.last() === value ? values.pop() : undefined;
}
"#,
        );

        session.assert_suggestions(
            r#"
function popExpected(values: int32[], value: int32): int32 | undefined {
    return values.popIf((value2) => value2 === value);
}
"#,
        );
    }

    /// Accept a pop from another array.
    #[test]
    fn test_accepts_other_array() {
        let session = TestSession::dir(
            &MANUAL_POP_IF,
            r#"
function popExpected(values: int32[], other: int32[], expected: int32): int32 | undefined {
    return values.last() === expected ? other.pop() : undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user-defined last method.
    #[test]
    fn test_accepts_user_method() {
        let session = TestSession::dir(
            &MANUAL_POP_IF,
            r#"
class Values {
    last(): int32 | undefined {
        return undefined;
    }

    pop(): int32 | undefined {
        return undefined;
    }
}

function popExpected(values: Values, expected: int32): int32 | undefined {
    return values.last() === expected ? values.pop() : undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a last-element comparison whose other operand performs work.
    #[test]
    fn test_accepts_effectful_predicate() {
        let session = TestSession::dir(
            &MANUAL_POP_IF,
            r#"
declare function expected(): int32;

function popExpected(values: int32[]): int32 | undefined {
    return values.last() === expected() ? values.pop() : undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
