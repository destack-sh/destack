use tspp_core::FxIndexMap;
use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow direct operator implementations built on a different operator.
    pub SUSPICIOUS_OPERATOR_IMPLEMENTATION {
        id: "suspicious-operator-implementation",
        summary: "Disallow direct operator implementations built on a different operator",
        explanation: r#"
Directly combining an operator receiver and operand with a different operator commonly indicates a copied or mistyped body.
Instead, you SHOULD use the operator implemented by the enclosing protocol.
"#,
        example: {
            reported: r#"
struct Score {
    value: int32;
}

extension of Score implements Add<Score> {
    type Output = Score;

    add(&readonly this, other: Score): Score {
        return Score { value: this.value - other.value };
    }
}
"#,
            accepted: r#"
struct Score {
    value: int32;
}

extension of Score implements Add<Score> {
    type Output = Score;

    add(&readonly this, other: Score): Score {
        return Score { value: this.value + other.value };
    }
}
"#,
        },
        provenance: [
            Clippy("suspicious_arithmetic_impl"),
            Clippy("suspicious_op_assign_impl"),
        ],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report mismatched operators within canonical operator implementations.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut implementations = FxIndexMap::default();

    // index method bodies that implement operator protocols
    for (member, declaration) in view.iter_nodes::<dir::Member>() {
        let dir::Member::Method {
            signature,
            body: Some(body),
            ..
        } = declaration
        else {
            continue;
        };
        let symbol = module.declaration_symbol(member)?;
        let Some(expected) = module
            .dir
            .implemented_language_member(symbol)?
            .map(|member| member.owner)
            .filter(dir::LanguageItem::is_arithmetic_protocol)
        else {
            continue;
        };

        // retain the first named operand parameter
        let Some(parameter) = signature.parameters.first() else {
            continue;
        };
        if view.get(*parameter).name().is_none() {
            continue;
        }
        let parameter = module.declaration_symbol(*parameter)?;
        implementations.insert(*body, (expected, parameter));
    }

    // compare operators directly owned by each indexed body
    let mut output = LintOutput::default();
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let Some(body) = module.enclosing_callable_body(expression.into_any()) else {
            continue;
        };
        let Some((expected, parameter)) = implementations.get(&body).copied() else {
            continue;
        };
        let dir::Expression::Binary {
            left,
            operator,
            right,
        } = node
        else {
            continue;
        };
        let Some(used) = operator
            .single_protocol()
            .filter(dir::LanguageItem::is_arithmetic_protocol)
            .filter(|used| *used != expected)
        else {
            continue;
        };

        // require direct access paths rooted at the receiver and operand
        let Some(left) = module
            .access_resolution(*left)
            .map(|access| access.path().root())
        else {
            continue;
        };
        let Some(right) = module
            .access_resolution(*right)
            .map(|access| access.path().root())
        else {
            continue;
        };
        let is_direct = matches!(
            (left, right),
            (dir::AccessRoot::Receiver, dir::AccessRoot::Symbol(symbol))
                | (dir::AccessRoot::Symbol(symbol), dir::AccessRoot::Receiver)
                if symbol == parameter
        );
        if !is_direct {
            continue;
        }

        // require the operation to produce a returned value or constructed field directly
        let Some(parent) = view.get_parent_for(expression) else {
            continue;
        };
        let is_direct_result = match parent.ty {
            dir::NodeType::Property => module.is_within_return_value(expression),
            dir::NodeType::Expression => {
                let parent = dir::LocalNodeId::<dir::Expression>::new(parent.id);
                matches!(view.get(parent), dir::Expression::Return { value: Some(value) } if *value == expression)
            }
            _ => false,
        };
        if !is_direct_result {
            continue;
        }

        // report the mismatched authored operator
        let span = module.main_span(expression.into_any())?;
        let message = format!(
            "{} implementation uses the {} operator",
            expected.export_name(),
            used.export_name(),
        );
        output.report(lint.diagnostic(message, span));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report subtraction inside an addition implementation.
    #[test]
    fn test_reports_mismatched_operator() {
        let session = TestSession::dir(
            &SUSPICIOUS_OPERATOR_IMPLEMENTATION,
            r#"
struct Score {
    value: int32;
}

extension of Score implements Add<Score> {
    type Output = Score;

    add(&readonly this, other: Score): Score {
        return Score { value: this.value - other.value };
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[suspicious-operator-implementation]: Add implementation uses the Subtract operator
  ──▶ main.tspp:9:42
   │
 7 │
 8 │     add(&readonly this, other: Score): Score {
 9 │         return Score { value: this.value - other.value };
   │                                          ^
10 │     }
11 │ }
   │
"#,
        );
    }

    /// Accept the matching operator inside an implementation.
    #[test]
    fn test_accepts_matching_operator() {
        let session = TestSession::dir(
            &SUSPICIOUS_OPERATOR_IMPLEMENTATION,
            r#"
struct Score {
    value: int32;
}

extension of Score implements Add<Score> {
    type Output = Score;

    add(&readonly this, other: Score): Score {
        return Score { value: this.value + other.value };
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept auxiliary operators within a composed multiplication formula.
    #[test]
    fn test_accepts_composed_operator() {
        let session = TestSession::dir(
            &SUSPICIOUS_OPERATOR_IMPLEMENTATION,
            r#"
struct Pair {
    left: int32;
    right: int32;
}

extension of Pair implements Multiply<Pair> {
    type Output = Pair;

    multiply(&readonly this, other: Pair): Pair {
        return Pair {
            left: this.left * other.left + this.right * other.right,
            right: this.left * other.right - this.right * other.left,
        };
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept auxiliary operators used through an intermediate value.
    #[test]
    fn test_accepts_intermediate_operator() {
        let session = TestSession::dir(
            &SUSPICIOUS_OPERATOR_IMPLEMENTATION,
            r#"
struct Pair {
    left: int32;
    right: int32;
}

extension of Pair implements Multiply<Pair> {
    type Output = Pair;

    multiply(&readonly this, other: Pair): Pair {
        const sum = Pair {
            left: this.left + other.left,
            right: this.right + other.right,
        };

        return Pair {
            left: sum.left * other.left,
            right: sum.right * other.right,
        };
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a different operator inside a nested function.
    #[test]
    fn test_accepts_nested_function_operator() {
        let session = TestSession::dir(
            &SUSPICIOUS_OPERATOR_IMPLEMENTATION,
            r#"
struct Score {
    value: int32;
}

extension of Score implements Add<Score> {
    type Output = Score;

    add(&readonly this, other: Score): Score {
        const difference = (left: int32, right: int32): int32 => left - right;
        return Score { value: this.value + difference(other.value, 0) };
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept the same method name outside a protocol implementation.
    #[test]
    fn test_accepts_unrelated_method() {
        let session = TestSession::dir(
            &SUSPICIOUS_OPERATOR_IMPLEMENTATION,
            r#"
class Score {
    add(other: int32): int32 {
        return other - 1;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
