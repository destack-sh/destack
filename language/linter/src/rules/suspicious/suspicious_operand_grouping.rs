use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow inconsistent member pairings in comparison chains.
    pub SUSPICIOUS_OPERAND_GROUPING {
        id: "suspicious-operand-grouping",
        summary: "Disallow inconsistent member pairings in comparison chains",
        explanation: r#"
One comparison pairing different members amid otherwise corresponding member comparisons is likely a typo.
Instead, you SHOULD compare corresponding members consistently.
"#,
        example: {
            reported: r#"
struct Point {
    x: int32;
    y: int32;
    z: int32;
}

function equals(left: Point, right: Point): boolean {
    return left.x == right.y && left.y == right.y && left.z == right.z;
}
"#,
            accepted: r#"
struct Point {
    x: int32;
    y: int32;
    z: int32;
}

function equals(left: Point, right: Point): boolean {
    return left.x == right.x && left.y == right.y && left.z == right.z;
}
"#,
        },
        provenance: [Clippy("suspicious_operation_groupings")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// One comparison between projected members.
struct MemberComparison {
    /// The complete comparison expression.
    expression: dir::LocalNodeId<dir::Expression>,
    /// The selected builtin comparison operator.
    operator: dir::BinaryOperator,
    /// The stable storage containing the left member.
    left_parent: dir::AccessPath,
    /// The left member key.
    left_key: dir::StaticKey,
    /// The stable storage containing the right member.
    right_parent: dir::AccessPath,
    /// The right member key.
    right_key: dir::StaticKey,
}

/// Report exceptional cross-member comparisons inside logical chains.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect roots of builtin logical chains
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let Some((logical, _)) = module.builtin_binary(expression)? else {
            continue;
        };
        if !matches!(logical, dir::BinaryOperator::And | dir::BinaryOperator::Or)
            || module.has_builtin_binary_parent(expression, logical)?
        {
            continue;
        }
        let terms = module.short_circuit_operands(expression, logical)?;
        let comparisons = terms
            .into_iter()
            .filter_map(|term| member_comparison(module, term).transpose())
            .collect::<Result<Vec<_>, _>>()?;

        // require two aligned peers before treating one crossed pairing as exceptional
        for candidate in &comparisons {
            if candidate.left_key == candidate.right_key {
                continue;
            }
            let aligned = comparisons
                .iter()
                .filter(|peer| candidate.is_aligned_peer(peer))
                .count();
            if aligned < 2 {
                continue;
            }

            let span = module.source_extent(candidate.expression.into_any())?;
            let diagnostic = lint.diagnostic("comparison pairs inconsistent members", span);
            output.report(diagnostic);
        }
    }

    Ok(output)
}

/// Return one builtin comparison between two stable projected members.
fn member_comparison(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<MemberComparison>, ProviderError> {
    let Some((operator, [left, right])) = module.builtin_binary(expression)? else {
        return Ok(None);
    };
    if !operator.is_comparison() {
        return Ok(None);
    }
    let Some(left) = module.access_resolution(left.source.local_id) else {
        return Ok(None);
    };
    let Some(right) = module.access_resolution(right.source.local_id) else {
        return Ok(None);
    };
    let Some((left_parent, left_key)) = left.path().split_last() else {
        return Ok(None);
    };
    let Some((right_parent, right_key)) = right.path().split_last() else {
        return Ok(None);
    };
    if left_parent == right_parent {
        return Ok(None);
    }

    Ok(Some(MemberComparison {
        expression,
        operator,
        left_parent,
        left_key,
        right_parent,
        right_key,
    }))
}

impl MemberComparison {
    /// Return whether another comparison aligns the same member containers.
    fn is_aligned_peer(&self, other: &Self) -> bool {
        // compare peers written in the same operand order
        if self.left_parent == other.left_parent && self.right_parent == other.right_parent {
            return self.operator == other.operator && other.left_key == other.right_key;
        }

        // normalize peers written in the opposite operand order
        self.left_parent == other.right_parent
            && self.right_parent == other.left_parent
            && self.operator.swapped() == Some(other.operator)
            && other.left_key == other.right_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report one crossed member comparison among aligned comparisons.
    #[test]
    fn test_reports_crossed_member() {
        let session = TestSession::dir(
            &SUSPICIOUS_OPERAND_GROUPING,
            r#"
struct Point {
    x: int32;
    y: int32;
    z: int32;
}

function equals(left: Point, right: Point): boolean {
    return left.x == right.y && left.y == right.y && left.z == right.z;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[suspicious-operand-grouping]: comparison pairs inconsistent members
 ──▶ main.tspp:8:12
  │
6 │
7 │ function equals(left: Point, right: Point): boolean {
8 │     return left.x == right.y && left.y == right.y && left.z == right.z;
  │            ^^^^^^^^^^^^^^^^^
9 │ }
  │
"#,
        );
    }

    /// Accept member comparisons aligned in either operand order.
    #[test]
    fn test_accepts_reversed_aligned_comparison() {
        let session = TestSession::dir(
            &SUSPICIOUS_OPERAND_GROUPING,
            r#"
struct Point {
    x: int32;
    y: int32;
    z: int32;
}

function equals(left: Point, right: Point): boolean {
    return left.x == right.x && right.y == left.y && left.z == right.z;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a deliberate sequence of crossed member comparisons.
    #[test]
    fn test_accepts_consistently_crossed_members() {
        let session = TestSession::dir(
            &SUSPICIOUS_OPERAND_GROUPING,
            r#"
struct Point {
    x: int32;
    y: int32;
    z: int32;
}

function rotates(left: Point, right: Point): boolean {
    return left.x == right.y && left.y == right.z && left.z == right.x;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept one cross-member comparison without enough peers to infer a typo.
    #[test]
    fn test_accepts_two_comparisons() {
        let session = TestSession::dir(
            &SUSPICIOUS_OPERAND_GROUPING,
            r#"
struct Point {
    x: int32;
    y: int32;
}

function overlaps(left: Point, right: Point): boolean {
    return left.x == right.y && left.y == right.y;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
