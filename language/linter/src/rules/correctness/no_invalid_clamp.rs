use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow clamp calls with invalid constant bounds.
    pub NO_INVALID_CLAMP {
        id: "no-invalid-clamp",
        summary: "Disallow clamp calls with invalid constant bounds",
        explanation: r#"
A clamp traps when its lower bound exceeds its upper bound or either bound is NaN.
Instead, you MUST pass ordered bounds that are not NaN.
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
        provenance: [],
        category: Correctness,
        level: Error,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report canonical scalar clamp calls with invalid exact bounds.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical scalar clamp calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        let member = module.language_member(expression)?;
        let is_clamp = member == Some(dir::LanguageItem::Integer.member("clamp"))
            || member == Some(dir::LanguageItem::Float.member("clamp"));
        if !is_clamp {
            continue;
        }
        let [minimum, maximum] = call.arguments else {
            continue;
        };
        let (
            dir::Argument::Positional { value: minimum },
            dir::Argument::Positional { value: maximum },
        ) = (view.get(*minimum), view.get(*maximum))
        else {
            continue;
        };

        // reject exact NaN bounds before ordering the remaining constants
        let message = if module.is_nan(*minimum)? || module.is_nan(*maximum)? {
            "clamp bound is NaN"
        } else {
            let Some(minimum) = module.scalar_constant(*minimum)? else {
                continue;
            };
            let Some(maximum) = module.scalar_constant(*maximum)? else {
                continue;
            };
            if minimum.ordering(&maximum) != Some(std::cmp::Ordering::Greater) {
                continue;
            }

            "clamp lower bound exceeds its upper bound"
        };

        // report the complete invalid call
        let span = module.source_extent(expression.into_any())?;
        output.report(lint.diagnostic(message, span));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report inverted floating-point bounds.
    #[test]
    fn test_reports_inverted_float_bounds() {
        let session = TestSession::dir(
            &NO_INVALID_CLAMP,
            r#"
function bounded(value: float64): float64 {
    return value.clamp(1.0, -1.0);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-invalid-clamp]: clamp lower bound exceeds its upper bound
 ──▶ main.tspp:2:12
  │
1 │ function bounded(value: float64): float64 {
2 │     return value.clamp(1.0, -1.0);
  │            ^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Report canonical and computed NaN bounds.
    #[test]
    fn test_reports_nan_bounds() {
        let session = TestSession::dir(
            &NO_INVALID_CLAMP,
            r#"
function lower(value: float64): float64 {
    return value.clamp(Number.NaN, 1.0);
}

function upper(value: float64): float64 {
    return value.clamp(-1.0, 0.0 / 0.0);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-invalid-clamp]: clamp bound is NaN
 ──▶ main.tspp:2:12
  │
1 │ function lower(value: float64): float64 {
2 │     return value.clamp(Number.NaN, 1.0);
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
4 │
  │

warning[no-invalid-clamp]: clamp bound is NaN
 ──▶ main.tspp:6:12
  │
4 │
5 │ function upper(value: float64): float64 {
6 │     return value.clamp(-1.0, 0.0 / 0.0);
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
7 │ }
  │
"#,
        );
    }

    /// Accept a user-defined method with the same name.
    #[test]
    fn test_accepts_user_clamp() {
        let session = TestSession::dir(
            &NO_INVALID_CLAMP,
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
