use destack_mir as mir;

use crate::ReferenceMeta;

use crate::program::{
    PointerClass, ValueRepr, pointer_class_from_reference, repr_type, value_repr_from_type,
};

/// One dense map from SSA value id to lowered representation.
pub(super) struct ValueReprMap {
    /// Representation for each SSA value id.
    reprs: Vec<Option<ValueRepr>>,
}

impl ValueReprMap {
    /// Create a new value representation table.
    pub(super) fn new(value_count: usize) -> Self {
        Self {
            reprs: vec![None; value_count],
        }
    }

    /// Get the representation for a value.
    pub(super) fn get(&self, value: mir::Value) -> Option<ValueRepr> {
        self.reprs.get(value.0 as usize).and_then(|repr| *repr)
    }

    /// Set the representation for a value.
    pub(super) fn set(&mut self, value: mir::Value, repr: ValueRepr) {
        if let Some(entry) = self.reprs.get_mut(value.0 as usize) {
            *entry = Some(repr);
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

/// One value representation builder for one function.
pub(super) struct ReprMapBuilder<'a> {
    /// The MIR tree.
    tree: &'a mir::Tree,
    /// The MIR function being lowered.
    func: &'a mir::Function,
    /// The MIR blocks in lowered order.
    mir_block: &'a [mir::LocalNodeId<mir::Block>],
    /// The lowered value types by SSA value id.
    value_type: &'a [mir::LocalNodeId<mir::Type>],
    /// The lowered value representation map.
    value_repr_map: ValueReprMap,
}

impl<'a> ReprMapBuilder<'a> {
    /// Create one value representation builder.
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
            value_repr_map: ValueReprMap::new(value_count),
        }
    }

    /// Build the lowered value representation map.
    pub(super) fn build(mut self) -> ValueReprMap {
        // seed explicit value types
        for (index, ty) in self.value_type.iter().enumerate() {
            let value = mir::Value(index as u32);
            self.value_repr_map
                .set(value, value_repr_from_type(self.tree, *ty));
        }

        // seed function parameter kinds
        for param in &self.func.parameters {
            let Some(value) = param.value.value() else {
                continue;
            };
            let Some(ty) = param.ty.ty() else {
                continue;
            };

            self.value_repr_map
                .set(value, value_repr_from_type(self.tree, ty));
        }

        // seed block parameter kinds
        for block_id in self.mir_block {
            let block = self.tree.get(*block_id);
            for param in &block.parameters {
                let Some(value) = param.value.value() else {
                    continue;
                };
                let Some(ty) = param.ty.ty() else {
                    continue;
                };
                let repr = value_repr_from_type(self.tree, ty);
                let block_repr = repr_for_block_parameter(repr);
                let next_repr = match self.value_repr_map.get(value) {
                    Some(existing) => merge_block_parameter_repr(existing, block_repr),
                    None => block_repr,
                };

                self.value_repr_map.set(value, next_repr);
            }
        }

        // iteratively infer instruction results
        let mut is_changed = true;
        while is_changed {
            // reset iteration state
            is_changed = false;

            // propagate block parameter kinds from control flow edges
            if propagate_block_parameter_reprs(self.tree, self.mir_block, &mut self.value_repr_map)
            {
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
                    let Some(repr) = infer_instruction_repr(
                        self.tree,
                        inst,
                        &self.value_repr_map,
                        self.value_type,
                    ) else {
                        continue;
                    };
                    let next_repr = match self.value_repr_map.get(destination) {
                        Some(existing) => merge_block_parameter_repr(existing, repr),
                        None => repr,
                    };

                    if self.value_repr_map.get(destination) != Some(next_repr) {
                        self.value_repr_map.set(destination, next_repr);
                        is_changed = true;
                    }
                }
            }
        }

        self.value_repr_map
    }
}

/// Get reference metadata for a value when available.
pub(super) fn reference_meta_for_value(
    value_repr_map: &ValueReprMap,
    value: mir::Value,
) -> ReferenceMeta {
    match value_repr_map.get(value) {
        Some(ValueRepr::Pointer { reference, .. }) => reference,
        _ => ReferenceMeta::NONE,
    }
}

/// Get the pointer class for a value when available.
pub(super) fn pointer_class_for_value(
    value_repr_map: &ValueReprMap,
    value: mir::Value,
) -> PointerClass {
    match value_repr_map.get(value) {
        Some(ValueRepr::Pointer { pointer_class, .. }) => pointer_class,
        Some(ValueRepr::FrameBytes { .. } | ValueRepr::Array { .. }) => PointerClass::Frame,
        _ => PointerClass::Unknown,
    }
}

/// Get reference metadata for a type when available.
pub(super) fn reference_meta_for_type(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> ReferenceMeta {
    match value_repr_from_type(tree, ty) {
        ValueRepr::Pointer { reference, .. } => reference,
        _ => ReferenceMeta::NONE,
    }
}

/// Resolve one heap pointee type from a pointer value when available.
pub(super) fn heap_pointee_type_for_value_repr(
    value_repr_map: &ValueReprMap,
    value: mir::Value,
) -> Option<mir::LocalNodeId<mir::Type>> {
    match value_repr_map.get(value) {
        Some(ValueRepr::Pointer {
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
pub(super) fn raw_pointee_type_for_value_repr(
    value_repr_map: &ValueReprMap,
    value: mir::Value,
) -> Option<mir::LocalNodeId<mir::Type>> {
    match value_repr_map.get(value) {
        Some(ValueRepr::Pointer {
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

/// Normalize one block parameter representation.
fn repr_for_block_parameter(repr: ValueRepr) -> ValueRepr {
    match repr {
        ValueRepr::Pointer {
            pointee, reference, ..
        } => ValueRepr::Pointer {
            pointee,
            pointer_class: PointerClass::Unknown,
            reference,
        },
        _ => repr,
    }
}

/// Merge pointer classes when propagating block parameter representations.
fn merge_pointer_class(existing: PointerClass, incoming: PointerClass) -> PointerClass {
    match (existing, incoming) {
        (PointerClass::Unknown, other) => other,
        (other, PointerClass::Unknown) => other,
        (left, right) if left == right => left,
        _ => PointerClass::Unknown,
    }
}

/// Merge one block parameter representation with one incoming argument representation.
fn merge_block_parameter_repr(existing: ValueRepr, incoming: ValueRepr) -> ValueRepr {
    match (existing, incoming) {
        (
            ValueRepr::Pointer {
                pointee,
                pointer_class,
                reference,
            },
            ValueRepr::Pointer {
                pointer_class: incoming_pointer_class,
                ..
            },
        ) => ValueRepr::Pointer {
            pointee,
            pointer_class: merge_pointer_class(pointer_class, incoming_pointer_class),
            reference,
        },
        (ValueRepr::Unknown, other) => other,
        (other, ValueRepr::Unknown) => other,
        (other, _) => other,
    }
}

/// Propagate value representations into block parameters from control flow edges.
fn propagate_block_parameter_reprs(
    tree: &mir::Tree,
    mir_blocks: &[mir::LocalNodeId<mir::Block>],
    value_repr_map: &mut ValueReprMap,
) -> bool {
    let mut is_changed = false;

    for block_id in mir_blocks {
        let block = tree.get(*block_id);
        let terminator = tree.get(block.terminator);

        match terminator {
            mir::Terminator::Jump { target } => {
                let Some(target_block) = target.block.block() else {
                    continue;
                };
                let arguments = target
                    .arguments
                    .iter()
                    .map(|argument| argument.value())
                    .collect::<Option<Vec<_>>>();
                let Some(arguments) = arguments else {
                    continue;
                };
                is_changed |= propagate_target_repr(tree, value_repr_map, target_block, &arguments);
            }
            mir::Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                let Some(then_target_block) = then_target.block.block() else {
                    continue;
                };
                let then_arguments = then_target
                    .arguments
                    .iter()
                    .map(|argument| argument.value())
                    .collect::<Option<Vec<_>>>();
                let Some(then_arguments) = then_arguments else {
                    continue;
                };
                let Some(else_target_block) = else_target.block.block() else {
                    continue;
                };
                let else_arguments = else_target
                    .arguments
                    .iter()
                    .map(|argument| argument.value())
                    .collect::<Option<Vec<_>>>();
                let Some(else_arguments) = else_arguments else {
                    continue;
                };
                is_changed |=
                    propagate_target_repr(tree, value_repr_map, then_target_block, &then_arguments);
                is_changed |=
                    propagate_target_repr(tree, value_repr_map, else_target_block, &else_arguments);
            }
            mir::Terminator::Check {
                success, failure, ..
            } => {
                let Some(success_target) = success.block.block() else {
                    continue;
                };
                let success_arguments = success
                    .arguments
                    .iter()
                    .map(|argument| argument.value())
                    .collect::<Option<Vec<_>>>();
                let Some(success_arguments) = success_arguments else {
                    continue;
                };
                let Some(failure_target) = failure.block.block() else {
                    continue;
                };
                let failure_arguments = failure
                    .arguments
                    .iter()
                    .map(|argument| argument.value())
                    .collect::<Option<Vec<_>>>();
                let Some(failure_arguments) = failure_arguments else {
                    continue;
                };
                is_changed |=
                    propagate_target_repr(tree, value_repr_map, success_target, &success_arguments);
                is_changed |=
                    propagate_target_repr(tree, value_repr_map, failure_target, &failure_arguments);
            }
            mir::Terminator::Switch { default, cases, .. } => {
                let Some(default_target) = default.block.block() else {
                    continue;
                };
                let default_arguments = default
                    .arguments
                    .iter()
                    .map(|argument| argument.value())
                    .collect::<Option<Vec<_>>>();
                let Some(default_arguments) = default_arguments else {
                    continue;
                };
                is_changed |=
                    propagate_target_repr(tree, value_repr_map, default_target, &default_arguments);

                for case in cases {
                    let Some(target) = case.target.block.block() else {
                        continue;
                    };
                    let arguments = case
                        .target
                        .arguments
                        .iter()
                        .map(|argument| argument.value())
                        .collect::<Option<Vec<_>>>();
                    let Some(arguments) = arguments else {
                        continue;
                    };
                    is_changed |= propagate_target_repr(tree, value_repr_map, target, &arguments);
                }
            }
            mir::Terminator::Yield { resume, .. } => {
                let Some(target) = resume.block.block() else {
                    continue;
                };
                let arguments = resume
                    .arguments
                    .iter()
                    .map(|argument| argument.value())
                    .collect::<Option<Vec<_>>>();
                let Some(arguments) = arguments else {
                    continue;
                };
                is_changed |= propagate_target_repr(tree, value_repr_map, target, &arguments);
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
                let Some(normal_target_block) = normal_target.block.block() else {
                    continue;
                };
                let normal_arguments = normal_target
                    .arguments
                    .iter()
                    .map(|argument| argument.value())
                    .collect::<Option<Vec<_>>>();
                let Some(normal_arguments) = normal_arguments else {
                    continue;
                };
                let Some(unwind_target_block) = unwind_target.block.block() else {
                    continue;
                };
                let unwind_arguments = unwind_target
                    .arguments
                    .iter()
                    .map(|argument| argument.value())
                    .collect::<Option<Vec<_>>>();
                let Some(unwind_arguments) = unwind_arguments else {
                    continue;
                };
                is_changed |= propagate_target_repr(
                    tree,
                    value_repr_map,
                    normal_target_block,
                    &normal_arguments,
                );
                is_changed |= propagate_target_repr(
                    tree,
                    value_repr_map,
                    unwind_target_block,
                    &unwind_arguments,
                );
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

/// Update one target block from incoming argument representations.
fn propagate_target_repr(
    tree: &mir::Tree,
    value_repr_map: &mut ValueReprMap,
    target: mir::LocalNodeId<mir::Block>,
    arguments: &[mir::Value],
) -> bool {
    let mut is_changed = false;
    let target_block = tree.get(target);

    for (parameter, argument) in target_block.parameters.iter().zip(arguments.iter()) {
        let Some(argument_repr) = value_repr_map.get(*argument) else {
            continue;
        };
        let Some(parameter_value) = parameter.value.value() else {
            continue;
        };
        let existing = value_repr_map.get(parameter_value);
        let next_repr = match existing {
            Some(repr) => merge_block_parameter_repr(repr, argument_repr),
            None => argument_repr,
        };

        if existing != Some(next_repr) {
            value_repr_map.set(parameter_value, next_repr);
            is_changed = true;
        }
    }

    is_changed
}

/// Infer the value representation for a MIR instruction.
fn infer_instruction_repr(
    tree: &mir::Tree,
    inst: &mir::Instruction,
    value_repr_map: &ValueReprMap,
    value_types: &[mir::LocalNodeId<mir::Type>],
) -> Option<ValueRepr> {
    match inst {
        mir::Instruction::Error => None,
        mir::Instruction::Const { destination, value } => {
            if matches!(value, mir::Constant::Null) {
                let destination = destination.value()?;
                let ty = value_type_for_value(destination, value_types)?;
                return Some(value_repr_from_type(tree, ty));
            }

            Some(repr_from_constant(value))
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
                return Some(ValueRepr::Bool);
            }

            let left_repr = value_repr_map.get(left);
            let right_repr = value_repr_map.get(right);
            let operand_repr = left_repr.or(right_repr);

            match (operator.is_float(), operand_repr) {
                (true, Some(ValueRepr::Float { width })) => Some(ValueRepr::Float { width }),
                (false, Some(ValueRepr::Bool))
                    if matches!(
                        operator,
                        mir::BinaryOperator::And
                            | mir::BinaryOperator::Or
                            | mir::BinaryOperator::Xor
                    ) =>
                {
                    Some(ValueRepr::Bool)
                }
                (false, Some(ValueRepr::Int { width, signed })) => {
                    Some(ValueRepr::Int { width, signed })
                }
                _ => None,
            }
        }
        mir::Instruction::Unary { argument, .. } => value_repr_map.get(argument.value()?),
        mir::Instruction::Cast {
            argument, to_type, ..
        } => {
            let to_type = to_type.ty()?;
            let result_repr = value_repr_from_type(tree, to_type);
            let argument_repr = value_repr_map.get(argument.value()?);

            match (result_repr, argument_repr) {
                (
                    ValueRepr::Pointer {
                        pointee,
                        pointer_class: PointerClass::Unknown,
                        reference,
                    },
                    Some(ValueRepr::Pointer { pointer_class, .. }),
                ) => Some(ValueRepr::Pointer {
                    pointee,
                    pointer_class,
                    reference,
                }),
                (result_repr, _) => Some(result_repr),
            }
        }
        mir::Instruction::Select { then_value, .. } => value_repr_map.get(then_value.value()?),
        mir::Instruction::Call {
            destination,
            function,
            ..
        } => {
            (*destination)?.value()?;
            let function = tree.get(function.function()?);
            Some(value_repr_from_type(tree, function.return_type.ty()?))
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

            Some(value_repr_from_type(tree, result.ty()?))
        }
        mir::Instruction::LocalGet { local, .. } => {
            let local = tree.get(local.local()?);
            Some(value_repr_from_type(tree, local.ty.ty()?))
        }
        mir::Instruction::LocalAddr { result_type, .. } => {
            let mut repr = value_repr_from_type(tree, result_type.ty()?);
            let ValueRepr::Pointer { pointer_class, .. } = &mut repr else {
                return None;
            };
            *pointer_class = PointerClass::Frame;
            Some(repr)
        }
        mir::Instruction::GlobalAddr { result_type, .. } => {
            let mut repr = value_repr_from_type(tree, result_type.ty()?);
            let ValueRepr::Pointer { pointer_class, .. } = &mut repr else {
                return None;
            };
            *pointer_class = PointerClass::Static;
            Some(repr)
        }
        mir::Instruction::FunctionAddr { function, .. } => {
            let function = tree.get(function.function()?);
            Some(ValueRepr::FunctionPointer {
                result: function.return_type.ty()?,
            })
        }
        mir::Instruction::CallableBind { destination, .. } => {
            let destination = destination.value()?;
            let ty = value_type_for_value(destination, value_types)?;
            Some(value_repr_from_type(tree, ty))
        }
        mir::Instruction::CallableEnvironment { destination } => {
            value_repr_map.get(destination.value()?)
        }
        mir::Instruction::Load { result_type, .. } => {
            Some(value_repr_from_type(tree, result_type.ty()?))
        }
        mir::Instruction::FieldGet {
            aggregate: base,
            index,
            ..
        } => {
            let base_repr = value_repr_map.get(base.value()?)?;
            repr_from_field(tree, base_repr, *index)
        }
        mir::Instruction::FieldAddr {
            aggregate: base,
            result_type,
            ..
        } => {
            let source_repr = value_repr_map.get(base.value()?)?;
            pointer_result_repr_from_source(tree, result_type.ty()?, source_repr)
        }
        mir::Instruction::FieldSet {
            aggregate: base, ..
        } => value_repr_map.get(base.value()?),
        mir::Instruction::ElementGet { array, .. } => {
            let array_repr = value_repr_map.get(array.value()?)?;
            repr_from_element(tree, array_repr)
        }
        mir::Instruction::ElementAddr {
            array, result_type, ..
        } => {
            let source_repr = value_repr_map.get(array.value()?)?;
            pointer_result_repr_from_source(tree, result_type.ty()?, source_repr)
        }
        mir::Instruction::ElementSet { array, .. } => value_repr_map.get(array.value()?),
        mir::Instruction::Struct { ty, .. }
        | mir::Instruction::Tuple { ty, .. }
        | mir::Instruction::Array { ty, .. } => Some(value_repr_from_type(tree, ty.ty()?)),
        mir::Instruction::TensorExtract { destination, .. } => {
            let destination = destination.value()?;
            let ty = value_type_for_value(destination, value_types)?;
            Some(value_repr_from_type(tree, ty))
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
            Some(value_repr_from_type(tree, result_type.ty()?))
        }
        mir::Instruction::AtomicCompareExchange { destination, .. }
        | mir::Instruction::AtomicRmw { destination, .. } => {
            let destination = destination.value()?;
            let ty = value_type_for_value(destination, value_types)?;
            Some(value_repr_from_type(tree, ty))
        }
        mir::Instruction::Intrinsic {
            intrinsic,
            arguments,
            ..
        } => infer_intrinsic_repr(tree, *intrinsic, *arguments, value_repr_map),
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

/// Infer the value representation for one intrinsic call.
fn infer_intrinsic_repr(
    tree: &mir::Tree,
    intrinsic: mir::Intrinsic,
    arguments: mir::ArgumentSlice,
    value_repr_map: &ValueReprMap,
) -> Option<ValueRepr> {
    let argument = tree.get_arguments(arguments);

    match intrinsic.result_type() {
        mir::IntrinsicResultType::Void => None,
        mir::IntrinsicResultType::Boolean => Some(ValueRepr::Bool),
        mir::IntrinsicResultType::I32 => Some(ValueRepr::Int {
            width: 32,
            signed: true,
        }),
        mir::IntrinsicResultType::Isize => Some(ValueRepr::Int {
            width: usize::BITS as u16,
            signed: true,
        }),
        mir::IntrinsicResultType::Usize => Some(ValueRepr::Int {
            width: usize::BITS as u16,
            signed: false,
        }),
        mir::IntrinsicResultType::SameAsArgument(index) => {
            let argument = argument.get(index as usize)?;
            value_repr_map.get(argument.value()?)
        }
        mir::IntrinsicResultType::Pointee(index) => {
            let argument = argument.get(index as usize)?;
            let pointer_repr = value_repr_map.get(argument.value()?)?;
            repr_from_pointer(tree, pointer_repr)
        }
        mir::IntrinsicResultType::CheckedArithmetic
        | mir::IntrinsicResultType::PointeeAndBool(_)
        | mir::IntrinsicResultType::TypeDescriptor
        | mir::IntrinsicResultType::Explicit => Some(ValueRepr::Unknown),
    }
}

/// Get the representation for one constant value.
fn repr_from_constant(constant: &mir::Constant) -> ValueRepr {
    match constant {
        mir::Constant::Null => ValueRepr::Unknown,
        mir::Constant::Boolean { .. } => ValueRepr::Bool,
        mir::Constant::Int {
            width, is_signed, ..
        } => ValueRepr::Int {
            width: *width,
            signed: *is_signed,
        },
        mir::Constant::UInt { width, .. } => ValueRepr::Int {
            width: *width,
            signed: false,
        },
        mir::Constant::Float { width, .. } => ValueRepr::Float {
            width: *width as u16,
        },
        mir::Constant::Char { .. } => ValueRepr::Char,
    }
}

/// Resolve one pointee representation from one pointer-like value.
fn repr_from_pointer(tree: &mir::Tree, repr: ValueRepr) -> Option<ValueRepr> {
    match repr {
        ValueRepr::Pointer { pointee, .. } => Some(value_repr_from_type(tree, pointee)),
        _ => None,
    }
}

/// Resolve the field representation for one payload value.
fn repr_from_field(tree: &mir::Tree, repr: ValueRepr, index: u32) -> Option<ValueRepr> {
    let ValueRepr::FrameBytes { ty } = repr else {
        return None;
    };

    match tree.get(ty) {
        mir::Type::Struct { fields, copy: _ } => {
            let field = fields.get(index as usize)?;
            let field = tree.get(*field);
            Some(value_repr_from_type(tree, field.ty.ty()?))
        }
        mir::Type::Tuple { elements, copy: _ } => {
            let field = elements.get(index as usize)?;
            Some(value_repr_from_type(tree, field.ty()?))
        }
        _ => None,
    }
}

/// Resolve the element representation for one array value.
fn repr_from_element(tree: &mir::Tree, repr: ValueRepr) -> Option<ValueRepr> {
    match repr {
        ValueRepr::Array { element, .. } => Some(value_repr_from_type(tree, element)),
        ValueRepr::FrameBytes { ty } => match tree.get(ty) {
            mir::Type::Array { element, .. } => Some(value_repr_from_type(tree, element.ty()?)),
            _ => None,
        },
        _ => None,
    }
}

/// Rebuild one address-producing result representation from the source pointer class.
fn pointer_result_repr_from_source(
    tree: &mir::Tree,
    result_type: mir::LocalNodeId<mir::Type>,
    source_repr: ValueRepr,
) -> Option<ValueRepr> {
    let ValueRepr::Pointer { pointer_class, .. } = source_repr else {
        return Some(value_repr_from_type(tree, result_type));
    };

    let ValueRepr::Pointer {
        pointee, reference, ..
    } = value_repr_from_type(tree, result_type)
    else {
        return None;
    };

    Some(ValueRepr::Pointer {
        pointee,
        pointer_class,
        reference,
    })
}
