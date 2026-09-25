use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer `startsWith` and `endsWith` for string boundary tests.
    pub PREFER_STRING_STARTS_ENDS_WITH {
        id: "prefer-string-starts-ends-with",
        summary: "Prefer `startsWith` and `endsWith` for string boundary tests",
        explanation: r#"
Comparing a search index or extracted slice expresses a string boundary test through intermediate values.
Instead, you SHOULD use `startsWith` or `endsWith` to state the tested boundary directly.
"#,
        example: {
            reported: r#"
function hasPrefix(text: string, prefix: string): boolean {
    return text.indexOf(prefix) === 0;
}
"#,
            accepted: r#"
function hasPrefix(text: string, prefix: string): boolean {
    return text.startsWith(prefix);
}
"#,
        },
        provenance: [
            TypeScriptEslint("prefer-string-starts-ends-with"),
            Unicorn("prefer-string-starts-ends-with"),
        ],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// One string boundary selected by a manual test.
#[derive(Debug, Clone, Copy)]
enum Boundary {
    /// The start of the string.
    Start,
    /// The end of the string.
    End,
}

impl Boundary {
    /// Return the canonical method for this boundary.
    fn method(self) -> &'static str {
        match self {
            Self::Start => "startsWith",
            Self::End => "endsWith",
        }
    }
}

/// Report strict comparisons that manually test string boundaries.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect authored strict equality comparisons
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Binary {
            left,
            operator,
            right,
        } = node
        else {
            continue;
        };
        if !operator.is_strict_equality() {
            continue;
        }

        // recognize an index search or extracted boundary
        let selected = if let Some(selected) = index_boundary(module, *left, *right)? {
            Some(selected)
        } else if let Some(selected) = index_boundary(module, *right, *left)? {
            Some(selected)
        } else if let Some(selected) = extracted_boundary(module, *left, *right)? {
            Some(selected)
        } else {
            extracted_boundary(module, *right, *left)?
        };
        let Some((boundary, receiver, search)) = selected else {
            continue;
        };

        // replace the complete comparison with the direct boundary query
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("string boundary is tested indirectly", span);
        let is_negated = operator.is_negative_equality();
        if let Some(fix) = fix(
            module, lint, expression, boundary, receiver, search, is_negated,
        )? {
            diagnostic = diagnostic.suggestion(fix);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Recognize an index search compared with its boundary position.
fn index_boundary(
    module: &DirModule<'_>,
    search: dir::LocalNodeId<dir::Expression>,
    position: dir::LocalNodeId<dir::Expression>,
) -> Result<
    Option<(
        Boundary,
        dir::LocalNodeId<dir::Expression>,
        dir::LocalNodeId<dir::Expression>,
    )>,
    ProviderError,
> {
    let view = module.view();
    let Some(call) = module.member_call(search) else {
        return Ok(None);
    };
    if call.is_optional() {
        return Ok(None);
    }
    let [argument] = call.arguments else {
        return Ok(None);
    };
    let Some(argument) = view.get(*argument).value() else {
        return Ok(None);
    };

    // indexOf at zero tests the start directly
    let member = module.language_member(search)?;
    if member == Some(dir::LanguageItem::String.member("indexOf"))
        && module.integral_constant(position)? == Some(0)
    {
        return Ok(Some((Boundary::Start, call.receiver, argument)));
    }

    Ok(None)
}

/// Recognize a sliced string compared with the extracted boundary text.
fn extracted_boundary(
    module: &DirModule<'_>,
    extraction: dir::LocalNodeId<dir::Expression>,
    search: dir::LocalNodeId<dir::Expression>,
) -> Result<
    Option<(
        Boundary,
        dir::LocalNodeId<dir::Expression>,
        dir::LocalNodeId<dir::Expression>,
    )>,
    ProviderError,
> {
    let view = module.view();
    let Some(call) = module.member_call(extraction) else {
        return Ok(None);
    };
    if call.is_optional() {
        return Ok(None);
    }
    let member = module.language_member(extraction)?;
    if member != Some(dir::LanguageItem::String.member("slice"))
        && member != Some(dir::LanguageItem::String.member("substring"))
    {
        return Ok(None);
    }

    // slice or substring from zero through the search length tests the start
    if let [start, end] = call.arguments {
        let (Some(start), Some(end)) = (view.get(*start).value(), view.get(*end).value()) else {
            return Ok(None);
        };
        let Some(length_receiver) = module.length_receiver(end)? else {
            return Ok(None);
        };
        if module.integral_constant(start)? == Some(0)
            && module.is_same_computation(length_receiver, search)?
            && module.is_duplicable_expression(search)?
        {
            return Ok(Some((Boundary::Start, call.receiver, search)));
        }
    }

    // require one source length minus search length start
    let [start] = call.arguments else {
        return Ok(None);
    };
    let Some(start) = view.get(*start).value() else {
        return Ok(None);
    };
    let dir::Expression::Binary {
        left: source_length,
        operator: dir::BinaryOperator::Subtract,
        right: search_length,
    } = view.get(start)
    else {
        return Ok(None);
    };
    let Some(source) = module.length_receiver(*source_length)? else {
        return Ok(None);
    };
    let Some(searched) = module.length_receiver(*search_length)? else {
        return Ok(None);
    };

    // require stable matching receivers
    if module.is_same_computation(source, call.receiver)?
        && module.is_same_computation(searched, search)?
        && module.is_duplicable_expression(call.receiver)?
        && module.is_duplicable_expression(search)?
    {
        return Ok(Some((Boundary::End, call.receiver, search)));
    }

    Ok(None)
}

/// Build one direct string boundary query.
fn fix(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    boundary: Boundary,
    receiver: dir::LocalNodeId<dir::Expression>,
    search: dir::LocalNodeId<dir::Expression>,
    is_negated: bool,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let receiver_span = module.source_extent(receiver.into_any())?;
    let search_span = module.source_extent(search.into_any())?;
    if module.has_unretained_comment(extent, &[receiver_span, search_span])? {
        return Ok(None);
    }

    // retain exact sources and group the postfix receiver when required
    let receiver = module.expression_source(receiver, dir::OperatorPrecedence::Postfix)?;
    let search = module.source(search_span)?;
    let negation = if is_negated { "!" } else { "" };
    let method = boundary.method();
    let replacement = format!("{negation}{receiver}.{method}({search})");
    let patch = Patch::replace(extent, replacement);
    let fix = lint.fix("test the string boundary directly", patch)?;

    Ok(Some(fix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a negated zero index search with a direct prefix test.
    #[test]
    fn test_replaces_negated_index_search() {
        let session = TestSession::dir(
            &PREFER_STRING_STARTS_ENDS_WITH,
            r#"
function lacksPrefix(text: string, prefix: string): boolean {
    return text.indexOf(prefix) !== 0;
}
"#,
        );

        session.assert_fixes(
            r#"
function lacksPrefix(text: string, prefix: string): boolean {
    return !text.startsWith(prefix);
}
"#,
        );
    }

    /// Accept a last-index comparison that differs for longer search strings.
    #[test]
    fn test_accepts_last_index() {
        let session = TestSession::dir(
            &PREFER_STRING_STARTS_ENDS_WITH,
            r#"
function hasSuffix(text: string, suffix: string): boolean {
    return text.lastIndexOf(suffix) === text.length - suffix.length;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace prefix and suffix slice comparisons.
    #[test]
    fn test_replaces_slice_comparisons() {
        let session = TestSession::dir(
            &PREFER_STRING_STARTS_ENDS_WITH,
            r#"
function hasBoundaries(text: string, prefix: string, suffix: string): boolean {
    return text.slice(0, prefix.length) === prefix
        && text.slice(text.length - suffix.length) !== suffix
        && text.substring(text.length - suffix.length) === suffix;
}
"#,
        );

        session.assert_fixes(
            r#"
function hasBoundaries(text: string, prefix: string, suffix: string): boolean {
    return text.startsWith(prefix)
        && !text.endsWith(suffix)
        && text.endsWith(suffix);
}
"#,
        );
    }

    /// Accept a search from a nonzero position.
    #[test]
    fn test_accepts_offset_search() {
        let session = TestSession::dir(
            &PREFER_STRING_STARTS_ENDS_WITH,
            r#"
function containsAfter(text: string, prefix: string): boolean {
    return text.indexOf(prefix, 1) === 0;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
