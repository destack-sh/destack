use destack_mir as mir;

use crate::{Error, ReferenceMeta};
use destack_program::TypeTable;

use destack_program::vm::{AddressSpace, ValueShape, address_space_from_reference};

/// One dense map from SSA value id to lowered shape.
pub(super) struct ValueShapeMap {
    /// Shape for each SSA value id.
    shapes: Vec<Option<ValueShape>>,
}

impl ValueShapeMap {
    /// Create a new value shape table.
    pub(super) fn new(value_count: usize) -> Self {
        Self {
            shapes: vec![None; value_count],
        }
    }

    /// Get the shape for a value.
    pub(super) fn get(&self, value: mir::Value) -> Option<ValueShape> {
        self.shapes.get(value.0 as usize).and_then(|shape| *shape)
    }

    /// Set the shape for a value.
    pub(super) fn set(&mut self, value: mir::Value, shape: ValueShape) {
        self.replace(value, Some(shape));
    }

    /// Replace the shape for a value.
    fn replace(&mut self, value: mir::Value, shape: Option<ValueShape>) {
        if let Some(entry) = self.shapes.get_mut(value.0 as usize) {
            *entry = shape;
        }
    }
}

/// Resolve a MIR value type from the function type table.
pub(super) fn value_type_from_table(
    value: mir::Value,
    value_types: &[mir::LocalNodeId<mir::Type>],
) -> Option<mir::LocalNodeId<mir::Type>> {
    value_types.get(value.0 as usize).copied()
}

/// One value shape builder for one function.
pub(super) struct ValueShapeMapBuilder<'a> {
    /// The lowered runtime type table.
    types: &'a TypeTable,
    /// Pointer byte width used by pointer-sized runtime values.
    pointer_bytes: u8,
    /// The MIR node tree.
    tree: &'a mir::Tree,
    /// The MIR function being lowered.
    func: &'a mir::Function,
    /// The MIR blocks in lowered order.
    mir_block: &'a [mir::LocalNodeId<mir::Block>],
    /// The lowered value types by SSA value id.
    value_type: &'a [mir::LocalNodeId<mir::Type>],
    /// The lowered value shape map.
    value_shape_map: ValueShapeMap,
}

impl<'a> ValueShapeMapBuilder<'a> {
    /// Create one value shape builder.
    pub(super) fn new(
        types: &'a TypeTable,
        pointer_bytes: u8,
        tree: &'a mir::Tree,
        func: &'a mir::Function,
        mir_block: &'a [mir::LocalNodeId<mir::Block>],
        value_type: &'a [mir::LocalNodeId<mir::Type>],
        value_count: usize,
    ) -> Self {
        Self {
            types,
            pointer_bytes,
            tree,
            func,
            mir_block,
            value_type,
            value_shape_map: ValueShapeMap::new(value_count),
        }
    }

    /// Build the lowered value shape map.
    pub(super) fn build(mut self) -> ValueShapeMap {
        // seed explicit value types
        for (index, ty) in self.value_type.iter().enumerate() {
            let value = mir::Value(index as u32);
            if let Some(shape) = self.types.value_shape_for_mir(*ty, self.pointer_bytes) {
                self.value_shape_map.set(value, shape);
            }
        }

        // seed function parameter shapes
        for param in &self.func.parameters {
            let value = param.value;
            let ty = param.ty;

            if let Some(shape) = self.types.value_shape_for_mir(ty, self.pointer_bytes) {
                self.value_shape_map.set(value, shape);
            }
        }

        // seed block parameter shapes
        for block_id in self.mir_block {
            let block = self.tree.get(*block_id);
            for param in &block.parameters {
                let value = param.value;
                let ty = param.ty;
                let Some(shape) = self.types.value_shape_for_mir(ty, self.pointer_bytes) else {
                    continue;
                };
                let block_shape = shape_for_block_parameter(shape);
                let next_shape = match self.value_shape_map.get(value) {
                    Some(existing) => merge_block_parameter_shape(existing, block_shape),
                    None => Some(block_shape),
                };

                self.value_shape_map.replace(value, next_shape);
            }
        }

        // iteratively infer instruction result shapes
        let mut is_changed = true;
        while is_changed {
            // reset iteration state
            is_changed = false;

            // propagate block parameter shapes from control flow edges
            if propagate_block_parameter_shapes(
                self.tree,
                self.types,
                self.pointer_bytes,
                self.mir_block,
                &mut self.value_shape_map,
            ) {
                is_changed = true;
            }

            // scan instructions for new kinds
            for block_id in self.mir_block {
                let block = self.tree.get(*block_id);

                for inst_id in &block.instructions {
                    let inst = self.tree.get(*inst_id);
                    let Some(destination) = inst.destination() else {
                        continue;
                    };
                    let Some(shape) = infer_instruction_shape(
                        self.tree,
                        self.types,
                        self.pointer_bytes,
                        inst,
                        &self.value_shape_map,
                        self.value_type,
                    ) else {
                        continue;
                    };
                    let next_shape = match self.value_shape_map.get(destination) {
                        Some(existing) => merge_block_parameter_shape(existing, shape),
                        None => Some(shape),
                    };

                    if self.value_shape_map.get(destination) != next_shape {
                        self.value_shape_map.replace(destination, next_shape);
                        is_changed = true;
                    }
                }
            }
        }

        self.value_shape_map
    }
}

/// Get the address space for a value when available.
pub(super) fn address_space_for_value(
    value_shape_map: &ValueShapeMap,
    value: mir::Value,
) -> Result<AddressSpace, Error> {
    match value_shape_map.get(value) {
        Some(ValueShape::Pointer { address_space, .. }) => Ok(address_space),
        Some(ValueShape::Aggregate { .. } | ValueShape::Array { .. }) => Ok(AddressSpace::Frame),
        _ => Err(Error::invalid_pointer_type(format!("{value:?}"))),
    }
}

/// Get reference metadata for a type when available.
pub(super) fn reference_meta_for_type(
    types: &TypeTable,
    ty: mir::LocalNodeId<mir::Type>,
    pointer_bytes: u8,
) -> ReferenceMeta {
    match types.value_shape_for_mir(ty, pointer_bytes) {
        Some(ValueShape::Pointer { reference, .. }) => reference,
        _ => ReferenceMeta::NONE,
    }
}

/// Resolve one heap pointee type from a pointer value when available.
pub(super) fn heap_pointee_type_from_shape(
    types: &TypeTable,
    value_shape_map: &ValueShapeMap,
    value: mir::Value,
) -> Option<mir::LocalNodeId<mir::Type>> {
    match value_shape_map.get(value) {
        Some(ValueShape::Pointer {
            pointee,
            address_space: AddressSpace::Local | AddressSpace::Shared,
            ..
        }) => types.mir_type_id(pointee),
        _ => None,
    }
}

/// Resolve one raw pointee type from a pointer value when available.
pub(super) fn raw_pointee_type_from_shape(
    types: &TypeTable,
    value_shape_map: &ValueShapeMap,
    value: mir::Value,
) -> Option<mir::LocalNodeId<mir::Type>> {
    match value_shape_map.get(value) {
        Some(ValueShape::Pointer {
            pointee,
            address_space:
                AddressSpace::Raw | AddressSpace::Stack | AddressSpace::Frame | AddressSpace::Static,
            ..
        }) => types.mir_type_id(pointee),
        _ => None,
    }
}

/// Resolve one heap pointee type from a value type when available.
pub(super) fn heap_pointee_type_for_value(
    tree: &mir::Tree,
    value_types: &[mir::LocalNodeId<mir::Type>],
    value: mir::Value,
) -> Option<mir::LocalNodeId<mir::Type>> {
    let ty = tree.repr_type(value_type_from_table(value, value_types)?);

    match tree.get(ty) {
        mir::Type::Reference {
            kind:
                kind @ (mir::ReferenceKind::Managed
                | mir::ReferenceKind::Unique
                | mir::ReferenceKind::Borrowed),
            space,
            pointee,
            ..
        } if matches!(
            address_space_from_reference(space.clone(), *kind),
            AddressSpace::Local | AddressSpace::Shared
        ) =>
        {
            Some(*pointee)
        }
        _ => None,
    }
}

/// Resolve one raw pointee type from a value type when available.
pub(super) fn raw_pointee_type_for_value(
    tree: &mir::Tree,
    value_types: &[mir::LocalNodeId<mir::Type>],
    value: mir::Value,
) -> Option<mir::LocalNodeId<mir::Type>> {
    let ty = tree.repr_type(value_type_from_table(value, value_types)?);

    match tree.get(ty) {
        mir::Type::Reference {
            kind,
            space,
            pointee,
            ..
        } if matches!(
            address_space_from_reference(space.clone(), *kind),
            AddressSpace::Raw | AddressSpace::Stack | AddressSpace::Frame
        ) =>
        {
            Some(*pointee)
        }
        _ => None,
    }
}

/// Normalize one block parameter shape.
fn shape_for_block_parameter(shape: ValueShape) -> ValueShape {
    shape
}

/// Merge one block parameter shape with one incoming argument shape.
fn merge_block_parameter_shape(existing: ValueShape, incoming: ValueShape) -> Option<ValueShape> {
    match (existing, incoming) {
        (left @ ValueShape::Pointer { .. }, right @ ValueShape::Pointer { .. })
            if left == right =>
        {
            Some(left)
        }
        (ValueShape::Pointer { .. }, ValueShape::Pointer { .. }) => None,
        (other, _) => Some(other),
    }
}

/// Propagate value shapes into block parameters from control flow edges.
fn propagate_block_parameter_shapes(
    tree: &mir::Tree,
    types: &TypeTable,
    pointer_bytes: u8,
    mir_blocks: &[mir::LocalNodeId<mir::Block>],
    value_shape_map: &mut ValueShapeMap,
) -> bool {
    let mut is_changed = false;

    // walk every terminator edge
    for block_id in mir_blocks {
        let block = tree.get(*block_id);
        let terminator = tree.get(block.terminator);

        match terminator {
            mir::Terminator::Jump { target } => {
                is_changed |= propagate_target_edge(tree, value_shape_map, target);
            }
            mir::Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                is_changed |= propagate_target_edge(tree, value_shape_map, then_target);
                is_changed |= propagate_target_edge(tree, value_shape_map, else_target);
            }
            mir::Terminator::Check {
                success, failure, ..
            } => {
                is_changed |= propagate_target_edge(tree, value_shape_map, success);
                is_changed |= propagate_target_edge(tree, value_shape_map, failure);
            }
            mir::Terminator::NewZeroedTry {
                success, failure, ..
            }
            | mir::Terminator::NewUninitTry {
                success, failure, ..
            } => {
                is_changed |= propagate_allocation_target_edge(
                    types,
                    pointer_bytes,
                    tree,
                    value_shape_map,
                    success,
                );
                is_changed |= propagate_target_edge(tree, value_shape_map, failure);
            }
            mir::Terminator::NewSliceZeroedTry {
                success, failure, ..
            }
            | mir::Terminator::NewSliceUninitTry {
                success, failure, ..
            } => {
                is_changed |= propagate_allocation_target_edge(
                    types,
                    pointer_bytes,
                    tree,
                    value_shape_map,
                    success,
                );
                is_changed |= propagate_target_edge(tree, value_shape_map, failure);
            }
            mir::Terminator::Switch { default, cases, .. } => {
                is_changed |= propagate_target_edge(tree, value_shape_map, default);

                for case in tree.get_switch_cases(*cases) {
                    is_changed |= propagate_target_edge(tree, value_shape_map, &case.target);
                }
            }
            mir::Terminator::Yield { resume, .. } => {
                is_changed |= propagate_target_edge(tree, value_shape_map, resume);
            }
            mir::Terminator::Call { target, unwind, .. }
            | mir::Terminator::CallIndirect { target, unwind, .. }
            | mir::Terminator::CallVirtual { target, unwind, .. }
            | mir::Terminator::CallDynamic { target, unwind, .. } => {
                is_changed |= propagate_target_edge(tree, value_shape_map, target);
                if let Some(unwind) = unwind {
                    is_changed |= propagate_target_edge(tree, value_shape_map, unwind);
                }
            }
            mir::Terminator::Error => {}
            mir::Terminator::Return { .. }
            | mir::Terminator::Panic { .. }
            | mir::Terminator::UnwindResume
            | mir::Terminator::Trap { .. }
            | mir::Terminator::Unreachable
            | mir::Terminator::TailCall { .. }
            | mir::Terminator::TailCallIndirect { .. }
            | mir::Terminator::TailCallVirtual { .. }
            | mir::Terminator::TailCallDynamic { .. } => {}
        }
    }

    is_changed
}

/// Update one target block from one control flow edge.
fn propagate_allocation_target_edge(
    types: &TypeTable,
    pointer_bytes: u8,
    tree: &mir::Tree,
    value_shape_map: &mut ValueShapeMap,
    target: &mir::BlockTarget,
) -> bool {
    let mut is_changed = false;
    let block = tree.get(target.block);
    let Some(result_parameter) = block.parameters.first() else {
        return false;
    };
    let result_value = result_parameter.value;
    let result_type = result_parameter.ty;
    let result_shape = types.value_shape_for_mir(result_type, pointer_bytes);
    if value_shape_map.get(result_value) != result_shape {
        value_shape_map.replace(result_value, result_shape);
        is_changed = true;
    }

    let parameters = block.parameters.iter().skip(1);

    for (parameter, argument) in parameters.zip(target.arguments(tree).iter()) {
        let Some(argument_shape) = value_shape_map.get(*argument) else {
            continue;
        };
        let parameter_value = parameter.value;
        let existing = value_shape_map.get(parameter_value);
        let next_shape = match existing {
            Some(shape) => merge_block_parameter_shape(shape, argument_shape),
            None => Some(argument_shape),
        };

        if existing != next_shape {
            value_shape_map.replace(parameter_value, next_shape);
            is_changed = true;
        }
    }

    is_changed
}

/// Update one target block from one control flow edge.
fn propagate_target_edge(
    tree: &mir::Tree,
    value_shape_map: &mut ValueShapeMap,
    target: &mir::BlockTarget,
) -> bool {
    let mut is_changed = false;
    let target_block = tree.get(target.block);
    let arguments = target.arguments(tree);

    for (parameter, argument) in target_block.parameters.iter().zip(arguments.iter()) {
        let Some(argument_shape) = value_shape_map.get(*argument) else {
            continue;
        };
        let parameter_value = parameter.value;
        let existing = value_shape_map.get(parameter_value);
        let next_shape = match existing {
            Some(shape) => merge_block_parameter_shape(shape, argument_shape),
            None => Some(argument_shape),
        };

        if existing != next_shape {
            value_shape_map.replace(parameter_value, next_shape);
            is_changed = true;
        }
    }

    is_changed
}

/// Infer the value shape for a MIR instruction.
fn infer_instruction_shape(
    tree: &mir::Tree,
    types: &TypeTable,
    pointer_bytes: u8,
    inst: &mir::Instruction,
    value_shape_map: &ValueShapeMap,
    value_types: &[mir::LocalNodeId<mir::Type>],
) -> Option<ValueShape> {
    match inst {
        mir::Instruction::Error => None,
        mir::Instruction::Const { destination, value } => {
            if matches!(value, mir::Constant::Null) {
                let ty = value_type_from_table(*destination, value_types)?;
                return types.value_shape_for_mir(ty, pointer_bytes);
            }

            shape_from_constant(value)
        }
        mir::Instruction::Binary {
            operator,
            left,
            right,
            ..
        } => {
            if operator.is_comparison() {
                return Some(ValueShape::Bool);
            }

            let left_shape = value_shape_map.get(*left);
            let right_shape = value_shape_map.get(*right);
            let input_shape = left_shape.or(right_shape);

            match (operator.is_float(), input_shape) {
                (true, Some(ValueShape::Float { format })) => Some(ValueShape::Float { format }),
                (false, Some(ValueShape::Bool))
                    if matches!(
                        operator,
                        mir::BinaryOperator::And
                            | mir::BinaryOperator::Or
                            | mir::BinaryOperator::Xor
                    ) =>
                {
                    Some(ValueShape::Bool)
                }
                (false, Some(ValueShape::Int { width, signed })) => {
                    Some(ValueShape::Int { width, signed })
                }
                _ => None,
            }
        }
        mir::Instruction::Unary { argument, .. } => value_shape_map.get(*argument),
        mir::Instruction::Cast { to_type, .. } => {
            types.value_shape_for_mir(*to_type, pointer_bytes)
        }
        mir::Instruction::Select { then_value, .. } => value_shape_map.get(*then_value),
        mir::Instruction::Call {
            destination,
            function,
            ..
        } => {
            destination.as_ref()?;
            let function = tree.get(*function);
            types.value_shape_for_mir(function.return_type, pointer_bytes)
        }
        mir::Instruction::CallVirtual {
            destination, call, ..
        }
        | mir::Instruction::CallDynamic {
            destination, call, ..
        }
        | mir::Instruction::CallIndirect {
            destination, call, ..
        } => {
            destination.as_ref()?;
            let signature = call.signature;
            let mir::Type::FunctionSignature { result, .. } = tree.get(signature) else {
                return None;
            };

            types.value_shape_for_mir(*result, pointer_bytes)
        }
        mir::Instruction::LocalGet { local, .. } => {
            let local = tree.get(*local);
            types.value_shape_for_mir(local.ty, pointer_bytes)
        }
        mir::Instruction::LocalAddr { result_type, .. } => {
            let mut shape = types.value_shape_for_mir(*result_type, pointer_bytes)?;
            let ValueShape::Pointer { address_space, .. } = &mut shape else {
                return None;
            };
            *address_space = AddressSpace::Frame;
            Some(shape)
        }
        mir::Instruction::GlobalAddr { result_type, .. } => {
            let mut shape = types.value_shape_for_mir(*result_type, pointer_bytes)?;
            let ValueShape::Pointer { address_space, .. } = &mut shape else {
                return None;
            };
            *address_space = AddressSpace::Static;
            Some(shape)
        }
        mir::Instruction::FunctionAddr { function, .. } => {
            let function = tree.get(*function);
            Some(ValueShape::FunctionPointer {
                result: types.type_id(function.return_type),
            })
        }
        mir::Instruction::FunctionBind { destination, .. } => {
            let ty = value_type_from_table(*destination, value_types)?;
            types.value_shape_for_mir(ty, pointer_bytes)
        }
        mir::Instruction::FunctionPointer { destination, .. }
        | mir::Instruction::FunctionEnvironment { destination, .. }
        | mir::Instruction::FunctionEnvironmentCurrent { destination } => {
            value_shape_map.get(*destination)
        }
        mir::Instruction::Load { result_type, .. } => {
            types.value_shape_for_mir(*result_type, pointer_bytes)
        }
        mir::Instruction::Struct { ty, .. }
        | mir::Instruction::Tuple { ty, .. }
        | mir::Instruction::Array { ty, .. } => types.value_shape_for_mir(*ty, pointer_bytes),
        mir::Instruction::FieldGet {
            aggregate: base,
            index,
            ..
        } => {
            let base_shape = value_shape_map.get(*base)?;
            shape_from_field(types, pointer_bytes, tree, base_shape, *index)
        }
        mir::Instruction::FieldSet {
            aggregate: base, ..
        } => value_shape_map.get(*base),
        mir::Instruction::FieldAddr {
            aggregate: base,
            result_type,
            ..
        } => {
            let source_shape = value_shape_map.get(*base)?;
            pointer_result_shape_from_source(types, pointer_bytes, *result_type, source_shape)
        }
        mir::Instruction::ElementAddr {
            array, result_type, ..
        } => {
            let source_shape = value_shape_map.get(*array)?;
            pointer_result_shape_from_source(types, pointer_bytes, *result_type, source_shape)
        }
        mir::Instruction::SliceView { result_type, .. } => {
            types.value_shape_for_mir(*result_type, pointer_bytes)
        }
        mir::Instruction::SliceLength { destination, .. }
        | mir::Instruction::DynamicPayload { destination, .. }
        | mir::Instruction::DynamicType { destination, .. }
        | mir::Instruction::VariantTag { destination, .. }
        | mir::Instruction::VariantPayload { destination, .. } => {
            let ty = value_type_from_table(*destination, value_types)?;
            types.value_shape_for_mir(ty, pointer_bytes)
        }
        mir::Instruction::TensorExtract { destination, .. } => {
            let ty = value_type_from_table(*destination, value_types)?;
            types.value_shape_for_mir(ty, pointer_bytes)
        }
        mir::Instruction::VectorSplat { .. }
        | mir::Instruction::VectorExtract { .. }
        | mir::Instruction::VectorInsert { .. }
        | mir::Instruction::VectorShuffle { .. }
        | mir::Instruction::VectorSelect { .. }
        | mir::Instruction::VectorReduce { .. }
        | mir::Instruction::VectorCompare { .. }
        | mir::Instruction::VectorConvert { .. }
        | mir::Instruction::TensorSplat { .. }
        | mir::Instruction::TensorLoad { .. }
        | mir::Instruction::TensorStore { .. }
        | mir::Instruction::TensorFill { .. }
        | mir::Instruction::TensorCopy { .. }
        | mir::Instruction::TensorReshape { .. }
        | mir::Instruction::TensorBroadcast { .. }
        | mir::Instruction::TensorTranspose { .. }
        | mir::Instruction::TensorCast { .. }
        | mir::Instruction::TensorView { .. }
        | mir::Instruction::TensorSlice { .. }
        | mir::Instruction::TensorPad { .. }
        | mir::Instruction::TensorConcat { .. }
        | mir::Instruction::TensorReduce { .. }
        | mir::Instruction::TensorIndexReduce { .. }
        | mir::Instruction::TensorDot { .. }
        | mir::Instruction::TensorConvolution { .. }
        | mir::Instruction::TensorGather { .. }
        | mir::Instruction::TensorScatter { .. }
        | mir::Instruction::TensorCompare { .. }
        | mir::Instruction::TensorSelect { .. }
        | mir::Instruction::TensorConvert { .. } => None,
        mir::Instruction::FrameAllocZeroed { result_type, .. }
        | mir::Instruction::FrameAllocUninit { result_type, .. } => {
            let mut shape = types.value_shape_for_mir(*result_type, pointer_bytes)?;
            let ValueShape::Pointer { address_space, .. } = &mut shape else {
                return None;
            };
            *address_space = AddressSpace::Stack;
            Some(shape)
        }
        mir::Instruction::NewZeroed { result_type, .. }
        | mir::Instruction::NewUninit { result_type, .. }
        | mir::Instruction::NewComplete { result_type, .. }
        | mir::Instruction::NewSliceZeroed { result_type, .. }
        | mir::Instruction::NewSliceUninit { result_type, .. }
        | mir::Instruction::AtomicLoad { result_type, .. } => {
            types.value_shape_for_mir(*result_type, pointer_bytes)
        }
        mir::Instruction::AtomicCompareExchange { destination, .. }
        | mir::Instruction::AtomicRmw { destination, .. } => {
            let ty = value_type_from_table(*destination, value_types)?;
            types.value_shape_for_mir(ty, pointer_bytes)
        }
        mir::Instruction::Intrinsic {
            intrinsic,
            arguments,
            ..
        } => infer_intrinsic_shape(
            types,
            pointer_bytes,
            tree,
            *intrinsic,
            *arguments,
            value_shape_map,
        ),
        mir::Instruction::LocalSet { .. }
        | mir::Instruction::Store { .. }
        | mir::Instruction::AtomicStore { .. }
        | mir::Instruction::AtomicFence { .. }
        | mir::Instruction::BarrierWrite { .. }
        | mir::Instruction::Free { .. }
        | mir::Instruction::Pin { .. }
        | mir::Instruction::Unpin { .. }
        | mir::Instruction::Assume { .. }
        | mir::Instruction::ProfileIncrement { .. }
        | mir::Instruction::ProfileValue { .. } => None,
    }
}

/// Infer the value shape for one intrinsic call.
fn infer_intrinsic_shape(
    types: &TypeTable,
    pointer_bytes: u8,
    tree: &mir::Tree,
    intrinsic: mir::Intrinsic,
    arguments: mir::ValueSlice,
    value_shape_map: &ValueShapeMap,
) -> Option<ValueShape> {
    let argument = tree.get_values(arguments);

    match intrinsic.result_type() {
        mir::IntrinsicResultType::Void => None,
        mir::IntrinsicResultType::Boolean => Some(ValueShape::Bool),
        mir::IntrinsicResultType::I32 => Some(ValueShape::Int {
            width: 32,
            signed: true,
        }),
        mir::IntrinsicResultType::Isize => Some(ValueShape::Int {
            width: u16::from(pointer_bytes) * 8,
            signed: true,
        }),
        mir::IntrinsicResultType::Usize => Some(ValueShape::Int {
            width: u16::from(pointer_bytes) * 8,
            signed: false,
        }),
        mir::IntrinsicResultType::SameAsArgument(index) => {
            let argument = argument.get(index as usize)?;
            value_shape_map.get(*argument)
        }
        mir::IntrinsicResultType::Pointee(index) => {
            let argument = argument.get(index as usize)?;
            let pointer_shape = value_shape_map.get(*argument)?;
            shape_from_pointer(types, pointer_bytes, pointer_shape)
        }
        mir::IntrinsicResultType::OverflowingArithmetic
        | mir::IntrinsicResultType::PointeeAndBool(_)
        | mir::IntrinsicResultType::TypeDescriptor
        | mir::IntrinsicResultType::Explicit => None,
    }
}

/// Get the shape for one constant value.
fn shape_from_constant(constant: &mir::Constant) -> Option<ValueShape> {
    match constant {
        mir::Constant::Null => None,
        mir::Constant::Boolean { .. } => Some(ValueShape::Bool),
        mir::Constant::Int {
            width, is_signed, ..
        } => Some(ValueShape::Int {
            width: *width,
            signed: *is_signed,
        }),
        mir::Constant::UInt { width, .. } => Some(ValueShape::Int {
            width: *width,
            signed: false,
        }),
        mir::Constant::Float { format, .. } => Some(ValueShape::Float { format: *format }),
        mir::Constant::Char { .. } => Some(ValueShape::Char),
    }
}

/// Resolve one pointee shape from one addressable value.
fn shape_from_pointer(
    types: &TypeTable,
    pointer_bytes: u8,
    shape: ValueShape,
) -> Option<ValueShape> {
    match shape {
        ValueShape::Pointer { pointee, .. } => types.value_shape(pointee, pointer_bytes),
        _ => None,
    }
}

/// Resolve the field shape for one payload value.
fn shape_from_field(
    types: &TypeTable,
    pointer_bytes: u8,
    tree: &mir::Tree,
    shape: ValueShape,
    index: u32,
) -> Option<ValueShape> {
    match shape {
        ValueShape::Array { element, .. } => types.value_shape(element, pointer_bytes),
        ValueShape::Aggregate { ty } => {
            shape_from_frame_field(types, pointer_bytes, tree, ty, index)
        }
        _ => None,
    }
}

/// Resolve the field shape for one frame-backed value.
fn shape_from_frame_field(
    types: &TypeTable,
    pointer_bytes: u8,
    tree: &mir::Tree,
    ty: destack_program::TypeId,
    index: u32,
) -> Option<ValueShape> {
    let ty = types.mir_type_id(ty)?;

    match tree.get(ty) {
        mir::Type::Struct { fields, copy: _ } => {
            let field = fields.get(index as usize)?;
            let field = tree.get(*field);

            types.value_shape_for_mir(field.ty, pointer_bytes)
        }
        mir::Type::Tuple { elements, copy: _ } => {
            let field = elements.get(index as usize)?;
            types.value_shape_for_mir(*field, pointer_bytes)
        }
        mir::Type::FixedArray { element, .. } => types.value_shape_for_mir(*element, pointer_bytes),
        _ => None,
    }
}

/// Rebuild one address-producing result shape from the MIR address space.
fn pointer_result_shape_from_source(
    types: &TypeTable,
    pointer_bytes: u8,
    result_type: mir::LocalNodeId<mir::Type>,
    source_shape: ValueShape,
) -> Option<ValueShape> {
    let ValueShape::Pointer { address_space, .. } = source_shape else {
        return types.value_shape_for_mir(result_type, pointer_bytes);
    };

    let ValueShape::Pointer {
        pointee, reference, ..
    } = types.value_shape_for_mir(result_type, pointer_bytes)?
    else {
        return None;
    };

    Some(ValueShape::Pointer {
        pointee,
        address_space,
        reference,
    })
}
