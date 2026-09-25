use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch, NodeSpanRegion, Patch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer while-let over an unconditional pattern loop.
    pub WHILE_LET_LOOP {
        id: "while-let-loop",
        summary: "Prefer while-let over an unconditional pattern loop",
        explanation: r#"
An unconditional loop that continues only while one pattern matches hides its condition in the body.
Instead, you SHOULD place the successful pattern and value in a while-let condition.

The matched expression remains evaluated once at the start of each iteration.
"#,
        example: {
            reported: r#"
declare function next(): Result<int32, void>;

function consume(): void {
    loop {
        match (next()) {
            Ok { value } => {
                value;
            }
            Err { error: _ } => break
        }
    }
}
"#,
            accepted: r#"
declare function next(): Result<int32, void>;

function consume(): void {
    while (let Ok { value } = next()) {
        value;
    }
}
"#,
        },
        provenance: [Clippy("while_let_loop")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One match arm retained as a while-let body.
#[derive(Debug, Clone, Copy)]
struct WhileLet {
    /// The match replaced by the while-let condition.
    span: Span,
    /// The successful pattern.
    pattern: dir::LocalNodeId<dir::Pattern>,
    /// The expression evaluated by each condition.
    value: dir::LocalNodeId<dir::Expression>,
    /// The loop body retained by the rewrite.
    body: WhileLetBody,
}

/// The loop body retained by a while-let rewrite.
#[derive(Debug, Clone, Copy)]
enum WhileLetBody {
    /// One complete successful branch body.
    Branch {
        /// The retained branch body.
        body: dir::LocalNodeIdAny,
    },
    /// The existing loop body after removing one declaration.
    Remainder {
        /// The declaration replaced by the while-let condition.
        declaration: dir::LocalNodeId<dir::Expression>,
    },
}

/// Report unconditional loops that only repeat one successful match arm.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect unconditional loops whose body is one match
    for (iteration, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Loop { body, .. } = node else {
            continue;
        };
        if module.has_valued_break(iteration)? {
            continue;
        }

        // select a sole control expression or one leading binding
        let block = view.get(*body);
        let retained = if let Some(expression) = block.only_expression() {
            WhileLet::select_control(module, iteration, expression)?
        } else if let Some(declaration) = block.leading_expressions.first() {
            let let_else = WhileLet::select_let_else(module, iteration, *declaration)?;
            if let_else.is_some() {
                let_else
            } else {
                WhileLet::select_binding(module, iteration, *declaration)?
            }
        } else {
            None
        };
        let Some(retained) = retained else {
            continue;
        };

        // replace the match loop with a while-let loop
        let mut diagnostic =
            lint.diagnostic("loop repeats while one pattern matches", retained.span);
        if let Some(suggestion) = suggestion(module, lint, iteration, *body, retained)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

impl WhileLet {
    /// Return a while-let rewrite from one complete match or if-let loop body.
    fn select_control(
        module: &DirModule<'_>,
        iteration: dir::LocalNodeId<dir::Expression>,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<Self>, ProviderError> {
        let view = module.view();

        // select one successful match or if-let branch followed by a break
        let (pattern, value, body) = match view.get(expression) {
            dir::Expression::Match { value, arms } => {
                let [first, second] = arms.as_slice() else {
                    return Ok(None);
                };

                // reorder a leading break arm only where the arms accept disjoint values
                let retained = if module.break_arm_target(*second)? == Some(iteration) {
                    first
                } else if module.break_arm_target(*first)? == Some(iteration)
                    && module
                        .match_coverage(expression)
                        .is_some_and(|coverage| coverage.is_disjoint)
                {
                    second
                } else {
                    return Ok(None);
                };
                let (pattern, body) = match view.get(*retained) {
                    dir::MatchArm::Expression {
                        pattern,
                        guard: None,
                        body,
                    } => (*pattern, body.into_any()),
                    dir::MatchArm::Block {
                        pattern,
                        guard: None,
                        body,
                    } => (*pattern, body.into_any()),
                    _ => return Ok(None),
                };

                (pattern, *value, body)
            }
            dir::Expression::If {
                form: dir::IfForm::If,
                condition,
                then_expression,
                else_expression: Some(else_expression),
            } => {
                let Some((_, _, declarator)) = condition.as_binding() else {
                    return Ok(None);
                };
                let declarator = view.get(declarator);
                if declarator.ty.is_some()
                    || module.plain_break_target(*else_expression)? != Some(iteration)
                {
                    return Ok(None);
                }
                let Some(value) = declarator.value else {
                    return Ok(None);
                };
                let body = match view.get(*then_expression) {
                    dir::Expression::Block(block) => block.into_any(),
                    _ => then_expression.into_any(),
                };

                (declarator.pattern, value, body)
            }
            _ => return Ok(None),
        };
        if matches!(view.get(pattern), dir::Pattern::Wildcard) {
            return Ok(None);
        }

        Ok(Some(Self {
            span: module.source_extent(expression.into_any())?,
            pattern,
            value,
            body: WhileLetBody::Branch { body },
        }))
    }

    /// Return a while-let rewrite from one leading let-else binding.
    fn select_let_else(
        module: &DirModule<'_>,
        iteration: dir::LocalNodeId<dir::Expression>,
        declaration: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<Self>, ProviderError> {
        let view = module.view();
        let dir::Expression::LetElse {
            declarator,
            else_branch,
            ..
        } = view.get(declaration)
        else {
            return Ok(None);
        };
        let declarator = view.get(*declarator);
        if declarator.ty.is_some() || module.plain_break_target(*else_branch)? != Some(iteration) {
            return Ok(None);
        }
        let Some(value) = declarator.value else {
            return Ok(None);
        };

        Ok(Some(Self {
            span: module.source_extent(declaration.into_any())?,
            pattern: declarator.pattern,
            value,
            body: WhileLetBody::Remainder { declaration },
        }))
    }

    /// Return a while-let rewrite from one leading value-or-break binding.
    fn select_binding(
        module: &DirModule<'_>,
        iteration: dir::LocalNodeId<dir::Expression>,
        declaration: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<Self>, ProviderError> {
        let view = module.view();
        let Some((_, declarator)) = module.binding_declarator(declaration) else {
            return Ok(None);
        };
        if declarator.ty.is_some() {
            return Ok(None);
        }
        let dir::Pattern::Binding {
            name: declared_name,
            ..
        } = view.get(declarator.pattern)
        else {
            return Ok(None);
        };
        let Some(initializer) = declarator.value else {
            return Ok(None);
        };

        // select one pattern value and one direct loop break
        let (pattern, value, body) = match view.get(initializer) {
            dir::Expression::Match { value, arms } => {
                let [first, second] = arms.as_slice() else {
                    return Ok(None);
                };
                let retained = if module.break_arm_target(*second)? == Some(iteration) {
                    first
                } else if module.break_arm_target(*first)? == Some(iteration)
                    && module
                        .match_coverage(initializer)
                        .is_some_and(|coverage| coverage.is_disjoint)
                {
                    second
                } else {
                    return Ok(None);
                };
                let Some((pattern, body)) = module.match_arm_value(*retained) else {
                    return Ok(None);
                };

                (pattern, *value, body)
            }
            dir::Expression::If {
                form: dir::IfForm::If,
                condition,
                then_expression,
                else_expression: Some(else_expression),
            } => {
                let Some((_, _, declarator)) = condition.as_binding() else {
                    return Ok(None);
                };
                let declarator = view.get(declarator);
                if declarator.ty.is_some()
                    || module.plain_break_target(*else_expression)? != Some(iteration)
                {
                    return Ok(None);
                }
                let Some(value) = declarator.value else {
                    return Ok(None);
                };
                let Some(body) = module.sole_value_expression(*then_expression) else {
                    return Ok(None);
                };

                (declarator.pattern, value, body)
            }
            _ => return Ok(None),
        };

        // require the successful branch to return the declared binding
        let dir::Expression::Identifier { name } = view.get(body) else {
            return Ok(None);
        };
        if name != declared_name || module.selected_pattern_binding(pattern, body)?.is_none() {
            return Ok(None);
        }

        Ok(Some(Self {
            span: module.source_extent(initializer.into_any())?,
            pattern,
            value,
            body: WhileLetBody::Remainder { declaration },
        }))
    }
}

/// Build one while-let loop rewrite.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    iteration: dir::LocalNodeId<dir::Expression>,
    body: dir::LocalNodeId<dir::Block>,
    retained: WhileLet,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(iteration.into_any())?;
    let pattern_span = module.source_extent(retained.pattern.into_any())?;
    let value_span = module.source_extent(retained.value.into_any())?;
    // build the while-let header
    let pattern = module.source(pattern_span)?;
    let value = module.expression_source(retained.value, dir::OperatorPrecedence::Lowest)?;
    let header = format!("while (let {pattern} = {value})");
    let mut file = FilePatch::new(extent.file);
    file.replace(
        module.source_region(iteration.into_any(), NodeSpanRegion::Keyword)?,
        header,
    );

    // retain one successful branch or remove one leading binding
    match retained.body {
        WhileLetBody::Branch { body: branch } => {
            let source = module.source_extent(branch)?;
            if module.has_unretained_comment(extent, &[pattern_span, value_span, source])? {
                return Ok(None);
            }
            let body_source = if branch.try_into_typed::<dir::Block>().is_ok() {
                let loop_indentation = module.source_indentation(extent)?.len();
                let arm_indentation = module.source_indentation(source)?.len();
                let indentation =
                    arm_indentation
                        .checked_sub(loop_indentation)
                        .ok_or_else(|| {
                            ProviderError::internal("branch is less indented than its loop")
                        })? as u32;
                let Some(body) = module.dedent_source(source, indentation)? else {
                    return Ok(None);
                };

                body
            } else {
                let indentation = module.source_indentation(extent)?;
                let expression = module.source(source)?;

                format!("{{\n{indentation}    {expression};\n{indentation}}}")
            };
            file.push(Patch::replace(
                module.source_extent(body.into_any())?,
                body_source,
            ));
        }
        WhileLetBody::Remainder { declaration } => {
            let statement = module.statement_span(declaration)?;
            let removed = module.line_removal_span(statement)?;
            if module.has_unretained_comment(removed, &[pattern_span, value_span])? {
                return Ok(None);
            }

            file.delete(removed);
        }
    };
    file.sort();
    let suggestion = lint.suggestion("use a while-let loop", file)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a match whose break arm appears first.
    #[test]
    fn test_replaces_reversed_arms() {
        let session = TestSession::dir(
            &WHILE_LET_LOOP,
            r#"
declare function next(): Result<int32, void>;

function consume(): void {
    loop {
        match (next()) {
            Err { error: _ } => break
            Ok { value } => value;
        }
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
declare function next(): Result<int32, void>;

function consume(): void {
    while (let Ok { value } = next()) {
        value;
    }
}
"#,
        );
    }

    /// Replace an if-let loop with a breaking fallback.
    #[test]
    fn test_replaces_conditional() {
        let session = TestSession::dir(
            &WHILE_LET_LOOP,
            r#"
declare function next(): Result<int32, void>;

function consume(): void {
    loop {
        if (let Ok { value } = next()) {
            value;
        } else {
            break;
        }
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
declare function next(): Result<int32, void>;

function consume(): void {
    while (let Ok { value } = next()) {
        value;
    }
}
"#,
        );
    }

    /// Move a leading let-else binding into the loop condition.
    #[test]
    fn test_replaces_leading_let_else() {
        let session = TestSession::dir(
            &WHILE_LET_LOOP,
            r#"
declare function next(): Result<int32, void>;

function consume(): void {
    loop {
        const Ok { value } = next() else {
            break;
        };
        value;
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
declare function next(): Result<int32, void>;

function consume(): void {
    while (let Ok { value } = next()) {
        value;
    }
}
"#,
        );
    }

    /// Move a leading match binding into the loop condition.
    #[test]
    fn test_replaces_leading_match_binding() {
        let session = TestSession::dir(
            &WHILE_LET_LOOP,
            r#"
declare function next(): Result<int32, void>;

function consume(): void {
    loop {
        const value = match (next()) {
            Ok { value } => value
            Err { error: _ } => break
        };
        value;
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
declare function next(): Result<int32, void>;

function consume(): void {
    while (let Ok { value } = next()) {
        value;
    }
}
"#,
        );
    }

    /// Move a reversed leading match binding into the loop condition.
    #[test]
    fn test_replaces_reversed_leading_match_binding() {
        let session = TestSession::dir(
            &WHILE_LET_LOOP,
            r#"
declare function next(): Result<int32, void>;

function consume(): void {
    loop {
        const value = match (next()) {
            Err { error: _ } => break
            Ok { value } => value
        };
        value;
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
declare function next(): Result<int32, void>;

function consume(): void {
    while (let Ok { value } = next()) {
        value;
    }
}
"#,
        );
    }

    /// Move a leading if-let binding into the loop condition.
    #[test]
    fn test_replaces_leading_conditional_binding() {
        let session = TestSession::dir(
            &WHILE_LET_LOOP,
            r#"
declare function next(): Result<int32, void>;

function consume(): void {
    loop {
        const value = if (let Ok { value } = next()) {
            value
        } else {
            break
        };
        value;
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
declare function next(): Result<int32, void>;

function consume(): void {
    while (let Ok { value } = next()) {
        value;
    }
}
"#,
        );
    }

    /// Accept a loop whose match is followed by more work.
    #[test]
    fn test_accepts_additional_work() {
        let session = TestSession::dir(
            &WHILE_LET_LOOP,
            r#"
declare function next(): Result<int32, void>;

function consume(): void {
    loop {
        match (next()) {
            Ok { value } => value;
            Err { error: _ } => break
        }
        "after";
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a loop whose exit carries its result.
    #[test]
    fn test_accepts_valued_break() {
        let session = TestSession::dir(
            &WHILE_LET_LOOP,
            r#"
declare function next(): Result<int32, string>;

function consume(): string {
    return loop {
        match (next()) {
            Ok { value } => value;
            Err { error } => break (error)
        }
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
