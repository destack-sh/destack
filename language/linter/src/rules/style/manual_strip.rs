use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer prefix or suffix stripping over checking and slicing.
    pub MANUAL_STRIP {
        id: "manual-strip",
        summary: "Prefer prefix or suffix stripping over checking and slicing",
        explanation: r#"
Checking a string boundary before slicing it repeats the affix and its length across two operations.
Instead, you SHOULD use `stripPrefix` or `stripSuffix` to test and return the remainder together.
"#,
        example: {
            reported: r#"
function removePrefix(text: string, prefix: string): string | undefined {
    return text.startsWith(prefix) ? text.slice(prefix.length) : undefined;
}
"#,
            accepted: r#"
function removePrefix(text: string, prefix: string): string | undefined {
    return text.stripPrefix(prefix);
}
"#,
        },
        provenance: [Clippy("manual_strip")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// The string boundary removed by one manual strip expression.
#[derive(Debug, Clone, Copy)]
enum Boundary {
    /// The beginning of the string.
    Prefix,
    /// The end of the string.
    Suffix,
}

impl Boundary {
    /// Return the canonical stripping member name.
    fn member(self) -> &'static str {
        match self {
            Self::Prefix => "stripPrefix",
            Self::Suffix => "stripSuffix",
        }
    }
}

/// Report guarded string slices that manually strip one boundary.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect value conditionals with undefined fallbacks
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
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
        let Some(then_expression) = module.sole_value_expression(*then_expression) else {
            continue;
        };
        let Some(else_expression) = module.sole_value_expression(*else_expression) else {
            continue;
        };
        if view.get(else_expression).as_scalar() != Some(dir::Literal::Undefined) {
            continue;
        }
        let Some((boundary, receiver, affix)) = strip_test(module, condition)? else {
            continue;
        };
        if !is_stripped_value(module, then_expression, boundary, receiver, affix)? {
            continue;
        }

        // suggest the canonical stripping operation
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("string boundary is tested before slicing", span);
        if let Some(fix) = fix(module, lint, expression, boundary, receiver, affix)? {
            diagnostic = diagnostic.suggestion(fix);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select one canonical startsWith or endsWith test.
fn strip_test(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<
    Option<(
        Boundary,
        dir::LocalNodeId<dir::Expression>,
        dir::LocalNodeId<dir::Expression>,
    )>,
    ProviderError,
> {
    let Some(call) = module.member_call(expression) else {
        return Ok(None);
    };
    if call.is_optional() || !call.generic_arguments.is_empty() {
        return Ok(None);
    }
    let [argument] = call.arguments else {
        return Ok(None);
    };
    let dir::Argument::Positional { value: affix } = module.view().get(*argument) else {
        return Ok(None);
    };
    let member = module.language_member(expression)?;
    let boundary = if member == Some(dir::LanguageItem::String.member("startsWith")) {
        Boundary::Prefix
    } else if member == Some(dir::LanguageItem::String.member("endsWith")) {
        Boundary::Suffix
    } else {
        return Ok(None);
    };

    Ok(Some((boundary, call.receiver, *affix)))
}

/// Return whether one expression slices the boundary selected by a matching test.
fn is_stripped_value(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
    boundary: Boundary,
    receiver: dir::LocalNodeId<dir::Expression>,
    affix: dir::LocalNodeId<dir::Expression>,
) -> Result<bool, ProviderError> {
    let Some(call) = module.member_call(expression) else {
        return Ok(false);
    };
    let member = module.language_member(expression)?;
    if call.is_optional()
        || !call.generic_arguments.is_empty()
        || member != Some(dir::LanguageItem::String.member("slice"))
            && member != Some(dir::LanguageItem::String.member("substring"))
        || !module.is_same_computation(call.receiver, receiver)?
    {
        return Ok(false);
    }

    // match the boundary-specific slice operands
    let matches = match (boundary, call.arguments) {
        (Boundary::Prefix, [start]) => {
            let Some(start) = module.view().get(*start).value() else {
                return Ok(false);
            };
            let Some(length_receiver) = module.length_receiver(start)? else {
                return Ok(false);
            };

            module.is_same_computation(length_receiver, affix)?
        }
        (Boundary::Suffix, [start, end]) => {
            let Some(start) = module.view().get(*start).value() else {
                return Ok(false);
            };
            let Some(end) = module.view().get(*end).value() else {
                return Ok(false);
            };
            if module.integral_constant(start)? != Some(0) {
                return Ok(false);
            }
            let Some((dir::BinaryOperator::Subtract, [left, right])) =
                module.builtin_binary(end)?
            else {
                return Ok(false);
            };

            let Some(left_receiver) = module.length_receiver(left.source.local_id)? else {
                return Ok(false);
            };
            let Some(right_receiver) = module.length_receiver(right.source.local_id)? else {
                return Ok(false);
            };

            module.is_same_computation(left_receiver, receiver)?
                && module.is_same_computation(right_receiver, affix)?
        }
        _ => false,
    };
    if !matches {
        return Ok(false);
    }

    // require repeated evaluations to preserve their values and effects
    Ok(module.is_duplicable_expression(receiver)? && module.is_duplicable_expression(affix)?)
}

/// Build one canonical strip call.
fn fix(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    boundary: Boundary,
    receiver: dir::LocalNodeId<dir::Expression>,
    affix: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let receiver_extent = module.source_extent(receiver.into_any())?;
    let affix_extent = module.source_extent(affix.into_any())?;
    if module.has_unretained_comment(extent, &[receiver_extent, affix_extent])? {
        return Ok(None);
    }

    // retain both operands and replace the complete conditional
    let receiver = module.expression_source(receiver, dir::OperatorPrecedence::Postfix)?;
    let affix = module.expression_source(affix, dir::OperatorPrecedence::Lowest)?;
    let replacement = format!("{receiver}.{}({affix})", boundary.member());
    let patch = Patch::replace(extent, replacement);
    let fix = lint.fix("use a string stripping operation", patch)?;

    Ok(Some(fix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a guarded prefix slice.
    #[test]
    fn test_replaces_prefix_slice() {
        let session = TestSession::dir(
            &MANUAL_STRIP,
            r#"
function removePrefix(text: string, prefix: string): string | undefined {
    return text.startsWith(prefix) ? text.substring(prefix.length) : undefined;
}
"#,
        );

        session.assert_fixes(
            r#"
function removePrefix(text: string, prefix: string): string | undefined {
    return text.stripPrefix(prefix);
}
"#,
        );
    }

    /// Replace a prefix stripped through a value-producing if expression.
    #[test]
    fn test_replaces_if_expression() {
        let session = TestSession::dir(
            &MANUAL_STRIP,
            r#"
function removePrefix(text: string, prefix: string): string | undefined {
    return if (text.startsWith(prefix)) {
        text.slice(prefix.length)
    } else {
        undefined
    };
}
"#,
        );

        session.assert_fixes(
            r#"
function removePrefix(text: string, prefix: string): string | undefined {
    return text.stripPrefix(prefix);
}
"#,
        );
    }

    /// Replace a guarded suffix slice.
    #[test]
    fn test_replaces_suffix_slice() {
        let session = TestSession::dir(
            &MANUAL_STRIP,
            r#"
function removeSuffix(text: string, suffix: string): string | undefined {
    return text.endsWith(suffix)
        ? text.slice(0, text.length - suffix.length)
        : undefined;
}
"#,
        );

        session.assert_fixes(
            r#"
function removeSuffix(text: string, suffix: string): string | undefined {
    return text.stripSuffix(suffix);
}
"#,
        );
    }

    /// Accept a startsWith position that changes the tested boundary.
    #[test]
    fn test_accepts_positioned_test() {
        let session = TestSession::dir(
            &MANUAL_STRIP,
            r#"
function removePrefix(text: string, prefix: string): string | undefined {
    return text.startsWith(prefix, 1) ? text.slice(prefix.length) : undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a suffix slice that does not retain the complete prefix.
    #[test]
    fn test_accepts_different_suffix_slice() {
        let session = TestSession::dir(
            &MANUAL_STRIP,
            r#"
function removeSuffix(text: string, suffix: string): string | undefined {
    return text.endsWith(suffix) ? text.slice(0, suffix.length) : undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
