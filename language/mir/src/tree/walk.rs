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
            if let TypeReference::Type(pointee) = *pointee {
                let pointee_ty = tree.get(pointee);
                visitor.visit_type(tree, pointee, pointee_ty);
            }
        }
        Type::Atomic { value } => {
            if let TypeReference::Type(value) = *value {
                let value_ty = tree.get(value);
                visitor.visit_type(tree, value, value_ty);
            }
        }
        Type::Any { interface } => {
            if let TypeReference::Type(interface) = *interface {
                let interface_ty = tree.get(interface);
                visitor.visit_type(tree, interface, interface_ty);
            }
        }
        Type::Array { element, .. } => {
            if let TypeReference::Type(element) = *element {
                let element_ty = tree.get(element);
                visitor.visit_type(tree, element, element_ty);
            }
        }
        Type::Slice { element, .. } => {
            if let TypeReference::Type(element) = *element {
                let element_ty = tree.get(element);
                visitor.visit_type(tree, element, element_ty);
            }
        }
        Type::Tuple { elements, copy: _ } => {
            for element_id in elements {
                if let TypeReference::Type(element_id) = *element_id {
                    let element_ty = tree.get(element_id);
                    visitor.visit_type(tree, element_id, element_ty);
                }
            }
        }
        Type::Struct { fields, copy: _ } => {
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_field(tree, *field_id, field);
            }
        }
        Type::Newtype { inner, .. } => {
            if let TypeReference::Type(inner) = *inner {
                let inner_ty = tree.get(inner);
                visitor.visit_type(tree, inner, inner_ty);
            }
        }
        Type::Variant {
            tag,
            storage,
            cases,
            copy: _,
        } => {
            if let TypeReference::Type(tag) = *tag {
                let tag_ty = tree.get(tag);
                visitor.visit_type(tree, tag, tag_ty);
            }
            if let TypeReference::Type(storage) = *storage {
                let storage_ty = tree.get(storage);
                visitor.visit_type(tree, storage, storage_ty);
            }
            for case in cases {
                if let TypeReference::Type(case_id) = case.ty {
                    let case_ty = tree.get(case_id);
                    visitor.visit_type(tree, case_id, case_ty);
                }
            }
        }
        Type::Vector { element, .. } => {
            if let TypeReference::Type(element) = *element {
                let element_ty = tree.get(element);
                visitor.visit_type(tree, element, element_ty);
            }
        }
        Type::Tensor { element, .. } => {
            if let TypeReference::Type(element) = *element {
                let element_ty = tree.get(element);
                visitor.visit_type(tree, element, element_ty);
            }
        }
        Type::TensorView { element, .. } => {
            if let TypeReference::Type(element) = *element {
                let element_ty = tree.get(element);
                visitor.visit_type(tree, element, element_ty);
            }
        }
        Type::FunctionSignature {
            parameters, result, ..
        } => {
            for parameter_id in parameters {
                if let TypeReference::Type(parameter_id) = *parameter_id {
                    let parameter_ty = tree.get(parameter_id);
                    visitor.visit_type(tree, parameter_id, parameter_ty);
                }
            }
            if let TypeReference::Type(result) = *result {
                let result_ty = tree.get(result);
                visitor.visit_type(tree, result, result_ty);
            }
        }
        Type::FunctionPointer { signature } | Type::Callable { signature } => {
            if let TypeReference::Type(signature) = *signature {
                let signature_ty = tree.get(signature);
                visitor.visit_type(tree, signature, signature_ty);
            }
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
    if let TypeReference::Type(aliased_ty_id) = type_alias.ty {
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
    if let TypeReference::Type(field_ty_id) = field.ty {
        let field_ty = tree.get(field_ty_id);
        visitor.visit_type(tree, field_ty_id, field_ty);
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
