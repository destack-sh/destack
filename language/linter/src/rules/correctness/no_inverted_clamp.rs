use std::cmp::Ordering;

use destack_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow clamp calls with inverted constant bounds.
    pub NO_INVERTED_CLAMP {
        id: "no-inverted-clamp",
        summary: "Disallow clamp calls with inverted constant bounds",
        explanation: r#"
A clamp whose constant lower bound exceeds its upper bound has no valid interval.
Instead, you MUST pass the lower bound before the upper bound.
"#,
        example: {
            reported: r#"
function bounded(value: int32): int32 {
    return value.clamp(10, 0);
}
"#,
            accepted: r#"
function bounded(value: int32): int32 {
    return value.clamp(0, 10);
}
"#,
        },
        category: Correctness,
        level: Error,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report canonical scalar clamp calls with reversed exact bounds.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical scalar clamp calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let member = module.language_member(expression)?;
        let is_clamp = member == Some(dir::LanguageItem::Integer.member("clamp"))
            || member == Some(dir::LanguageItem::Float.member("clamp"));
        if !is_clamp {
            continue;
        }
        let dir::Expression::Call { arguments, .. } = view.get(expression) else {
            continue;
        };
        let [minimum, maximum] = arguments.as_slice() else {
            continue;
        };
        let (
            dir::Argument::Positional { value: minimum },
            dir::Argument::Positional { value: maximum },
        ) = (view.get(*minimum), view.get(*maximum))
        else {
            continue;
        };

        // compare exact bounds from one numeric domain
        let Some(minimum) = module.scalar_constant(*minimum)? else {
            continue;
        };
        let Some(maximum) = module.scalar_constant(*maximum)? else {
            continue;
        };
        if numeric_ordering(minimum, maximum) != Some(Ordering::Greater) {
            continue;
        }

        // report the complete invalid call
        let span = module.source_extent(expression.into_any())?;
        output.report(lint.diagnostic("clamp lower bound exceeds its upper bound", span));
    }

    Ok(output)
}

/// Compare exact constants from one numeric domain.
fn numeric_ordering(left: dir::ScalarLiteral, right: dir::ScalarLiteral) -> Option<Ordering> {
    match (left, right) {
        (dir::ScalarLiteral::Integer(left), dir::ScalarLiteral::Integer(right))
        | (dir::ScalarLiteral::Bigint(left), dir::ScalarLiteral::Bigint(right)) => {
            Some(left.cmp(&right))
        }
        (dir::ScalarLiteral::Float(left), dir::ScalarLiteral::Float(right)) => {
            left.partial_cmp(&right)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report inverted floating-point bounds.
    #[test]
    fn test_reports_inverted_float_bounds() {
        let session = TestSession::dir(
            &NO_INVERTED_CLAMP,
            r#"
function bounded(value: float64): float64 {
    return value.clamp(1.0, -1.0);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-inverted-clamp]: clamp lower bound exceeds its upper bound
 ──▶ main.ds:2:12
  │
1 │ function bounded(value: float64): float64 {
2 │     return value.clamp(1.0, -1.0);
  │            ^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Accept a user-defined method with the same name.
    #[test]
    fn test_accepts_user_clamp() {
        let session = TestSession::dir(
            &NO_INVERTED_CLAMP,
            r#"
class Bounds {
    clamp(minimum: int32, maximum: int32): int32 {
        return minimum;
    }
}
const bounded = new Bounds().clamp(10, 0);
"#,
        );

        session.assert_no_diagnostics();
    }
}
