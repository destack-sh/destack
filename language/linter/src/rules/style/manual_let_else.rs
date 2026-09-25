use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer let-else over an equivalent match binding.
    pub MANUAL_LET_ELSE {
        id: "manual-let-else",
        summary: "Prefer let-else over an equivalent match binding",
        explanation: r#"
Binding a value through a match or if-let whose other branch exits obscures the required pattern and early exit.
Instead, you SHOULD use a let-else binding to state both directly.
"#,
        example: {
            reported: r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const value = match (result) {
        Ok { value } => value
        _ => return Result.err("missing value")
    };
    return Result.ok(value);
}
"#,
            accepted: r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const Ok { value } = result else {
        return Result.err("missing value");
    };
    return Result.ok(value);
}
"#,
        },
        provenance: [Clippy("manual_let_else")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report direct bindings initialized through a value-or-exit match.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect single local bindings without a declared payload type
    for (declaration, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Let {
            export: None,
            declarators,
            is_ambient: false,
            ..
        } = node
        else {
            continue;
        };
        let [declarator] = declarators.as_slice() else {
            continue;
        };
        let declarator_node = view.get(*declarator);
        if declarator_node.ty.is_some() {
            continue;
        }
        let dir::Pattern::Binding {
            name: declared_name,
            pattern: None,
        } = view.get(declarator_node.pattern)
        else {
            continue;
        };
        let Some(initializer) = declarator_node.value else {
            continue;
        };
        // recognize match and if-let value-or-exit expressions
        let let_else = match view.get(initializer) {
            dir::Expression::Match { value, arms } => {
                let [first, second] = arms.as_slice() else {
                    continue;
                };

                let mut selected = LetElse::select_match(
                    module,
                    *first,
                    *second,
                    *value,
                    *declared_name,
                    &occurrences,
                )?;

                // reorder a leading fallback only where the arms accept disjoint values
                if selected.is_none()
                    && module
                        .match_coverage(initializer)
                        .is_some_and(|coverage| coverage.is_disjoint)
                {
                    selected = LetElse::select_match(
                        module,
                        *second,
                        *first,
                        *value,
                        *declared_name,
                        &occurrences,
                    )?;
                }

                selected
            }
            dir::Expression::If { .. } => {
                LetElse::select_conditional(module, initializer, *declared_name)?
            }
            _ => None,
        };
        let Some(let_else) = let_else else {
            continue;
        };

        // replace the match initializer with a let-else binding
        let extent = module.source_extent(declaration.into_any())?;
        let mut diagnostic = lint.diagnostic("binding manually matches a value or exits", extent);
        if let Some(suggestion) =
            suggestion(module, lint, initializer, declarator_node.pattern, let_else)?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// One successful pattern and diverging fallback body.
#[derive(Debug, Clone, Copy)]
struct LetElse {
    /// The expression matched by the new binding.
    value: dir::LocalNodeId<dir::Expression>,
    /// The pattern that introduces the bound value.
    pattern: dir::LocalNodeId<dir::Pattern>,
    /// The complete fallback body.
    fallback: dir::LocalNodeIdAny,
}

impl LetElse {
    /// Return a let-else rewrite from one match binding and exiting fallback.
    fn select_match(
        module: &DirModule<'_>,
        value_arm: dir::LocalNodeId<dir::MatchArm>,
        exit_arm: dir::LocalNodeId<dir::MatchArm>,
        value: dir::LocalNodeId<dir::Expression>,
        declared_name: dir::StringId,
        occurrences: &[dir::BindingOccurrence],
    ) -> Result<Option<Self>, ProviderError> {
        let view = module.view();

        // require one pattern binding returned under the declaration's name
        let Some((pattern, body)) = module.match_arm_value(value_arm) else {
            return Ok(None);
        };
        let dir::Expression::Identifier { name } = view.get(body) else {
            return Ok(None);
        };
        if *name != declared_name {
            return Ok(None);
        }
        if module.selected_pattern_binding(pattern, body)?.is_none() {
            return Ok(None);
        }

        // require an unguarded fallback whose complete body exits
        let (fallback_pattern, fallback) = match view.get(exit_arm) {
            dir::MatchArm::Expression {
                pattern,
                guard: None,
                body,
            } if module.is_diverging(body.into_any())? => (*pattern, body.into_any()),
            dir::MatchArm::Block {
                pattern,
                guard: None,
                body,
            } if module.is_diverging(body.into_any())? => (*pattern, body.into_any()),
            _ => return Ok(None),
        };

        // reject fallbacks that depend on bindings unavailable in let-else
        for binding in module.symbols_declared_within(fallback_pattern.into_any()) {
            let uses = module.binding_uses_within(binding, fallback, occurrences);
            if !uses.is_empty() {
                return Ok(None);
            }
        }

        Ok(Some(Self {
            value,
            pattern,
            fallback,
        }))
    }

    /// Return a let-else rewrite from one if-let binding and exiting fallback.
    fn select_conditional(
        module: &DirModule<'_>,
        expression: dir::LocalNodeId<dir::Expression>,
        declared_name: dir::StringId,
    ) -> Result<Option<Self>, ProviderError> {
        let view = module.view();
        let dir::Expression::If {
            form: dir::IfForm::If,
            condition,
            then_expression,
            else_expression: Some(else_expression),
        } = view.get(expression)
        else {
            return Ok(None);
        };
        let Some((_, _, declarator)) = condition.as_binding() else {
            return Ok(None);
        };
        let declarator = view.get(declarator);
        if declarator.ty.is_some() {
            return Ok(None);
        }
        let Some(value) = declarator.value else {
            return Ok(None);
        };

        // require the successful branch to return its sole pattern binding
        let Some(body) = module.sole_value_expression(*then_expression) else {
            return Ok(None);
        };
        let dir::Expression::Identifier { name } = view.get(body) else {
            return Ok(None);
        };
        if *name != declared_name {
            return Ok(None);
        }
        if module
            .selected_pattern_binding(declarator.pattern, body)?
            .is_none()
        {
            return Ok(None);
        }

        // require the complete fallback branch to exit
        if !module.is_diverging(else_expression.into_any())? {
            return Ok(None);
        }

        Ok(Some(Self {
            value,
            pattern: declarator.pattern,
            fallback: else_expression.into_any(),
        }))
    }
}

/// Build one let-else binding rewrite.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    initializer: dir::LocalNodeId<dir::Expression>,
    declaration: dir::LocalNodeId<dir::Pattern>,
    let_else: LetElse,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(initializer.into_any())?;
    let pattern = module.source_extent(let_else.pattern.into_any())?;
    let value_extent = module.source_extent(let_else.value.into_any())?;
    let fallback = module.source_extent(let_else.fallback)?;
    if module.has_unretained_comment(extent, &[pattern, value_extent, fallback])? {
        return Ok(None);
    }

    // retain the successful pattern as the declaration pattern
    let pattern_source = module.source(pattern)?;
    let declaration = module.source_extent(declaration.into_any())?;
    let mut file = FilePatch::new(extent.file);
    file.replace(declaration, pattern_source);

    // retain the fallback block or wrap one exiting expression
    let value = module.expression_source(let_else.value, dir::OperatorPrecedence::Lowest)?;
    let fallback_block = if let Ok(block) = let_else.fallback.try_into_typed::<dir::Block>() {
        Some(block)
    } else if let Ok(expression) = let_else.fallback.try_into_typed::<dir::Expression>()
        && let dir::Expression::Block(block) = module.view().get(expression)
    {
        Some(*block)
    } else {
        None
    };
    let fallback = if fallback_block.is_some() {
        let declaration_indentation = module.source_indentation(declaration)?.len();
        let fallback_indentation = module.source_indentation(fallback)?.len();
        let indentation = fallback_indentation
            .checked_sub(declaration_indentation)
            .ok_or_else(|| {
                ProviderError::internal("fallback is less indented than its declaration")
            })? as u32;
        let Some(fallback) = module.dedent_source(fallback, indentation)? else {
            return Ok(None);
        };

        fallback
    } else {
        let indentation = module.source_indentation(extent)?;
        let fallback = module.source(fallback)?;

        format!("{{\n{indentation}    {fallback};\n{indentation}}}")
    };
    file.push(Patch::replace(extent, format!("{value} else {fallback}")));
    file.sort();
    let suggestion = lint.suggestion("use a let-else binding", file)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace disjoint match arms whose fallback appears first.
    #[test]
    fn test_replaces_reversed_disjoint_match() {
        let session = TestSession::dir(
            &MANUAL_LET_ELSE,
            r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const value = match (result) {
        Err { error: _ } => return Result.err("missing value")
        Ok { value } => value
    };
    return Result.ok(value);
}
"#,
        );

        session.assert_suggestions(
            r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const Ok { value } = result else {
        return Result.err("missing value");
    };
    return Result.ok(value);
}
"#,
        );
    }

    /// Replace an if-let binding whose fallback returns.
    #[test]
    fn test_replaces_returning_conditional() {
        let session = TestSession::dir(
            &MANUAL_LET_ELSE,
            r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const value = if (let Ok { value } = result) {
        value
    } else {
        return Result.err("missing value");
    };
    return Result.ok(value);
}
"#,
        );

        session.assert_suggestions(
            r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const Ok { value } = result else {
        return Result.err("missing value");
    };
    return Result.ok(value);
}
"#,
        );
    }

    /// Preserve an authored fallback block.
    #[test]
    fn test_replaces_fallback_block() {
        let session = TestSession::dir(
            &MANUAL_LET_ELSE,
            r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const value = match (result) {
        Ok { value } => value
        _ => {
            "missing value";
            return Result.err("missing value");
        }
    };
    return Result.ok(value);
}
"#,
        );

        session.assert_suggestions(
            r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const Ok { value } = result else {
        "missing value";
        return Result.err("missing value");
    };
    return Result.ok(value);
}
"#,
        );
    }

    /// Accept a fallback that produces a value instead of exiting.
    #[test]
    fn test_accepts_value_fallback() {
        let session = TestSession::dir(
            &MANUAL_LET_ELSE,
            r#"
function value(result: Result<int32, string>): int32 {
    const value = match (result) {
        Ok { value } => value
        _ => 0
    };
    return value;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a fallback that reads a binding unavailable to let-else.
    #[test]
    fn test_accepts_fallback_pattern_binding() {
        let session = TestSession::dir(
            &MANUAL_LET_ELSE,
            r#"
function value(result: Result<int32, string>): Result<int32, string> {
    const value = match (result) {
        Ok { value } => value
        Err { error } => return Result.err(error)
    };
    return Result.ok(value);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
