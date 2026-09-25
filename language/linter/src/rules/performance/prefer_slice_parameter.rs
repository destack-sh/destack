use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

const SLICE_EQUIVALENT_ARRAY_MEMBERS: &[&str] = &[
    "length", "size", "isEmpty", "rest", "iterator", "first", "last", "view",
];

declare_lint! {
    /// Prefer slice parameters over borrowed arrays.
    pub PREFER_SLICE_PARAMETER {
        id: "prefer-slice-parameter",
        summary: "Prefer slice parameters over borrowed arrays",
        explanation: r#"
A borrowed array parameter requires callers to provide one specific owned collection when a contiguous view is sufficient.
Instead, you SHOULD accept a borrowed slice so arrays, slices, and other contiguous storage can be passed directly.
"#,
        example: {
            reported: r#"
function first(values: &readonly int32[]): int32 {
    return values[0];
}
"#,
            accepted: r#"
function first(values: &readonly [int32]): int32 {
    return values[0];
}
"#,
        },
        provenance: [Clippy("ptr_arg")],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report parameters written as borrowed arrays.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect borrowed parameter types whose value is an authored array
    for (parameter_id, parameter) in view.iter_nodes::<dir::Parameter>() {
        let Some(declared_type) = parameter.declared_type() else {
            continue;
        };
        let dir::TypeExpression::BorrowedOf { target_type, .. } = view.get(declared_type) else {
            continue;
        };
        let dir::TypeExpression::Array { element } = view.get(*target_type) else {
            continue;
        };
        let Some(symbol) = module.sole_declared_symbol(parameter_id.into_any()) else {
            continue;
        };
        if !can_use_slice_parameter(module, parameter_id, symbol)? {
            continue;
        }

        // replace only the owned array type with its slice counterpart
        let span = module.source_extent(target_type.into_any())?;
        let mut diagnostic = lint.diagnostic("parameter borrows an owned array", span);
        if let Some(suggestion) = suggestion(module, lint, *target_type, *element)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return whether every parameter use remains valid for a slice.
fn can_use_slice_parameter(
    module: &DirModule<'_>,
    parameter: dir::LocalNodeId<dir::Parameter>,
    symbol: dir::GlobalSymbolId,
) -> Result<bool, ProviderError> {
    let view = module.view();
    let Some(body) = module.enclosing_callable_body(parameter.into_any()) else {
        return Ok(false);
    };

    // require every binding reference to remain valid
    for expression in module.binding_references(symbol) {
        if !view.is_inside(expression.into_any(), body.into_any()) {
            continue;
        }
        if !use_accepts_slice(module, expression)? {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Return whether one parameter use remains valid for a slice.
fn use_accepts_slice(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<bool, ProviderError> {
    let view = module.view();

    // accept coercions of the complete parameter to a slice
    let adjusted = module.adjusted_type_id(expression.into_any())?;
    if module.dir.representation_item(adjusted)? == Some(dir::LanguageItem::Slice) {
        return Ok(true);
    }
    let Some(parent) = view.get_parent_for(expression) else {
        return Ok(false);
    };
    let Ok(parent) = parent.try_into_typed::<dir::Expression>() else {
        return Ok(false);
    };

    // classify the direct authored use
    let is_compatible = match view.get(parent) {
        dir::Expression::Index { left, .. } if *left == expression => true,
        dir::Expression::Return { value: Some(value) } if *value == expression => {
            returns_slice(module, expression)?
        }
        dir::Expression::Member { left, .. } if *left == expression => {
            let Some(member) = module.language_member(parent)? else {
                return Ok(false);
            };

            matches!(
                member.owner,
                dir::LanguageItem::Sequence | dir::LanguageItem::Iterable
            ) || SLICE_EQUIVALENT_ARRAY_MEMBERS
                .iter()
                .any(|name| member == dir::LanguageItem::Array.member(name))
        }
        _ => false,
    };

    Ok(is_compatible)
}

/// Return whether the callable containing one node declares a slice result.
fn returns_slice(
    module: &DirModule<'_>,
    node: dir::LocalNodeId<dir::Expression>,
) -> Result<bool, ProviderError> {
    let Some(callable) = module.enclosing_callable(node.into_any()) else {
        return Ok(false);
    };
    let Some(return_type) = module.callable_return_type(callable) else {
        return Ok(false);
    };
    let return_type = module.node_type_id(return_type.into_any())?;

    Ok(module.dir.representation_item(return_type)? == Some(dir::LanguageItem::Slice))
}

/// Build one borrowed slice parameter type.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    array: dir::LocalNodeId<dir::TypeExpression>,
    element: dir::LocalNodeId<dir::TypeExpression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(array.into_any())?;
    let retained = module.source_extent(element.into_any())?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // retain the exact authored element type
    let element = module.source(retained)?;
    let patch = Patch::replace(extent, format!("[{element}]"));
    let suggestion = lint.suggestion("accept a borrowed slice", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Preserve exclusive access when replacing an array parameter.
    #[test]
    fn test_replaces_exclusive_array() {
        let session = TestSession::dir(
            &PREFER_SLICE_PARAMETER,
            r#"
function clear(values: &int32[]): void {
    values[0] = 0;
}
"#,
        );

        session.assert_suggestions(
            r#"
function clear(values: &[int32]): void {
    values[0] = 0;
}
"#,
        );
    }

    /// Accept an existing slice parameter.
    #[test]
    fn test_accepts_slice() {
        let session = TestSession::dir(
            &PREFER_SLICE_PARAMETER,
            r#"
function first(values: &readonly [int32]): int32 {
    return values[0];
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an array passed by value.
    #[test]
    fn test_accepts_owned_parameter() {
        let session = TestSession::dir(
            &PREFER_SLICE_PARAMETER,
            r#"
function consume(values: int32[]): void {
    values.length;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve an array parameter used by an array-only operation.
    #[test]
    fn test_accepts_array_operation() {
        let session = TestSession::dir(
            &PREFER_SLICE_PARAMETER,
            r#"
function append(values: &int32[]): void {
    values.push(1);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace an array borrow used through equivalent slice members.
    #[test]
    fn test_replaces_shared_members() {
        let session = TestSession::dir(
            &PREFER_SLICE_PARAMETER,
            r#"
function inspect(values: &readonly int32[]): void {
    values.length;
    values.size;
    values.isEmpty;
    values.rest(0);
    values.first();
    values.last();
    values.view(0);
}
"#,
        );

        session.assert_suggestions(
            r#"
function inspect(values: &readonly [int32]): void {
    values.length;
    values.size;
    values.isEmpty;
    values.rest(0);
    values.first();
    values.last();
    values.view(0);
}
"#,
        );
    }

    /// Preserve an array borrow whose lifetime escapes through the return type.
    #[test]
    fn test_accepts_returned_array_borrow() {
        let session = TestSession::dir(
            &PREFER_SLICE_PARAMETER,
            r#"
function identity<T, 'a>(values: &'a readonly T[]): &'a readonly T[] {
    return values;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace an array borrow that already returns a slice over its elements.
    #[test]
    fn test_replaces_array_borrow_returning_slice() {
        let session = TestSession::dir(
            &PREFER_SLICE_PARAMETER,
            r#"
function view<T, 'a>(values: &'a readonly T[]): &'a readonly [T] {
    return values;
}
"#,
        );

        session.assert_suggestions(
            r#"
function view<T, 'a>(values: &'a readonly [T]): &'a readonly [T] {
    return values;
}
"#,
        );
    }
}
