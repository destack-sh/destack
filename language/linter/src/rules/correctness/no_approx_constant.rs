use std::f64::consts;

use tspp_dir as dir;
use tspp_source::Patch;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

const KNOWN_CONSTANTS: [KnownConstant; 8] = [
    KnownConstant {
        value: consts::E,
        name: "E",
        minimum_digits: 4,
    },
    KnownConstant {
        value: consts::LN_10,
        name: "LN10",
        minimum_digits: 5,
    },
    KnownConstant {
        value: consts::LN_2,
        name: "LN2",
        minimum_digits: 5,
    },
    KnownConstant {
        value: consts::LOG2_E,
        name: "LOG2E",
        minimum_digits: 5,
    },
    KnownConstant {
        value: consts::LOG10_E,
        name: "LOG10E",
        minimum_digits: 5,
    },
    KnownConstant {
        value: consts::PI,
        name: "PI",
        minimum_digits: 3,
    },
    KnownConstant {
        value: consts::FRAC_1_SQRT_2,
        name: "SQRT1_2",
        minimum_digits: 5,
    },
    KnownConstant {
        value: consts::SQRT_2,
        name: "SQRT2",
        minimum_digits: 5,
    },
];

declare_lint! {
    /// Disallow numeric literals that approximate well-known constants.
    pub NO_APPROX_CONSTANT {
        id: "no-approx-constant",
        summary: "Disallow numeric literals that approximate well-known constants",
        explanation: r#"
A decimal approximation can contain fewer significant digits than the corresponding standard-library constant.
Instead, you SHOULD use the corresponding `Math` member.
"#,
        example: {
            reported: r#"
declare const radius: number;
const circumference = 2.0 * 3.14 * radius;
"#,
            accepted: r#"
declare const radius: number;
const circumference = 2.0 * Math.PI * radius;
"#,
        },
        provenance: [Clippy("approx_constant")],
        category: Correctness,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One canonical mathematical constant recognized from decimal literals.
#[derive(Debug, Clone, Copy)]
struct KnownConstant {
    /// The canonical float64 value.
    value: f64,
    /// The `Math` member name.
    name: &'static str,
    /// The minimum authored precision.
    minimum_digits: usize,
}

impl KnownConstant {
    /// Return whether an authored decimal approximates this constant.
    fn approximates(self, source: &str, value: f64) -> bool {
        let source = source.replace('_', "");
        let digits = source.bytes().filter(u8::is_ascii_digit).count();
        if digits < self.minimum_digits {
            return false;
        }

        // recognize a literal prefix with enough authored precision
        let value = value.to_string();
        if value.len() > self.minimum_digits && self.value.to_string().starts_with(&value) {
            return true;
        }

        // recognize the constant rounded to the authored decimal places
        let Some(decimal) = source.find('.') else {
            return false;
        };
        if source.contains(['e', 'E']) {
            return false;
        }
        let precision = source.len() - decimal - 1;
        let rounded = format!("{:.*}", precision, self.value);

        source == rounded
    }
}

/// Report decimal literals that approximate canonical `Math` constants.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect authored floating-point literals
    for (expression, value) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Literal(dir::Literal::Float(value)) = value else {
            continue;
        };
        let adjusted_type = module.adjusted_type(expression.into_any())?;
        if !matches!(
            adjusted_type,
            dir::Type::Literal(dir::Literal::Float(_))
                | dir::Type::Primitive(dir::PrimitiveType::Float(dir::FloatType::Float64))
        ) {
            continue;
        }
        let span = module.source_extent(expression.into_any())?;
        let source = module.source(span)?;

        // select the first canonical approximation
        let Some(constant) = KNOWN_CONSTANTS
            .iter()
            .copied()
            .find(|constant| constant.approximates(source, *value))
        else {
            continue;
        };

        // suggest the canonical standard-library member
        let replacement = format!("Math.{}", constant.name);
        let patch = Patch::replace(span, replacement.clone());
        let suggestion = lint.suggestion(format!("use `{replacement}`"), patch)?;
        let diagnostic = lint
            .diagnostic(format!("approximate value of `{replacement}`"), span)
            .suggestion(suggestion);
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a rounded approximation of pi.
    #[test]
    fn test_reports_rounded_pi() {
        let session = TestSession::dir(
            &NO_APPROX_CONSTANT,
            r#"
const angle = 3.1416;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-approx-constant]: approximate value of `Math.PI`
 ──▶ main.tspp:1:15
  │
1 │ const angle = 3.1416;
  │               ^^^^^^
  │

 = suggestion: use `Math.PI` (requires review)
--- a/main.tspp
+++ b/main.tspp

-   1│ const angle = 3.1416;
+   1│ const angle = Math.PI;
"#,
        );
    }

    /// Report a truncated approximation of Euler's number.
    #[test]
    fn test_reports_truncated_e() {
        let session = TestSession::dir(
            &NO_APPROX_CONSTANT,
            r#"
const growth = 2.718;
"#,
        );

        session.assert_suggestions(
            r#"
const growth = Math.E;
"#,
        );
    }

    /// Accept a decimal with insufficient precision to identify pi.
    #[test]
    fn test_accepts_short_decimal() {
        let session = TestSession::dir(
            &NO_APPROX_CONSTANT,
            r#"
const estimate = 3.1;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an unrelated decimal with the same authored precision as pi.
    #[test]
    fn test_accepts_unrelated_decimal() {
        let session = TestSession::dir(
            &NO_APPROX_CONSTANT,
            r#"
const estimate = 3.10;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Ignore separators when measuring authored precision.
    #[test]
    fn test_reports_separated_pi() {
        let session = TestSession::dir(
            &NO_APPROX_CONSTANT,
            r#"
const angle = 3.1_4;
"#,
        );

        session.assert_suggestions(
            r#"
const angle = Math.PI;
"#,
        );
    }

    /// Accept the canonical standard-library constant.
    #[test]
    fn test_accepts_math_constant() {
        let session = TestSession::dir(
            &NO_APPROX_CONSTANT,
            r#"
const angle = Math.PI;
"#,
        );

        session.assert_no_diagnostics();
    }
}
