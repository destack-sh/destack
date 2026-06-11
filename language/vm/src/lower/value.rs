use destack_mir as mir;

use crate::{Error, ReferenceMeta};

use crate::program::{
    AddressSpace, ValueShape, address_space_from_reference, repr_type, value_shape_from_type,
};

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
pub(super) fn value_type_for_value(
    value: mir::Value,
    value_types: &[mir::LocalNodeId<mir::Type>],
) -> Option<mir::LocalNodeId<mir::Type>> {
    value_types.get(value.0 as usize).copied()
}

/// One value shape builder for one function.
pub(super) struct ValueShapeMapBuilder<'a> {
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
        tree: &'a mir::Tree,
        func: &'a mir::Function,
        mir_block: &'a [mir::LocalNodeId<mir::Block>],
        value_type: &'a [mir::LocalNodeId<mir::Type>],
        value_count: usize,
    ) -> Self {
        Self {
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
            if let Some(shape) = value_shape_from_type(self.tree, *ty) {
                self.value_shape_map.set(value, shape);
            }
        }

        // seed function parameter shapes
        for param in &self.func.parameters {
            let Some(value) = param.value.value() else {
                continue;
            };
            let Some(ty) = param.ty.ty() else {
                continue;
            };

            if let Some(shape) = value_shape_from_type(self.tree, ty) {
                self.value_shape_map.set(value, shape);
            }
        }

        // seed block parameter shapes
        for block_id in self.mir_block {
            let block = self.tree.get(*block_id);
            for param in &block.parameters {
                let Some(value) = param.value.value() else {
                    continue;
                };
                let Some(ty) = param.ty.ty() else {
                    continue;
                };
                let Some(shape) = value_shape_from_type(self.tree, ty) else {
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
                    let Some(destination) = destination.value() else {
                        continue;
                    };
                    let Some(shape) = infer_instruction_shape(
                        self.tree,
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
        Some(ValueShape::FrameBytes { .. } | ValueShape::Array { .. }) => Ok(AddressSpace::Frame),
        _ => Err(Error::invalid_pointer_type(format!("{value:?}"))),
    }
}

/// Get reference metadata for a type when available.
pub(super) fn reference_meta_for_type(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> ReferenceMeta {
    match value_shape_from_type(tree, ty) {
        Some(ValueShape::Pointer { reference, .. }) => reference,
        _ => ReferenceMeta::NONE,
    }
}

/// Resolve one heap pointee type from a pointer value when available.
pub(super) fn heap_pointee_type_for_storage_id(
    value_shape_map: &ValueShapeMap,
    value: mir::Value,
) -> Option<mir::LocalNodeId<mir::Type>> {
    match value_shape_map.get(value) {
        Some(ValueShape::Pointer {
            pointee,
            address_space: AddressSpace::Local | AddressSpace::Shared,
            ..
        }) => Some(pointee),
        _ => None,
    }
}

/// Resolve one raw pointee type from a pointer value when available.
pub(super) fn raw_pointee_type_for_storage_id(
    value_shape_map: &ValueShapeMap,
    value: mir::Value,
) -> Option<mir::LocalNodeId<mir::Type>> {
    match value_shape_map.get(value) {
        Some(ValueShape::Pointer {
            pointee,
            address_space:
                AddressSpace::Raw | AddressSpace::Stack | AddressSpace::Frame | AddressSpace::Static,
            ..
        }) => Some(pointee),
        _ => None,
    }
}

/// Resolve one heap pointee type from a value type when available.
pub(super) fn heap_pointee_type_for_value(
    tree: &mir::Tree,
    value_types: &[mir::LocalNodeId<mir::Type>],
    value: mir::Value,
) -> Option<mir::LocalNodeId<mir::Type>> {
    let ty = repr_type(tree, value_type_for_value(value, value_types)?);

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
            pointee.ty()
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
    let ty = repr_type(tree, value_type_for_value(value, value_types)?);

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
            pointee.ty()
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
                is_changed |= propagate_allocation_target_edge(tree, value_shape_map, success);
                is_changed |= propagate_target_edge(tree, value_shape_map, failure);
            }
            mir::Terminator::NewSliceZeroedTry {
                success, failure, ..
            }
            | mir::Terminator::NewSliceUninitTry {
                success, failure, ..
            } => {
                is_changed |= propagate_allocation_target_edge(tree, value_shape_map, success);
                is_changed |= propagate_target_edge(tree, value_shape_map, failure);
            }
            mir::Terminator::Switch { default, cases, .. } => {
                is_changed |= propagate_target_edge(tree, value_shape_map, default);

                for case in cases {
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
            | mir::Terminator::ResumeUnwind
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
    tree: &mir::Tree,
    value_shape_map: &mut ValueShapeMap,
    target: &mir::BlockTarget,
) -> bool {
    let Some(target_block) = target.block.block() else {
        return false;
    };

    let mut is_changed = false;
    let block = tree.get(target_block);
    let Some(result_parameter) = block.parameters.first() else {
        return false;
    };
    let Some(result_value) = result_parameter.value.value() else {
        return false;
    };
    let Some(result_type) = result_parameter.ty.ty() else {
        return false;
    };
    let result_shape = value_shape_from_type(tree, result_type);
    if value_shape_map.get(result_value) != result_shape {
        value_shape_map.replace(result_value, result_shape);
        is_changed = true;
    }

    let arguments = target
        .arguments
        .iter()
        .map(|argument| argument.value())
        .collect::<Option<Vec<_>>>();
    let Some(arguments) = arguments else {
        return is_changed;
    };
    let parameters = block.parameters.iter().skip(1);

    for (parameter, argument) in parameters.zip(arguments.iter()) {
        let Some(argument_shape) = value_shape_map.get(*argument) else {
            continue;
        };
        let Some(parameter_value) = parameter.value.value() else {
            continue;
        };
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
    let Some(target_block) = target.block.block() else {
        return false;
    };

    let arguments = target
        .arguments
        .iter()
        .map(|argument| argument.value())
        .collect::<Option<Vec<_>>>();
    let Some(arguments) = arguments else {
        return false;
    };

    propagate_target_shape(tree, value_shape_map, target_block, &arguments)
}

/// Update one target block from incoming argument shapes.
fn propagate_target_shape(
    tree: &mir::Tree,
    value_shape_map: &mut ValueShapeMap,
    target: mir::LocalNodeId<mir::Block>,
    arguments: &[mir::Value],
) -> bool {
    let mut is_changed = false;
    let target_block = tree.get(target);

    for (parameter, argument) in target_block.parameters.iter().zip(arguments.iter()) {
        let Some(argument_shape) = value_shape_map.get(*argument) else {
            continue;
        };
        let Some(parameter_value) = parameter.value.value() else {
            continue;
        };
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
    inst: &mir::Instruction,
    value_shape_map: &ValueShapeMap,
    value_types: &[mir::LocalNodeId<mir::Type>],
) -> Option<ValueShape> {
    match inst {
        mir::Instruction::Error => None,
        mir::Instruction::Const { destination, value } => {
            if matches!(value, mir::Constant::Null) {
                let destination = destination.value()?;
                let ty = value_type_for_value(destination, value_types)?;
                return value_shape_from_type(tree, ty);
            }

            shape_from_constant(value)
        }
        mir::Instruction::Binary {
            operator,
            left,
            right,
            ..
        } => {
            let left = left.value()?;
            let right = right.value()?;
            if operator.is_comparison() {
                return Some(ValueShape::Bool);
            }

            let left_shape = value_shape_map.get(left);
            let right_shape = value_shape_map.get(right);
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
        mir::Instruction::Unary { argument, .. } => value_shape_map.get(argument.value()?),
        mir::Instruction::Cast { to_type, .. } => {
            let to_type = to_type.ty()?;

            value_shape_from_type(tree, to_type)
        }
        mir::Instruction::Select { then_value, .. } => value_shape_map.get(then_value.value()?),
        mir::Instruction::Call {
            destination,
            function,
            ..
        } => {
            (*destination)?.value()?;
            let function = tree.get(function.function()?);
            value_shape_from_type(tree, function.return_type.ty()?)
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
            (*destination)?.value()?;
            let signature = call.signature.ty()?;
            let mir::Type::FunctionSignature { result, .. } = tree.get(signature) else {
                return None;
            };

            value_shape_from_type(tree, result.ty()?)
        }
        mir::Instruction::LocalGet { local, .. } => {
            let local = tree.get(local.local()?);
            value_shape_from_type(tree, local.ty.ty()?)
        }
        mir::Instruction::LocalAddr { result_type, .. } => {
            let mut shape = value_shape_from_type(tree, result_type.ty()?)?;
            let ValueShape::Pointer { address_space, .. } = &mut shape else {
                return None;
            };
            *address_space = AddressSpace::Frame;
            Some(shape)
        }
        mir::Instruction::GlobalAddr { result_type, .. } => {
            let mut shape = value_shape_from_type(tree, result_type.ty()?)?;
            let ValueShape::Pointer { address_space, .. } = &mut shape else {
                return None;
            };
            *address_space = AddressSpace::Static;
            Some(shape)
        }
        mir::Instruction::FunctionAddr { function, .. } => {
            let function = tree.get(function.function()?);
            Some(ValueShape::FunctionPointer {
                result: function.return_type.ty()?,
            })
        }
        mir::Instruction::ClosureBind { destination, .. } => {
            let destination = destination.value()?;
            let ty = value_type_for_value(destination, value_types)?;
            value_shape_from_type(tree, ty)
        }
        mir::Instruction::ClosureEnvironment { destination } => {
            value_shape_map.get(destination.value()?)
        }
        mir::Instruction::Load { result_type, .. } => {
            value_shape_from_type(tree, result_type.ty()?)
        }
        mir::Instruction::FieldGet {
            aggregate: base,
            index,
            ..
        } => {
            let base_shape = value_shape_map.get(base.value()?)?;
            shape_from_field(tree, base_shape, *index)
        }
        mir::Instruction::FieldAddr {
            aggregate: base,
            result_type,
            ..
        } => {
            let source_shape = value_shape_map.get(base.value()?)?;
            pointer_result_shape_from_source(tree, result_type.ty()?, source_shape)
        }
        mir::Instruction::FieldSet {
            aggregate: base, ..
        } => value_shape_map.get(base.value()?),
        mir::Instruction::ElementGet { array, .. } => {
            let array_shape = value_shape_map.get(array.value()?)?;
            shape_from_element(tree, array_shape)
        }
        mir::Instruction::ElementAddr {
            array, result_type, ..
        } => {
            let source_shape = value_shape_map.get(array.value()?)?;
            pointer_result_shape_from_source(tree, result_type.ty()?, source_shape)
        }
        mir::Instruction::ElementSet { array, .. } => value_shape_map.get(array.value()?),
        mir::Instruction::Struct { ty, .. }
        | mir::Instruction::Tuple { ty, .. }
        | mir::Instruction::Array { ty, .. } => value_shape_from_type(tree, ty.ty()?),
        mir::Instruction::Slice { result_type, .. } => {
            value_shape_from_type(tree, result_type.ty()?)
        }
        mir::Instruction::TensorExtract { destination, .. } => {
            let destination = destination.value()?;
            let ty = value_type_for_value(destination, value_types)?;
            value_shape_from_type(tree, ty)
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
            let mut shape = value_shape_from_type(tree, result_type.ty()?)?;
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
            value_shape_from_type(tree, result_type.ty()?)
        }
        mir::Instruction::AtomicCompareExchange { destination, .. }
        | mir::Instruction::AtomicRmw { destination, .. } => {
            let destination = destination.value()?;
            let ty = value_type_for_value(destination, value_types)?;
            value_shape_from_type(tree, ty)
        }
        mir::Instruction::Intrinsic {
            intrinsic,
            arguments,
            ..
        } => infer_intrinsic_shape(tree, *intrinsic, *arguments, value_shape_map),
        mir::Instruction::LocalSet { .. }
        | mir::Instruction::Store { .. }
        | mir::Instruction::AtomicStore { .. }
        | mir::Instruction::AtomicFence { .. }
        | mir::Instruction::BarrierWrite { .. }
        | mir::Instruction::Free { .. }
        | mir::Instruction::Drop { .. }
        | mir::Instruction::Pin { .. }
        | mir::Instruction::Unpin { .. }
        | mir::Instruction::Assume { .. }
        | mir::Instruction::ProfileIncrement { .. }
        | mir::Instruction::ProfileValue { .. } => None,
    }
}

/// Infer the value shape for one intrinsic call.
fn infer_intrinsic_shape(
    tree: &mir::Tree,
    intrinsic: mir::Intrinsic,
    arguments: mir::ArgumentSlice,
    value_shape_map: &ValueShapeMap,
) -> Option<ValueShape> {
    let argument = tree.get_arguments(arguments);

    match intrinsic.result_type() {
        mir::IntrinsicResultType::Void => None,
        mir::IntrinsicResultType::Boolean => Some(ValueShape::Bool),
        mir::IntrinsicResultType::I32 => Some(ValueShape::Int {
            width: 32,
            signed: true,
        }),
        mir::IntrinsicResultType::Isize => Some(ValueShape::Int {
            width: usize::BITS as u16,
            signed: true,
        }),
        mir::IntrinsicResultType::Usize => Some(ValueShape::Int {
            width: usize::BITS as u16,
            signed: false,
        }),
        mir::IntrinsicResultType::SameAsArgument(index) => {
            let argument = argument.get(index as usize)?;
            value_shape_map.get(argument.value()?)
        }
        mir::IntrinsicResultType::Pointee(index) => {
            let argument = argument.get(index as usize)?;
            let pointer_shape = value_shape_map.get(argument.value()?)?;
            shape_from_pointer(tree, pointer_shape)
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
fn shape_from_pointer(tree: &mir::Tree, shape: ValueShape) -> Option<ValueShape> {
    match shape {
        ValueShape::Pointer { pointee, .. } => value_shape_from_type(tree, pointee),
        _ => None,
    }
}

/// Resolve the field shape for one payload value.
fn shape_from_field(tree: &mir::Tree, shape: ValueShape, index: u32) -> Option<ValueShape> {
    let ValueShape::FrameBytes { ty } = shape else {
        return None;
    };

    match tree.get(ty) {
        mir::Type::Struct { fields, copy: _ } => {
            let field = fields.get(index as usize)?;
            let field = tree.get(*field);
            value_shape_from_type(tree, field.ty.ty()?)
        }
        mir::Type::Tuple { elements, copy: _ } => {
            let field = elements.get(index as usize)?;
            value_shape_from_type(tree, field.ty()?)
        }
        _ => None,
    }
}

/// Resolve the element shape for one array value.
fn shape_from_element(tree: &mir::Tree, shape: ValueShape) -> Option<ValueShape> {
    match shape {
        ValueShape::Array { element, .. } => value_shape_from_type(tree, element),
        ValueShape::FrameBytes { ty } => match tree.get(ty) {
            mir::Type::Array { element, .. } => value_shape_from_type(tree, element.ty()?),
            _ => None,
        },
        _ => None,
    }
}

/// Rebuild one address-producing result shape from the MIR address space.
fn pointer_result_shape_from_source(
    tree: &mir::Tree,
    result_type: mir::LocalNodeId<mir::Type>,
    source_shape: ValueShape,
) -> Option<ValueShape> {
    let ValueShape::Pointer { address_space, .. } = source_shape else {
        return value_shape_from_type(tree, result_type);
    };

    let ValueShape::Pointer {
        pointee, reference, ..
    } = value_shape_from_type(tree, result_type)?
    else {
        return None;
    };

    Some(ValueShape::Pointer {
        pointee,
        address_space,
        reference,
    })
}
