use destack_dir as dir;
use destack_source::FileId;
use rustc_hash::FxHashSet;

use super::CompletionContext;
use super::builder::CompletionBuilder;
use crate::{
    CompletionCandidate, CompletionItemKind, CompletionOrigin, ModuleQueryContext, QueryError,
    QueryResult, SORT_CONTEXTUAL, SORT_LOCAL_SYMBOL,
};

/// Source spans owned by one object literal.
struct ObjectLiteralSpans<'a> {
    /// The queried module.
    module: &'a ModuleQueryContext<'a>,
    /// The visible DIR.
    view: dir::View<'a>,
    /// The property node ids.
    properties: &'a [dir::LocalNodeId<dir::Property>],
}

impl ModuleQueryContext<'_> {
    /// Classify object literal completion at one offset.
    pub(super) fn classify_object_literal(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<CompletionContext>> {
        // resolve enclosing spans from innermost to outermost
        let enclosing = self.sorted_enclosing_spans(file_id, offset, offset)?;

        // bail out early when there are no enclosing spans
        if enclosing.is_empty() {
            return Ok(None);
        }

        // resolve visible DIR for object literal context
        let view = self.view()?;

        let mut literal = None;

        // select the innermost object expression by its complete source span
        for enclosing_span in &enclosing {
            let Some(node_id) = view.get_node_id_by_source_id(enclosing_span.source_id) else {
                continue;
            };
            if node_id.ty != dir::NodeType::Expression {
                continue;
            }
            let expression_id = dir::LocalNodeId::<dir::Expression>::new(node_id.id);
            let dir::Expression::ObjectExpression { properties, .. } = view.get(expression_id)
            else {
                continue;
            };
            let span = self.node_span(view, expression_id.into())?;
            if !span.owns_cursor(offset) {
                continue;
            }
            if literal.is_none_or(|(length, _, _)| span.len() < length) {
                literal = Some((span.len(), expression_id, properties.as_slice()));
            }
        }

        let Some((_, expression_id, properties)) = literal else {
            return Ok(None);
        };
        let Some(scope) = self.expression_scope_at_offset(expression_id)? else {
            return Ok(None);
        };
        let object_spans = ObjectLiteralSpans {
            module: self,
            view,
            properties,
        };

        // property values stay in the surrounding expression scope
        if !object_spans.owns_key_cursor(offset)? {
            return Ok(Some(CompletionContext::ObjectLiteralValue { scope }));
        }

        Ok(Some(CompletionContext::ObjectLiteralKey {
            literal: expression_id,
            scope,
        }))
    }
}

impl ObjectLiteralSpans<'_> {
    /// Return whether the object literal key position owns the cursor.
    fn owns_key_cursor(&self, offset: u32) -> QueryResult<bool> {
        for property_id in self.properties {
            if let Some(span) = self
                .module
                .node_selection_span(self.view, (*property_id).into())?
                && span.owns_cursor(offset)
            {
                return Ok(true);
            }
        }

        if self.owns_value_cursor(offset)? {
            return Ok(false);
        }

        for property_id in self.properties {
            let span = self.module.node_span(self.view, (*property_id).into())?;
            if span.contains(offset) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return whether a property value owns the cursor.
    fn owns_value_cursor(&self, offset: u32) -> QueryResult<bool> {
        let Some(cursor) = offset.checked_sub(1) else {
            return Ok(false);
        };

        // scan property value spans
        for property_id in self.properties {
            let property = self.view.get(*property_id);

            match property {
                dir::Property::Field { value, .. } => {
                    let span = self.module.node_span(self.view, (*value).into())?;
                    if span.contains(cursor) {
                        return Ok(true);
                    }
                }
                dir::Property::Method { body, .. } => {
                    if let Some(body_id) = body {
                        let span = self.module.node_span(self.view, (*body_id).into())?;
                        if span.contains(cursor) {
                            return Ok(true);
                        }
                    }
                }
                dir::Property::Spread { value, .. } => {
                    let span = self.module.node_span(self.view, (*value).into())?;
                    if span.contains(cursor) {
                        return Ok(true);
                    }
                }
                dir::Property::Error => {}
            }
        }

        // use property spans outside keys
        for property_id in self.properties {
            let span = self.module.node_span(self.view, (*property_id).into())?;
            if !span.contains(cursor) {
                continue;
            }

            let node = property_id.into_global_any(self.module.module_id());
            let key_span = self
                .module
                .node_selection_span(self.view, (*property_id).into())?
                .ok_or(QueryError::missing(format!(
                    "object property span: {node:?}"
                )))?;
            if !key_span.contains(cursor) {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

impl CompletionBuilder<'_, '_, '_> {
    /// Complete the keys of one object literal.
    pub(super) fn complete_object_literal(
        &self,
        literal: dir::LocalNodeId<dir::Expression>,
        scope: dir::LocalScope,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let node = literal.into_global_any(self.module.module_id());

        // remove fields already supplied by this literal
        let supplied = self.object_keys(literal)?;
        let mut shorthands = self.visible_value_bindings(scope)?;
        for key in &supplied {
            shorthands.shift_remove(key);
        }

        // read contextual fields
        let members = self.module.members()?;
        let site = dir::MemberSite::Node(node);
        let expected = match members.subject(site) {
            Some(subject) => Some(members.members(site).ok_or(QueryError::missing(format!(
                "object literal bindings: source={node:?}, subject={subject:?}"
            )))?),
            None if self.module.types()?.get_expected_type_id(node).is_some() => {
                return Err(QueryError::missing(format!(
                    "object literal member subject: {node:?}"
                )));
            }
            None => None,
        };

        // overlay missing contextual fields onto visible shorthands
        let mut results = Vec::new();
        if let Some(expected) = expected {
            for member in expected {
                // require construction fields
                if member.kind != dir::MemberKind::Field {
                    return Err(QueryError::invalid(format!(
                        "object literal member is not a field: {node:?}, {:?}",
                        member.key
                    )));
                }

                // omit keys already supplied by the literal
                if supplied.contains(&member.key) {
                    continue;
                }

                // omit keys without identifier text
                let dir::StaticKey::Name(name) = member.key else {
                    continue;
                };

                // describe the selected field
                let label = self.module.strings().get(name).to_string();
                let completion = CompletionCandidate::new(
                    &label,
                    CompletionItemKind::Field,
                    CompletionOrigin::Contextual,
                    SORT_CONTEXTUAL,
                )
                .with_type_id(member.access.store());
                let completion = match member.declarations.first() {
                    Some(declaration) => {
                        self.resolve_declaration(completion, declaration.symbol)?
                    }
                    None => completion,
                };

                // prefer shorthand insertion when the value is visible
                if shorthands.shift_remove(&member.key).is_some() {
                    results.push(completion);
                } else {
                    results.push(completion.with_snippet(format!("{label}: ${{1}}")));
                }
            }
        }

        // build the remaining shorthand fields
        for (key, symbol) in shorthands {
            let dir::StaticKey::Name(name) = key else {
                continue;
            };
            let label = self.module.strings().get(name).to_string();
            let completion = CompletionCandidate::new(
                label,
                CompletionItemKind::Field,
                CompletionOrigin::Local,
                SORT_LOCAL_SYMBOL,
            );
            results.push(self.resolve_symbol(completion, symbol)?);
        }

        Ok(results)
    }

    /// Return the keys already supplied by one object literal.
    fn object_keys(
        &self,
        literal: dir::LocalNodeId<dir::Expression>,
    ) -> QueryResult<FxHashSet<dir::StaticKey>> {
        let node = literal.into_global_any(self.module.module_id());
        let view = self.module.view()?;
        let dir::Expression::ObjectExpression { properties, .. } = view.get(literal) else {
            return Err(QueryError::invalid(format!("object literal: {node:?}")));
        };
        let mut keys = FxHashSet::default();

        // collect direct and spread property keys
        for property in properties.iter() {
            match view.get(*property) {
                dir::Property::Field { key, .. } => {
                    if let Some(key) = key.direct_static_key() {
                        keys.insert(key);
                    }
                }
                dir::Property::Spread { .. } => {
                    keys.extend(self.spread_field_keys(*property)?);
                }
                dir::Property::Method { key, .. } => {
                    if let Some(key) = key.as_ref().and_then(|key| key.direct_static_key()) {
                        keys.insert(key);
                    }
                }
                dir::Property::Error => {}
            }
        }

        Ok(keys)
    }

    /// Return the field keys supplied by one object spread.
    fn spread_field_keys(
        &self,
        property: dir::LocalNodeId<dir::Property>,
    ) -> QueryResult<Vec<dir::StaticKey>> {
        let node = property.into_global_any(self.module.module_id());
        let members = self
            .module
            .members()?
            .members(dir::MemberSite::Node(node))
            .ok_or(QueryError::missing(format!(
                "object spread members: {node:?}"
            )))?;

        let mut keys = Vec::with_capacity(members.len());
        for member in members {
            if member.kind != dir::MemberKind::Field {
                return Err(QueryError::invalid(format!(
                    "object spread member is not a field: {node:?}, {:?}",
                    member.key
                )));
            }
            keys.push(member.key);
        }

        Ok(keys)
    }
}
