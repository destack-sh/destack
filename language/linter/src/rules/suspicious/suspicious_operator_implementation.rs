use destack_core::FxIndexMap;
use destack_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow operator implementations built on a different operator.
    pub SUSPICIOUS_OPERATOR_IMPLEMENTATION {
        id: "suspicious-operator-implementation",
        summary: "Disallow operator implementations built on a different operator",
        explanation: r#"
Using a different operator inside an operator implementation commonly indicates a copied or mistyped body.
Instead, you SHOULD use the operator implemented by the enclosing protocol.

Calls and operators inside nested functions are evaluated independently.
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
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report mismatched operators within canonical operator implementations.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut protocols = FxIndexMap::default();

    // index method bodies that implement operator protocols
    for (member, declaration) in view.iter_nodes::<dir::Member>() {
        let dir::Member::Method {
            body: Some(body), ..
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
        protocols.insert(*body, expected);
    }

    // compare operators directly owned by each indexed body
    let mut output = LintOutput::default();
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let Some(body) = module.enclosing_callable_body(expression.into_any()) else {
            continue;
        };
        let Some(expected) = protocols.get(&body).copied() else {
            continue;
        };
        let used = match node {
            dir::Expression::Unary { operator, .. } => operator.single_protocol(),
            dir::Expression::Binary { operator, .. } => operator.single_protocol(),
            dir::Expression::Assign { operator, .. } => operator.single_protocol(),
            _ => None,
        };
        let Some(used) = used
            .filter(dir::LanguageItem::is_arithmetic_protocol)
            .filter(|used| *used != expected)
        else {
            continue;
        };

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
  ──▶ main.ds:9:42
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
