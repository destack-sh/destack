use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer indexOf when an index predicate only compares one value.
    pub PREFER_INDEX_OF {
        id: "prefer-index-of",
        summary: "Prefer indexOf when an index predicate only compares one value",
        explanation: r#"
An index search whose predicate only compares each element with one stable value performs a callback for built-in equality.
Instead, you SHOULD call `indexOf` with the compared value.
"#,
        example: {
            reported: r#"
function locate(values: int32[], target: int32): isize | undefined {
    return values.findIndex((value) => value === target);
}
"#,
            accepted: r#"
function locate(values: int32[], target: int32): isize | undefined {
    return values.indexOf(target);
}
"#,
        },
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One direct equality index search.
#[derive(Debug, Clone, Copy)]
struct IndexSearch {
    /// The searched array.
    receiver: dir::LocalNodeId<dir::Expression>,
    /// The compared value.
    target: dir::LocalNodeId<dir::Expression>,
    /// The direct search method.
    method: &'static str,
}

/// Report canonical index searches with direct equality predicates.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect canonical Array index searches
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(search) = index_search(module, expression, &occurrences)? else {
            continue;
        };

        // replace the callback search with direct equality
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("index predicate only compares one value", span);
        if let Some(suggestion) = suggestion(module, lint, expression, search)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select one Array index search whose predicate is direct strict equality.
fn index_search(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
    occurrences: &[dir::BindingOccurrence],
) -> Result<Option<IndexSearch>, ProviderError> {
    let Some(call) = module.member_call(expression) else {
        return Ok(None);
    };
    if call.is_optional() || !call.generic_arguments.is_empty() {
        return Ok(None);
    }
    let method = match module.language_member(expression)? {
        Some(member) if member == dir::LanguageItem::Array.member("findIndex") => "indexOf",
        Some(member) if member == dir::LanguageItem::Array.member("findLastIndex") => "lastIndexOf",
        _ => return Ok(None),
    };
    let [argument] = call.arguments else {
        return Ok(None);
    };
    let dir::Argument::Positional { value: callback } = module.view().get(*argument) else {
        return Ok(None);
    };

    // require one synchronous lambda with one direct parameter
    let Some(lambda) = module.lambda(*callback) else {
        return Ok(None);
    };
    if lambda.signature.asynchrony != dir::Asynchrony::Sync || lambda.signature.is_generator {
        return Ok(None);
    }
    let [parameter] = lambda.signature.parameters.as_slice() else {
        return Ok(None);
    };
    if !matches!(
        module.view().get(*parameter),
        dir::Parameter::Named {
            default: None,
            is_optional: false,
            ..
        }
    ) {
        return Ok(None);
    }
    let Some(body) = lambda
        .body
        .and_then(|body| module.sole_value_expression(body))
    else {
        return Ok(None);
    };

    // normalize the element parameter and stable target from strict equality
    let Some((dir::BinaryOperator::EqualStrict, [left, right])) = module.builtin_binary(body)?
    else {
        return Ok(None);
    };
    let parameter = module.declaration_symbol(*parameter)?;
    let left = left.source.local_id;
    let right = right.source.local_id;
    let target = if module.selected_symbol(left)? == Some(parameter) {
        right
    } else if module.selected_symbol(right)? == Some(parameter) {
        left
    } else {
        return Ok(None);
    };
    if !module.is_speculatable_expression(target)?
        || !module
            .binding_uses_within(parameter, target.into_any(), occurrences)
            .is_empty()
    {
        return Ok(None);
    }

    Ok(Some(IndexSearch {
        receiver: call.receiver,
        target,
        method,
    }))
}

/// Build one direct Array index search.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    search: IndexSearch,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let receiver = module.source_extent(search.receiver.into_any())?;
    let target = module.source_extent(search.target.into_any())?;
    if module.has_unretained_comment(extent, &[receiver, target])? {
        return Ok(None);
    }

    // preserve the authored receiver and compared value
    let receiver = module.expression_source(search.receiver, dir::OperatorPrecedence::Postfix)?;
    let target = module.source(target)?;
    let replacement = format!("{receiver}.{}({target})", search.method);
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.suggestion("search for the value directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a forward strict equality index search.
    #[test]
    fn test_replaces_forward_search() {
        let session = TestSession::dir(
            &PREFER_INDEX_OF,
            r#"
function locate(values: int32[], target: int32): isize | undefined {
    return values.findIndex((value) => value === target);
}
"#,
        );

        session.assert_suggestions(
            r#"
function locate(values: int32[], target: int32): isize | undefined {
    return values.indexOf(target);
}
"#,
        );
    }

    /// Replace a reversed operand and reverse index search.
    #[test]
    fn test_replaces_reverse_search() {
        let session = TestSession::dir(
            &PREFER_INDEX_OF,
            r#"
function locate(values: int32[], target: int32): isize | undefined {
    return values.findLastIndex((value) => target === value);
}
"#,
        );

        session.assert_suggestions(
            r#"
function locate(values: int32[], target: int32): isize | undefined {
    return values.lastIndexOf(target);
}
"#,
        );
    }

    /// Accept a predicate that transforms the element.
    #[test]
    fn test_accepts_transformed_element() {
        let session = TestSession::dir(
            &PREFER_INDEX_OF,
            r#"
function locate(values: int32[], target: int32): isize | undefined {
    return values.findIndex((value) => value + 1 === target);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a predicate that compares against its index parameter.
    #[test]
    fn test_accepts_index_parameter() {
        let session = TestSession::dir(
            &PREFER_INDEX_OF,
            r#"
function locate(values: isize[]): isize | undefined {
    return values.findIndex((value, index) => value === index);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a compared value that depends on the element parameter.
    #[test]
    fn test_accepts_element_dependent_target() {
        let session = TestSession::dir(
            &PREFER_INDEX_OF,
            r#"
function locate(values: float64[]): isize | undefined {
    return values.findIndex((value) => value === value + 1.0);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a compared value whose evaluation has effects.
    #[test]
    fn test_accepts_effectful_target() {
        let session = TestSession::dir(
            &PREFER_INDEX_OF,
            r#"
function next(): int32 {
    return 1;
}

function locate(values: int32[]): isize | undefined {
    return values.findIndex((value) => value === next());
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
