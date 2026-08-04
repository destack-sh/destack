use destack_dir as dir;
use destack_source::FileId;

use super::CompletionContext;
use super::builder::CompletionBuilder;
use crate::{
    CompletionCandidate, CompletionItemKind, CompletionOrigin, ModuleQueryContext, QueryError,
    QueryResult, SORT_BUILTIN,
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

        // select the innermost object expression by its complete authored span
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
    /// Complete the expected object type's missing fields at one literal.
    pub(super) fn complete_expected_fields(
        &self,
        literal: dir::LocalNodeId<dir::Expression>,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let node = literal.into_global_any(self.module.module_id());

        // read the contextual type expected at the literal
        let types = self.module.types()?;
        let Some(expected) = types
            .get_expected_type_id(node)
            .or_else(|| types.get_node_type_id(node))
        else {
            return Ok(Vec::new());
        };
        let expected = types.get_reduced_type_id(expected);

        // collect the keys the literal already authored
        let view = self.module.view()?;
        let dir::Expression::ObjectExpression { properties, .. } = view.get(literal) else {
            return Err(QueryError::invalid(format!("object literal: {node:?}")));
        };
        let mut authored = Vec::new();
        for property in properties.iter() {
            if let dir::Property::Field { key, .. } = view.get(*property)
                && let Some(key) = key.direct_static_key()
            {
                authored.push(key);
            }
        }

        // offer each expected field the literal has not authored
        let mut results = Vec::new();
        for (key, ty) in self.expected_field_entries(expected)? {
            if authored.contains(&key) {
                continue;
            }
            let dir::StaticKey::Name(name) = key else {
                continue;
            };
            let label = self.module.strings().get(name).to_string();
            let mut completion = CompletionCandidate::new(
                label,
                CompletionItemKind::Field,
                CompletionOrigin::Member,
                SORT_BUILTIN,
            );
            if let Some(ty) = ty {
                completion = completion.with_type_id(ty);
            }
            results.push(completion);
        }

        Ok(results)
    }

    /// Return the field keys and types declared by one expected type.
    fn expected_field_entries(
        &self,
        expected: dir::GlobalTypeId,
    ) -> QueryResult<Vec<(dir::StaticKey, Option<dir::GlobalTypeId>)>> {
        // structural expectations list their properties directly
        let source = self.program.read_type(expected, |ty, owner| match ty {
            dir::Type::Shape(shape) | dir::Type::Object(shape) => {
                let entries = owner
                    .types()?
                    .properties(shape.properties)
                    .iter()
                    .map(|field| (field.key, field.access.read()))
                    .collect();

                Ok(ExpectedFields::Structural(entries))
            }
            dir::Type::Application(instance) => Ok(ExpectedFields::Nominal(instance.symbol)),
            dir::Type::Reference(reference) => Ok(ExpectedFields::Nominal(reference.symbol)),
            _ => Ok(ExpectedFields::Structural(Vec::new())),
        })?;
        let symbol = match source {
            ExpectedFields::Nominal(symbol) => symbol,
            ExpectedFields::Structural(entries) => return Ok(entries),
        };

        // nominal expectations list their declared instance fields
        let Some(symbol) = self.program.canonical_symbol(symbol)? else {
            return Ok(Vec::new());
        };
        let declaration = self.program.module(symbol.module_id)?;
        let Some(definition) = declaration.definitions()?.definition(symbol) else {
            return Ok(Vec::new());
        };
        let mut entries = Vec::new();
        for member in definition.members() {
            let dir::DefinitionMember::Field(field) = member else {
                continue;
            };
            if field.space != dir::MemberSpace::Instance {
                continue;
            }
            let ty = declaration.types()?.get_symbol_type_id(field.symbol);
            entries.push((field.key, ty));
        }

        Ok(entries)
    }
}

/// Field source resolved from one expected literal type.
enum ExpectedFields {
    /// Structural fields listed by the type itself.
    Structural(Vec<(dir::StaticKey, Option<dir::GlobalTypeId>)>),
    /// A nominal declaration listing its instance fields.
    Nominal(dir::GlobalSymbolId),
}
