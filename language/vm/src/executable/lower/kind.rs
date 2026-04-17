use destack_mir as mir;

use destack_heap::ReferenceMeta;

use crate::executable::layout::repr_type;
use crate::executable::value::{
    PointerStorage, ValueKind, kind_from_type, pointer_storage_from_reference,
};

/// One dense map from SSA value id to inferred lowered kind.
pub(super) struct ValueKindMap {
    /// Kind for each SSA value id.
    kinds: Vec<Option<ValueKind>>,
}

impl ValueKindMap {
    /// Create a new value kind table.
    pub(super) fn new(value_count: usize) -> Self {
        Self {
            kinds: vec![None; value_count],
        }
    }

    /// Get the kind for a value.
    pub(super) fn get(&self, value: mir::Value) -> Option<ValueKind> {
        self.kinds.get(value.0 as usize).and_then(|kind| *kind)
    }

    /// Set the kind for a value.
    pub(super) fn set(&mut self, value: mir::Value, kind: ValueKind) {
        if let Some(slot) = self.kinds.get_mut(value.0 as usize) {
            *slot = Some(kind);
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

/// One value-kind inference builder for one function.
pub(super) struct KindMapBuilder<'a> {
    /// The MIR node tree.
    tree: &'a mir::NodeTree,
    /// The MIR function being lowered.
    func: &'a mir::Function,
    /// The MIR blocks in lowered order.
    mir_block: &'a [mir::LocalNodeId<mir::Block>],
    /// The lowered value types by SSA value id.
    value_type: &'a [mir::LocalNodeId<mir::Type>],
    /// The inferred value-kind map.
    value_kind_map: ValueKindMap,
}

impl<'a> KindMapBuilder<'a> {
    /// Create one value-kind inference builder.
    pub(super) fn new(
        tree: &'a mir::NodeTree,
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
            value_kind_map: ValueKindMap::new(value_count),
        }
    }

    /// Build the inferred value-kind map.
    pub(super) fn build(mut self) -> ValueKindMap {
        // seed explicit value types
        for (index, ty) in self.value_type.iter().enumerate() {
            let value = mir::Value(index as u32);
            self.value_kind_map
                .set(value, kind_from_type(self.tree, *ty));
        }

        // seed function parameter kinds
        for param in &self.func.parameters {
            let Some(value) = param.value.value() else {
                continue;
            };
            let Some(ty) = param.ty.ty() else {
                continue;
            };

            self.value_kind_map
                .set(value, kind_from_type(self.tree, ty));
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
                let kind = kind_from_type(self.tree, ty);
                let block_kind = kind_for_block_parameter(kind);
                let next_kind = match self.value_kind_map.get(value) {
                    Some(existing) => merge_block_parameter_kind(existing, block_kind),
                    None => block_kind,
                };

                self.value_kind_map.set(value, next_kind);
            }
        }

        // iteratively infer instruction results
        let mut is_changed = true;
        while is_changed {
            // reset iteration state
            is_changed = false;

            // propagate block parameter kinds from control flow edges
            if propagate_block_parameter_kinds(self.tree, self.mir_block, &mut self.value_kind_map)
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
                    let Some(kind) = infer_instruction_kind(
                        self.tree,
                        inst,
                        &self.value_kind_map,
                        self.value_type,
                    ) else {
                        continue;
                    };
                    let next_kind = match self.value_kind_map.get(destination) {
                        Some(existing) => merge_block_parameter_kind(existing, kind),
                        None => kind,
                    };

                    if self.value_kind_map.get(destination) != Some(next_kind) {
                        self.value_kind_map.set(destination, next_kind);
                        is_changed = true;
                    }
                }
            }
        }

        self.value_kind_map
    }
}

/// Get reference metadata for a value when available.
pub(super) fn reference_meta_for_value(
    value_kind_map: &ValueKindMap,
    value: mir::Value,
) -> ReferenceMeta {
    match value_kind_map.get(value) {
        Some(ValueKind::Pointer { reference, .. }) => reference,
        _ => ReferenceMeta::NONE,
    }
}

/// Get reference metadata for a type when available.
pub(super) fn reference_meta_for_type(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
) -> ReferenceMeta {
    match kind_from_type(tree, ty) {
        ValueKind::Pointer { reference, .. } => reference,
        _ => ReferenceMeta::NONE,
    }
}

/// Resolve one managed pointee type from a pointer value when available.
pub(super) fn managed_pointee_type_for_value_kind(
    value_kind_map: &ValueKindMap,
    value: mir::Value,
) -> Option<mir::LocalNodeId<mir::Type>> {
    match value_kind_map.get(value) {
        Some(ValueKind::Pointer {
            pointee,
            storage: PointerStorage::Managed,
            ..
        }) => Some(pointee),
        _ => None,
    }
}

/// Resolve one raw pointee type from a pointer value when available.
pub(super) fn raw_pointee_type_for_value_kind(
    value_kind_map: &ValueKindMap,
    value: mir::Value,
) -> Option<mir::LocalNodeId<mir::Type>> {
    match value_kind_map.get(value) {
        Some(ValueKind::Pointer {
            pointee,
            storage: PointerStorage::Raw | PointerStorage::Stack,
            ..
        }) => Some(pointee),
        _ => None,
    }
}

/// Resolve one managed pointee type from a value type when available.
pub(super) fn managed_pointee_type_for_value(
    tree: &mir::NodeTree,
    value_types: &[mir::LocalNodeId<mir::Type>],
    value: mir::Value,
) -> Option<mir::LocalNodeId<mir::Type>> {
    let ty = repr_type(tree, value_type_for_value(value, value_types)?);

    match tree.get(ty) {
        mir::Type::Reference {
            kind: mir::ReferenceKind::Managed,
            pointee,
            ..
        } => pointee.ty(),
        mir::Type::TensorReference {
            kind: mir::ReferenceKind::Managed,
            element,
            ..
        } => element.ty(),
        _ => None,
    }
}

/// Resolve one raw pointee type from a value type when available.
pub(super) fn raw_pointee_type_for_value(
    tree: &mir::NodeTree,
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
            pointer_storage_from_reference(*address_space, *kind),
            PointerStorage::Raw | PointerStorage::Stack
        ) =>
        {
            pointee.ty()
        }
        mir::Type::TensorReference {
            kind,
            address_space,
            element,
            ..
        } if matches!(
            pointer_storage_from_reference(*address_space, *kind),
            PointerStorage::Raw | PointerStorage::Stack
        ) =>
        {
            element.ty()
        }
        _ => None,
    }
}

/// Normalize one block parameter kind.
fn kind_for_block_parameter(kind: ValueKind) -> ValueKind {
    match kind {
        ValueKind::Pointer {
            pointee, reference, ..
        } => ValueKind::Pointer {
            pointee,
            storage: PointerStorage::Unknown,
            reference,
        },
        _ => kind,
    }
}

/// Merge pointer storage classes when propagating block parameter kinds.
fn merge_pointer_storage(existing: PointerStorage, incoming: PointerStorage) -> PointerStorage {
    match (existing, incoming) {
        (PointerStorage::Unknown, other) => other,
        (other, PointerStorage::Unknown) => other,
        (left, right) if left == right => left,
        _ => PointerStorage::Unknown,
    }
}

/// Merge one block parameter kind with one incoming argument kind.
fn merge_block_parameter_kind(existing: ValueKind, incoming: ValueKind) -> ValueKind {
    match (existing, incoming) {
        (
            ValueKind::Pointer {
                pointee,
                storage,
                reference,
            },
            ValueKind::Pointer {
                storage: incoming_storage,
                ..
            },
        ) => ValueKind::Pointer {
            pointee,
            storage: merge_pointer_storage(storage, incoming_storage),
            reference,
        },
        (ValueKind::Unknown, other) => other,
        (other, ValueKind::Unknown) => other,
        (other, _) => other,
    }
}

/// Propagate value kinds into block parameters from control flow edges.
fn propagate_block_parameter_kinds(
    tree: &mir::NodeTree,
    mir_blocks: &[mir::LocalNodeId<mir::Block>],
    value_kind_map: &mut ValueKindMap,
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
                is_changed |= propagate_target_kind(tree, value_kind_map, target_block, &arguments);
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
                    propagate_target_kind(tree, value_kind_map, then_target_block, &then_arguments);
                is_changed |=
                    propagate_target_kind(tree, value_kind_map, else_target_block, &else_arguments);
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
                    propagate_target_kind(tree, value_kind_map, success_target, &success_arguments);
                is_changed |=
                    propagate_target_kind(tree, value_kind_map, failure_target, &failure_arguments);
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
                    propagate_target_kind(tree, value_kind_map, default_target, &default_arguments);

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
                    is_changed |= propagate_target_kind(tree, value_kind_map, target, &arguments);
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
                is_changed |= propagate_target_kind(tree, value_kind_map, target, &arguments);
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
                is_changed |= propagate_target_kind(
                    tree,
                    value_kind_map,
                    normal_target_block,
                    &normal_arguments,
                );
                is_changed |= propagate_target_kind(
                    tree,
                    value_kind_map,
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

/// Update one target block from incoming argument kinds.
fn propagate_target_kind(
    tree: &mir::NodeTree,
    value_kind_map: &mut ValueKindMap,
    target: mir::LocalNodeId<mir::Block>,
    arguments: &[mir::Value],
) -> bool {
    let mut is_changed = false;
    let target_block = tree.get(target);

    for (parameter, argument) in target_block.parameters.iter().zip(arguments.iter()) {
        let Some(argument_kind) = value_kind_map.get(*argument) else {
            continue;
        };
        let Some(parameter_value) = parameter.value.value() else {
            continue;
        };
        let existing = value_kind_map.get(parameter_value);
        let next_kind = match existing {
            Some(kind) => merge_block_parameter_kind(kind, argument_kind),
            None => argument_kind,
        };

        if existing != Some(next_kind) {
            value_kind_map.set(parameter_value, next_kind);
            is_changed = true;
        }
    }

    is_changed
}

/// Infer the value kind for a MIR instruction.
fn infer_instruction_kind(
    tree: &mir::NodeTree,
    inst: &mir::Instruction,
    value_kind_map: &ValueKindMap,
    value_types: &[mir::LocalNodeId<mir::Type>],
) -> Option<ValueKind> {
    match inst {
        mir::Instruction::Error => None,
        mir::Instruction::Const { destination, value } => {
            if matches!(value, mir::Constant::Null) {
                let destination = destination.value()?;
                let ty = value_type_for_value(destination, value_types)?;
                return Some(kind_from_type(tree, ty));
            }

            Some(kind_from_constant(value))
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
                return Some(ValueKind::Bool);
            }

            let left_kind = value_kind_map.get(left);
            let right_kind = value_kind_map.get(right);
            let operand_kind = left_kind.or(right_kind);

            match (operator.is_float(), operand_kind) {
                (true, Some(ValueKind::Float { width })) => Some(ValueKind::Float { width }),
                (false, Some(ValueKind::Bool))
                    if matches!(
                        operator,
                        mir::BinaryOperator::And
                            | mir::BinaryOperator::Or
                            | mir::BinaryOperator::Xor
                    ) =>
                {
                    Some(ValueKind::Bool)
                }
                (false, Some(ValueKind::Int { width, signed })) => {
                    Some(ValueKind::Int { width, signed })
                }
                _ => None,
            }
        }
        mir::Instruction::Unary { argument, .. } => value_kind_map.get(argument.value()?),
        mir::Instruction::Cast { to_type, .. } => Some(kind_from_type(tree, to_type.ty()?)),
        mir::Instruction::Select { then_value, .. } => value_kind_map.get(then_value.value()?),
        mir::Instruction::Call {
            destination,
            function,
            ..
        } => {
            let destination = (*destination)?.value()?;
            let _ = destination;
            let function = tree.get(function.function()?);
            Some(kind_from_type(tree, function.return_type.ty()?))
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
            let destination = (*destination)?.value()?;
            let _ = destination;
            let signature = call.signature.ty()?;
            let mir::Type::FunctionPointer { result, .. } = tree.get(signature) else {
                return None;
            };

            Some(kind_from_type(tree, result.ty()?))
        }
        mir::Instruction::LocalGet { local, .. } => {
            let local = tree.get(local.local()?);
            Some(kind_from_type(tree, local.ty.ty()?))
        }
        mir::Instruction::LocalAddr { result_type, .. } => {
            let mut kind = kind_from_type(tree, result_type.ty()?);
            let ValueKind::Pointer { storage, .. } = &mut kind else {
                return None;
            };
            *storage = PointerStorage::Local;
            Some(kind)
        }
        mir::Instruction::GlobalAddr { result_type, .. } => {
            Some(kind_from_type(tree, result_type.ty()?))
        }
        mir::Instruction::GlobalConst { global, .. } => {
            let global = tree.get(global.global()?);
            Some(kind_from_type(tree, global.ty.ty()?))
        }
        mir::Instruction::FunctionAddr { function, .. } => {
            let function = tree.get(function.function()?);
            Some(ValueKind::FunctionPointer {
                result: function.return_type.ty()?,
            })
        }
        mir::Instruction::FunctionBind { destination, .. } => {
            let destination = destination.value()?;
            let ty = value_type_for_value(destination, value_types)?;
            Some(kind_from_type(tree, ty))
        }
        mir::Instruction::FunctionEnvironment { destination } => {
            value_kind_map.get(destination.value()?)
        }
        mir::Instruction::Load { result_type, .. } => Some(kind_from_type(tree, result_type.ty()?)),
        mir::Instruction::FieldGet {
            aggregate, index, ..
        } => {
            let aggregate_kind = value_kind_map.get(aggregate.value()?)?;
            kind_from_field(tree, aggregate_kind, *index)
        }
        mir::Instruction::FieldAddr {
            aggregate,
            result_type,
            ..
        } => {
            let source_kind = value_kind_map.get(aggregate.value()?)?;
            pointer_result_kind_from_source(tree, result_type.ty()?, source_kind)
        }
        mir::Instruction::FieldSet { aggregate, .. } => value_kind_map.get(aggregate.value()?),
        mir::Instruction::ElementGet { array, .. } => {
            let array_kind = value_kind_map.get(array.value()?)?;
            kind_from_element(tree, array_kind)
        }
        mir::Instruction::ElementAddr {
            array, result_type, ..
        } => {
            let source_kind = value_kind_map.get(array.value()?)?;
            pointer_result_kind_from_source(tree, result_type.ty()?, source_kind)
        }
        mir::Instruction::ElementSet { array, .. } => value_kind_map.get(array.value()?),
        mir::Instruction::Struct { ty, .. }
        | mir::Instruction::Tuple { ty, .. }
        | mir::Instruction::Array { ty, .. } => Some(kind_from_type(tree, ty.ty()?)),
        mir::Instruction::VectorSplat { .. }
        | mir::Instruction::VectorExtract { .. }
        | mir::Instruction::VectorInsert { .. }
        | mir::Instruction::VectorShuffle { .. }
        | mir::Instruction::VectorSelect { .. }
        | mir::Instruction::VectorReduce { .. }
        | mir::Instruction::VectorCompare { .. }
        | mir::Instruction::VectorConvert { .. }
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
        mir::Instruction::ManagedAlloc { result_type, .. }
        | mir::Instruction::ManagedAllocArray { result_type, .. }
        | mir::Instruction::RawAlloc { result_type, .. }
        | mir::Instruction::StackAlloc { result_type, .. }
        | mir::Instruction::AtomicLoad { result_type, .. } => {
            Some(kind_from_type(tree, result_type.ty()?))
        }
        mir::Instruction::AtomicCompareExchange { destination, .. }
        | mir::Instruction::AtomicRmw { destination, .. } => {
            let destination = destination.value()?;
            let ty = value_type_for_value(destination, value_types)?;
            Some(kind_from_type(tree, ty))
        }
        mir::Instruction::Intrinsic {
            intrinsic,
            arguments,
            ..
        } => infer_intrinsic_kind(tree, *intrinsic, *arguments, value_kind_map),
        mir::Instruction::LocalSet { .. }
        | mir::Instruction::Store { .. }
        | mir::Instruction::AtomicStore { .. }
        | mir::Instruction::AtomicFence { .. }
        | mir::Instruction::Barrier { .. }
        | mir::Instruction::RawFree { .. }
        | mir::Instruction::Dispose { .. }
        | mir::Instruction::AsyncDispose { .. }
        | mir::Instruction::Drop { .. }
        | mir::Instruction::AsyncDrop { .. }
        | mir::Instruction::Assume { .. } => None,
    }
}

/// Infer the value kind for one intrinsic call.
fn infer_intrinsic_kind(
    tree: &mir::NodeTree,
    intrinsic: mir::Intrinsic,
    arguments: mir::ArgumentSlice,
    value_kind_map: &ValueKindMap,
) -> Option<ValueKind> {
    let argument = tree.get_arguments(arguments);

    match intrinsic.result_type() {
        mir::IntrinsicResultType::Void => None,
        mir::IntrinsicResultType::Boolean => Some(ValueKind::Bool),
        mir::IntrinsicResultType::I32 => Some(ValueKind::Int {
            width: 32,
            signed: true,
        }),
        mir::IntrinsicResultType::Isize => Some(ValueKind::Int {
            width: usize::BITS as u8,
            signed: true,
        }),
        mir::IntrinsicResultType::Usize => Some(ValueKind::Int {
            width: usize::BITS as u8,
            signed: false,
        }),
        mir::IntrinsicResultType::SameAsArgument(index) => {
            let argument = argument.get(index as usize)?;
            value_kind_map.get(argument.value()?)
        }
        mir::IntrinsicResultType::Pointee(index) => {
            let argument = argument.get(index as usize)?;
            let pointer_kind = value_kind_map.get(argument.value()?)?;
            kind_from_pointer(tree, pointer_kind)
        }
        mir::IntrinsicResultType::CheckedArithmetic
        | mir::IntrinsicResultType::PointeeAndBool(_)
        | mir::IntrinsicResultType::TypeDescriptor
        | mir::IntrinsicResultType::Explicit => Some(ValueKind::Unknown),
    }
}

/// Get the kind for one constant value.
fn kind_from_constant(constant: &mir::Constant) -> ValueKind {
    match constant {
        mir::Constant::Null => ValueKind::Unknown,
        mir::Constant::Boolean { .. } => ValueKind::Bool,
        mir::Constant::Int {
            width, is_signed, ..
        } => ValueKind::Int {
            width: *width,
            signed: *is_signed,
        },
        mir::Constant::UInt { width, .. } => ValueKind::Int {
            width: *width,
            signed: false,
        },
        mir::Constant::Float { width, .. } => ValueKind::Float { width: *width },
        mir::Constant::Char { .. } => ValueKind::Char,
    }
}

/// Resolve one pointee kind from one pointer-like value.
fn kind_from_pointer(tree: &mir::NodeTree, kind: ValueKind) -> Option<ValueKind> {
    match kind {
        ValueKind::Pointer { pointee, .. } => Some(kind_from_type(tree, pointee)),
        _ => None,
    }
}

/// Resolve the field kind for one aggregate value.
fn kind_from_field(tree: &mir::NodeTree, kind: ValueKind, index: u32) -> Option<ValueKind> {
    let ValueKind::Composite { ty } = kind else {
        return None;
    };

    match tree.get(ty) {
        mir::Type::Struct {
            fields,
            copyability: _,
        } => {
            let field = fields.get(index as usize)?;
            let field = tree.get(*field);
            Some(kind_from_type(tree, field.ty.ty()?))
        }
        mir::Type::Tuple {
            elements,
            copyability: _,
        } => {
            let field = elements.get(index as usize)?;
            Some(kind_from_type(tree, field.ty()?))
        }
        _ => None,
    }
}

/// Resolve the element kind for one array value.
fn kind_from_element(tree: &mir::NodeTree, kind: ValueKind) -> Option<ValueKind> {
    match kind {
        ValueKind::Array { element, .. } => Some(kind_from_type(tree, element)),
        ValueKind::Composite { ty } => match tree.get(ty) {
            mir::Type::Array { element, .. } => Some(kind_from_type(tree, element.ty()?)),
            _ => None,
        },
        _ => None,
    }
}

/// Rebuild one address-producing result kind from the source storage class.
fn pointer_result_kind_from_source(
    tree: &mir::NodeTree,
    result_type: mir::LocalNodeId<mir::Type>,
    source_kind: ValueKind,
) -> Option<ValueKind> {
    let ValueKind::Pointer { storage, .. } = source_kind else {
        return Some(kind_from_type(tree, result_type));
    };

    let ValueKind::Pointer {
        pointee, reference, ..
    } = kind_from_type(tree, result_type)
    else {
        return None;
    };

    Some(ValueKind::Pointer {
        pointee,
        storage,
        reference,
    })
}
