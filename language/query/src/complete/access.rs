use tspp_dir as dir;
use tspp_source::{FileId, Span};

use crate::cursor::Cursor;
use crate::{DeclarationUse, ModuleQueryContext, QueryError, QueryResult};

use super::{CompletionPosition, CompletionPrefix, CompletionReceiver};

impl Cursor<'_, '_> {
    /// Classify member access completion near the cursor.
    pub(super) fn classify_member_access(
        &self,
        prefix: Option<&CompletionPrefix>,
    ) -> QueryResult<Option<CompletionPosition>> {
        // select the member name or the insertion point following its receiver
        let mut position = if let Some(offset) = self.offset.checked_sub(1)
            && let Some(position) = self
                .module
                .classify_member_access_name(self.file_id, offset)?
        {
            Some(position)
        } else if let Some(position) = self
            .module
            .classify_member_access_dot(self.file_id, self.offset)?
        {
            Some(position)
        } else if let Some(prefix) = prefix {
            self.module
                .classify_member_access_dot(self.file_id, prefix.start)?
        } else {
            None
        };

        // select constructor insertion within a qualified construction target
        if let Some(CompletionPosition::MemberAccess {
            receiver: CompletionReceiver::Namespace { usage, .. },
        }) = &mut position
            && self.classify_constructor()?.is_some()
        {
            *usage = DeclarationUse::Constructor;
        }

        Ok(position)
    }
}

impl ModuleQueryContext<'_> {
    /// Classify member access from an existing member name.
    fn classify_member_access_name(
        &self,
        file_id: FileId,
        cursor_position: u32,
    ) -> QueryResult<Option<CompletionPosition>> {
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
            let Some(node) = view.get_node_id_by_source_id(enclosing_span.source_id) else {
                continue;
            };
            let Some(main_span) = self.source_index()?.get_main(enclosing_span.source_id) else {
                continue;
            };
            let Some(span) = self.name_span(view, node, main_span, cursor_position)? else {
                continue;
            };
            let Some(receiver) = CompletionReceiver::resolve_member(node, Some(span), self)? else {
                continue;
            };
            let context = CompletionPosition::MemberAccess { receiver };

            return Ok(Some(context));
        }

        Ok(None)
    }

    /// Classify member access from a receiver position.
    fn classify_member_access_receiver(
        &self,
        file_id: FileId,
        receiver_position: u32,
    ) -> QueryResult<Option<CompletionPosition>> {
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

            return Ok(Some(CompletionPosition::MemberAccess { receiver }));
        }

        Ok(None)
    }

    /// Classify member access after one dot.
    fn classify_member_access_dot(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<CompletionPosition>> {
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
                        return Self::resolve_path(global_source, selected_span, module);
                    }
                    _ => return Ok(None),
                }
            }
            _ => return Ok(None),
        };

        // retain the declaration use recorded by the source node
        let usage = if source.ty == dir::NodeType::TypeExpression {
            DeclarationUse::Type
        } else {
            DeclarationUse::Expression
        };
        let receiver = receiver.into_global(module.module_id());
        let namespace = module
            .resolved()?
            .references
            .get(receiver)
            .and_then(dir::Reference::namespace);
        let receiver = if let Some(module_id) = namespace {
            Self::Namespace { module_id, usage }
        } else {
            let site = dir::MemberSite::Node(global_source);

            Self::Access { site }
        };

        Ok(Some(receiver))
    }

    /// Return the namespace or checked subject preceding one type path segment.
    fn resolve_path(
        source: dir::GlobalNodeIdAny,
        selected_span: Option<Span>,
        module: &ModuleQueryContext<'_>,
    ) -> QueryResult<Option<Self>> {
        let view = module.view()?;
        let references = &module.resolved()?.references;

        // a bare namespace can precede a newly typed dot
        let Some(selected_span) = selected_span else {
            return Ok(references
                .get(source)
                .and_then(dir::Reference::namespace)
                .map(|module_id| Self::Namespace {
                    module_id,
                    usage: DeclarationUse::Type,
                }));
        };
        let Some((segment, _)) =
            module.qualified_type_segment(view, source.local_id, selected_span)?
        else {
            return Ok(None);
        };
        let segment = u16::try_from(segment)
            .map_err(|_| QueryError::invalid(format!("completion member path: {source:?}")))?;
        let Some(previous) = segment.checked_sub(1) else {
            return Ok(None);
        };
        let site = dir::ReferenceSite::Path {
            node: source,
            segment: previous,
        };

        match (references.get(site), references.get(source)) {
            // read the exact preceding namespace retained by resolve
            (
                Some(dir::Reference::Namespace {
                    module: module_id, ..
                }),
                _,
            ) => Ok(Some(Self::Namespace {
                module_id: *module_id,
                usage: DeclarationUse::Type,
            })),
            // read a nominal projection retained by the checker
            (
                _,
                Some(dir::Reference::Projected {
                    base: dir::ReferenceTarget::Symbol(_),
                    from,
                }),
            ) if u32::from(segment) >= *from => {
                let site = dir::MemberSite::Path {
                    node: source,
                    segment,
                };

                Ok(Some(Self::Access { site }))
            }
            _ => Ok(None),
        }
    }
}
