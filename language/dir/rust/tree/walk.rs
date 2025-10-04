use dyst_ast::NodeId;

use crate::{NodeTree, NodeType, NodeVisitor};

// ----------------------------------------------------------------------------
// Traversal functions
// ----------------------------------------------------------------------------

pub fn walk_any(visitor: &mut dyn NodeVisitor, tree: &NodeTree, node_type: NodeType, node_id: u32) {
    let local_idx = tree.local_id_by_node[node_id as usize];
    match node_type {
        // --------------------------------------------------------------------
        // Groupings
        // --------------------------------------------------------------------
        NodeType::Expression => {
            let expression = tree.expressions.get(local_idx);
            walk_expression(visitor, tree, NodeId::new(node_id), expression);
        }
        NodeType::Block => {
            let block = tree.blocks.get(local_idx);
            walk_block(visitor, tree, NodeId::new(node_id), block);
        }
        // --------------------------------------------------------------------
        // Declarations
        // --------------------------------------------------------------------
        NodeType::Definition => {
            let definition = tree.definitions.get(local_idx);
            walk_definition(visitor, tree, NodeId::new(node_id), definition);
        }
        // --------------------------------------------------------------------
        // Types
        // --------------------------------------------------------------------
        NodeType::Type => {
            let ty = tree.types.get(local_idx);
            walk_type(visitor, tree, NodeId::new(node_id), ty);
        }
        NodeType::Variant => {
            let variant = tree.variants.get(local_idx);
            walk_variant(visitor, tree, NodeId::new(node_id), variant);
        }
        NodeType::VariantField => {
            let variant_field = tree.variant_fields.get(local_idx);
            walk_variant_field(visitor, tree, NodeId::new(node_id), variant_field);
        }
        // --------------------------------------------------------------------
        // Context
        // --------------------------------------------------------------------
        NodeType::WithDeclaration => {
            let with_declaration = tree.with_declarations.get(local_idx);
            walk_with_declaration(visitor, tree, NodeId::new(node_id), with_declaration);
        }
        NodeType::WithAssertion => {
            let with_assertion = tree.with_assertions.get(local_idx);
            walk_with_assertion(visitor, tree, NodeId::new(node_id), with_assertion);
        }
        NodeType::UseItem => {
            let use_item = tree.use_items.get(local_idx);
            walk_use_item(visitor, tree, NodeId::new(node_id), use_item);
        }
        // --------------------------------------------------------------------
        // Bindings
        // --------------------------------------------------------------------
        NodeType::Parameter => {
            let parameter = tree.parameters.get(local_idx);
            walk_parameter(visitor, tree, NodeId::new(node_id), parameter);
        }
        NodeType::Argument => {
            let argument = tree.arguments.get(local_idx);
            walk_argument(visitor, tree, NodeId::new(node_id), argument);
        }
        // --------------------------------------------------------------------
        // Matching
        // --------------------------------------------------------------------
        NodeType::Pattern => {
            let pattern = tree.patterns.get(local_idx);
            walk_pattern(visitor, tree, NodeId::new(node_id), pattern);
        }
        NodeType::PatternField => {
            let pattern_field = tree.pattern_fields.get(local_idx);
            walk_pattern_field(visitor, tree, NodeId::new(node_id), pattern_field);
        }
        NodeType::MatchCase => {
            let match_case = tree.match_cases.get(local_idx);
            walk_match_case(visitor, tree, NodeId::new(node_id), match_case);
        }
        // --------------------------------------------------------------------
        // Annotations
        // --------------------------------------------------------------------
        NodeType::Annotation => {
            let annotation = tree.annotations.get(local_idx);
            walk_annotation(visitor, tree, NodeId::new(node_id), annotation);
        }
    }
}
