use crate::{
    Annotation, Argument, Block, Definition, Expression, MatchCase, NodeId, NodeTree, NodeType,
    NodeVisitor, Parameter, Pattern, PatternField, Type, UseItem, Variant, VariantField,
    WithAssertion, WithDeclaration,
};

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

// ----------------------------------------------------------------------------
// Groupings
// ----------------------------------------------------------------------------

/// Walk the Expression.
pub fn walk_expression(
    visitor: &mut dyn NodeVisitor,
    tree: &NodeTree,
    id: NodeId<Expression>,
    expression: &Expression,
) {
    todo!()
}

/// Walk the Block.
pub fn walk_block(
    visitor: &mut dyn NodeVisitor,
    tree: &NodeTree,
    id: NodeId<Block>,
    block: &Block,
) {
    todo!()
}

// ----------------------------------------------------------------------------
// Declarations
// ----------------------------------------------------------------------------

/// Walk the Definition.
pub fn walk_definition(
    visitor: &mut dyn NodeVisitor,
    tree: &NodeTree,
    id: NodeId<Definition>,
    definition: &Definition,
) {
    todo!()
}

// ----------------------------------------------------------------------------
// Types
// ----------------------------------------------------------------------------

/// Walk the Type.
pub fn walk_type(visitor: &mut dyn NodeVisitor, tree: &NodeTree, id: NodeId<Type>, ty: &Type) {
    todo!()
}

/// Walk the Variant.
pub fn walk_variant(
    visitor: &mut dyn NodeVisitor,
    tree: &NodeTree,
    id: NodeId<Variant>,
    variant: &Variant,
) {
    todo!()
}

/// Walk the VariantField.
pub fn walk_variant_field(
    visitor: &mut dyn NodeVisitor,
    tree: &NodeTree,
    id: NodeId<VariantField>,
    variant_field: &VariantField,
) {
    todo!()
}

// ----------------------------------------------------------------------------
// Context
// ----------------------------------------------------------------------------

/// Walk the WithDeclaration.
pub fn walk_with_declaration(
    visitor: &mut dyn NodeVisitor,
    tree: &NodeTree,
    id: NodeId<WithDeclaration>,
    with_declaration: &WithDeclaration,
) {
    todo!()
}

/// Walk the WithAssertion.
pub fn walk_with_assertion(
    visitor: &mut dyn NodeVisitor,
    tree: &NodeTree,
    id: NodeId<WithAssertion>,
    with_assertion: &WithAssertion,
) {
    todo!()
}

/// Walk the UseItem.
pub fn walk_use_item(
    visitor: &mut dyn NodeVisitor,
    tree: &NodeTree,
    id: NodeId<UseItem>,
    use_item: &UseItem,
) {
    todo!()
}

// ----------------------------------------------------------------------------
// Bindings
// ----------------------------------------------------------------------------

/// Walk the Parameter.
pub fn walk_parameter(
    visitor: &mut dyn NodeVisitor,
    tree: &NodeTree,
    id: NodeId<Parameter>,
    parameter: &Parameter,
) {
    todo!()
}

/// Walk the Argument.
pub fn walk_argument(
    visitor: &mut dyn NodeVisitor,
    tree: &NodeTree,
    id: NodeId<Argument>,
    argument: &Argument,
) {
    todo!()
}

// ----------------------------------------------------------------------------
// Matching
// ----------------------------------------------------------------------------

/// Walk the Pattern.
pub fn walk_pattern(
    visitor: &mut dyn NodeVisitor,
    tree: &NodeTree,
    id: NodeId<Pattern>,
    pattern: &Pattern,
) {
    todo!()
}

/// Walk the PatternField.
pub fn walk_pattern_field(
    visitor: &mut dyn NodeVisitor,
    tree: &NodeTree,
    id: NodeId<PatternField>,
    pattern_field: &PatternField,
) {
    todo!()
}

/// Walk the MatchCase.
pub fn walk_match_case(
    visitor: &mut dyn NodeVisitor,
    tree: &NodeTree,
    id: NodeId<MatchCase>,
    match_case: &MatchCase,
) {
    todo!()
}

// ----------------------------------------------------------------------------
// Annotations
// ----------------------------------------------------------------------------

/// Walk the Annotation.
pub fn walk_annotation(
    visitor: &mut dyn NodeVisitor,
    tree: &NodeTree,
    id: NodeId<Annotation>,
    annotation: &Annotation,
) {
    todo!()
}
