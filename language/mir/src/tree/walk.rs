//! MIR tree walking functions.

use crate::{
    Block, Function, Instruction, Local, LocalNodeId, NodeTree, NodeType, NodeVisitor, Type,
};

/// Walk any node by its type and id.
pub fn walk_any<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    node_type: NodeType,
    node_id: u32,
) {
    match node_type {
        NodeType::Function => {
            let id = LocalNodeId::new(node_id);
            let function = tree.get(id);
            visitor.visit_function(tree, id, function);
        }
        NodeType::Block => {
            let id = LocalNodeId::new(node_id);
            let block = tree.get(id);
            visitor.visit_block(tree, id, block);
        }
        NodeType::Instruction => {
            let id = LocalNodeId::new(node_id);
            let instruction = tree.get(id);
            visitor.visit_instruction(tree, id, instruction);
        }
        NodeType::Local => {
            let id = LocalNodeId::new(node_id);
            let local = tree.get(id);
            visitor.visit_local(tree, id, local);
        }
        NodeType::Type => {
            let id = LocalNodeId::new(node_id);
            let ty = tree.get(id);
            visitor.visit_type(tree, id, ty);
        }
    }
}

/// Walk a Function and its children.
pub fn walk_function<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Function>,
    function: &Function,
) {
    visitor.visit_any(tree, NodeType::Function, id.id);

    // visit locals
    for local_id in &function.locals {
        let local = tree.get(*local_id);
        visitor.visit_local(tree, *local_id, local);
    }

    // visit blocks
    for block_id in &function.blocks {
        let block = tree.get(*block_id);
        visitor.visit_block(tree, *block_id, block);
    }
}

/// Walk a Block and its children.
pub fn walk_block<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Block>,
    block: &Block,
) {
    visitor.visit_any(tree, NodeType::Block, id.id);

    // visit instructions
    for inst_id in &block.instructions {
        let instruction = tree.get(*inst_id);
        visitor.visit_instruction(tree, *inst_id, instruction);
    }

    // terminator doesn't need visiting (it's inline data, not a node)
}

/// Walk an Instruction (leaf node, nothing to recurse into).
pub fn walk_instruction<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Instruction>,
    _instruction: &Instruction,
) {
    visitor.visit_any(tree, NodeType::Instruction, id.id);
    // instructions are leaf nodes - they reference Values/Locals/Types but don't own child nodes
}

/// Walk a Local (leaf node).
pub fn walk_local<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Local>,
    _local: &Local,
) {
    visitor.visit_any(tree, NodeType::Local, id.id);
}

/// Walk a Type (may have nested types).
pub fn walk_type<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Type>,
    ty: &Type,
) {
    visitor.visit_any(tree, NodeType::Type, id.id);

    // recursively visit nested types
    match ty {
        Type::Pointer { pointee } => {
            let pointee_ty = tree.get(*pointee);
            visitor.visit_type(tree, *pointee, pointee_ty);
        }
        Type::Array { element, .. } => {
            let element_ty = tree.get(*element);
            visitor.visit_type(tree, *element, element_ty);
        }
        Type::Tuple { elements } => {
            for element_id in elements {
                let element_ty = tree.get(*element_id);
                visitor.visit_type(tree, *element_id, element_ty);
            }
        }
        Type::Struct { fields } => {
            for field in fields {
                let field_ty = tree.get(field.ty);
                visitor.visit_type(tree, field.ty, field_ty);
            }
        }
        Type::FunctionPointer { parameters, result } => {
            for param_id in parameters {
                let param_ty = tree.get(*param_id);
                visitor.visit_type(tree, *param_id, param_ty);
            }
            let result_ty = tree.get(*result);
            visitor.visit_type(tree, *result, result_ty);
        }
        // primitive types have no children
        Type::Void | Type::Boolean | Type::Int { .. } | Type::Float { .. } => {}
    }
}

