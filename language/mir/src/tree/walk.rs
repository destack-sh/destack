use crate::{
    Block, Field, Function, Global, Instruction, Local, LocalNodeId, NodeType, NodeVisitor,
    Terminator, Tree, Type, TypeAlias, TypeReference,
};

/// Walk any node.
pub fn walk_any<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
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
        NodeType::Terminator => {
            let id = LocalNodeId::new(node_id);
            let terminator = tree.get(id);
            visitor.visit_terminator(tree, id, terminator);
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
        NodeType::TypeAlias => {
            let id = LocalNodeId::new(node_id);
            let type_alias = tree.get(id);
            visitor.visit_type_alias(tree, id, type_alias);
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
    tree: &Tree,
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
    tree: &Tree,
    id: LocalNodeId<Block>,
    block: &Block,
) {
    visitor.visit_any(tree, NodeType::Block, id.id);

    for inst_id in &block.instructions {
        let instruction = tree.get(*inst_id);
        visitor.visit_instruction(tree, *inst_id, instruction);
    }

    let terminator = tree.get(block.terminator);
    visitor.visit_terminator(tree, block.terminator, terminator);
}

/// Walk an Instruction.
pub fn walk_instruction<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Instruction>,
    _instruction: &Instruction,
) {
    visitor.visit_any(tree, NodeType::Instruction, id.id);
}

/// Walk a Terminator.
pub fn walk_terminator<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Terminator>,
    _terminator: &Terminator,
) {
    visitor.visit_any(tree, NodeType::Terminator, id.id);
}

/// Walk a Local.
pub fn walk_local<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Local>,
    _local: &Local,
) {
    visitor.visit_any(tree, NodeType::Local, id.id);
}

/// Walk a Type.
pub fn walk_type<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Type>,
    ty: &Type,
) {
    visitor.visit_any(tree, NodeType::Type, id.id);

    match ty {
        Type::Reference { pointee, .. } => {
            walk_type_reference(visitor, tree, pointee);
        }
        Type::Atomic { value } => {
            walk_type_reference(visitor, tree, value);
        }
        Type::Dynamic { constraint } => {
            walk_type_reference(visitor, tree, constraint);
        }
        Type::Uninit { value } => {
            walk_type_reference(visitor, tree, value);
        }
        Type::Array { element, .. } => {
            walk_type_reference(visitor, tree, element);
        }
        Type::Slice { element, .. } => {
            walk_type_reference(visitor, tree, element);
        }
        Type::Tuple { elements, copy: _ } => {
            for element_id in elements {
                walk_type_reference(visitor, tree, element_id);
            }
        }
        Type::Struct { fields, copy: _ } => {
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_field(tree, *field_id, field);
            }
        }
        Type::Newtype { inner, .. } => {
            walk_type_reference(visitor, tree, inner);
        }
        Type::Variant {
            tag,
            storage,
            cases,
            copy: _,
        } => {
            walk_type_reference(visitor, tree, tag);
            walk_type_reference(visitor, tree, storage);
            for case in cases {
                walk_type_reference(visitor, tree, &case.ty);
            }
        }
        Type::Vector { element, .. } => {
            walk_type_reference(visitor, tree, element);
        }
        Type::Tensor { element, .. } => {
            walk_type_reference(visitor, tree, element);
        }
        Type::TensorView { element, .. } => {
            walk_type_reference(visitor, tree, element);
        }
        Type::FunctionSignature {
            parameters, result, ..
        } => {
            for parameter_id in parameters {
                walk_type_reference(visitor, tree, parameter_id);
            }
            walk_type_reference(visitor, tree, result);
        }
        Type::FunctionPointer { signature } => {
            walk_type_reference(visitor, tree, signature);
        }
        Type::Closure {
            signature,
            environment,
        } => {
            walk_type_reference(visitor, tree, signature);
            walk_type_reference(visitor, tree, environment);
        }
        Type::Void
        | Type::Boolean
        | Type::Int { .. }
        | Type::Isize
        | Type::Usize
        | Type::Float { .. }
        | Type::TypeDescriptor
        | Type::TypeId => {}
    }
}

/// Walk a TypeAlias.
pub fn walk_type_alias<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<TypeAlias>,
    type_alias: &TypeAlias,
) {
    visitor.visit_any(tree, NodeType::TypeAlias, id.id);
    if let Some(aliased_ty_id) = type_alias.ty.ty() {
        let aliased_ty = tree.get(aliased_ty_id);
        visitor.visit_type(tree, aliased_ty_id, aliased_ty);
    }
}

/// Walk a Field.
pub fn walk_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Field>,
    field: &Field,
) {
    visitor.visit_any(tree, NodeType::Field, id.id);
    if let Some(field_ty_id) = field.ty.ty() {
        let field_ty = tree.get(field_ty_id);
        visitor.visit_type(tree, field_ty_id, field_ty);
    }
}

/// Walk one referenced type node when present.
fn walk_type_reference<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    reference: &TypeReference,
) {
    if let Some(type_id) = reference.ty() {
        let ty = tree.get(type_id);
        visitor.visit_type(tree, type_id, ty);
    }
}

/// Walk a Global.
pub fn walk_global<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Global>,
    _global: &Global,
) {
    visitor.visit_any(tree, NodeType::Global, id.id);
}
