use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow rebuilding an owned array through an identity map.
    pub MAP_IDENTITY {
        id: "map-identity",
        summary: "Disallow rebuilding an owned array through an identity map",
        explanation: r#"
Mapping every element to itself rebuilds an owned array without changing its values.
Instead, you SHOULD use the original owned array directly.
"#,
        example: {
            reported: r#"
function identity(values: ^int32[]): ^int32[] {
    return values.map((value) => value);
}
"#,
            accepted: r#"
function identity(values: ^int32[]): ^int32[] {
    return values;
}
"#,
        },
        provenance: [Clippy("map_identity")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report canonical Array map calls whose callback returns its first parameter.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical Array.map calls with one callback
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional()
            || module.language_member(expression)? != Some(dir::LanguageItem::Array.member("map"))
        {
            continue;
        }
        let [argument] = call.arguments else {
            continue;
        };
        let dir::Argument::Positional { value: callback } = view.get(*argument) else {
            continue;
        };

        // require an expression lambda that returns its first parameter unchanged
        let Some(lambda) = module.lambda(*callback) else {
            continue;
        };
        if lambda.signature.asynchrony != dir::Asynchrony::Sync || lambda.signature.is_generator {
            continue;
        }
        let Some(parameter) = lambda.signature.parameters.first().copied() else {
            continue;
        };
        if !matches!(view.get(parameter), dir::Parameter::Named { .. }) {
            continue;
        }
        let Some(body) = lambda.body.and_then(|body| module.value_expression(body)) else {
            continue;
        };
        let parameter = module.declaration_symbol(parameter)?;
        if module.selected_symbol(body)? != Some(parameter) {
            continue;
        }

        // require owned arrays and an unchanged callback result
        let receiver_type_id = module.node_type_id(call.receiver.into_any())?;
        let result_type_id = module.node_type_id(expression.into_any())?;
        let receiver_type = module.dir.get_type(receiver_type_id)?;
        let result_type = module.dir.get_type(result_type_id)?;
        let is_owned = matches!(receiver_type, dir::Type::Form(form) if form.form == dir::Form::Owned)
            && matches!(result_type, dir::Type::Form(form) if form.form == dir::Form::Owned);
        let is_array = module.representation_item(call.receiver.into_any())?
            == Some(dir::LanguageItem::Array)
            && module.representation_item(expression.into_any())? == Some(dir::LanguageItem::Array);
        let is_unadjusted = module.is_unadjusted(body.into_any());
        if !is_owned || !is_array || !is_unadjusted {
            continue;
        }

        // remove the identity operation while retaining the owned receiver
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("array is mapped through the identity function", span);
        if let Some(suggestion) = suggestion(module, lint, expression, call.receiver)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Remove one identity map suffix.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    receiver: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let receiver = module.source_extent(receiver.into_any())?;
    if !extent.contains_span(receiver) {
        return Err(ProviderError::internal(
            "identity map extent does not contain its receiver",
        ));
    }
    if module.has_unretained_comment(extent, &[receiver])? {
        return Ok(None);
    }

    // retain the complete receiver and remove the mapping suffix
    let suffix = Span::new(extent.file, receiver.end, extent.end);
    let mut file = FilePatch::new(extent.file);
    file.delete(suffix);
    let suggestion = lint.fix("use the owned array directly", file)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Remove an identity map from an owned array.
    #[test]
    fn test_removes_owned_identity_map() {
        let session = TestSession::dir(
            &MAP_IDENTITY,
            r#"
function identity(values: ^int32[]): ^int32[] {
    return values.map((value) => value);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[map-identity]: array is mapped through the identity function
 ──▶ main.tspp:2:12
  │
1 │ function identity(values: ^int32[]): ^int32[] {
2 │     return values.map((value) => value);
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: use the owned array directly
--- a/main.tspp
+++ b/main.tspp

    1│ function identity(values: ^int32[]): ^int32[] {
-   2│     return values.map((value) => value);
+   2│     return values;
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function identity(values: ^int32[]): ^int32[] {
    return values;
}
"#,
        );
    }

    /// Accept a map whose callback changes each value.
    #[test]
    fn test_accepts_transform() {
        let session = TestSession::dir(
            &MAP_IDENTITY,
            r#"
function increment(values: ^int32[]): int32[] {
    return values.map((value) => value + 1);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a borrowed array because map materializes an owned result.
    #[test]
    fn test_accepts_borrowed_array() {
        let session = TestSession::dir(
            &MAP_IDENTITY,
            r#"
function copy(values: int32[]): int32[] {
    return values.map((value) => value);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user-defined map method.
    #[test]
    fn test_accepts_user_map() {
        let session = TestSession::dir(
            &MAP_IDENTITY,
            r#"
class Value {
    map(mapper: (value: int32) => int32): int32 {
        return mapper(1);
    }
}

function identity(value: Value): int32 {
    return value.map((item) => item);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an identity expression whose contextual result changes the element type.
    #[test]
    fn test_accepts_changed_result_type() {
        let session = TestSession::dir(
            &MAP_IDENTITY,
            r#"
function widen(values: ^int32[]): ^unknown[] {
    return values.map((value): unknown => value);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an async callback because it wraps each returned value.
    #[test]
    fn test_accepts_async_identity_body() {
        let session = TestSession::dir(
            &MAP_IDENTITY,
            r#"
function schedule(values: ^int32[]): void {
    const promises = values.map(async (value) => value);
    promises;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
