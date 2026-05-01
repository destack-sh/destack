use destack_mir as mir;

use crate::ReferenceMeta;

use crate::program::{
    PointerClass, ValueLayout, pointer_class_from_reference, repr_type, value_layout_from_type,
};

/// One dense map from SSA value id to lowered layout.
pub(super) struct ValueLayoutMap {
    /// Layout for each SSA value id.
    layouts: Vec<Option<ValueLayout>>,
}

impl ValueLayoutMap {
    /// Create a new value layout table.
    pub(super) fn new(value_count: usize) -> Self {
        Self {
            layouts: vec![None; value_count],
        }
    }

    /// Get the layout for a value.
    pub(super) fn get(&self, value: mir::Value) -> Option<ValueLayout> {
        self.layouts
            .get(value.0 as usize)
            .and_then(|layout| *layout)
    }

    /// Set the layout for a value.
    pub(super) fn set(&mut self, value: mir::Value, layout: ValueLayout) {
        if let Some(entry) = self.layouts.get_mut(value.0 as usize) {
            *entry = Some(layout);
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

/// One value layout builder for one function.
pub(super) struct ValueLayoutMapBuilder<'a> {
    /// The MIR node tree.
    tree: &'a mir::Tree,
    /// The MIR function being lowered.
    func: &'a mir::Function,
    /// The MIR blocks in lowered order.
    mir_block: &'a [mir::LocalNodeId<mir::Block>],
    /// The lowered value types by SSA value id.
    value_type: &'a [mir::LocalNodeId<mir::Type>],
    /// The lowered value layout map.
    value_layout_map: ValueLayoutMap,
}

impl<'a> ValueLayoutMapBuilder<'a> {
    /// Create one value layout builder.
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
            value_layout_map: ValueLayoutMap::new(value_count),
        }
    }

    /// Build the lowered value layout map.
    pub(super) fn build(mut self) -> ValueLayoutMap {
        // seed explicit value types
        for (index, ty) in self.value_type.iter().enumerate() {
            let value = mir::Value(index as u32);
            self.value_layout_map
                .set(value, value_layout_from_type(self.tree, *ty));
        }

        // seed function parameter layouts
        for param in &self.func.parameters {
            let Some(value) = param.value.value() else {
                continue;
            };
            let Some(ty) = param.ty.ty() else {
                continue;
            };

            self.value_layout_map
                .set(value, value_layout_from_type(self.tree, ty));
        }

        // seed block parameter layouts
        for block_id in self.mir_block {
            let block = self.tree.get(*block_id);
            for param in &block.parameters {
                let Some(value) = param.value.value() else {
                    continue;
                };
                let Some(ty) = param.ty.ty() else {
                    continue;
                };
                let layout = value_layout_from_type(self.tree, ty);
                let block_layout = layout_for_block_parameter(layout);
                let next_layout = match self.value_layout_map.get(value) {
                    Some(existing) => merge_block_parameter_layout(existing, block_layout),
                    None => block_layout,
                };

                self.value_layout_map.set(value, next_layout);
            }
        }

        // iteratively infer instruction result layouts
        let mut is_changed = true;
        while is_changed {
            // reset iteration state
            is_changed = false;

            // propagate block parameter layouts from control flow edges
            if propagate_block_parameter_layouts(
                self.tree,
                self.mir_block,
                &mut self.value_layout_map,
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
                    let Some(layout) = infer_instruction_layout(
                        self.tree,
                        inst,
                        &self.value_layout_map,
                        self.value_type,
                    ) else {
                        continue;
                    };
                    let next_layout = match self.value_layout_map.get(destination) {
                        Some(existing) => merge_block_parameter_layout(existing, layout),
                        None => layout,
                    };

                    if self.value_layout_map.get(destination) != Some(next_layout) {
                        self.value_layout_map.set(destination, next_layout);
                        is_changed = true;
                    }
                }
            }
        }

        self.value_layout_map
    }
}

/// Get reference metadata for a value when available.
pub(super) fn reference_meta_for_value(
    value_layout_map: &ValueLayoutMap,
    value: mir::Value,
) -> ReferenceMeta {
    match value_layout_map.get(value) {
        Some(ValueLayout::Pointer { reference, .. }) => reference,
        _ => ReferenceMeta::NONE,
    }
}

/// Get the pointer class for a value when available.
pub(super) fn pointer_class_for_value(
    value_layout_map: &ValueLayoutMap,
    value: mir::Value,
) -> PointerClass {
    match value_layout_map.get(value) {
        Some(ValueLayout::Pointer { pointer_class, .. }) => pointer_class,
        Some(ValueLayout::FrameBytes { .. } | ValueLayout::Array { .. }) => PointerClass::Frame,
        _ => PointerClass::Unknown,
    }
}

/// Get reference metadata for a type when available.
pub(super) fn reference_meta_for_type(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> ReferenceMeta {
    match value_layout_from_type(tree, ty) {
        ValueLayout::Pointer { reference, .. } => reference,
        _ => ReferenceMeta::NONE,
    }
}

/// Resolve one heap pointee type from a pointer value when available.
pub(super) fn heap_pointee_type_for_value_layout(
    value_layout_map: &ValueLayoutMap,
    value: mir::Value,
) -> Option<mir::LocalNodeId<mir::Type>> {
    match value_layout_map.get(value) {
        Some(ValueLayout::Pointer {
            pointee,
            pointer_class:
                PointerClass::Heap
                | PointerClass::SharedHeap
                | PointerClass::HeapAddress
                | PointerClass::SharedHeapAddress,
            ..
        }) => Some(pointee),
        _ => None,
    }
}

/// Resolve one raw pointee type from a pointer value when available.
pub(super) fn raw_pointee_type_for_value_layout(
    value_layout_map: &ValueLayoutMap,
    value: mir::Value,
) -> Option<mir::LocalNodeId<mir::Type>> {
    match value_layout_map.get(value) {
        Some(ValueLayout::Pointer {
            pointee,
            pointer_class:
                PointerClass::Raw
                | PointerClass::SharedRaw
                | PointerClass::Stack
                | PointerClass::Frame
                | PointerClass::Static,
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
                | mir::ReferenceKind::Owned
                | mir::ReferenceKind::Borrowed),
            address_space,
            pointee,
            ..
        } if matches!(
            pointer_class_from_reference(address_space.clone(), *kind),
            PointerClass::Heap
                | PointerClass::SharedHeap
                | PointerClass::HeapAddress
                | PointerClass::SharedHeapAddress
        ) =>
        {
            pointee.ty()
        }
        mir::Type::TensorView {
            kind:
                kind @ (mir::ReferenceKind::Managed
                | mir::ReferenceKind::Owned
                | mir::ReferenceKind::Borrowed),
            address_space,
            element,
            ..
        } if matches!(
            pointer_class_from_reference(address_space.clone(), *kind),
            PointerClass::Heap
                | PointerClass::SharedHeap
                | PointerClass::HeapAddress
                | PointerClass::SharedHeapAddress
        ) =>
        {
            element.ty()
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
            address_space,
            pointee,
            ..
        } if matches!(
            pointer_class_from_reference(address_space.clone(), *kind),
            PointerClass::Raw | PointerClass::Stack | PointerClass::Frame
        ) =>
        {
            pointee.ty()
        }
        mir::Type::TensorView {
            kind,
            address_space,
            element,
            ..
        } if matches!(
            pointer_class_from_reference(address_space.clone(), *kind),
            PointerClass::Raw | PointerClass::Stack | PointerClass::Frame
        ) =>
        {
            element.ty()
        }
        _ => None,
    }
}

/// Normalize one block parameter layout.
fn layout_for_block_parameter(layout: ValueLayout) -> ValueLayout {
    match layout {
        ValueLayout::Pointer {
            pointee, reference, ..
        } => ValueLayout::Pointer {
            pointee,
            pointer_class: PointerClass::Unknown,
            reference,
        },
        _ => layout,
    }
}

/// Merge pointer classes when propagating block parameter layouts.
fn merge_pointer_class(existing: PointerClass, incoming: PointerClass) -> PointerClass {
    match (existing, incoming) {
        (PointerClass::Unknown, other) => other,
        (other, PointerClass::Unknown) => other,
        (left, right) if left == right => left,
        _ => PointerClass::Unknown,
    }
}

/// Merge one block parameter layout with one incoming argument layout.
fn merge_block_parameter_layout(existing: ValueLayout, incoming: ValueLayout) -> ValueLayout {
    match (existing, incoming) {
        (
            ValueLayout::Pointer {
                pointee,
                pointer_class,
                reference,
            },
            ValueLayout::Pointer {
                pointer_class: incoming_pointer_class,
                ..
            },
        ) => ValueLayout::Pointer {
            pointee,
            pointer_class: merge_pointer_class(pointer_class, incoming_pointer_class),
            reference,
        },
        (ValueLayout::Unknown, other) => other,
        (other, ValueLayout::Unknown) => other,
        (other, _) => other,
    }
}

/// Propagate value layouts into block parameters from control flow edges.
fn propagate_block_parameter_layouts(
    tree: &mir::Tree,
    mir_blocks: &[mir::LocalNodeId<mir::Block>],
    value_layout_map: &mut ValueLayoutMap,
) -> bool {
    let mut is_changed = false;

    // walk every terminator edge
    for block_id in mir_blocks {
        let block = tree.get(*block_id);
        let terminator = tree.get(block.terminator);

        match terminator {
            mir::Terminator::Jump { target } => {
                is_changed |= propagate_target_edge(tree, value_layout_map, target);
            }
            mir::Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                is_changed |= propagate_target_edge(tree, value_layout_map, then_target);
                is_changed |= propagate_target_edge(tree, value_layout_map, else_target);
            }
            mir::Terminator::Check {
                success, failure, ..
            } => {
                is_changed |= propagate_target_edge(tree, value_layout_map, success);
                is_changed |= propagate_target_edge(tree, value_layout_map, failure);
            }
            mir::Terminator::Switch { default, cases, .. } => {
                is_changed |= propagate_target_edge(tree, value_layout_map, default);

                for case in cases {
                    is_changed |= propagate_target_edge(tree, value_layout_map, &case.target);
                }
            }
            mir::Terminator::Yield { resume, .. } => {
                is_changed |= propagate_target_edge(tree, value_layout_map, resume);
            }
            mir::Terminator::Invoke {
                normal_target,
                unwind_target,
                ..
            }
            | mir::Terminator::InvokeIndirect {
                normal_target,
                unwind_target,
                ..
            }
            | mir::Terminator::InvokeVirtual {
                normal_target,
                unwind_target,
                ..
            }
            | mir::Terminator::InvokeInterface {
                normal_target,
                unwind_target,
                ..
            } => {
                is_changed |= propagate_target_edge(tree, value_layout_map, normal_target);
                is_changed |= propagate_target_edge(tree, value_layout_map, unwind_target);
            }
            mir::Terminator::Error => {}
            mir::Terminator::Return { .. }
            | mir::Terminator::Throw { .. }
            | mir::Terminator::Trap { .. }
            | mir::Terminator::Unreachable
            | mir::Terminator::TailCall { .. }
            | mir::Terminator::TailCallIndirect { .. }
            | mir::Terminator::TailCallVirtual { .. }
            | mir::Terminator::TailCallInterface { .. } => {}
        }
    }

    is_changed
}

/// Update one target block from one control flow edge.
fn propagate_target_edge(
    tree: &mir::Tree,
    value_layout_map: &mut ValueLayoutMap,
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

    propagate_target_layout(tree, value_layout_map, target_block, &arguments)
}

/// Update one target block from incoming argument layouts.
fn propagate_target_layout(
    tree: &mir::Tree,
    value_layout_map: &mut ValueLayoutMap,
    target: mir::LocalNodeId<mir::Block>,
    arguments: &[mir::Value],
) -> bool {
    let mut is_changed = false;
    let target_block = tree.get(target);

    for (parameter, argument) in target_block.parameters.iter().zip(arguments.iter()) {
        let Some(argument_layout) = value_layout_map.get(*argument) else {
            continue;
        };
        let Some(parameter_value) = parameter.value.value() else {
            continue;
        };
        let existing = value_layout_map.get(parameter_value);
        let next_layout = match existing {
            Some(layout) => merge_block_parameter_layout(layout, argument_layout),
            None => argument_layout,
        };

        if existing != Some(next_layout) {
            value_layout_map.set(parameter_value, next_layout);
            is_changed = true;
        }
    }

    is_changed
}

/// Infer the value layout for a MIR instruction.
fn infer_instruction_layout(
    tree: &mir::Tree,
    inst: &mir::Instruction,
    value_layout_map: &ValueLayoutMap,
    value_types: &[mir::LocalNodeId<mir::Type>],
) -> Option<ValueLayout> {
    match inst {
        mir::Instruction::Error => None,
        mir::Instruction::Const { destination, value } => {
            if matches!(value, mir::Constant::Null) {
                let destination = destination.value()?;
                let ty = value_type_for_value(destination, value_types)?;
                return Some(value_layout_from_type(tree, ty));
            }

            Some(layout_from_constant(value))
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
                return Some(ValueLayout::Bool);
            }

            let left_layout = value_layout_map.get(left);
            let right_layout = value_layout_map.get(right);
            let operand_layout = left_layout.or(right_layout);

            match (operator.is_float(), operand_layout) {
                (true, Some(ValueLayout::Float { width })) => Some(ValueLayout::Float { width }),
                (false, Some(ValueLayout::Bool))
                    if matches!(
                        operator,
                        mir::BinaryOperator::And
                            | mir::BinaryOperator::Or
                            | mir::BinaryOperator::Xor
                    ) =>
                {
                    Some(ValueLayout::Bool)
                }
                (false, Some(ValueLayout::Int { width, signed })) => {
                    Some(ValueLayout::Int { width, signed })
                }
                _ => None,
            }
        }
        mir::Instruction::Unary { argument, .. } => value_layout_map.get(argument.value()?),
        mir::Instruction::Cast {
            argument, to_type, ..
        } => {
            let to_type = to_type.ty()?;
            let result_layout = value_layout_from_type(tree, to_type);
            let argument_layout = value_layout_map.get(argument.value()?);

            match (result_layout, argument_layout) {
                (
                    ValueLayout::Pointer {
                        pointee,
                        pointer_class: PointerClass::Unknown,
                        reference,
                    },
                    Some(ValueLayout::Pointer { pointer_class, .. }),
                ) => Some(ValueLayout::Pointer {
                    pointee,
                    pointer_class,
                    reference,
                }),
                (result_layout, _) => Some(result_layout),
            }
        }
        mir::Instruction::Select { then_value, .. } => value_layout_map.get(then_value.value()?),
        mir::Instruction::Call {
            destination,
            function,
            ..
        } => {
            (*destination)?.value()?;
            let function = tree.get(function.function()?);
            Some(value_layout_from_type(tree, function.return_type.ty()?))
        }
        mir::Instruction::CallVirtual {
            destination, call, ..
        }
        | mir::Instruction::CallInterface {
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

            Some(value_layout_from_type(tree, result.ty()?))
        }
        mir::Instruction::LocalGet { local, .. } => {
            let local = tree.get(local.local()?);
            Some(value_layout_from_type(tree, local.ty.ty()?))
        }
        mir::Instruction::LocalAddr { result_type, .. } => {
            let mut layout = value_layout_from_type(tree, result_type.ty()?);
            let ValueLayout::Pointer { pointer_class, .. } = &mut layout else {
                return None;
            };
            *pointer_class = PointerClass::Frame;
            Some(layout)
        }
        mir::Instruction::GlobalAddr { result_type, .. } => {
            let mut layout = value_layout_from_type(tree, result_type.ty()?);
            let ValueLayout::Pointer { pointer_class, .. } = &mut layout else {
                return None;
            };
            *pointer_class = PointerClass::Static;
            Some(layout)
        }
        mir::Instruction::FunctionAddr { function, .. } => {
            let function = tree.get(function.function()?);
            Some(ValueLayout::FunctionPointer {
                result: function.return_type.ty()?,
            })
        }
        mir::Instruction::CallableBind { destination, .. } => {
            let destination = destination.value()?;
            let ty = value_type_for_value(destination, value_types)?;
            Some(value_layout_from_type(tree, ty))
        }
        mir::Instruction::CallableEnvironment { destination } => {
            value_layout_map.get(destination.value()?)
        }
        mir::Instruction::Load { result_type, .. } => {
            Some(value_layout_from_type(tree, result_type.ty()?))
        }
        mir::Instruction::FieldGet {
            aggregate: base,
            index,
            ..
        } => {
            let base_layout = value_layout_map.get(base.value()?)?;
            layout_from_field(tree, base_layout, *index)
        }
        mir::Instruction::FieldAddr {
            aggregate: base,
            result_type,
            ..
        } => {
            let source_layout = value_layout_map.get(base.value()?)?;
            pointer_result_layout_from_source(tree, result_type.ty()?, source_layout)
        }
        mir::Instruction::FieldSet {
            aggregate: base, ..
        } => value_layout_map.get(base.value()?),
        mir::Instruction::ElementGet { array, .. } => {
            let array_layout = value_layout_map.get(array.value()?)?;
            layout_from_element(tree, array_layout)
        }
        mir::Instruction::ElementAddr {
            array, result_type, ..
        } => {
            let source_layout = value_layout_map.get(array.value()?)?;
            pointer_result_layout_from_source(tree, result_type.ty()?, source_layout)
        }
        mir::Instruction::ElementSet { array, .. } => value_layout_map.get(array.value()?),
        mir::Instruction::Struct { ty, .. }
        | mir::Instruction::Tuple { ty, .. }
        | mir::Instruction::Array { ty, .. } => Some(value_layout_from_type(tree, ty.ty()?)),
        mir::Instruction::TensorExtract { destination, .. } => {
            let destination = destination.value()?;
            let ty = value_type_for_value(destination, value_types)?;
            Some(value_layout_from_type(tree, ty))
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
        | mir::Instruction::TensorDot { .. }
        | mir::Instruction::TensorConvolution { .. }
        | mir::Instruction::TensorGather { .. }
        | mir::Instruction::TensorScatter { .. }
        | mir::Instruction::TensorCompare { .. }
        | mir::Instruction::TensorSelect { .. }
        | mir::Instruction::TensorConvert { .. } => None,
        mir::Instruction::New { result_type, .. }
        | mir::Instruction::NewSlice { result_type, .. }
        | mir::Instruction::RawAlloc { result_type, .. }
        | mir::Instruction::StackAlloc { result_type, .. }
        | mir::Instruction::AtomicLoad { result_type, .. } => {
            Some(value_layout_from_type(tree, result_type.ty()?))
        }
        mir::Instruction::AtomicCompareExchange { destination, .. }
        | mir::Instruction::AtomicRmw { destination, .. } => {
            let destination = destination.value()?;
            let ty = value_type_for_value(destination, value_types)?;
            Some(value_layout_from_type(tree, ty))
        }
        mir::Instruction::Intrinsic {
            intrinsic,
            arguments,
            ..
        } => infer_intrinsic_layout(tree, *intrinsic, *arguments, value_layout_map),
        mir::Instruction::LocalSet { .. }
        | mir::Instruction::Store { .. }
        | mir::Instruction::AtomicStore { .. }
        | mir::Instruction::AtomicFence { .. }
        | mir::Instruction::BarrierWrite { .. }
        | mir::Instruction::RawFree { .. }
        | mir::Instruction::Dispose { .. }
        | mir::Instruction::AsyncDispose { .. }
        | mir::Instruction::Pin { .. }
        | mir::Instruction::Unpin { .. }
        | mir::Instruction::Drop { .. }
        | mir::Instruction::Assume { .. } => None,
    }
}

/// Infer the value layout for one intrinsic call.
fn infer_intrinsic_layout(
    tree: &mir::Tree,
    intrinsic: mir::Intrinsic,
    arguments: mir::ArgumentSlice,
    value_layout_map: &ValueLayoutMap,
) -> Option<ValueLayout> {
    let argument = tree.get_arguments(arguments);

    match intrinsic.result_type() {
        mir::IntrinsicResultType::Void => None,
        mir::IntrinsicResultType::Boolean => Some(ValueLayout::Bool),
        mir::IntrinsicResultType::I32 => Some(ValueLayout::Int {
            width: 32,
            signed: true,
        }),
        mir::IntrinsicResultType::Isize => Some(ValueLayout::Int {
            width: usize::BITS as u16,
            signed: true,
        }),
        mir::IntrinsicResultType::Usize => Some(ValueLayout::Int {
            width: usize::BITS as u16,
            signed: false,
        }),
        mir::IntrinsicResultType::SameAsArgument(index) => {
            let argument = argument.get(index as usize)?;
            value_layout_map.get(argument.value()?)
        }
        mir::IntrinsicResultType::Pointee(index) => {
            let argument = argument.get(index as usize)?;
            let pointer_layout = value_layout_map.get(argument.value()?)?;
            layout_from_pointer(tree, pointer_layout)
        }
        mir::IntrinsicResultType::CheckedArithmetic
        | mir::IntrinsicResultType::PointeeAndBool(_)
        | mir::IntrinsicResultType::TypeDescriptor
        | mir::IntrinsicResultType::Explicit => Some(ValueLayout::Unknown),
    }
}

/// Get the layout for one constant value.
fn layout_from_constant(constant: &mir::Constant) -> ValueLayout {
    match constant {
        mir::Constant::Null => ValueLayout::Unknown,
        mir::Constant::Boolean { .. } => ValueLayout::Bool,
        mir::Constant::Int {
            width, is_signed, ..
        } => ValueLayout::Int {
            width: *width,
            signed: *is_signed,
        },
        mir::Constant::UInt { width, .. } => ValueLayout::Int {
            width: *width,
            signed: false,
        },
        mir::Constant::Float { width, .. } => ValueLayout::Float {
            width: *width as u16,
        },
        mir::Constant::Char { .. } => ValueLayout::Char,
    }
}

/// Resolve one pointee layout from one pointer-like value.
fn layout_from_pointer(tree: &mir::Tree, layout: ValueLayout) -> Option<ValueLayout> {
    match layout {
        ValueLayout::Pointer { pointee, .. } => Some(value_layout_from_type(tree, pointee)),
        _ => None,
    }
}

/// Resolve the field layout for one payload value.
fn layout_from_field(tree: &mir::Tree, layout: ValueLayout, index: u32) -> Option<ValueLayout> {
    let ValueLayout::FrameBytes { ty } = layout else {
        return None;
    };

    match tree.get(ty) {
        mir::Type::Struct { fields, copy: _ } => {
            let field = fields.get(index as usize)?;
            let field = tree.get(*field);
            Some(value_layout_from_type(tree, field.ty.ty()?))
        }
        mir::Type::Tuple { elements, copy: _ } => {
            let field = elements.get(index as usize)?;
            Some(value_layout_from_type(tree, field.ty()?))
        }
        _ => None,
    }
}

/// Resolve the element layout for one array value.
fn layout_from_element(tree: &mir::Tree, layout: ValueLayout) -> Option<ValueLayout> {
    match layout {
        ValueLayout::Array { element, .. } => Some(value_layout_from_type(tree, element)),
        ValueLayout::FrameBytes { ty } => match tree.get(ty) {
            mir::Type::Array { element, .. } => Some(value_layout_from_type(tree, element.ty()?)),
            _ => None,
        },
        _ => None,
    }
}

/// Rebuild one address-producing result layout from the source pointer class.
fn pointer_result_layout_from_source(
    tree: &mir::Tree,
    result_type: mir::LocalNodeId<mir::Type>,
    source_layout: ValueLayout,
) -> Option<ValueLayout> {
    let ValueLayout::Pointer { pointer_class, .. } = source_layout else {
        return Some(value_layout_from_type(tree, result_type));
    };

    let ValueLayout::Pointer {
        pointee, reference, ..
    } = value_layout_from_type(tree, result_type)
    else {
        return None;
    };

    Some(ValueLayout::Pointer {
        pointee,
        pointer_class,
        reference,
    })
}
