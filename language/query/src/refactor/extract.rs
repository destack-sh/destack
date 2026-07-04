use destack_dir as dir;
use destack_source::Span;

use crate::ModuleQueryContext;
use crate::source::offset_line_start;

/// Refactor extraction behavior for expressions.
pub(crate) trait ExtractExpression {
    /// Return whether this expression can be extracted into a refactor target.
    fn is_extractable(&self) -> bool;
}

/// Source line position and indentation for inserting extracted declarations.
pub(crate) struct LineIndent {
    /// The line start offset.
    pub(crate) start: u32,
    /// The indentation text before the selected node.
    pub(crate) indent: String,
}

/// Source text behavior for extraction edits.
pub(crate) trait ExtractSourceText {
    /// Return expression source text that can be embedded in generated code.
    fn insertion_expression_text(&self) -> String;
}

impl ExtractExpression for dir::Expression {
    fn is_extractable(&self) -> bool {
        !matches!(
            self,
            dir::Expression::Let { .. }
                | dir::Expression::LetElse { .. }
                | dir::Expression::Using { .. }
                | dir::Expression::Declaration { .. }
                | dir::Expression::Block { .. }
                | dir::Expression::Import { .. }
                | dir::Expression::Export { .. }
        )
    }
}

impl LineIndent {
    /// Resolve the line start offset and indentation for a source position.
    pub(crate) fn at(source: &str, offset: u32) -> Self {
        let line_start = offset_line_start(source, offset as usize);

        let mut indent = String::new();
        for character in source[line_start..offset as usize].chars() {
            if character.is_whitespace() && character != '\n' && character != '\r' {
                indent.push(character);
            } else {
                break;
            }
        }

        Self {
            start: line_start as u32,
            indent,
        }
    }
}

impl ExtractSourceText for str {
    fn insertion_expression_text(&self) -> String {
        let trimmed = self.trim();
        let trimmed = trimmed.strip_suffix(';').unwrap_or(trimmed).trim_end();

        trimmed.to_string()
    }
}

impl ModuleQueryContext<'_> {
    /// Resolve the extractable expression at a selection span.
    pub(crate) fn resolve_extract_expression(
        &self,
        selection: Span,
    ) -> Option<(dir::LocalNodeId<dir::Expression>, Span)> {
        // scan expressions for the smallest span that contains the selection
        let view = self.view();
        let mut best: Option<(dir::LocalNodeId<dir::Expression>, Span)> = None;
        let mut best_len = u32::MAX;

        for (expr_id, expression) in view.iter_nodes_of_type::<dir::Expression>() {
            let span = self.get_span(view, expr_id.into());
            if !span.contains_span(selection) {
                continue;
            }

            if !expression.is_extractable() {
                continue;
            }

            let length = span.end.saturating_sub(span.start);
            if length < best_len {
                best = Some((expr_id, span));
                best_len = length;
            }
        }

        best
    }

    /// Resolve the statement span that owns an expression.
    pub(crate) fn expression_statement_span(
        &self,
        expr_id: dir::LocalNodeId<dir::Expression>,
        expression_span: Span,
    ) -> Span {
        // walk parent expressions until we find a statement boundary
        let view = self.view();
        let mut current = dir::LocalNodeIdAny::from(expr_id);

        while let Some(parent) = view.get_parent_any(current) {
            if parent.ty == dir::NodeType::Expression {
                let Ok(parent_id) = parent.try_into() else {
                    current = parent;
                    continue;
                };
                let parent_expression = view.get::<dir::Expression>(parent_id);
                if matches!(
                    parent_expression,
                    dir::Expression::Let { .. }
                        | dir::Expression::LetElse { .. }
                        | dir::Expression::Using { .. }
                        | dir::Expression::Declaration { .. }
                ) {
                    return self.get_span(view, parent);
                }
            }

            current = parent;
        }

        expression_span
    }
}
