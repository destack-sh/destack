use destack_dir as dir;
use destack_source::{FileId, ModuleId};

use crate::{ModuleQueryContext, QueryError, QueryResult};

use super::{CompletionContext, CompletionReceiver, CursorToken};

impl ModuleQueryContext<'_> {
    /// Classify member access completion near the cursor.
    pub(super) fn classify_member_access(
        &self,
        file_id: FileId,
        token: &Option<CursorToken>,
        offset: u32,
    ) -> QueryResult<Option<CompletionContext>> {
        // detect member access inside an existing member name token
        if let Some(cursor_position) = offset.checked_sub(1)
            && let Some(context) = self.classify_member_access_name(file_id, cursor_position)?
        {
            return Ok(Some(context));
        }

        // resolve member access context when immediately after one dot boundary
        if let Some(context) = self.classify_member_access_dot(file_id, offset)? {
            return Ok(Some(context));
        }

        // resolve member access when the cursor is inside a member name
        if let Some(token_at_cursor) = token.as_ref()
            && let Some(context) =
                self.classify_member_access_dot(file_id, token_at_cursor.start)?
        {
            return Ok(Some(context));
        }

        Ok(None)
    }

    /// Classify member access from an existing member name.
    fn classify_member_access_name(
        &self,
        file_id: FileId,
        cursor_position: u32,
    ) -> QueryResult<Option<CompletionContext>> {
        let Some(token_at_cursor) = self.token_span_at_offset(file_id, cursor_position)? else {
            return Ok(None);
        };
        if token_at_cursor.token.ty() != dir::TokenType::Identifier {
            return Ok(None);
        }

        let view = self.view();

        // resolve enclosing spans from innermost to outermost
        let enclosing = self.sorted_enclosing_spans(file_id, cursor_position, cursor_position);

        // scan enclosing spans for one member expression at the cursor
        for enclosing_span in &enclosing {
            let main_span = self.source_index().get_main(enclosing_span.source_id);
            let Some(main_span) = main_span else {
                continue;
            };
            let is_in_member_name = main_span.contains(cursor_position);
            if !is_in_member_name {
                continue;
            }

            let Some(dir_node_id) = view.get_node_id_by_source_id(enclosing_span.source_id) else {
                continue;
            };
            if dir_node_id.ty != dir::NodeType::Expression {
                continue;
            }

            let expression_id = dir::LocalNodeId::<dir::Expression>::new(dir_node_id.id);
            let expression = view.get::<dir::Expression>(expression_id);

            // use the left operand when inside a member expression
            let dir::Expression::Member {
                left, is_optional, ..
            } = expression
            else {
                continue;
            };

            let receiver = CompletionReceiver::resolve((*left).into(), *is_optional, self)?;
            let context = CompletionContext::MemberAccess { receiver };

            return Ok(Some(context));
        }

        Ok(None)
    }

    /// Classify member access from a receiver position.
    fn classify_member_access_receiver(
        &self,
        file_id: FileId,
        receiver_position: u32,
    ) -> QueryResult<Option<CompletionContext>> {
        let enclosing = self.sorted_enclosing_spans(file_id, receiver_position, receiver_position);
        let view = self.view();

        // scan for the nearest enclosing expression
        for enclosing_span in &enclosing {
            let Some(node_id) = view.get_node_id_by_source_id(enclosing_span.source_id) else {
                continue;
            };
            if node_id.ty != dir::NodeType::Expression {
                continue;
            }

            let expression_id = dir::LocalNodeId::<dir::Expression>::new(node_id.id);
            let expression = view.get::<dir::Expression>(expression_id);

            // prefer the member left operand as the receiver
            let (receiver, is_optional) = match expression {
                dir::Expression::Member {
                    left, is_optional, ..
                } => ((*left).into(), *is_optional),
                _ => (node_id, false),
            };
            let receiver = CompletionReceiver::resolve(receiver, is_optional, self)?;

            return Ok(Some(CompletionContext::MemberAccess { receiver }));
        }

        Ok(None)
    }

    /// Classify member access after one dot.
    fn classify_member_access_dot(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<CompletionContext>> {
        let Some(dot) = self.member_access_dot_before_offset(file_id, offset)? else {
            return Ok(None);
        };
        let is_optional = self
            .previous_significant_token(file_id, dot.span.start)?
            .is_some_and(|token| token.token.ty() == dir::TokenType::Maybe);
        let Some(receiver_token) = self.receiver_token_before_member_access_dot(dot)? else {
            return Ok(None);
        };
        let receiver_offset = receiver_token
            .span
            .end
            .checked_sub(1)
            .ok_or(QueryError::missing(format!(
                "completion receiver: {:?}",
                receiver_token.span
            )))?;

        let context = self.classify_member_access_receiver(file_id, receiver_offset)?;
        let context = context.map(|context| context.with_optional_access(is_optional));

        Ok(context)
    }
}

impl CompletionContext {
    /// Return this context with optional access applied to a typed receiver.
    fn with_optional_access(self, is_optional: bool) -> Self {
        match self {
            Self::MemberAccess {
                receiver: CompletionReceiver::Type { type_id, .. },
            } => Self::MemberAccess {
                receiver: CompletionReceiver::Type {
                    type_id,
                    is_optional,
                },
            },
            context => context,
        }
    }
}

impl CompletionReceiver {
    /// Resolve one member receiver.
    fn resolve(
        node_id: dir::LocalNodeIdAny,
        is_optional: bool,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Self> {
        let global_id = node_id.into_global(module.module_id());
        let receiver = if let Some(module_id) = Self::namespace(global_id, module)? {
            Self::Namespace { module_id }
        } else if let Some(type_id) = module.types().get_node_type_id(global_id) {
            Self::Type {
                type_id,
                is_optional,
            }
        } else {
            return Err(QueryError::missing(format!(
                "completion receiver: {global_id:?}"
            )));
        };

        Ok(receiver)
    }

    /// Return the imported namespace selected by one receiver expression.
    fn namespace(
        receiver: dir::GlobalNodeIdAny,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<ModuleId>> {
        if let Some(dir::Reference::Namespace(module_id)) =
            module.resolved().references.get(receiver)
        {
            return Ok(Some(*module_id));
        }

        let Some(symbols) = module.recorded_symbol_targets(receiver)? else {
            return Ok(None);
        };
        let mut modules = Vec::new();

        // follow exact namespace references on local dependency bindings
        for symbol_id in symbols {
            if symbol_id.module_id != module.module_id() {
                continue;
            }

            let symbol = module.symbols().get_symbol(symbol_id.local_id);
            let Some(declaration) = symbol.declaration else {
                continue;
            };
            if declaration.local_id.ty != dir::NodeType::DependencyItem {
                continue;
            }

            let reference =
                module
                    .resolved()
                    .references
                    .get(declaration)
                    .ok_or(QueryError::missing(format!(
                        "canonical reference: {declaration:?}"
                    )))?;
            if let dir::Reference::Namespace(module_id) = reference {
                modules.push(*module_id);
            }
        }

        modules.sort();
        modules.dedup();
        match modules.as_slice() {
            [] => Ok(None),
            [module_id] => Ok(Some(*module_id)),
            _ => Err(QueryError::invalid(format!(
                "completion namespace: {receiver:?}"
            ))),
        }
    }
}
