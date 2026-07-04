use destack_dir as dir;

use crate::ModuleQueryContext;

use super::{CompletionContext, CursorToken};

impl ModuleQueryContext<'_> {
    /// Return member access context near the cursor.
    pub(super) fn member_access_context(
        &self,
        token: &Option<CursorToken>,
        offset: u32,
    ) -> Option<CompletionContext> {
        let cursor_position = offset.saturating_sub(1);

        // detect member access inside an existing member name token
        if let Some(context) = self.member_access_context_at_member_name(cursor_position) {
            return Some(context);
        }

        // resolve member access context when immediately after one dot boundary
        if let Some(context) = self.member_access_context_after_dot(offset) {
            return Some(context);
        }

        // resolve member access when the cursor is inside a member name
        if let Some(token_at_cursor) = token.as_ref() {
            if let Some(context) = self.member_access_context_after_dot(token_at_cursor.start) {
                return Some(context);
            }
        }

        None
    }

    /// Return member access from an existing member name token.
    fn member_access_context_at_member_name(
        &self,
        cursor_position: u32,
    ) -> Option<CompletionContext> {
        let token_at_cursor = self.token_span_at_offset(cursor_position)?;
        if token_at_cursor.token.ty() != dir::TokenType::Identifier {
            return None;
        }

        let view = self.view();

        // resolve enclosing spans from innermost to outermost
        let enclosing = self.sorted_enclosing_spans(cursor_position, cursor_position);

        // scan enclosing spans for one member expression at the cursor
        for enc in &enclosing {
            let main_span = self.tree().source_index.get_main(enc.source_id);
            let Some(main_span) = main_span else {
                continue;
            };
            let is_in_member_name = main_span.contains(cursor_position);
            if !is_in_member_name {
                continue;
            }

            let Some(dir_node_id) = view.get_node_id_by_source_id(enc.source_id) else {
                continue;
            };
            if dir_node_id.ty != dir::NodeType::Expression {
                continue;
            }

            let expr_id = dir_node_id.try_into().unwrap_or_else(|_| {
                panic!("member access source node is not an expression: {dir_node_id:?}")
            });
            let expr = view.get::<dir::Expression>(expr_id);

            // use the left operand when inside a member expression
            let dir::Expression::Member { left, .. } = expr else {
                continue;
            };

            let receiver_local: dir::LocalNodeIdAny = (*left).into();
            let receiver_symbol = self.expression_symbol_target(*left);
            return Some(self.member_access_completion_context(receiver_local, receiver_symbol));
        }

        None
    }

    /// Return the type of a receiver expression.
    fn receiver_type(
        &self,
        receiver_global: dir::GlobalNodeIdAny,
        receiver_symbol: Option<dir::GlobalSymbolId>,
    ) -> Option<dir::GlobalTypeId> {
        let types = self.types();
        let node_type_id = types.get_node_type_id(receiver_global);
        let symbol_type_id = receiver_symbol.and_then(|symbol| types.get_symbol_type_id(symbol));

        node_type_id.or(symbol_type_id)
    }

    /// Return the completion context for a member access receiver.
    fn member_access_completion_context(
        &self,
        receiver_node: dir::LocalNodeIdAny,
        receiver_symbol: Option<dir::GlobalSymbolId>,
    ) -> CompletionContext {
        let receiver_global = receiver_node.into_global(self.module_id());
        let receiver_type = self.receiver_type(receiver_global, receiver_symbol);

        CompletionContext::MemberAccess {
            receiver_node,
            receiver_symbol,
            receiver_type,
        }
    }

    /// Return an unresolved member access completion context.
    fn unresolved_member_access_completion_context(
        &self,
        receiver_node: dir::LocalNodeIdAny,
    ) -> CompletionContext {
        CompletionContext::MemberAccess {
            receiver_node,
            receiver_symbol: None,
            receiver_type: None,
        }
    }

    /// Resolve member access context for a receiver position.
    fn member_access_context_at_offset(&self, receiver_position: u32) -> Option<CompletionContext> {
        let enclosing = self.sorted_enclosing_spans(receiver_position, receiver_position);
        let view = self.view();
        let mut partial_context = None;

        // scan for the nearest enclosing expression
        for enc in &enclosing {
            let Some(dir_node_id) = view.get_node_id_by_source_id(enc.source_id) else {
                continue;
            };
            if dir_node_id.ty != dir::NodeType::Expression {
                continue;
            }

            let expr_id = dir_node_id.try_into().unwrap_or_else(|_| {
                panic!("member access source node is not an expression: {dir_node_id:?}")
            });
            let expr = view.get::<dir::Expression>(expr_id);

            // prefer the member left operand as the receiver
            if let dir::Expression::Member { left, .. } = expr {
                let receiver_local: dir::LocalNodeIdAny = (*left).into();
                let receiver_symbol = self.expression_symbol_target(*left);
                if receiver_symbol.is_none() {
                    if partial_context.is_none() {
                        partial_context =
                            Some(self.unresolved_member_access_completion_context(receiver_local));
                    }
                    continue;
                }

                return Some(
                    self.member_access_completion_context(receiver_local, receiver_symbol),
                );
            }

            // otherwise treat the expression itself as the receiver
            let receiver_symbol = self.expression_symbol_target(expr_id);
            if receiver_symbol.is_none() {
                if partial_context.is_none() {
                    partial_context =
                        Some(self.unresolved_member_access_completion_context(dir_node_id));
                }
                continue;
            }

            return Some(self.member_access_completion_context(dir_node_id, receiver_symbol));
        }

        partial_context
    }

    /// Resolve member access context from one dot owned cursor.
    fn member_access_context_after_dot(&self, offset: u32) -> Option<CompletionContext> {
        let dot = self.member_access_dot_before_offset(offset)?;
        let receiver_token = self.receiver_token_before_member_access_dot(dot)?;
        let receiver_offset = receiver_token.span.end.saturating_sub(1);

        self.member_access_context_at_offset(receiver_offset)
    }
}
