use crate::{
    Block, Call, Callee, CheckConstraint, Constant, Field, Function, GenericArgument, Global,
    Instruction, Local, LocalNodeId, NodeType, NodeVisitor, Static, StaticId, Terminator, Tree,
    Type, TypeDeclaration, TypeId,
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
        NodeType::TypeDeclaration => {
            let id = LocalNodeId::new(node_id);
            let type_declaration = tree.get(id);
            visitor.visit_type_declaration(tree, id, type_declaration);
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
        walk_argument(visitor, tree, argument);
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
        Instruction::Address { result_type, .. }
        | Instruction::Load { result_type, .. }
        | Instruction::VariantNew { result_type, .. }
        | Instruction::DynamicPayload { result_type, .. }
        | Instruction::DynamicRead { result_type, .. }
        | Instruction::DynamicFind { result_type, .. }
        | Instruction::NewComplete { result_type, .. }
        | Instruction::AtomicLoad { result_type, .. } => {
            walk_type_id(visitor, tree, result_type);
        }
        Instruction::DynamicBind { concrete, .. } => walk_type_id(visitor, tree, concrete),
        Instruction::Call { call, .. } => walk_call(visitor, tree, call),
        Instruction::FunctionAddr { arguments, .. }
        | Instruction::FunctionBind { arguments, .. } => {
            for argument in arguments {
                walk_argument(visitor, tree, argument);
            }
        }
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
        Instruction::Const {
            value: Constant::Layout { ty, .. },
            ..
        } => walk_type_id(visitor, tree, ty),
        Instruction::Const {
            value:
                Constant::Witness {
                    receiver,
                    interface,
                    ..
                },
            ..
        } => {
            walk_type_id(visitor, tree, receiver);
            walk_type_id(visitor, tree, interface);
        }
        Instruction::Error
        | Instruction::Const { .. }
        | Instruction::Copy { .. }
        | Instruction::Binary { .. }
        | Instruction::Unary { .. }
        | Instruction::Select { .. }
        | Instruction::FunctionEnvironment { .. }
        | Instruction::FunctionEnvironmentCurrent { .. }
        | Instruction::ContextCurrent { .. }
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
        | Instruction::Drop { .. }
        | Instruction::Release { .. }
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
pub fn walk_type<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, ty: &Type) {
    match ty {
        Type::Declaration { declaration } => {
            visitor.visit_type_declaration(tree, *declaration, tree.get(*declaration));
        }
        Type::Error => {}
        Type::Reference { pointee, .. } | Type::Pointer { pointee, .. } => {
            walk_type_id(visitor, tree, pointee);
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
        Type::Tuple { elements } => {
            for element_id in elements {
                walk_type_id(visitor, tree, element_id);
            }
        }
        Type::Struct { fields } => {
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
        } => {
            walk_type_id(visitor, tree, discriminant);
            for case in cases {
                walk_type_id(visitor, tree, &case.ty);
            }
        }
        Type::Vector { element, .. } => {
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
        Type::Witness {
            receiver,
            interface,
            ..
        } => {
            walk_type_id(visitor, tree, receiver);
            walk_type_id(visitor, tree, interface);
        }
        Type::Function { signature, .. } => {
            walk_type_id(visitor, tree, signature);
        }
        Type::Application {
            base, arguments, ..
        } => {
            walk_type_id(visitor, tree, base);
            for argument in arguments {
                walk_argument(visitor, tree, argument);
            }
        }
        Type::Never
        | Type::Void
        | Type::Null
        | Type::Boolean
        | Type::Character
        | Type::Int { .. }
        | Type::Isize
        | Type::Usize
        | Type::Float { .. }
        | Type::Parameter { .. }
        | Type::TypeId => {}
    }
}

/// Walk the types referenced by one generic argument.
fn walk_argument<V: NodeVisitor + ?Sized>(
    visitor: &mut V,
    tree: &Tree,
    argument: &GenericArgument,
) {
    match argument {
        GenericArgument::Type(ty) => walk_type_id(visitor, tree, ty),
        GenericArgument::Value(value) => walk_static(visitor, tree, *value),
        GenericArgument::Region(_) | GenericArgument::Access(_) => {}
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

    if let Some(definition) = type_declaration.definition {
        walk_type_id(visitor, tree, &definition);
    }
    for base in type_declaration
        .heritage
        .extends
        .iter()
        .chain(&type_declaration.heritage.implements)
    {
        walk_type_id(visitor, tree, base);
    }
}

/// Walk a Field.
pub fn walk_field<V: NodeVisitor + ?Sized>(visitor: &mut V, tree: &Tree, field: &Field) {
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
        Static::Parameter(_)
        | Static::Null
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
        Callee::Direct { arguments, .. } => {
            for argument in arguments {
                walk_argument(visitor, tree, argument);
            }
        }
        Callee::Virtual { class, .. } => walk_type_id(visitor, tree, class),
        Callee::Dynamic { constraint, .. } => walk_type_id(visitor, tree, constraint),
        Callee::Witness {
            receiver,
            interface,
            arguments,
            ..
        } => {
            walk_type_id(visitor, tree, receiver);
            walk_type_id(visitor, tree, interface);
            for argument in arguments {
                walk_argument(visitor, tree, argument);
            }
        }
        Callee::Indirect { .. } => {}
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
