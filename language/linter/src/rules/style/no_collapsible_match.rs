use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Merge nested matches whose inner pattern fits the outer arm.
    pub NO_COLLAPSIBLE_MATCH {
        id: "no-collapsible-match",
        summary: "Merge nested matches whose inner pattern fits the outer arm",
        explanation: r#"
A nested match on a value bound by its outer arm repeats the same selection in two places.
Instead, you SHOULD place the inner pattern at the binding site when both fallback arms have the same result.
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
        category: Style,
        level: Warning,
        fixable: Suggestion,
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
        let dir::MatchArm::Expression {
            pattern: outer_pattern,
            guard: None,
            body: inner,
        } = view.get(*selected)
        else {
            continue;
        };
        let dir::MatchArm::Expression {
            pattern: outer_fallback,
            guard: None,
            body: outer_fallback_body,
        } = view.get(*fallback)
        else {
            continue;
        };
        if !matches!(
            module.pattern_decision(*outer_fallback)?,
            dir::PatternDecision::Ignore
        ) {
            continue;
        }

        // select a nested two arm match with the same wildcard result
        let dir::Expression::Match {
            value,
            arms: inner_arms,
        } = view.get(*inner)
        else {
            continue;
        };
        let [inner_selected, inner_fallback] = inner_arms.as_slice() else {
            continue;
        };
        let dir::MatchArm::Expression {
            pattern: inner_pattern,
            guard: None,
            body: inner_body,
        } = view.get(*inner_selected)
        else {
            continue;
        };
        let dir::MatchArm::Expression {
            pattern: inner_fallback_pattern,
            guard: None,
            body: inner_fallback_body,
        } = view.get(*inner_fallback)
        else {
            continue;
        };
        if !matches!(
            module.pattern_decision(*inner_fallback_pattern)?,
            dir::PatternDecision::Ignore
        ) || matches!(
            module.pattern_decision(*inner_pattern)?,
            dir::PatternDecision::Ignore | dir::PatternDecision::Bind(_)
        ) {
            continue;
        }
        let inner_fallback_source =
            module.source(module.source_extent(inner_fallback_body.into_any())?)?;
        let outer_fallback_source =
            module.source(module.source_extent(outer_fallback_body.into_any())?)?;
        if inner_fallback_source != outer_fallback_source {
            continue;
        }

        // require the inner value to be one binding from the outer pattern
        let Some(symbol) = module.selected_symbol(*value)? else {
            continue;
        };
        let Some(binding) = authored_binding(module, *outer_pattern, symbol)? else {
            continue;
        };
        let has_other_use = module.flows.binding_occurrences().any(|occurrence| {
            occurrence.symbol == symbol
                && view.is_inside(occurrence.node, inner.into_any())
                && occurrence.node != value.into_any()
        });
        if has_other_use {
            continue;
        }

        // compose the outer and inner patterns without discarding comments
        let extent = module.source_extent(inner.into_any())?;
        let mut diagnostic =
            lint.diagnostic("nested match can be folded into its outer pattern", extent);
        if let Some(suggestion) =
            suggestion(module, lint, extent, binding, *inner_pattern, *inner_body)?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// One authored declaration for a pattern binding.
#[derive(Debug, Clone, Copy)]
struct AuthoredBinding {
    /// The complete authored declaration extent.
    extent: Span,
    /// Whether the declaration is a shorthand named field.
    is_shorthand: bool,
}

/// Return one binding declaration owned uniquely by a pattern.
fn authored_binding(
    module: &DirModule<'_>,
    pattern: dir::LocalNodeId<dir::Pattern>,
    symbol: dir::GlobalSymbolId,
) -> Result<Option<AuthoredBinding>, ProviderError> {
    let view = module.view();

    // require exactly one declaration of the binding within the outer pattern
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

    Ok(Some(AuthoredBinding {
        extent,
        is_shorthand,
    }))
}

/// Build one composed outer arm.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: Span,
    binding: AuthoredBinding,
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
    let binding_source = module.source(binding.extent)?;
    let replacement = if binding.is_shorthand {
        format!("{binding_source}: {pattern_source}")
    } else {
        pattern_source.to_string()
    };
    let body_source = module.source(body)?;
    let mut patch = FilePatch::new(extent.file);
    patch.replace(binding.extent, replacement);
    patch.replace(extent, body_source);
    let suggestion = lint.suggestion("merge the nested pattern", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;
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

    /// Accept a nested match that reads the outer binding in its selected body.
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
