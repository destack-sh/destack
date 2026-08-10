use destack_dir as dir;
use destack_source::{FileId, ModuleId, Span};

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

        // resolve member access context immediately after a dot
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

        let view = self.view()?;

        // resolve enclosing spans from innermost to outermost
        let enclosing = self.sorted_enclosing_spans(file_id, cursor_position, cursor_position)?;

        // scan enclosing spans for one member expression at the cursor
        for enclosing_span in &enclosing {
            let main_span = self.source_index()?.get_main(enclosing_span.source_id);
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
            let Some(receiver) =
                CompletionReceiver::resolve_member(dir_node_id, Some(main_span), self)?
            else {
                continue;
            };
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
        let enclosing =
            self.sorted_enclosing_spans(file_id, receiver_position, receiver_position)?;
        let view = self.view()?;

        // scan for the nearest enclosing expression
        for enclosing_span in &enclosing {
            let Some(node_id) = view.get_node_id_by_source_id(enclosing_span.source_id) else {
                continue;
            };
            let Some(receiver) = CompletionReceiver::resolve_member(node_id, None, self)? else {
                continue;
            };

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

        self.classify_member_access_receiver(file_id, receiver_offset)
    }
}

impl CompletionReceiver {
    /// Resolve one value or type member projection.
    fn resolve_member(
        source: dir::LocalNodeIdAny,
        selected_span: Option<Span>,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<Self>> {
        let view = module.view()?;
        let global_source = source.into_global(module.module_id());
        let receiver = match source.ty {
            dir::NodeType::Expression => {
                let source = dir::LocalNodeId::<dir::Expression>::new(source.id);
                let dir::Expression::Member { left, .. } = view.get(source) else {
                    return Ok(None);
                };

                left.into_any()
            }
            dir::NodeType::TypeExpression => {
                let source = dir::LocalNodeId::<dir::TypeExpression>::new(source.id);
                match view.get(source) {
                    dir::TypeExpression::Member { left, .. } => left.into_any(),
                    dir::TypeExpression::Reference { .. } => {
                        return match module.resolved()?.references.get(global_source) {
                            Some(dir::Reference::Namespace {
                                module: module_id, ..
                            }) => Ok(Some(Self::Namespace {
                                module_id: *module_id,
                            })),
                            Some(dir::Reference::Projected {
                                base: dir::ReferenceTarget::Namespace(module_id),
                                ..
                            }) => Ok(Some(Self::Namespace {
                                module_id: *module_id,
                            })),
                            Some(dir::Reference::Projected {
                                base: dir::ReferenceTarget::Symbol(_),
                                ..
                            }) => {
                                let Some(selected_span) = selected_span else {
                                    return Ok(None);
                                };
                                let Some((segment, _)) = module.qualified_type_segment(
                                    view,
                                    source.into(),
                                    selected_span,
                                )?
                                else {
                                    return Ok(None);
                                };
                                let segment = u16::try_from(segment).map_err(|_| {
                                    QueryError::invalid(format!(
                                        "completion member path: {global_source:?}"
                                    ))
                                })?;
                                let site = dir::MemberSite::Path {
                                    node: global_source,
                                    segment,
                                };

                                Ok(Some(Self::Access { site }))
                            }
                            _ => Ok(None),
                        };
                    }
                    _ => return Ok(None),
                }
            }
            _ => return Ok(None),
        };
        let receiver = receiver.into_global(module.module_id());
        let receiver = if let Some(module_id) = Self::namespace(receiver, module)? {
            Self::Namespace { module_id }
        } else {
            let site = dir::MemberSite::Node(global_source);

            Self::Access { site }
        };

        Ok(Some(receiver))
    }

    /// Return the imported namespace selected by one receiver expression.
    fn namespace(
        receiver: dir::GlobalNodeIdAny,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<ModuleId>> {
        let module_id = match module.resolved()?.references.get(receiver) {
            Some(dir::Reference::Namespace {
                module: module_id, ..
            }) => Some(*module_id),
            _ => None,
        };

        Ok(module_id)
    }
}
