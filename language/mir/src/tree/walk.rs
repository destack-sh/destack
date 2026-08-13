use crate::{
    Block, Call, Callee, CheckConstraint, Field, Function, Global, Instruction, Local, LocalNodeId,
    NodeType, NodeVisitor, Static, StaticId, Terminator, Tree, Type, TypeDeclaration, TypeId,
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
        NodeType::TypeDeclaration => {
            let id = LocalNodeId::new(node_id);
            let type_declaration = tree.get(id);
            visitor.visit_type_declaration(tree, id, type_declaration);
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

    for argument in &function.arguments {
        walk_static(visitor, tree, *argument);
    }
    for parameter in &function.parameters {
        walk_type_id(visitor, tree, &parameter.ty);
    }
    walk_type_id(visitor, tree, &function.return_type);
    if let Some(environment) = &function.environment {
        walk_type_id(visitor, tree, environment);
    }
    for ty in function.value_types().iter().flatten() {
        walk_type_id(visitor, tree, ty);
    }

    for local_id in function.locals() {
        let local = tree.get(*local_id);
        visitor.visit_local(tree, *local_id, local);
    }
    for block_id in function.blocks() {
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

    for parameter in &block.parameters {
        walk_type_id(visitor, tree, &parameter.ty);
    }

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
    instruction: &Instruction,
) {
    visitor.visit_any(tree, NodeType::Instruction, id.id);

    match instruction {
        Instruction::Cast { to_type, .. } => walk_type_id(visitor, tree, to_type),
        Instruction::LocalAddr { result_type, .. }
        | Instruction::GlobalAddr { result_type, .. }
        | Instruction::Load { result_type, .. }
        | Instruction::FieldAddr { result_type, .. }
        | Instruction::ElementAddr { result_type, .. }
        | Instruction::VariantNew { result_type, .. }
        | Instruction::SliceView { result_type, .. }
        | Instruction::DynamicPayload { result_type, .. }
        | Instruction::DynamicFind { result_type, .. }
        | Instruction::NewComplete { result_type, .. }
        | Instruction::Pin { result_type, .. }
        | Instruction::AtomicLoad { result_type, .. } => {
            walk_type_id(visitor, tree, result_type);
        }
        Instruction::DynamicBind { concrete, .. } => walk_type_id(visitor, tree, concrete),
        Instruction::Call { call, .. } => walk_call(visitor, tree, call),
        Instruction::NewZeroed {
            storage_type,
            result_type,
            ..
        }
        | Instruction::NewUninit {
            storage_type,
            result_type,
            ..
        } => {
            walk_type_id(visitor, tree, storage_type);
            walk_type_id(visitor, tree, result_type);
        }
        Instruction::ContextBind {
            node_type,
            result_type,
            ..
        }
        | Instruction::ContextGet {
            node_type,
            result_type,
            ..
        } => {
            walk_type_id(visitor, tree, node_type);
            walk_type_id(visitor, tree, result_type);
        }
        Instruction::NewSliceZeroed {
            element,
            result_type,
            ..
        }
        | Instruction::NewSliceUninit {
            element,
            result_type,
            ..
        } => {
            walk_type_id(visitor, tree, element);
            walk_type_id(visitor, tree, result_type);
        }
        Instruction::Error
        | Instruction::Const { .. }
        | Instruction::Binary { .. }
        | Instruction::Unary { .. }
        | Instruction::Select { .. }
        | Instruction::LocalGet { .. }
        | Instruction::LocalSet { .. }
        | Instruction::FunctionAddr { .. }
        | Instruction::FunctionBind { .. }
        | Instruction::FunctionEnvironment { .. }
        | Instruction::FunctionEnvironmentCurrent { .. }
        | Instruction::ContextCurrent { .. }
        | Instruction::CallDetach { .. }
        | Instruction::ContextReplace { .. }
        | Instruction::Store { .. }
        | Instruction::Aggregate { .. }
        | Instruction::FieldGet { .. }
        | Instruction::FieldSet { .. }
        | Instruction::ElementGet { .. }
        | Instruction::ElementSet { .. }
        | Instruction::VariantTag { .. }
        | Instruction::VariantTagLoad { .. }
        | Instruction::VariantPayload { .. }
        | Instruction::VariantPayloadAddr { .. }
        | Instruction::SliceLength { .. }
        | Instruction::DynamicType { .. }
        | Instruction::VectorSplat { .. }
        | Instruction::VectorExtract { .. }
        | Instruction::VectorInsert { .. }
        | Instruction::VectorShuffle { .. }
        | Instruction::VectorSelect { .. }
        | Instruction::VectorReduce { .. }
        | Instruction::VectorCompare { .. }
        | Instruction::VectorConvert { .. }
        | Instruction::TensorSplat { .. }
        | Instruction::TensorLoad { .. }
        | Instruction::TensorExtract { .. }
        | Instruction::TensorStore { .. }
        | Instruction::TensorFill { .. }
        | Instruction::TensorCopy { .. }
        | Instruction::TensorReshape { .. }
        | Instruction::TensorBroadcast { .. }
        | Instruction::TensorTranspose { .. }
        | Instruction::TensorCast { .. }
        | Instruction::TensorView { .. }
        | Instruction::TensorSlice { .. }
        | Instruction::TensorPad { .. }
        | Instruction::TensorConcat { .. }
        | Instruction::TensorCompare { .. }
        | Instruction::TensorSelect { .. }
        | Instruction::TensorReduce { .. }
        | Instruction::TensorIndexReduce { .. }
        | Instruction::TensorDot { .. }
        | Instruction::TensorConvolution { .. }
        | Instruction::TensorGather { .. }
        | Instruction::TensorScatter { .. }
        | Instruction::TensorConvert { .. }
        | Instruction::Drop { .. }
        | Instruction::Free { .. }
        | Instruction::Unpin { .. }
        | Instruction::BarrierWrite { .. }
        | Instruction::AtomicStore { .. }
        | Instruction::AtomicCompareExchange { .. }
        | Instruction::AtomicRmw { .. }
        | Instruction::AtomicFence { .. }
        | Instruction::Assume { .. }
        | Instruction::ProfileIncrement { .. }
        | Instruction::ProfileSample { .. }
        | Instruction::Poll
        | Instruction::Breakpoint
        | Instruction::Intrinsic { .. } => {}
    }
}

/// Walk a Terminator.
pub fn walk_terminator<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Terminator>,
    terminator: &Terminator,
) {
    visitor.visit_any(tree, NodeType::Terminator, id.id);

    match terminator {
        Terminator::Check { constraint, .. } => match constraint {
            CheckConstraint::IsType { expected, .. }
            | CheckConstraint::IsSubtype { expected, .. } => {
                walk_type_id(visitor, tree, expected);
            }
            CheckConstraint::Bounds { .. }
            | CheckConstraint::Null { .. }
            | CheckConstraint::DivZero { .. }
            | CheckConstraint::ShiftRange { .. }
            | CheckConstraint::Narrow { .. }
            | CheckConstraint::Overflow { .. } => {}
        },
        Terminator::Invoke { call, .. } | Terminator::TailCall { call } => {
            walk_call(visitor, tree, call);
        }
        Terminator::NewZeroedTry { storage_type, .. }
        | Terminator::NewUninitTry { storage_type, .. } => {
            walk_type_id(visitor, tree, storage_type)
        }
        Terminator::NewSliceZeroedTry { element, .. }
        | Terminator::NewSliceUninitTry { element, .. } => walk_type_id(visitor, tree, element),
        Terminator::Error
        | Terminator::Return { .. }
        | Terminator::Jump { .. }
        | Terminator::Branch { .. }
        | Terminator::Switch { .. }
        | Terminator::VariantSwitch { .. }
        | Terminator::Panic { .. }
        | Terminator::UnwindResume
        | Terminator::Abort { .. }
        | Terminator::Unreachable => {}
    }
}

/// Walk a Local.
pub fn walk_local<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Local>,
    local: &Local,
) {
    visitor.visit_any(tree, NodeType::Local, id.id);
    walk_type_id(visitor, tree, &local.ty);
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
        Type::Error => {}
        Type::Reference { pointee, .. } | Type::Pointer { pointee, .. } => {
            walk_type_id(visitor, tree, pointee);
        }
        Type::Atomic { value } => {
            walk_type_id(visitor, tree, value);
        }
        Type::Dynamic { constraint, .. } => {
            walk_type_id(visitor, tree, constraint);
        }
        Type::Uninit { value } => {
            walk_type_id(visitor, tree, value);
        }
        Type::ManuallyDrop { value } => {
            walk_type_id(visitor, tree, value);
        }
        Type::FixedArray { element, .. } => {
            walk_type_id(visitor, tree, element);
        }
        Type::Slice { element, .. } => {
            walk_type_id(visitor, tree, element);
        }
        Type::Tuple { elements, copy: _ } => {
            for element_id in elements {
                walk_type_id(visitor, tree, element_id);
            }
        }
        Type::Struct { fields, copy: _ } => {
            for field_id in fields {
                let field = tree.get(*field_id);
                visitor.visit_field(tree, *field_id, field);
            }
        }
        Type::Newtype { inner, .. } => {
            walk_type_id(visitor, tree, inner);
        }
        Type::Variant {
            discriminant,
            cases,
            copy: _,
        } => {
            walk_type_id(visitor, tree, discriminant);
            for case in cases {
                walk_type_id(visitor, tree, &case.ty);
            }
        }
        Type::Vector { element, .. } => {
            walk_type_id(visitor, tree, element);
        }
        Type::Tensor { element, .. } => {
            walk_type_id(visitor, tree, element);
        }
        Type::TensorView { element, .. } => {
            walk_type_id(visitor, tree, element);
        }
        Type::FunctionSignature {
            parameters, result, ..
        } => {
            for parameter in parameters {
                walk_type_id(visitor, tree, &parameter.ty);
            }
            walk_type_id(visitor, tree, result);
        }
        Type::FunctionPointer { signature } => {
            walk_type_id(visitor, tree, signature);
        }
        Type::Function { signature, .. } => {
            walk_type_id(visitor, tree, signature);
        }
        Type::Application { base, .. } => {
            walk_type_id(visitor, tree, base);
        }
        Type::Never
        | Type::Void
        | Type::Boolean
        | Type::Character
        | Type::Int { .. }
        | Type::Isize
        | Type::Usize
        | Type::Float { .. }
        | Type::TypeDescriptor
        | Type::TypeId => {}
    }
}

/// Walk a TypeDeclaration.
pub fn walk_type_declaration<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<TypeDeclaration>,
    type_declaration: &TypeDeclaration,
) {
    visitor.visit_any(tree, NodeType::TypeDeclaration, id.id);

    for argument in &type_declaration.arguments {
        walk_static(visitor, tree, *argument);
    }
    let declared_ty = tree.get(type_declaration.ty);
    visitor.visit_type(tree, type_declaration.ty, declared_ty);
}

/// Walk a Field.
pub fn walk_field<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Field>,
    field: &Field,
) {
    visitor.visit_any(tree, NodeType::Field, id.id);
    let field_ty = tree.get(field.ty);
    visitor.visit_type(tree, field.ty, field_ty);
}

/// Walk one referenced type node when present.
fn walk_type_id<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, reference: &TypeId) {
    let ty = tree.get(*reference);
    visitor.visit_type(tree, *reference, ty);
}

/// Walk the types referenced by one interned static value.
fn walk_static<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, id: StaticId) {
    match tree.static_value(id) {
        Static::Type(ty) => walk_type_id(visitor, tree, ty),
        Static::Array(values) | Static::Tuple(values) => {
            for value in values {
                walk_static(visitor, tree, *value);
            }
        }
        Static::FixedArray { value, .. } => walk_static(visitor, tree, *value),
        Static::Newtype { ty, value } => {
            walk_type_id(visitor, tree, ty);
            walk_static(visitor, tree, *value);
        }
        Static::Object(fields) => {
            for field in fields {
                walk_static(visitor, tree, field.value);
            }
        }
        Static::Struct { ty, fields } => {
            walk_type_id(visitor, tree, ty);
            for field in fields {
                walk_static(visitor, tree, field.value);
            }
        }
        Static::Null
        | Static::Undefined
        | Static::Boolean(_)
        | Static::Integer(_)
        | Static::Bigint(_)
        | Static::Float(_)
        | Static::Character(_)
        | Static::String(_)
        | Static::Regex { .. } => {}
    }
}

/// Walk the types owned by one call operation.
fn walk_call<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, call: &Call) {
    walk_type_id(visitor, tree, &call.signature);

    match &call.callee {
        Callee::Virtual { class, .. } => walk_type_id(visitor, tree, class),
        Callee::Dynamic { constraint, .. } => walk_type_id(visitor, tree, constraint),
        Callee::Direct { .. } | Callee::Indirect { .. } => {}
    }
}

/// Walk a Global.
pub fn walk_global<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    id: LocalNodeId<Global>,
    global: &Global,
) {
    visitor.visit_any(tree, NodeType::Global, id.id);
    walk_type_id(visitor, tree, &global.ty);
}
