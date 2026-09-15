use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Merge nested matches whose nested pattern fits the enclosing arm.
    pub NO_COLLAPSIBLE_MATCH {
        id: "no-collapsible-match",
        summary: "Merge nested matches whose nested pattern fits the enclosing arm",
        explanation: r#"
A nested match on a value bound by its enclosing arm repeats the same selection in two places.
Instead, you SHOULD place the nested pattern at the binding site when both fallback arms have the same result.
"#,
        example: {
            reported: r#"
function describe(result: Result<int32, string>): string {
    return match (result) {
        Ok { value } => match (value) {
            0 => "zero"
            _ => "other"
        }
        _ => "other"
    };
}
"#,
            accepted: r#"
function describe(result: Result<int32, string>): string {
    return match (result) {
        Ok { value: 0 } => "zero"
        _ => "other"
    };
}
"#,
        },
        provenance: [Clippy("collapsible_match")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        indexes: [Code],
        check: DirModule(check),
    }
}

/// Report nested matches whose fallback behavior permits pattern composition.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect two arm matches with a final wildcard fallback
    for (_, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Match { arms, .. } = node else {
            continue;
        };
        let [selected, fallback] = arms.as_slice() else {
            continue;
        };
        let Some((enclosing_pattern, nested)) = module.match_arm_expression(*selected) else {
            continue;
        };
        let Some((enclosing_fallback, enclosing_fallback_body)) =
            module.match_arm_expression(*fallback)
        else {
            continue;
        };
        if !matches!(
            module.pattern_decision(enclosing_fallback)?,
            dir::PatternDecision::Ignore
        ) {
            continue;
        }

        // select a nested two arm match with the same wildcard result
        let dir::Expression::Match {
            value,
            arms: nested_arms,
        } = view.get(nested)
        else {
            continue;
        };
        let [nested_selected, nested_fallback] = nested_arms.as_slice() else {
            continue;
        };
        let Some((nested_pattern, nested_body)) = module.match_arm_expression(*nested_selected)
        else {
            continue;
        };
        let Some((nested_fallback_pattern, nested_fallback_body)) =
            module.match_arm_expression(*nested_fallback)
        else {
            continue;
        };
        if !matches!(
            module.pattern_decision(nested_fallback_pattern)?,
            dir::PatternDecision::Ignore
        ) || matches!(
            module.pattern_decision(nested_pattern)?,
            dir::PatternDecision::Ignore | dir::PatternDecision::Bind(_)
        ) {
            continue;
        }

        // require equivalent fallback behavior
        if !module.is_alpha_equivalent(
            nested_fallback_body.into_any(),
            enclosing_fallback_body.into_any(),
        )? {
            continue;
        }

        // require the nested value to be one binding from the enclosing pattern
        let Some(symbol) = module.selected_symbol(*value)? else {
            continue;
        };
        let Some((binding_extent, is_shorthand)) =
            binding_declaration(module, enclosing_pattern, symbol)?
        else {
            continue;
        };
        let has_other_use = module.flows.binding_occurrences().any(|occurrence| {
            occurrence.symbol == symbol
                && view.is_inside(occurrence.node, nested.into_any())
                && occurrence.node != value.into_any()
        });
        if has_other_use {
            continue;
        }

        // compose the enclosing and nested patterns without discarding comments
        let extent = module.source_extent(nested.into_any())?;
        let mut diagnostic = lint.diagnostic(
            "nested match can be folded into its enclosing pattern",
            extent,
        );
        if let Some(suggestion) = suggestion(
            module,
            lint,
            extent,
            binding_extent,
            is_shorthand,
            nested_pattern,
            nested_body,
        )? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return the unique authored declaration of one pattern symbol.
fn binding_declaration(
    module: &DirModule<'_>,
    pattern: dir::LocalNodeId<dir::Pattern>,
    symbol: dir::GlobalSymbolId,
) -> Result<Option<(Span, bool)>, ProviderError> {
    let view = module.view();

    // require exactly one declaration of the binding within the enclosing pattern
    let declaration_count = module
        .symbols_declared_within(pattern.into_any())
        .filter(|declared| *declared == symbol)
        .count();
    if declaration_count != 1 {
        return Ok(None);
    }
    let declaration = module.symbol_declaration(symbol)?;
    if !view.is_inside(declaration.local_id, pattern.into_any()) {
        return Ok(None);
    }

    // distinguish shorthand fields that need their field name retained
    let is_shorthand = match declaration.local_id.ty {
        dir::NodeType::Pattern => false,
        dir::NodeType::PatternField => {
            let field = declaration.local_id.into_typed::<dir::PatternField>();
            if !matches!(
                view.get(field),
                dir::PatternField::Named {
                    is_shorthand: true,
                    ..
                }
            ) {
                return Err(ProviderError::internal(
                    "pattern field binding declaration is not a shorthand named field",
                ));
            }

            true
        }
        _ => {
            return Err(ProviderError::internal(format!(
                "pattern binding is declared by unexpected {:?} node",
                declaration.local_id.ty
            )));
        }
    };
    let extent = module.source_extent(declaration.local_id)?;

    Ok(Some((extent, is_shorthand)))
}

/// Build one composed enclosing arm.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: Span,
    binding_extent: Span,
    is_shorthand: bool,
    pattern: dir::LocalNodeId<dir::Pattern>,
    body: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let pattern = module.source_extent(pattern.into_any())?;
    let body = module.source_extent(body.into_any())?;
    if module.has_unretained_comment(extent, &[pattern, body])? {
        return Ok(None);
    }

    // replace the binding and nested match body independently
    let pattern_source = module.source(pattern)?;
    let binding_source = module.source(binding_extent)?;
    let replacement = if is_shorthand {
        format!("{binding_source}: {pattern_source}")
    } else {
        pattern_source.to_string()
    };
    let body_source = module.source(body)?;
    let mut patch = FilePatch::new(extent.file);
    patch.replace(binding_extent, replacement);
    patch.replace(extent, body_source);
    let suggestion = lint.suggestion("merge the nested pattern", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Merge nested matches written with single-expression block arms.
    #[test]
    fn test_replaces_block_arms() {
        let session = TestSession::dir(
            &NO_COLLAPSIBLE_MATCH,
            r#"
function describe(result: Result<int32, string>): string {
    return match (result) {
        Ok { value } => {
            match (value) {
                0 => { "zero" }
                _ => { "other" }
            }
        }
        _ => { "other" }
    };
}
"#,
        );

        session.assert_suggestions(
            r#"
function describe(result: Result<int32, string>): string {
    return match (result) {
        Ok { value: 0 } => {
            "zero"
        }
        _ => { "other" }
    };
}
"#,
        );
    }

    /// Merge fallbacks whose local binding names differ.
    #[test]
    fn test_replaces_alpha_equivalent_fallbacks() {
        let session = TestSession::dir(
            &NO_COLLAPSIBLE_MATCH,
            r#"
function describe(result: Result<int32, string>): string {
    return match (result) {
        Ok { value } => match (value) {
            0 => "zero"
            _ => match ((1, "other")) {
                (nested, value) => value
            }
        }
        _ => match ((1, "other")) {
            (enclosing, value) => value
        }
    };
}
"#,
        );

        session.assert_suggestions(
            r#"
function describe(result: Result<int32, string>): string {
    return match (result) {
        Ok { value: 0 } => "zero"
        _ => match ((1, "other")) {
            (enclosing, value) => value
        }
    };
}
"#,
        );
    }

    /// Accept nested matches with distinct fallback results.
    #[test]
    fn test_accepts_distinct_fallbacks() {
        let session = TestSession::dir(
            &NO_COLLAPSIBLE_MATCH,
            r#"
function describe(result: Result<int32, string>): string {
    return match (result) {
        Ok { value } => match (value) {
            0 => "zero"
            _ => "positive"
        }
        _ => "error"
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a nested match that reads the enclosing binding in its selected body.
    #[test]
    fn test_accepts_reused_binding() {
        let session = TestSession::dir(
            &NO_COLLAPSIBLE_MATCH,
            r#"
function describe(result: Result<int32, string>): string {
    return match (result) {
        Ok { value } => match (value) {
            0 => `${value}`
            _ => "other"
        }
        _ => "other"
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
