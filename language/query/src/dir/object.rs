use destack_core::StringPool;
use destack_dir as dir;

use super::ScopeAtOffset;
use crate::core::DirQueryContext;

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

impl DirQueryContext<'_> {
    /// Resolve object literal cursor information at one offset.
    pub(crate) fn object_literal_cursor_context(
        self,
        offset: u32,
    ) -> Option<ObjectLiteralCursorContext> {
        let ctx = self;

        // resolve enclosing spans from innermost to outermost
        let enclosing = ctx.sorted_enclosing_spans(offset, offset);

        // bail out early when there are no enclosing spans
        if enclosing.is_empty() {
            return None;
        }

        // resolve dir tree and types for object literal analysis
        let dir_tree = ctx.view();
        let types = ctx.types();
        let parsed_tree = ctx.tree();

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

            let is_key_position = is_object_literal_key_position(parsed_tree, properties, offset);

            let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enc.source_id) else {
                continue;
            };
            if dir_node_id.ty != dir::NodeType::Expression {
                continue;
            }

            let Ok(dir_expr_id) = dir_node_id.try_into() else {
                continue;
            };
            let dir_expr: &dir::Expression = dir_tree.get(dir_expr_id);
            let scope = ctx.expression_scope_at_offset(dir_expr_id, offset);

            // property values stay in value position inside the surrounding expression scope
            if !is_key_position {
                return Some(ObjectLiteralCursorContext::Value(scope));
            }

            let properties = match dir_expr {
                dir::Expression::ObjectExpression { properties, .. } => properties,
                _ => continue,
            };

            let existing_fields =
                extract_object_property_names(dir_tree, properties, ctx.strings());
            let contextual_type = ctx.contextual_object_type(types, dir_node_id);

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
    pub(crate) fn is_inside_object_literal_expression(self, offset: u32) -> bool {
        let ctx = self;

        // resolve enclosing spans from innermost to outermost
        let enclosing = ctx.sorted_enclosing_spans(offset, offset);

        // bail out early when there are no enclosing spans
        if enclosing.is_empty() {
            return false;
        }

        // scan enclosing expressions for object literal nodes
        for enc in &enclosing {
            if ctx.tree().get_node_type(enc.source_id) != dir::NodeType::Expression {
                continue;
            }

            let expr_id = dir::LocalNodeId::<dir::Expression>::new(enc.source_id);
            let expr = ctx.tree().get(expr_id);

            if matches!(expr, dir::Expression::ObjectExpression { .. }) {
                return true;
            }
        }

        false
    }
}

/// Check if the cursor is in a property key position of an object literal.
pub(crate) fn is_object_literal_key_position(
    parsed_tree: &dir::Tree,
    properties: &[dir::LocalNodeId<dir::Property>],
    offset: u32,
) -> bool {
    // check for any property key span hit
    for property_id in properties {
        if let Some(span) = parsed_tree.source_index.get_main(property_id.id)
            && span.contains(offset)
        {
            return true;
        }
    }

    // avoid property values
    if is_object_literal_value_position(parsed_tree, properties, offset) {
        return false;
    }

    // avoid property spans when not on keys
    for property_id in properties {
        let span = parsed_tree.source_index.get(property_id.id);
        if span.contains(offset) {
            return false;
        }
    }

    true
}

/// Extract property names from object literal properties.
fn extract_object_property_names(
    dir_tree: dir::View<'_>,
    properties: &[dir::LocalNodeId<dir::Property>],
    strings: &StringPool,
) -> Vec<String> {
    // collect property names
    let mut names = Vec::new();

    for &property_id in properties {
        let property = dir_tree.get::<dir::Property>(property_id);
        match property {
            dir::Property::Field { key, .. } => {
                if let dir::Key::Name(name) = *key {
                    names.push(strings.get(name.string()).to_string());
                }
            }
            dir::Property::Method { key, .. } => {
                if let Some(dir::Key::Name(name)) = key {
                    names.push(strings.get(name.string()).to_string());
                }
            }
            dir::Property::Spread { .. } | dir::Property::Error => {}
        }
    }

    names
}

/// Check if the cursor is inside a value span of an object literal.
fn is_object_literal_value_position(
    parsed_tree: &dir::Tree,
    properties: &[dir::LocalNodeId<dir::Property>],
    offset: u32,
) -> bool {
    // compute the cursor location inside the literal
    let cursor = offset.saturating_sub(1);

    // scan property value spans
    for property_id in properties {
        let property = parsed_tree.get(*property_id);

        match property {
            dir::Property::Field { value, .. } => {
                let span = parsed_tree.source_index.get(value.id);
                if span.contains(cursor) {
                    return true;
                }
            }
            dir::Property::Method { body, .. } => {
                if let Some(body_id) = body {
                    let span = parsed_tree.source_index.get(body_id.id);
                    if span.contains(cursor) {
                        return true;
                    }
                }
            }
            dir::Property::Spread { value, .. } => {
                let span = parsed_tree.source_index.get(value.id);
                if span.contains(cursor) {
                    return true;
                }
            }
            dir::Property::Error => {}
        }
    }

    // use property spans outside keys
    for property_id in properties {
        let span = parsed_tree.source_index.get(property_id.id);
        if !span.contains(cursor) {
            continue;
        }

        let key_span = parsed_tree.source_index.get_main(property_id.id);
        let is_in_key = key_span
            .map(|key_span| key_span.contains(cursor))
            .unwrap_or(false);

        if !is_in_key {
            return true;
        }
    }

    false
}

impl DirQueryContext<'_> {
    /// Resolve one recorded contextual object type.
    fn contextual_object_type(
        self,
        types: &dir::TypeTable<'_>,
        node_id: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalTypeId> {
        let ctx = self;
        let global_node_id = node_id.into_global(ctx.module_id());
        types.get_node_type_id(global_node_id)
    }
}
