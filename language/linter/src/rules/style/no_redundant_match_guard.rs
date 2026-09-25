use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, NodeSpanRegion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer literal patterns over equivalent equality guards.
    pub NO_REDUNDANT_MATCH_GUARD {
        id: "no-redundant-match-guard",
        summary: "Prefer literal patterns over equivalent equality guards",
        explanation: r#"
An equality guard on a newly bound match value repeats a test that the pattern can perform directly.
Instead, you SHOULD match the compared literal in the arm pattern.
"#,
        example: {
            reported: r#"
function classify(value: int32): string {
    return match (value) {
        matched if (matched === 0) => "zero"
        _ => "other"
    };
}
"#,
            accepted: r#"
function classify(value: int32): string {
    return match (value) {
        0 => "zero"
        _ => "other"
    };
}
"#,
        },
        provenance: [Clippy("redundant_guards")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report equality guards that only constrain their arm binding to one literal.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect expression-only guards
    for (arm, node) in view.iter_nodes::<dir::MatchArm>() {
        let Some(condition) = node.guard() else {
            continue;
        };
        let Some(guard) = condition.as_expression() else {
            continue;
        };

        // require one direct binding pattern
        let pattern = node.pattern();
        let dir::PatternDecision::Bind(binding) = module.pattern_decision(pattern)? else {
            continue;
        };
        let (Some(symbol), None) = (binding.symbol, binding.pattern) else {
            continue;
        };

        // select an equality between that binding and one scalar literal
        let Some((operator, operands)) = module.builtin_binary(guard)? else {
            continue;
        };
        if operator != dir::BinaryOperator::EqualStrict {
            continue;
        }
        let literal = if module.selected_symbol(operands[0].source.local_id)? == Some(symbol)
            && view.get(operands[1].source.local_id).as_scalar().is_some()
        {
            operands[1].source.local_id
        } else if module.selected_symbol(operands[1].source.local_id)? == Some(symbol)
            && view.get(operands[0].source.local_id).as_scalar().is_some()
        {
            operands[0].source.local_id
        } else {
            continue;
        };

        // retain the binding when the arm body still reads it
        let body = match node {
            dir::MatchArm::Expression { body, .. } => body.into_any(),
            dir::MatchArm::Block { body, .. } => body.into_any(),
        };
        let is_used = module.flows.binding_occurrences().any(|occurrence| {
            occurrence.symbol == symbol
                && occurrence.uses.contains(dir::BindingUse::READ)
                && view.is_inside(occurrence.node, body)
        });
        if is_used {
            continue;
        }

        // identify the redundant equality expression
        let span = module.source_extent(guard.into_any())?;

        // replace the binding and guard with the literal pattern
        let mut diagnostic =
            lint.diagnostic("equality guard can be expressed by the pattern", span);
        if let Some(suggestion) = suggestion(module, lint, arm, pattern, literal)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one literal pattern replacement.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    arm: dir::LocalNodeId<dir::MatchArm>,
    pattern: dir::LocalNodeId<dir::Pattern>,
    literal: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let pattern = module.source_extent(pattern.into_any())?;
    let guard = module.source_region(arm.into_any(), NodeSpanRegion::Guard)?;
    let extent = pattern.merge(guard);
    let retained = module.source_extent(literal.into_any())?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // replace the complete guarded pattern head
    let patch = Patch::replace(extent, module.source(retained)?);
    let suggestion = lint.fix("match the literal directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace an equality guard with its reversed literal operand.
    #[test]
    fn test_replaces_reversed_equality() {
        let session = TestSession::dir(
            &NO_REDUNDANT_MATCH_GUARD,
            r#"
function classify(value: int32): string {
    return match (value) {
        matched if (1 === matched) => "one"
        _ => "other"
    };
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-redundant-match-guard]: equality guard can be expressed by the pattern
 ──▶ main.tspp:3:21
  │
1 │ function classify(value: int32): string {
2 │     return match (value) {
3 │         matched if (1 === matched) => "one"
  │                     ^^^^^^^^^^^^^
4 │         _ => "other"
5 │     };
  │

 = fix: match the literal directly
--- a/main.tspp
+++ b/main.tspp

    2│     return match (value) {
-   3│         matched if (1 === matched) => "one"
+   3│         1 => "one"
    4│         _ => "other"
"#,
        );
        session.assert_fixes(
            r#"
function classify(value: int32): string {
    return match (value) {
        1 => "one"
        _ => "other"
    };
}
"#,
        );
    }

    /// Accept a guard whose binding remains in use by the arm body.
    #[test]
    fn test_accepts_used_binding() {
        let session = TestSession::dir(
            &NO_REDUNDANT_MATCH_GUARD,
            r#"
function classify(value: int32): int32 {
    return match (value) {
        matched if (matched == 0) => matched
        _ => value
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a guard that compares the binding with a runtime value.
    #[test]
    fn test_accepts_runtime_comparison() {
        let session = TestSession::dir(
            &NO_REDUNDANT_MATCH_GUARD,
            r#"
function equals(value: int32, expected: int32): boolean {
    return match (value) {
        matched if (matched == expected) => true
        _ => false
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept loose equality whose matching behavior can differ from a literal pattern.
    #[test]
    fn test_accepts_loose_equality() {
        let session = TestSession::dir(
            &NO_REDUNDANT_MATCH_GUARD,
            r#"
function classify(value: int32 | null | undefined): string {
    return match (value) {
        matched if (matched == null) => "missing"
        _ => "present"
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a compound guard with a binding condition.
    #[test]
    fn test_accepts_binding_guard() {
        let session = TestSession::dir(
            &NO_REDUNDANT_MATCH_GUARD,
            r#"
function classify(value: (int32, boolean) | null): int32 {
    return match (value) {
        pair if (let (number, ready) = pair && ready) => number
        _ => 0
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
