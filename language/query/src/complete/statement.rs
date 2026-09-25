use tspp_dir as dir;
use tspp_source::EnclosingSpan;

use super::CompletionPosition;
use crate::cursor::Cursor;
use crate::{ModuleQueryContext, QueryResult};

impl ModuleQueryContext<'_> {
    /// Return whether the cursor is at a statement position inside one block.
    pub(super) fn block_owns_statement_cursor(
        &self,
        enclosing: &EnclosingSpan,
        offset: u32,
    ) -> QueryResult<bool> {
        let view = self.view()?;

        // only block spans can expose statement positions
        let Some(node_id) = view.get_node_id_by_source_id(enclosing.source_id) else {
            return Ok(false);
        };
        if node_id.ty != dir::NodeType::Block {
            return Ok(false);
        }

        let block_id = dir::LocalNodeId::<dir::Block>::new(node_id.id);
        let block = view.get(block_id);

        // empty blocks always expose one statement gap
        if block.is_empty() {
            return Ok(true);
        }

        // track the last completed expression before the cursor
        let mut last_expression_before_cursor = None;

        for expression_id in block.iter_expressions() {
            let span = self.node_span(view, expression_id.into())?;

            // statement heads only count when the cursor is on the owning statement span
            if span.owns_cursor(offset) {
                let expression = view.get(expression_id);
                if matches!(expression, dir::Expression::Missing) || expression.is_wide() {
                    return Ok(false);
                }

                return Ok(self
                    .node_selection_span(view, expression_id.into())?
                    .is_some_and(|main_span| main_span.owns_cursor(offset)));
            }

            // remember the last expression before the cursor
            if span.end <= offset {
                last_expression_before_cursor = Some(expression_id);
            }
        }

        // trailing missing slots still belong to the current statement
        if let Some(expression_id) = last_expression_before_cursor
            && self.has_trailing_expression_hole(expression_id)?
        {
            return Ok(false);
        }

        Ok(true)
    }

    /// Return whether one expression still owns a trailing expression hole.
    fn has_trailing_expression_hole(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> QueryResult<bool> {
        let view = self.view()?;
        let expression = view.get(expression_id);

        let has_hole = match expression {
            dir::Expression::Let { declarators, .. }
            | dir::Expression::Using { declarators, .. } => {
                let Some(last_declarator) = declarators.last() else {
                    return Ok(false);
                };
                let declarator = view.get(*last_declarator);
                let Some(value) = declarator.value else {
                    return Ok(false);
                };

                matches!(view.get(value), dir::Expression::Missing)
            }
            dir::Expression::Assign { right, .. } => {
                matches!(view.get(*right), dir::Expression::Missing)
            }
            dir::Expression::Return { value } | dir::Expression::Yield { value, .. } => {
                value.is_some_and(|value| matches!(view.get(value), dir::Expression::Missing))
            }
            _ => false,
        };

        Ok(has_hole)
    }
}

impl Cursor<'_, '_> {
    /// Classify statement completion at one offset.
    pub(super) fn classify_statement(&self) -> QueryResult<Option<CompletionPosition>> {
        // treat the start of the file as a statement position
        if self.offset == 0 {
            let scope = self.module.module_start_scope()?;

            return Ok(Some(CompletionPosition::Statement { scope }));
        }

        // skip declarator initializer holes
        if self.is_declarator_value_hole()? {
            return Ok(None);
        }

        // check block based statement gaps first
        if let Some(scope) = self.statement_scope_in_block()? {
            return Ok(Some(CompletionPosition::Statement { scope }));
        }

        // use token based statement boundaries
        if let Some(token) = self
            .module
            .previous_significant_token(self.file_id, self.offset)?
        {
            let opens_statement = matches!(
                token.token.ty(),
                dir::TokenType::Semicolon | dir::TokenType::OpenBrace | dir::TokenType::CloseBrace
            );
            if opens_statement {
                let Some(scope) = self.scope()? else {
                    return Ok(None);
                };

                return Ok(Some(CompletionPosition::Statement { scope }));
            }
        }

        Ok(None)
    }

    /// Resolve a statement position inside a block expression.
    fn statement_scope_in_block(&self) -> QueryResult<Option<dir::LocalScope>> {
        let offset = self.offset;
        let enclosing = self.enclosing();

        // scan for the nearest block that opens one statement position
        for enclosing_span in enclosing {
            if self
                .module
                .block_owns_statement_cursor(enclosing_span, offset)?
            {
                return self.scope();
            }
        }

        Ok(None)
    }
}
