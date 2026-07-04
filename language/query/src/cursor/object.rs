use destack_dir as dir;

use super::ScopeAtOffset;
use crate::ModuleQueryContext;

/// Describes the cursor position inside one object literal.
#[derive(Debug, Clone)]
pub(crate) enum ObjectLiteralCursorContext {
    /// The cursor is in one object literal key position.
    Key(ObjectLiteralKeyContext),
    /// The cursor is in one object literal value position.
    Value(ScopeAtOffset),
}

/// Object literal key completion context.
#[derive(Debug, Clone)]
pub(crate) struct ObjectLiteralKeyContext {
    /// The object expression node id.
    pub object_node: dir::LocalNodeIdAny,
    /// The contextual type when one is available.
    pub contextual_type: Option<dir::GlobalTypeId>,
    /// Field names already present in the literal.
    pub existing_fields: Vec<String>,
    /// The visible scope for the object literal expression.
    pub scope: ScopeAtOffset,
}

impl ModuleQueryContext<'_> {
    /// Resolve object literal cursor information at one offset.
    pub(crate) fn object_literal_context_at_offset(
        &self,
        offset: u32,
    ) -> Option<ObjectLiteralCursorContext> {
        // resolve enclosing spans from innermost to outermost
        let enclosing = self.sorted_enclosing_spans(offset, offset);

        // bail out early when there are no enclosing spans
        if enclosing.is_empty() {
            return None;
        }

        // resolve DIR tables for object literal context
        let view = self.view();
        let parsed_tree = self.tree();

        // look for an object expression under the cursor
        for enc in &enclosing {
            if parsed_tree.get_node_type(enc.source_id) != dir::NodeType::Expression {
                continue;
            }

            let parsed_expr_id = dir::LocalNodeId::<dir::Expression>::new(enc.source_id);
            let parsed_expr = parsed_tree.get(parsed_expr_id);

            let dir::Expression::ObjectExpression { properties, .. } = parsed_expr else {
                continue;
            };

            let object_spans = ObjectLiteralSpans {
                tree: parsed_tree,
                properties,
            };
            let is_key_position = object_spans.owns_key_cursor(offset);

            let dir_node_id = view
                .get_node_id_by_source_id(enc.source_id)
                .unwrap_or_else(|| {
                    panic!(
                        "missing checked object expression for source {}",
                        enc.source_id
                    )
                });
            if dir_node_id.ty != dir::NodeType::Expression {
                panic!(
                    "checked object expression source mapped to {:?}",
                    dir_node_id.ty
                );
            }

            let dir_expr_id = dir_node_id
                .try_into()
                .unwrap_or_else(|_| panic!("checked object source is not an expression"));
            let dir_expr: &dir::Expression = view.get(dir_expr_id);
            let scope = self.expression_scope_at_offset(dir_expr_id, offset)?;

            // property values stay in value position inside the surrounding expression scope
            if !is_key_position {
                return Some(ObjectLiteralCursorContext::Value(scope));
            }

            let properties = match dir_expr {
                dir::Expression::ObjectExpression { properties, .. } => properties,
                _ => panic!("checked object source is not an object expression"),
            };

            let existing_fields = self.object_property_names(properties);
            let contextual_type = self.node_type_id(dir_node_id);

            return Some(ObjectLiteralCursorContext::Key(ObjectLiteralKeyContext {
                object_node: dir_node_id,
                contextual_type,
                existing_fields,
                scope,
            }));
        }

        None
    }

    /// Check if the cursor is inside an object literal expression.
    pub(crate) fn is_inside_object_literal_expression(&self, offset: u32) -> bool {
        // resolve enclosing spans from innermost to outermost
        let enclosing = self.sorted_enclosing_spans(offset, offset);

        // bail out early when there are no enclosing spans
        if enclosing.is_empty() {
            return false;
        }

        // scan enclosing expressions for object literal nodes
        for enc in &enclosing {
            if self.tree().get_node_type(enc.source_id) != dir::NodeType::Expression {
                continue;
            }

            let expr_id = dir::LocalNodeId::<dir::Expression>::new(enc.source_id);
            let expr = self.tree().get(expr_id);

            if matches!(expr, dir::Expression::ObjectExpression { .. }) {
                return true;
            }
        }

        false
    }

    /// Return object literal property names.
    fn object_property_names(&self, properties: &[dir::LocalNodeId<dir::Property>]) -> Vec<String> {
        let view = self.view();
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
                dir::Property::Error => panic!("error property reached object source query"),
            }
        }

        names
    }
}

/// Source spans owned by one object literal.
struct ObjectLiteralSpans<'a> {
    /// The parsed source tree.
    tree: &'a dir::Tree,
    /// The property node ids.
    properties: &'a [dir::LocalNodeId<dir::Property>],
}

impl ObjectLiteralSpans<'_> {
    /// Return whether the object literal key position owns the cursor.
    fn owns_key_cursor(&self, offset: u32) -> bool {
        for property_id in self.properties {
            if let Some(span) = self.tree.source_index.get_main(property_id.id) {
                if span.contains(offset) {
                    return true;
                }
            }
        }

        if self.owns_value_cursor(offset) {
            return false;
        }

        for property_id in self.properties {
            let span = self.tree.source_index.get(property_id.id);
            if span.contains(offset) {
                return false;
            }
        }

        true
    }

    /// Return whether a property value owns the cursor.
    fn owns_value_cursor(&self, offset: u32) -> bool {
        let cursor = offset.saturating_sub(1);

        // scan property value spans
        for property_id in self.properties {
            let property = self.tree.get(*property_id);

            match property {
                dir::Property::Field { value, .. } => {
                    let span = self.tree.source_index.get(value.id);
                    if span.contains(cursor) {
                        return true;
                    }
                }
                dir::Property::Method { body, .. } => {
                    if let Some(body_id) = body {
                        let span = self.tree.source_index.get(body_id.id);
                        if span.contains(cursor) {
                            return true;
                        }
                    }
                }
                dir::Property::Spread { value, .. } => {
                    let span = self.tree.source_index.get(value.id);
                    if span.contains(cursor) {
                        return true;
                    }
                }
                dir::Property::Error => panic!("error property reached object source query"),
            }
        }

        // use property spans outside keys
        for property_id in self.properties {
            let span = self.tree.source_index.get(property_id.id);
            if !span.contains(cursor) {
                continue;
            }

            let key_span = self
                .tree
                .source_index
                .get_main(property_id.id)
                .unwrap_or_else(|| {
                    panic!(
                        "missing main source span for object property {}",
                        property_id.id
                    )
                });
            if !key_span.contains(cursor) {
                return true;
            }
        }

        false
    }
}
