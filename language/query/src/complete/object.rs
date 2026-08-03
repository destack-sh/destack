use destack_dir as dir;
use destack_source::FileId;

use rustc_hash::FxHashSet;

use super::CompletionContext;
use super::builder::CompletionBuilder;
use crate::{
    CompletionCandidate, CompletionItemKind, CompletionOrigin, ModuleQueryContext, QueryError,
    QueryResult, SORT_BUILTIN, SymbolUse,
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

        // look for an object expression under the cursor
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

            let existing_fields = self.object_property_names(properties)?;

            return Ok(Some(CompletionContext::ObjectLiteralKey {
                literal: expression_id,
                existing_fields,
                scope,
            }));
        }

        Ok(None)
    }

    /// Return object literal property names.
    fn object_property_names(
        &self,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> QueryResult<Vec<String>> {
        let view = self.view()?;
        let mut names = Vec::new();

        // collect static property names from declared fields and methods
        for &property_id in properties {
            let property = view.get::<dir::Property>(property_id);
            match property {
                dir::Property::Field { key, .. } => {
                    if let dir::Key::Name(name) = *key {
                        names.push(self.strings().get(name.string()).to_string());
                    }
                }
                dir::Property::Method { key, .. } => {
                    if let Some(dir::Key::Name(name)) = key {
                        names.push(self.strings().get(name.string()).to_string());
                    }
                }
                dir::Property::Spread { .. } => {}
                dir::Property::Error => {}
            }
        }

        Ok(names)
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
    /// Complete the expected object type's missing fields at one literal.
    pub(super) fn complete_expected_fields(
        &self,
        literal: dir::LocalNodeId<dir::Expression>,
        existing_fields: &[String],
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let mut results = Vec::new();

        // read the contextual expectation retained by check
        let node = literal.into_global_any(self.module.module_id());
        let types = self.module.types()?;
        let Some(expected) = types
            .get_expected_type_id(node)
            .or_else(|| types.get_node_type_id(node))
        else {
            return Ok(results);
        };

        // offer each missing field declared by the expected type
        for (name, symbol) in self.program.type_field_names(expected)? {
            if existing_fields.contains(&name) {
                continue;
            }
            let completion = CompletionCandidate::new(
                name,
                CompletionItemKind::Field,
                CompletionOrigin::Local,
                SORT_BUILTIN,
            );
            let completion = match symbol {
                Some(symbol) => self.attach_symbol_completion(completion, symbol)?,
                None => completion,
            };
            results.push(completion);
        }

        Ok(results)
    }

    /// Complete visible shorthand values inside an object literal.
    pub(super) fn complete_object_literal_shorthands(
        &self,
        existing_fields: &[String],
        scope: dir::LocalScope,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let mut results = Vec::new();
        let mut seen_names = FxHashSet::default();

        // collect visible values that can form shorthand fields
        let symbols = self.module.bindings()?;
        for visible in symbols
            .visible_bindings(scope)
            .filter(|binding| SymbolUse::Value.accepts_symbol_kind(binding.symbol.kind))
        {
            let dir::StaticKey::Name(name_id) = visible.key else {
                continue;
            };

            let name = self.module.strings().get(name_id).to_string();
            if !seen_names.insert(name.clone()) {
                continue;
            }

            if existing_fields.contains(&name) {
                continue;
            }

            let symbol_id = dir::GlobalSymbolId {
                module_id: self.module.module_id(),
                local_id: visible.symbol_id,
            };
            let completion = CompletionCandidate::new(
                name,
                CompletionItemKind::Field,
                CompletionOrigin::Local,
                SORT_BUILTIN,
            );
            results.push(self.attach_symbol_completion(completion, symbol_id)?);
        }

        Ok(results)
    }
}
