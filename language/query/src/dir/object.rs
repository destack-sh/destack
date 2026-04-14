use destack_core::StringPool;
use destack_workspace::Repository;
use {destack_ast as ast, destack_dir as dir};

use super::{ScopeAtOffset, expression_scope_at_offset};
use crate::ast::sorted_enclosing_spans;
use crate::core::{AstQuery, DirQuery};

/// Describes the cursor position inside one object literal.
#[derive(Debug, Clone)]
pub(crate) enum ObjectLiteralCursorContext {
    /// The cursor is in one object literal key position.
    Key(ObjectLiteralContextInfo),
    /// The cursor is in one object literal value position.
    Value(ScopeAtOffset),
}

/// Contextual information for one object literal completion site.
#[derive(Debug, Clone)]
pub(crate) struct ObjectLiteralContextInfo {
    /// The object expression node id.
    pub object_node: dir::LocalNodeIdAny,
    /// The contextual type when one is available.
    pub contextual_type: Option<dir::LocalTypeId>,
    /// Field names already present in the literal.
    pub existing_fields: Vec<String>,
    /// The visible scope for the object literal expression.
    pub scope: ScopeAtOffset,
}

/// Resolve object literal cursor information at one offset.
pub(crate) fn object_literal_cursor_context(
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    offset: u32,
    repository: &Repository,
) -> Option<ObjectLiteralCursorContext> {
    // resolve enclosing spans from innermost to outermost
    let enclosing = sorted_enclosing_spans(ast, offset, offset);

    // bail out early when there are no enclosing spans
    if enclosing.is_empty() {
        return None;
    }

    // resolve dir tree and types for object literal analysis
    let dir_tree = dir.tree();
    let types = dir.types();
    let ast_tree = ast.tree();

    // look for an object expression under the cursor
    for enc in &enclosing {
        if ast_tree.get_node_type(enc.idx) != ast::NodeType::Expression {
            continue;
        }

        let ast_expr_id = ast::LocalNodeId::<ast::Expression>::new(enc.idx);
        let ast_expr = ast_tree.get(ast_expr_id);

        let ast::Expression::ObjectExpression { properties, .. } = ast_expr else {
            continue;
        };

        let is_key_position = is_object_literal_key_position(ast_tree, properties, offset);

        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enc.idx) else {
            continue;
        };
        if dir_node_id.ty != dir::NodeType::Expression {
            continue;
        }

        let Ok(dir_expr_id) = dir_node_id.try_into() else {
            continue;
        };
        let dir_expr: &dir::Expression = dir_tree.get(dir_expr_id);
        let scope = expression_scope_at_offset(ast, dir, dir_expr_id, offset);

        // property values stay in value position inside the surrounding expression scope
        if !is_key_position {
            return Some(ObjectLiteralCursorContext::Value(scope));
        }

        let properties = match dir_expr {
            dir::Expression::ObjectExpression { properties, .. }
            | dir::Expression::TaggedObjectExpression { properties, .. } => properties,
            _ => continue,
        };

        let existing_fields =
            extract_object_property_names(dir_tree, properties, &repository.strings);
        let contextual_type = contextual_object_type(dir, types, dir_node_id);

        return Some(ObjectLiteralCursorContext::Key(ObjectLiteralContextInfo {
            object_node: dir_node_id,
            contextual_type,
            existing_fields,
            scope,
        }));
    }

    None
}

/// Check if the cursor is inside an object literal expression.
pub(crate) fn is_inside_object_literal_expression(ast: AstQuery<'_>, offset: u32) -> bool {
    // resolve enclosing spans from innermost to outermost
    let enclosing = sorted_enclosing_spans(ast, offset, offset);

    // bail out early when there are no enclosing spans
    if enclosing.is_empty() {
        return false;
    }

    // scan enclosing expressions for object literal nodes
    for enc in &enclosing {
        if ast.tree().get_node_type(enc.idx) != ast::NodeType::Expression {
            continue;
        }

        let expr_id = ast::LocalNodeId::<ast::Expression>::new(enc.idx);
        let expr = ast.tree().get(expr_id);

        if matches!(expr, ast::Expression::ObjectExpression { .. }) {
            return true;
        }
    }

    false
}

/// Check if the cursor is in a property key position of an object literal.
pub(crate) fn is_object_literal_key_position(
    ast_tree: &ast::NodeTree,
    properties: &[ast::LocalNodeId<ast::Property>],
    offset: u32,
) -> bool {
    // check for any property key span hit
    for property_id in properties {
        if let Some(span) = ast_tree.source_map.get_main(property_id.id)
            && span.contains(offset)
        {
            return true;
        }
    }

    // avoid property values
    if is_object_literal_value_position(ast_tree, properties, offset) {
        return false;
    }

    // avoid property spans when not on keys
    for property_id in properties {
        let span = ast_tree.source_map.get(property_id.id);
        if span.contains(offset) {
            return false;
        }
    }

    true
}

/// Resolve one recorded contextual object type.
fn contextual_object_type(
    dir: DirQuery<'_>,
    types: &dir::TypeTable,
    node_id: dir::LocalNodeIdAny,
) -> Option<dir::LocalTypeId> {
    let global_node_id = node_id.into_global(dir.module_id());
    types.get_contextual_object_type_id_for_node(global_node_id)
}

/// Extract property names from object literal properties.
fn extract_object_property_names(
    dir_tree: &dir::NodeTree,
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
            dir::Property::Spread { .. } | dir::Property::Error { .. } => {}
        }
    }

    names
}

/// Check if the cursor is inside a value span of an object literal.
fn is_object_literal_value_position(
    ast_tree: &ast::NodeTree,
    properties: &[ast::LocalNodeId<ast::Property>],
    offset: u32,
) -> bool {
    // compute the cursor location inside the literal
    let cursor = offset.saturating_sub(1);

    // scan property value spans
    for property_id in properties {
        let property = ast_tree.get(*property_id);

        match property {
            ast::Property::Field { value, .. } => {
                let span = ast_tree.source_map.get(value.id);
                if span.contains(cursor) {
                    return true;
                }
            }
            ast::Property::Method { body, .. } => {
                if let Some(body_id) = body {
                    let span = ast_tree.source_map.get(body_id.id);
                    if span.contains(cursor) {
                        return true;
                    }
                }
            }
            ast::Property::Spread { value, .. } => {
                let span = ast_tree.source_map.get(value.id);
                if span.contains(cursor) {
                    return true;
                }
            }
            ast::Property::Error => {}
        }
    }

    // fall back to property spans outside keys
    for property_id in properties {
        let span = ast_tree.source_map.get(property_id.id);
        if !span.contains(cursor) {
            continue;
        }

        let key_span = ast_tree.source_map.get_main(property_id.id);
        let is_in_key = key_span
            .map(|key_span| key_span.contains(cursor))
            .unwrap_or(false);

        if !is_in_key {
            return true;
        }
    }

    false
}
