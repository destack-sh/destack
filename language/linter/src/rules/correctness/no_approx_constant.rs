use crate::LintMeta;
use destack_dir::{self as dir, Expression, ScalarLiteral};
use destack_repository::LintSeverity;

use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow approximate representations of mathematical constants.
    ///
    /// Using approximate values like `3.14` instead of `Math.PI` can lead to
    /// precision issues and makes the intent less clear. Use the standard
    /// library constants instead.
    #[lint(
        id = "no-approx-constant",
        code = "LC002",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoApproxConstant,
    "Disallow approximate math constants"
}

// mathematical constants: (name, value, minimum prefix to match)
// we match if the literal starts with the prefix and is <= the constant
const CONSTANTS: &[(&str, f64, &str)] = &[
    ("PI", std::f64::consts::PI, "3.14"),
    ("E", std::f64::consts::E, "2.71"),
    ("SQRT_2", std::f64::consts::SQRT_2, "1.41"),
    ("LN_2", std::f64::consts::LN_2, "0.69"),
    ("LN_10", std::f64::consts::LN_10, "2.30"),
    ("TAU", std::f64::consts::TAU, "6.28"),
];

impl LintRule for NoApproxConstant {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoApproxConstant::meta()
    }

    /// Check module source nodes for approximate math constants.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        // resolve lint metadata
        let meta = self.meta();

        // walk scalar float literals
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);

            let Expression::ScalarLiteral(ScalarLiteral::Float(value)) = expression else {
                continue;
            };

            // resolve literal source text for precision aware matching
            let span = ctx.dir.get_span(node_id);
            let literal_text = ctx.get_span_text(span);

            // match literal against known constants
            if let Some((name, _constant)) = find_approximate_constant(*value, literal_text) {
                // resolve effective severity
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                // build diagnostic
                let mut diagnostic = LintReport::new(
                    NO_APPROX_CONSTANT.id,
                    NO_APPROX_CONSTANT.code,
                    NO_APPROX_CONSTANT.category,
                    severity,
                    format!("approximate value of `Math.{name}`"),
                    span,
                )
                .label(format!("use `Math.{name}` instead"));

                // attach canonical replacement fix when enabled
                if ctx.compute_fixes {
                    let replacement = format!("Math.{name}");
                    let edits = ctx.edit_builder().replace(span, replacement).into_edits();
                    let fix = LintFix::r#unsafe("Replace approximation with Math constant")
                        .with_edits(edits);
                    diagnostic = diagnostic.fix(fix);
                }

                // report lint
                ctx.report(diagnostic);
            }
        }
    }
}

/// find if a float value approximates a known mathematical constant
fn find_approximate_constant(value: f64, literal_text: &str) -> Option<(&'static str, f64)> {
    // normalize literal text and derive precision threshold
    let normalized_mantissa = normalize_float_mantissa(literal_text)?;
    let precision_threshold = literal_precision_threshold(&normalized_mantissa)?;

    // match against known constants by prefix and precision distance
    for &(name, constant, prefix) in CONSTANTS {
        if !normalized_mantissa.starts_with(prefix) {
            continue;
        }

        let distance = (value - constant).abs();
        if distance <= precision_threshold {
            return Some((name, constant));
        }
    }

    None
}

/// Normalize one float literal mantissa for prefix and precision checks.
fn normalize_float_mantissa(literal_text: &str) -> Option<String> {
    // trim and reject signed constants
    let trimmed = literal_text.trim();
    if trimmed.starts_with('-') {
        return None;
    }

    // remove leading plus for explicit positive literals
    let trimmed = trimmed.strip_prefix('+').unwrap_or(trimmed);

    // split off exponent and remove separators
    let mantissa = trimmed
        .split_once(['e', 'E'])
        .map_or(trimmed, |(base, _)| base);
    let normalized: String = mantissa
        .chars()
        .filter(|character| *character != '_')
        .collect();
    if normalized.is_empty() {
        return None;
    }

    Some(normalized)
}

/// Return one threshold from decimal precision of the literal mantissa.
fn literal_precision_threshold(normalized_mantissa: &str) -> Option<f64> {
    // require fractional precision
    let (_, fractional_part) = normalized_mantissa.split_once('.')?;
    let fractional_digits = fractional_part
        .chars()
        .filter(|character| character.is_ascii_digit())
        .count();
    if fractional_digits == 0 {
        return None;
    }

    // allow one unit in the final fractional place
    Some(10f64.powi(-(fractional_digits as i32)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_approx_pi() {
        let test = TestProgram::for_rule_without_prelude(NoApproxConstant);
        let result = test.lint(
            "no_approx_constant/test_detects_approx_pi.ds",
            r#"
let pi = 3.14
"#,
        );
        test.result(result).assert_lint("no-approx-constant");
    }

    #[test]
    fn test_detects_approx_pi_more_digits() {
        let test = TestProgram::for_rule_without_prelude(NoApproxConstant);
        let result = test.lint(
            "no_approx_constant/test_detects_approx_pi_more_digits.ds",
            r#"
let pi = 3.14159
"#,
        );
        test.result(result).assert_lint("no-approx-constant");
    }

    #[test]
    fn test_fix_rewrites_approx_pi_to_math_constant() {
        let test = TestProgram::for_rule_without_prelude(NoApproxConstant);
        let result = test.lint(
            "no_approx_constant/test_fix_rewrites_approx_pi_to_math_constant.ds",
            r#"
let pi = 3.14159
"#,
        );
        test.result(result)
            .assert_lint("no-approx-constant")
            .assert_has_fix("no-approx-constant")
            .assert_unsafe_fixed(
                r#"
let pi = Math.PI;
"#,
            );
    }

    #[test]
    fn test_mutation_fix_rewrites_approx_tau_to_math_constant() {
        let test = TestProgram::for_rule_without_prelude(NoApproxConstant);
        let result = test.lint(
            "no_approx_constant/test_mutation_fix_rewrites_approx_tau_to_math_constant.ds",
            r#"
let tau = 6.28318
"#,
        );
        test.result(result)
            .assert_lint("no-approx-constant")
            .assert_has_fix("no-approx-constant")
            .assert_unsafe_fixed(
                r#"
let tau = Math.TAU;
"#,
            );
    }

    #[test]
    fn test_detects_approx_e() {
        let test = TestProgram::for_rule_without_prelude(NoApproxConstant);
        let result = test.lint(
            "no_approx_constant/test_detects_approx_e.ds",
            r#"
let e = 2.71828
"#,
        );
        test.result(result).assert_lint("no-approx-constant");
    }

    #[test]
    fn test_detects_approx_sqrt2() {
        let test = TestProgram::for_rule_without_prelude(NoApproxConstant);
        let result = test.lint(
            "no_approx_constant/test_detects_approx_sqrt2.ds",
            r#"
let sqrt2 = 1.414
"#,
        );
        test.result(result).assert_lint("no-approx-constant");
    }

    #[test]
    fn test_detects_trailing_zero_precision() {
        let test = TestProgram::for_rule_without_prelude(NoApproxConstant);
        let result = test.lint(
            "no_approx_constant/test_detects_trailing_zero_precision.ds",
            r#"
let ln10 = 2.30
"#,
        );
        test.result(result).assert_lint("no-approx-constant");
    }

    #[test]
    fn test_allows_far_prefix_match_outside_precision_threshold() {
        let test = TestProgram::for_rule_without_prelude(NoApproxConstant);
        let result = test.lint(
            "no_approx_constant/test_allows_far_prefix_match_outside_precision_threshold.ds",
            r#"
let maybePi = 3.149
"#,
        );
        test.result(result).assert_no_lint("no-approx-constant");
    }

    #[test]
    fn test_allows_unrelated_floats() {
        let test = TestProgram::for_rule_without_prelude(NoApproxConstant);
        let result = test.lint(
            "no_approx_constant/test_allows_unrelated_floats.ds",
            r#"
let x = 1.5
let y = 2.0
let z = 0.5
"#,
        );
        test.result(result).assert_no_lint("no-approx-constant");
    }

    #[test]
    fn test_allows_small_integers_as_floats() {
        let test = TestProgram::for_rule_without_prelude(NoApproxConstant);
        let result = test.lint(
            "no_approx_constant/test_allows_small_integers_as_floats.ds",
            r#"
let x = 3.0
let y = 2.0
"#,
        );
        test.result(result).assert_no_lint("no-approx-constant");
    }

    #[test]
    fn test_detects_approx_pi_scientific_notation() {
        let test = TestProgram::for_rule_without_prelude(NoApproxConstant);
        let result = test.lint(
            "no_approx_constant/test_detects_approx_pi_scientific_notation.ds",
            r#"
let pi = 3.14159e0
"#,
        );
        test.result(result).assert_lint("no-approx-constant");
    }
}
