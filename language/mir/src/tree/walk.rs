use crate::{
    Block, Field, Function, Global, Instruction, Local, LocalNodeId, NodeTree, NodeType,
    NodeVisitor, Type,
};

/// Walk any node.
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
        NodeType::Field => {
            let id = LocalNodeId::new(node_id);
            let field = tree.get(id);
            visitor.visit_field(tree, id, field);
        }
        NodeType::Global => {
            let id = LocalNodeId::new(node_id);
            let global = tree.get(id);
            visitor.visit_global(tree, id, global);
        }
    }
}

/// Walk a Function.
pub fn walk_function<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Function>,
    function: &Function,
) {
    visitor.visit_any(tree, NodeType::Function, id.id);
    for local_id in &function.locals {
        let local = tree.get(*local_id);
        visitor.visit_local(tree, *local_id, local);
    }
    for block_id in &function.blocks {
        let block = tree.get(*block_id);
        visitor.visit_block(tree, *block_id, block);
    }
}

/// Walk a Block.
pub fn walk_block<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Block>,
    block: &Block,
) {
    visitor.visit_any(tree, NodeType::Block, id.id);

    for inst_id in &block.instructions {
        let instruction = tree.get(*inst_id);
        visitor.visit_instruction(tree, *inst_id, instruction);
    }
}

/// Walk an Instruction.
pub fn walk_instruction<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Instruction>,
    _instruction: &Instruction,
) {
    visitor.visit_any(tree, NodeType::Instruction, id.id);
}

/// Walk a Local.
pub fn walk_local<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Local>,
    _local: &Local,
) {
    visitor.visit_any(tree, NodeType::Local, id.id);
}

/// Walk a Type.
pub fn walk_type<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Type>,
    ty: &Type,
) {
    visitor.visit_any(tree, NodeType::Type, id.id);

    match ty {
        Type::RawPointer { pointee } => {
            let pointee_ty = tree.get(*pointee);
            visitor.visit_type(tree, *pointee, pointee_ty);
        }
        Type::ManagedReference { pointee, .. } => {
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
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_field(tree, *field_id, field);
            }
        }
        Type::FunctionPointer { parameters, result } => {
            for parameter_id in parameters {
                let parameter_ty = tree.get(*parameter_id);
                visitor.visit_type(tree, *parameter_id, parameter_ty);
            }
            let result_ty = tree.get(*result);
            visitor.visit_type(tree, *result, result_ty);
        }
        Type::Void | Type::Boolean | Type::Int { .. } | Type::Float { .. } => {}
    }
}

/// Walk a Field.
pub fn walk_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Field>,
    field: &Field,
) {
    visitor.visit_any(tree, NodeType::Field, id.id);
    let field_ty = tree.get(field.ty);
    visitor.visit_type(tree, field.ty, field_ty);
}

/// Walk a Global.
pub fn walk_global<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &NodeTree,
    id: LocalNodeId<Global>,
    _global: &Global,
) {
    visitor.visit_any(tree, NodeType::Global, id.id);
}
