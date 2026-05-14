use std::collections::{HashMap, HashSet};

use crate::common::mir::analysis::{MemoryAccess, MemorySSA};
use crate::common::mir::terminator_substitute_uses;
use destack_mir as mir;
use mir::Instruction;

/// Clone one call payload with remapped arguments.
fn clone_call_with_arguments<A: Clone>(call: &mir::Call<A>, arguments: A) -> mir::Call<A> {
    mir::Call {
        arguments,
        ..call.clone()
    }
}

/// Substitute values inside one place.
fn place_substitute_uses(
    place: &mir::Place,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> mir::Place {
    let mut place = place.clone();

    // apply value substitutions to origins and dynamic projections
    for (from, to) in substitutions {
        place.replace_value(*from, *to);
    }

    place
}

/// Remap values and locals inside one place.
fn place_map_values_and_locals(
    place: &mir::Place,
    value_map: &HashMap<mir::Value, mir::Value>,
    local_map: &HashMap<mir::LocalNodeId<mir::Local>, mir::LocalNodeId<mir::Local>>,
) -> mir::Place {
    let mut place = place_substitute_uses(place, value_map);

    // remap local origins
    if let mir::PlaceOrigin::Local(local) = &mut place.origin
        && let Some(local_id) = local.local()
        && let Some(mapped) = local_map.get(&local_id)
    {
        *local = (*mapped).into();
    }

    place
}

/// Check if an instruction is pure (result depends only on operands).
///
/// A pure instruction has no side effects AND does not read mutable state.
/// This is stricter than `!instruction_has_side_effects`.
/// `Load` and `LocalGet` have no side effects so they can be removed if unused.
/// They still read mutable state, so they cannot be hoisted out of a loop.
///
/// Use this for LICM, code motion, and speculation optimizations.
pub fn instruction_is_pure(instruction: &Instruction) -> bool {
    // classify instructions by purity
    match instruction {
        Instruction::Error => {
            panic!("recovered MIR instruction reached optimizer");
        }

        // pure computations
        Instruction::Const { .. }
        | Instruction::Binary { .. }
        | Instruction::Unary { .. }
        | Instruction::Cast { .. }
        | Instruction::Select { .. }
        | Instruction::Assume { .. } => true,

        // pure aggregate operations
        Instruction::Struct { .. }
        | Instruction::Tuple { .. }
        | Instruction::Array { .. }
        | Instruction::Slice { .. }
        | Instruction::VectorSplat { .. }
        | Instruction::VectorExtract { .. }
        | Instruction::VectorInsert { .. }
        | Instruction::VectorShuffle { .. }
        | Instruction::VectorSelect { .. }
        | Instruction::VectorReduce { .. }
        | Instruction::VectorCompare { .. }
        | Instruction::VectorConvert { .. }
        | Instruction::TensorSplat { .. }
        | Instruction::TensorExtract { .. }
        | Instruction::TensorReshape { .. }
        | Instruction::TensorBroadcast { .. }
        | Instruction::TensorTranspose { .. }
        | Instruction::TensorCast { .. }
        | Instruction::TensorView { .. }
        | Instruction::TensorSlice { .. }
        | Instruction::TensorPad { .. }
        | Instruction::TensorConcat { .. }
        | Instruction::TensorReduce { .. }
        | Instruction::TensorIndexReduce { .. }
        | Instruction::TensorDot { .. }
        | Instruction::TensorConvolution { .. }
        | Instruction::TensorGather { .. }
        | Instruction::TensorScatter { .. }
        | Instruction::TensorCompare { .. }
        | Instruction::TensorSelect { .. }
        | Instruction::TensorConvert { .. }
        | Instruction::FieldGet { .. }
        | Instruction::FieldSet { .. }
        | Instruction::ElementGet { .. }
        | Instruction::ElementSet { .. } => true,

        // immutable global references
        Instruction::GlobalAddr { .. }
        | Instruction::FunctionAddr { .. }
        | Instruction::CallableBind { .. }
        | Instruction::CallableEnvironment { .. } => true,

        // borrow producing address computations are not speculatable
        Instruction::FieldAddr { .. }
        | Instruction::ElementAddr { .. }
        | Instruction::LocalAddr { .. } => false,

        // tensor loads read memory
        Instruction::TensorLoad { .. } => false,

        // tensor stores mutate memory
        Instruction::TensorStore { .. }
        | Instruction::TensorFill { .. }
        | Instruction::TensorCopy { .. } => false,

        // reads mutable state, not speculatable
        Instruction::LocalGet { .. }
        | Instruction::Load { .. }
        | Instruction::AtomicLoad { .. }
        | Instruction::AtomicCompareExchange { .. }
        | Instruction::AtomicRmw { .. } => false,

        // writes have side effects
        Instruction::LocalSet { .. }
        | Instruction::Store { .. }
        | Instruction::AtomicStore { .. }
        | Instruction::AtomicFence { .. }
        | Instruction::BarrierWrite { .. } => false,

        // pinning and drops have side effects
        Instruction::Pin { .. } | Instruction::Unpin { .. } | Instruction::Drop { .. } => false,

        // calls may have side effects
        Instruction::Call { .. }
        | Instruction::CallClass { .. }
        | Instruction::CallInterface { .. }
        | Instruction::CallIndirect { .. } => false,

        // allocations have side effects
        Instruction::New { .. }
        | Instruction::NewSlice { .. }
        | Instruction::RawAlloc { .. }
        | Instruction::StackAlloc { .. } => false,

        // deallocation has side effects
        Instruction::RawFree { .. } | Instruction::Free { .. } => false,

        // intrinsics may have side effects
        Instruction::Intrinsic { .. } => false,
    }
}

/// Check if an instruction can be speculated without trapping.
///
/// This is a stricter predicate than purity: some pure operations may trap.
pub fn instruction_is_speculatable(instruction: &Instruction, tree: &mir::Tree) -> bool {
    // classify instructions by speculative safety
    match instruction {
        // borrow producing address computations are not speculatable
        Instruction::FieldAddr { result_type, .. }
        | Instruction::ElementAddr { result_type, .. }
        | Instruction::LocalAddr { result_type, .. } => {
            let Some(result_type) = result_type.ty() else {
                return false;
            };

            let ty = tree.get(result_type);
            matches!(
                ty,
                mir::Type::Reference {
                    kind: mir::ReferenceKind::Raw,
                    ..
                } | mir::Type::TensorView {
                    kind: mir::ReferenceKind::Raw,
                    ..
                }
            )
        }

        // assumptions must not be speculated across control flow
        Instruction::Assume { .. } => false,

        // non-saturating float to integer casts can trap on NaN or out of range inputs
        Instruction::Cast {
            operator: mir::CastOperator::FloatToSignedInt | mir::CastOperator::FloatToUnsignedInt,
            ..
        } => false,

        // integer division and remainder may trap
        Instruction::Binary {
            operator:
                mir::BinaryOperator::SignedDivide
                | mir::BinaryOperator::UnsignedDivide
                | mir::BinaryOperator::SignedRemainder
                | mir::BinaryOperator::UnsignedRemainder,
            ..
        } => false,

        _ => instruction_is_pure(instruction),
    }
}

/// Check if an instruction computes an address value.
///
/// The result may be borrowed or raw depending on its reference type.
pub fn instruction_is_borrow_address(instruction: &Instruction) -> bool {
    matches!(
        instruction,
        Instruction::FieldAddr { .. }
            | Instruction::ElementAddr { .. }
            | Instruction::LocalAddr { .. }
    )
}

/// Check if an instruction has side effects and cannot be removed even if unused.
///
/// Instructions with side effects must be preserved regardless of whether their
/// result is used. This includes stores, calls, allocations, and drops.
pub fn instruction_has_side_effects(instruction: &Instruction) -> bool {
    // classify instructions by side effects
    match instruction {
        Instruction::Error => {
            panic!("recovered MIR instruction reached optimizer");
        }

        // pure computations, no side effects
        Instruction::Const { .. }
        | Instruction::Binary { .. }
        | Instruction::Unary { .. }
        | Instruction::Cast { .. }
        | Instruction::Select { .. }
        | Instruction::Struct { .. }
        | Instruction::Tuple { .. }
        | Instruction::Array { .. }
        | Instruction::Slice { .. }
        | Instruction::VectorSplat { .. }
        | Instruction::VectorExtract { .. }
        | Instruction::VectorInsert { .. }
        | Instruction::VectorShuffle { .. }
        | Instruction::VectorSelect { .. }
        | Instruction::VectorReduce { .. }
        | Instruction::VectorCompare { .. }
        | Instruction::VectorConvert { .. }
        | Instruction::TensorSplat { .. }
        | Instruction::TensorExtract { .. }
        | Instruction::TensorLoad { .. }
        | Instruction::TensorReshape { .. }
        | Instruction::TensorBroadcast { .. }
        | Instruction::TensorTranspose { .. }
        | Instruction::TensorCast { .. }
        | Instruction::TensorView { .. }
        | Instruction::TensorSlice { .. }
        | Instruction::TensorPad { .. }
        | Instruction::TensorConcat { .. }
        | Instruction::TensorReduce { .. }
        | Instruction::TensorIndexReduce { .. }
        | Instruction::TensorDot { .. }
        | Instruction::TensorConvolution { .. }
        | Instruction::TensorGather { .. }
        | Instruction::TensorScatter { .. }
        | Instruction::TensorCompare { .. }
        | Instruction::TensorSelect { .. }
        | Instruction::TensorConvert { .. }
        | Instruction::FieldGet { .. }
        | Instruction::FieldAddr { .. }
        | Instruction::ElementGet { .. }
        | Instruction::ElementAddr { .. }
        | Instruction::GlobalAddr { .. }
        | Instruction::FunctionAddr { .. }
        | Instruction::CallableBind { .. }
        | Instruction::CallableEnvironment { .. }
        | Instruction::LocalAddr { .. }
        | Instruction::Assume { .. } => false,

        // memory reads are pure (assuming no volatile)
        Instruction::LocalGet { .. } | Instruction::Load { .. } => false,

        // memory writes have side effects
        Instruction::LocalSet { .. }
        | Instruction::Store { .. }
        | Instruction::TensorStore { .. }
        | Instruction::TensorFill { .. }
        | Instruction::TensorCopy { .. }
        | Instruction::AtomicLoad { .. }
        | Instruction::AtomicStore { .. }
        | Instruction::AtomicCompareExchange { .. }
        | Instruction::AtomicRmw { .. }
        | Instruction::AtomicFence { .. }
        | Instruction::BarrierWrite { .. } => true,

        // aggregate updates create new values, but FieldSet/ElementSet don't have
        // side effects if the result is unused (they produce new values, not mutate)
        Instruction::FieldSet { .. } | Instruction::ElementSet { .. } => false,

        // pinning and drops have side effects
        Instruction::Pin { .. } | Instruction::Unpin { .. } | Instruction::Drop { .. } => true,

        // calls may have side effects
        Instruction::Call { .. }
        | Instruction::CallClass { .. }
        | Instruction::CallInterface { .. }
        | Instruction::CallIndirect { .. } => true,

        // allocations have side effects (memory allocation)
        Instruction::New { .. }
        | Instruction::NewSlice { .. }
        | Instruction::RawAlloc { .. }
        | Instruction::StackAlloc { .. } => true,

        // deallocation has side effects
        Instruction::RawFree { .. } | Instruction::Free { .. } => true,

        // intrinsics may have side effects (check purity for safe removal)
        Instruction::Intrinsic { intrinsic, .. } => {
            !intrinsic.is_pure() || matches!(intrinsic, mir::Intrinsic::BlackBox)
        }
    }
}

/// Check if an instruction reads from memory.
///
/// Memory reads include loads from pointers and gets from locals. These
/// instructions don't have side effects but read mutable state, so they
/// cannot be freely reordered past memory writes.
pub fn instruction_is_memory_read(instruction: &Instruction) -> bool {
    // identify instructions that read mutable memory
    matches!(
        instruction,
        Instruction::Load { .. }
            | Instruction::LocalGet { .. }
            | Instruction::TensorLoad { .. }
            | Instruction::AtomicLoad { .. }
            | Instruction::AtomicCompareExchange { .. }
            | Instruction::AtomicRmw { .. }
    )
}

/// Check if an instruction may write memory or have other side effects that could affect memory.
///
/// This is used to determine if it's safe to sink loads past an instruction.
/// Any instruction that writes memory, calls functions (which might write memory),
/// or performs allocations/deallocations is considered to affect memory.
pub fn instruction_may_affect_memory(instruction: &Instruction) -> bool {
    // identify instructions that can modify memory state
    matches!(
        instruction,
        Instruction::Store { .. }
            | Instruction::LocalSet { .. }
            | Instruction::TensorStore { .. }
            | Instruction::TensorFill { .. }
            | Instruction::TensorCopy { .. }
            | Instruction::Call { .. }
            | Instruction::CallClass { .. }
            | Instruction::CallInterface { .. }
            | Instruction::CallIndirect { .. }
            | Instruction::Intrinsic { .. }
            | Instruction::AtomicLoad { .. }
            | Instruction::AtomicStore { .. }
            | Instruction::AtomicCompareExchange { .. }
            | Instruction::AtomicRmw { .. }
            | Instruction::AtomicFence { .. }
            | Instruction::BarrierWrite { .. }
            | Instruction::New { .. }
            | Instruction::NewSlice { .. }
            | Instruction::RawAlloc { .. }
            | Instruction::RawFree { .. }
            | Instruction::Free { .. }
            | Instruction::Pin { .. }
            | Instruction::Unpin { .. }
            | Instruction::Drop { .. }
            | Instruction::StackAlloc { .. }
    )
}

/// Check if an instruction is a read only memory access under MemorySSA.
pub fn instruction_is_read_only_access(
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    memory_ssa: &MemorySSA,
) -> bool {
    // load memory accesses for this instruction
    let Some(accesses) = memory_ssa.accesses_for_instruction(instruction_id) else {
        return false;
    };

    // require read only effects across all accesses
    let mut reads = false;
    for access_id in accesses {
        // read the access effect
        let effect = match memory_ssa.access(*access_id) {
            MemoryAccess::Use(use_access) => &use_access.effect,
            MemoryAccess::Def(def_access) => &def_access.effect,
            MemoryAccess::Phi(_) | MemoryAccess::LiveOnEntry => continue,
        };

        // reject write or ordered accesses
        if effect.writes || effect.is_volatile || effect.is_barrier {
            return false;
        }

        // record any read access
        reads |= effect.reads;
    }

    reads
}

/// Check if a read only instruction can be moved before other memory operations.
pub fn instruction_allows_read_only_motion(
    _instruction_id: mir::LocalNodeId<mir::Instruction>,
    instruction: &Instruction,
    _tree: &mir::Tree,
) -> bool {
    // accept non call instructions
    let is_call = matches!(
        instruction,
        Instruction::Call { .. }
            | Instruction::CallClass { .. }
            | Instruction::CallInterface { .. }
            | Instruction::CallIndirect { .. }
    );
    if !is_call {
        return true;
    }

    // require call metadata to be present
    let Some(behavior) = instruction.call_behavior() else {
        return false;
    };
    if behavior.must_not_duplicate
        || behavior.return_behavior.is_no_return()
        || behavior.allocation.allocate.is_some()
        || behavior.allocation.free.is_some()
    {
        return false;
    }

    true
}

/// Collect all values that are used by instructions or terminators in a function.
///
/// This is useful for dead code elimination and other analyses that need to know
/// which values are live.
pub fn instruction_collect_used_values(
    function: &mir::Function,
    tree: &mir::Tree,
) -> HashSet<mir::Value> {
    // seed the used value set
    let mut used = HashSet::new();

    // add function parameters as implicitly used (they're inputs)
    for param in &function.parameters {
        if let Some(value) = param.value.value() {
            used.insert(value);
        }
    }

    // scan blocks for instruction and terminator uses
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);

        // collect uses from instructions
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);

            // add inline uses
            for value in instruction.uses() {
                if let Some(value) = value.value() {
                    used.insert(value);
                }
            }

            // add externalized argument uses for calls and intrinsics
            if let Some(args_slice) = instruction.argument_slice() {
                for &arg in tree.get_arguments(args_slice) {
                    if let Some(arg) = arg.value() {
                        used.insert(arg);
                    }
                }
            }
        }

        // collect uses from terminator
        for value in terminator.uses() {
            if let Some(value) = value.value() {
                used.insert(value);
            }
        }
    }

    used
}

/// Substitute values in an instruction according to the given map.
///
/// Creates a new instruction with value references replaced according to the substitution map.
/// Values not in the map are left unchanged.
pub fn instruction_substitute_uses(
    instruction: &mir::Instruction,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> mir::Instruction {
    // skip when no substitutions are provided
    if substitutions.is_empty() {
        return instruction.clone();
    }

    // resolve a value through the substitution map
    let substitute = |value: &mir::ValueReference| -> mir::ValueReference {
        let Some(concrete_value) = value.value() else {
            return *value;
        };

        substitutions
            .get(&concrete_value)
            .copied()
            .map(Into::into)
            .unwrap_or(*value)
    };

    // rebuild the instruction with substituted operands
    match instruction {
        mir::Instruction::Error => {
            panic!("recovered MIR instruction reached optimizer");
        }
        mir::Instruction::Binary {
            destination,
            operator,
            left,
            right,
        } => mir::Instruction::Binary {
            destination: *destination,
            operator: *operator,
            left: substitute(left),
            right: substitute(right),
        },
        mir::Instruction::Unary {
            destination,
            operator,
            argument,
        } => mir::Instruction::Unary {
            destination: *destination,
            operator: *operator,
            argument: substitute(argument),
        },
        mir::Instruction::Cast {
            destination,
            operator,
            argument,
            to_type,
        } => mir::Instruction::Cast {
            destination: *destination,
            operator: *operator,
            argument: substitute(argument),
            to_type: *to_type,
        },
        mir::Instruction::Select {
            destination,
            condition,
            then_value,
            else_value,
        } => mir::Instruction::Select {
            destination: *destination,
            condition: substitute(condition),
            then_value: substitute(then_value),
            else_value: substitute(else_value),
        },
        mir::Instruction::Load {
            destination,
            pointer,
            result_type,
        } => mir::Instruction::Load {
            destination: *destination,
            pointer: substitute(pointer),
            result_type: *result_type,
        },
        mir::Instruction::Store { pointer, value } => mir::Instruction::Store {
            pointer: substitute(pointer),
            value: substitute(value),
        },
        mir::Instruction::AtomicLoad {
            destination,
            pointer,
            result_type,
            access,
        } => mir::Instruction::AtomicLoad {
            destination: *destination,
            pointer: substitute(pointer),
            result_type: *result_type,
            access: *access,
        },
        mir::Instruction::AtomicStore {
            pointer,
            value,
            access,
        } => mir::Instruction::AtomicStore {
            pointer: substitute(pointer),
            value: substitute(value),
            access: *access,
        },
        mir::Instruction::AtomicCompareExchange {
            destination,
            pointer,
            expected,
            new_value,
            is_weak,
            access,
        } => mir::Instruction::AtomicCompareExchange {
            destination: *destination,
            pointer: substitute(pointer),
            expected: substitute(expected),
            new_value: substitute(new_value),
            is_weak: *is_weak,
            access: *access,
        },
        mir::Instruction::AtomicRmw {
            destination,
            operator,
            pointer,
            value,
            access,
        } => mir::Instruction::AtomicRmw {
            destination: *destination,
            operator: *operator,
            pointer: substitute(pointer),
            value: substitute(value),
            access: *access,
        },
        mir::Instruction::AtomicFence { access } => {
            mir::Instruction::AtomicFence { access: *access }
        }
        mir::Instruction::BarrierWrite {
            object,
            offset,
            byte_len,
        } => mir::Instruction::BarrierWrite {
            object: substitute(object),
            offset: substitute(offset),
            byte_len: substitute(byte_len),
        },
        mir::Instruction::Pin {
            destination,
            value,
            result_type,
        } => mir::Instruction::Pin {
            destination: *destination,
            value: substitute(value),
            result_type: *result_type,
        },
        mir::Instruction::Unpin { value } => mir::Instruction::Unpin {
            value: substitute(value),
        },
        mir::Instruction::Free { value } => mir::Instruction::Free {
            value: substitute(value),
        },
        mir::Instruction::Drop { place } => mir::Instruction::Drop {
            place: place_substitute_uses(place, substitutions),
        },
        mir::Instruction::FieldGet {
            destination,
            aggregate,
            index,
        } => mir::Instruction::FieldGet {
            destination: *destination,
            aggregate: substitute(aggregate),
            index: *index,
        },
        mir::Instruction::FieldAddr {
            destination,
            aggregate,
            index,
            result_type,
        } => mir::Instruction::FieldAddr {
            destination: *destination,
            aggregate: substitute(aggregate),
            index: *index,
            result_type: *result_type,
        },
        mir::Instruction::FieldSet {
            destination,
            aggregate,
            index,
            value,
        } => mir::Instruction::FieldSet {
            destination: *destination,
            aggregate: substitute(aggregate),
            index: *index,
            value: substitute(value),
        },
        mir::Instruction::ElementGet {
            destination,
            array,
            index,
        } => mir::Instruction::ElementGet {
            destination: *destination,
            array: substitute(array),
            index: *index,
        },
        mir::Instruction::ElementAddr {
            destination,
            array,
            index,
            result_type,
        } => mir::Instruction::ElementAddr {
            destination: *destination,
            array: substitute(array),
            index: substitute(index),
            result_type: *result_type,
        },
        mir::Instruction::ElementSet {
            destination,
            array,
            index,
            value,
        } => mir::Instruction::ElementSet {
            destination: *destination,
            array: substitute(array),
            index: *index,
            value: substitute(value),
        },
        mir::Instruction::Slice {
            destination,
            source,
            start,
            length,
            result_type,
        } => mir::Instruction::Slice {
            destination: *destination,
            source: substitute(source),
            start: substitute(start),
            length: substitute(length),
            result_type: *result_type,
        },
        mir::Instruction::VectorSplat { destination, value } => mir::Instruction::VectorSplat {
            destination: *destination,
            value: substitute(value),
        },
        mir::Instruction::VectorExtract {
            destination,
            vector,
            index,
        } => mir::Instruction::VectorExtract {
            destination: *destination,
            vector: substitute(vector),
            index: substitute(index),
        },
        mir::Instruction::VectorInsert {
            destination,
            vector,
            index,
            value,
        } => mir::Instruction::VectorInsert {
            destination: *destination,
            vector: substitute(vector),
            index: substitute(index),
            value: substitute(value),
        },
        mir::Instruction::VectorShuffle {
            destination,
            left,
            right,
            mask,
        } => mir::Instruction::VectorShuffle {
            destination: *destination,
            left: substitute(left),
            right: substitute(right),
            mask: mask.clone(),
        },
        mir::Instruction::VectorSelect {
            destination,
            mask,
            then_value,
            else_value,
        } => mir::Instruction::VectorSelect {
            destination: *destination,
            mask: substitute(mask),
            then_value: substitute(then_value),
            else_value: substitute(else_value),
        },
        mir::Instruction::VectorReduce {
            destination,
            operator,
            vector,
        } => mir::Instruction::VectorReduce {
            destination: *destination,
            operator: *operator,
            vector: substitute(vector),
        },
        mir::Instruction::VectorCompare {
            destination,
            operator,
            left,
            right,
        } => mir::Instruction::VectorCompare {
            destination: *destination,
            operator: *operator,
            left: substitute(left),
            right: substitute(right),
        },
        mir::Instruction::VectorConvert {
            destination,
            mode,
            vector,
        } => mir::Instruction::VectorConvert {
            destination: *destination,
            mode: *mode,
            vector: substitute(vector),
        },
        mir::Instruction::TensorSplat { destination, value } => mir::Instruction::TensorSplat {
            destination: *destination,
            value: substitute(value),
        },
        mir::Instruction::TensorLoad {
            destination,
            view,
            indices,
        } => mir::Instruction::TensorLoad {
            destination: *destination,
            view: substitute(view),
            indices: *indices,
        },
        mir::Instruction::TensorExtract {
            destination,
            tensor,
            indices,
        } => mir::Instruction::TensorExtract {
            destination: *destination,
            tensor: substitute(tensor),
            indices: *indices,
        },
        mir::Instruction::TensorStore {
            view,
            indices,
            value,
        } => mir::Instruction::TensorStore {
            view: substitute(view),
            indices: *indices,
            value: substitute(value),
        },
        mir::Instruction::TensorFill { view, value } => mir::Instruction::TensorFill {
            view: substitute(view),
            value: substitute(value),
        },
        mir::Instruction::TensorCopy { target, source } => mir::Instruction::TensorCopy {
            target: substitute(target),
            source: substitute(source),
        },
        mir::Instruction::TensorReshape {
            destination,
            tensor,
            shape,
        } => mir::Instruction::TensorReshape {
            destination: *destination,
            tensor: substitute(tensor),
            shape: *shape,
        },
        mir::Instruction::TensorBroadcast {
            destination,
            tensor,
            dimensions,
        } => mir::Instruction::TensorBroadcast {
            destination: *destination,
            tensor: substitute(tensor),
            dimensions: dimensions.clone(),
        },
        mir::Instruction::TensorTranspose {
            destination,
            tensor,
            permutation,
        } => mir::Instruction::TensorTranspose {
            destination: *destination,
            tensor: substitute(tensor),
            permutation: permutation.clone(),
        },
        mir::Instruction::TensorCast {
            destination,
            tensor,
        } => mir::Instruction::TensorCast {
            destination: *destination,
            tensor: substitute(tensor),
        },
        mir::Instruction::TensorView {
            destination,
            view,
            arguments,
            offsets_count,
            sizes_count,
            strides_count,
        } => mir::Instruction::TensorView {
            destination: *destination,
            view: substitute(view),
            arguments: *arguments,
            offsets_count: *offsets_count,
            sizes_count: *sizes_count,
            strides_count: *strides_count,
        },
        mir::Instruction::TensorSlice {
            destination,
            tensor,
            arguments,
            offsets_count,
            sizes_count,
            strides_count,
        } => mir::Instruction::TensorSlice {
            destination: *destination,
            tensor: substitute(tensor),
            arguments: *arguments,
            offsets_count: *offsets_count,
            sizes_count: *sizes_count,
            strides_count: *strides_count,
        },
        mir::Instruction::TensorPad {
            destination,
            tensor,
            arguments,
            low_count,
            high_count,
            interior_count,
            value,
        } => mir::Instruction::TensorPad {
            destination: *destination,
            tensor: substitute(tensor),
            arguments: *arguments,
            low_count: *low_count,
            high_count: *high_count,
            interior_count: *interior_count,
            value: substitute(value),
        },
        mir::Instruction::TensorConcat {
            destination,
            tensors,
            axis,
        } => mir::Instruction::TensorConcat {
            destination: *destination,
            tensors: *tensors,
            axis: *axis,
        },
        mir::Instruction::TensorReduce {
            destination,
            operator,
            tensor,
            initial,
            axes,
        } => mir::Instruction::TensorReduce {
            destination: *destination,
            operator: *operator,
            tensor: substitute(tensor),
            initial: substitute(initial),
            axes: axes.clone(),
        },
        mir::Instruction::TensorIndexReduce {
            destination,
            operator,
            tensor,
            axis,
            tie_break,
        } => mir::Instruction::TensorIndexReduce {
            destination: *destination,
            operator: *operator,
            tensor: substitute(tensor),
            axis: *axis,
            tie_break: *tie_break,
        },
        mir::Instruction::TensorDot {
            destination,
            left,
            right,
            dimensions,
        } => mir::Instruction::TensorDot {
            destination: *destination,
            left: substitute(left),
            right: substitute(right),
            dimensions: dimensions.clone(),
        },
        mir::Instruction::TensorConvolution {
            destination,
            input,
            kernel,
            dimensions,
            window,
            feature_group_count,
            batch_group_count,
        } => mir::Instruction::TensorConvolution {
            destination: *destination,
            input: substitute(input),
            kernel: substitute(kernel),
            dimensions: dimensions.clone(),
            window: window.clone(),
            feature_group_count: *feature_group_count,
            batch_group_count: *batch_group_count,
        },
        mir::Instruction::TensorGather {
            destination,
            operand,
            indices,
            dimensions,
            slice_sizes,
        } => mir::Instruction::TensorGather {
            destination: *destination,
            operand: substitute(operand),
            indices: substitute(indices),
            dimensions: dimensions.clone(),
            slice_sizes: slice_sizes.clone(),
        },
        mir::Instruction::TensorScatter {
            destination,
            operand,
            indices,
            updates,
            dimensions,
            mode,
        } => mir::Instruction::TensorScatter {
            destination: *destination,
            operand: substitute(operand),
            indices: substitute(indices),
            updates: substitute(updates),
            dimensions: dimensions.clone(),
            mode: *mode,
        },
        mir::Instruction::TensorCompare {
            destination,
            operator,
            left,
            right,
        } => mir::Instruction::TensorCompare {
            destination: *destination,
            operator: *operator,
            left: substitute(left),
            right: substitute(right),
        },
        mir::Instruction::TensorSelect {
            destination,
            mask,
            then_value,
            else_value,
        } => mir::Instruction::TensorSelect {
            destination: *destination,
            mask: substitute(mask),
            then_value: substitute(then_value),
            else_value: substitute(else_value),
        },
        mir::Instruction::TensorConvert {
            destination,
            mode,
            tensor,
        } => mir::Instruction::TensorConvert {
            destination: *destination,
            mode: *mode,
            tensor: substitute(tensor),
        },
        mir::Instruction::LocalSet { local, value } => mir::Instruction::LocalSet {
            local: *local,
            value: substitute(value),
        },
        mir::Instruction::Assume { condition } => mir::Instruction::Assume {
            condition: substitute(condition),
        },
        mir::Instruction::CallClass {
            destination,
            receiver,
            call,
            declaring_type,
            slot,
            declared_target,
        } => mir::Instruction::CallClass {
            destination: *destination,
            receiver: substitute(receiver),
            call: call.clone(),
            declaring_type: *declaring_type,
            slot: *slot,
            declared_target: *declared_target,
        },
        mir::Instruction::CallInterface {
            destination,
            receiver,
            call,
            declaring_type,
            slot,
        } => mir::Instruction::CallInterface {
            destination: *destination,
            receiver: substitute(receiver),
            call: call.clone(),
            declaring_type: *declaring_type,
            slot: *slot,
        },
        mir::Instruction::CallIndirect {
            destination,
            callee,
            call,
        } => mir::Instruction::CallIndirect {
            destination: *destination,
            callee: substitute(callee),
            call: call.clone(),
        },
        mir::Instruction::NewSlice {
            destination,
            element,
            length,
            result_type,
        } => mir::Instruction::NewSlice {
            destination: *destination,
            element: *element,
            length: substitute(length),
            result_type: *result_type,
        },
        mir::Instruction::RawFree { pointer } => mir::Instruction::RawFree {
            pointer: substitute(pointer),
        },
        // instructions without value operands or with externalized arguments
        mir::Instruction::Const { .. }
        | mir::Instruction::LocalGet { .. }
        | mir::Instruction::GlobalAddr { .. }
        | mir::Instruction::FunctionAddr { .. }
        | mir::Instruction::CallableBind { .. }
        | mir::Instruction::LocalAddr { .. }
        | mir::Instruction::Struct { .. }
        | mir::Instruction::Tuple { .. }
        | mir::Instruction::Array { .. }
        | mir::Instruction::Call { .. }
        | mir::Instruction::CallableEnvironment { .. }
        | mir::Instruction::New { .. }
        | mir::Instruction::RawAlloc { .. }
        | mir::Instruction::StackAlloc { .. }
        | mir::Instruction::Intrinsic { .. } => instruction.clone(),
    }
}

/// Substitute values in an instruction, including externalized arguments.
///
/// Creates a new instruction with value references replaced according to the substitution map.
/// Values not in the map are left unchanged.
pub fn instruction_substitute_uses_in_tree(
    instruction: &mir::Instruction,
    substitutions: &HashMap<mir::Value, mir::Value>,
    tree: &mut mir::Tree,
) -> mir::Instruction {
    // skip when no substitutions are provided
    if substitutions.is_empty() {
        return instruction.clone();
    }

    // resolve values through the substitution map
    let substitute = |value: mir::ValueReference| -> mir::ValueReference {
        let Some(value_id) = value.value() else {
            return value;
        };

        substitutions
            .get(&value_id)
            .copied()
            .map(Into::into)
            .unwrap_or(value)
    };

    // rebuild argument slices when needed
    let mut substitute_arguments = |slice: mir::ArgumentSlice| -> mir::ArgumentSlice {
        // read existing arguments
        let arguments = tree.get_arguments(slice);

        // skip when no arguments are substituted
        if !arguments.iter().any(|value| {
            value
                .value()
                .is_some_and(|value| substitutions.contains_key(&value))
        }) {
            return slice;
        }

        // build remapped arguments
        let new_arguments: Vec<_> = arguments.iter().map(|value| substitute(*value)).collect();

        tree.add_arguments(&new_arguments)
    };

    // rebuild the instruction using substituted operands
    match instruction {
        mir::Instruction::Struct {
            destination,
            ty,
            fields,
        } => mir::Instruction::Struct {
            destination: *destination,
            ty: *ty,
            fields: substitute_arguments(*fields),
        },
        mir::Instruction::Tuple {
            destination,
            ty,
            elements,
        } => mir::Instruction::Tuple {
            destination: *destination,
            ty: *ty,
            elements: substitute_arguments(*elements),
        },
        mir::Instruction::Array {
            destination,
            ty,
            elements,
        } => mir::Instruction::Array {
            destination: *destination,
            ty: *ty,
            elements: substitute_arguments(*elements),
        },
        mir::Instruction::VectorSplat { destination, value } => mir::Instruction::VectorSplat {
            destination: *destination,
            value: substitute(*value),
        },
        mir::Instruction::VectorExtract {
            destination,
            vector,
            index,
        } => mir::Instruction::VectorExtract {
            destination: *destination,
            vector: substitute(*vector),
            index: substitute(*index),
        },
        mir::Instruction::VectorInsert {
            destination,
            vector,
            index,
            value,
        } => mir::Instruction::VectorInsert {
            destination: *destination,
            vector: substitute(*vector),
            index: substitute(*index),
            value: substitute(*value),
        },
        mir::Instruction::VectorShuffle {
            destination,
            left,
            right,
            mask,
        } => mir::Instruction::VectorShuffle {
            destination: *destination,
            left: substitute(*left),
            right: substitute(*right),
            mask: mask.clone(),
        },
        mir::Instruction::VectorSelect {
            destination,
            mask,
            then_value,
            else_value,
        } => mir::Instruction::VectorSelect {
            destination: *destination,
            mask: substitute(*mask),
            then_value: substitute(*then_value),
            else_value: substitute(*else_value),
        },
        mir::Instruction::VectorReduce {
            destination,
            operator,
            vector,
        } => mir::Instruction::VectorReduce {
            destination: *destination,
            operator: *operator,
            vector: substitute(*vector),
        },
        mir::Instruction::VectorCompare {
            destination,
            operator,
            left,
            right,
        } => mir::Instruction::VectorCompare {
            destination: *destination,
            operator: *operator,
            left: substitute(*left),
            right: substitute(*right),
        },
        mir::Instruction::VectorConvert {
            destination,
            mode,
            vector,
        } => mir::Instruction::VectorConvert {
            destination: *destination,
            mode: *mode,
            vector: substitute(*vector),
        },
        mir::Instruction::TensorSplat { destination, value } => mir::Instruction::TensorSplat {
            destination: *destination,
            value: substitute(*value),
        },
        mir::Instruction::TensorLoad {
            destination,
            view,
            indices,
        } => mir::Instruction::TensorLoad {
            destination: *destination,
            view: substitute(*view),
            indices: substitute_arguments(*indices),
        },
        mir::Instruction::TensorExtract {
            destination,
            tensor,
            indices,
        } => mir::Instruction::TensorExtract {
            destination: *destination,
            tensor: substitute(*tensor),
            indices: substitute_arguments(*indices),
        },
        mir::Instruction::TensorStore {
            view,
            indices,
            value,
        } => mir::Instruction::TensorStore {
            view: substitute(*view),
            indices: substitute_arguments(*indices),
            value: substitute(*value),
        },
        mir::Instruction::TensorFill { view, value } => mir::Instruction::TensorFill {
            view: substitute(*view),
            value: substitute(*value),
        },
        mir::Instruction::TensorCopy { target, source } => mir::Instruction::TensorCopy {
            target: substitute(*target),
            source: substitute(*source),
        },
        mir::Instruction::TensorReshape {
            destination,
            tensor,
            shape,
        } => mir::Instruction::TensorReshape {
            destination: *destination,
            tensor: substitute(*tensor),
            shape: substitute_arguments(*shape),
        },
        mir::Instruction::TensorBroadcast {
            destination,
            tensor,
            dimensions,
        } => mir::Instruction::TensorBroadcast {
            destination: *destination,
            tensor: substitute(*tensor),
            dimensions: dimensions.clone(),
        },
        mir::Instruction::TensorTranspose {
            destination,
            tensor,
            permutation,
        } => mir::Instruction::TensorTranspose {
            destination: *destination,
            tensor: substitute(*tensor),
            permutation: permutation.clone(),
        },
        mir::Instruction::TensorCast {
            destination,
            tensor,
        } => mir::Instruction::TensorCast {
            destination: *destination,
            tensor: substitute(*tensor),
        },
        mir::Instruction::TensorView {
            destination,
            view,
            arguments,
            offsets_count,
            sizes_count,
            strides_count,
        } => mir::Instruction::TensorView {
            destination: *destination,
            view: substitute(*view),
            arguments: substitute_arguments(*arguments),
            offsets_count: *offsets_count,
            sizes_count: *sizes_count,
            strides_count: *strides_count,
        },
        mir::Instruction::TensorSlice {
            destination,
            tensor,
            arguments,
            offsets_count,
            sizes_count,
            strides_count,
        } => mir::Instruction::TensorSlice {
            destination: *destination,
            tensor: substitute(*tensor),
            arguments: substitute_arguments(*arguments),
            offsets_count: *offsets_count,
            sizes_count: *sizes_count,
            strides_count: *strides_count,
        },
        mir::Instruction::TensorPad {
            destination,
            tensor,
            arguments,
            low_count,
            high_count,
            interior_count,
            value,
        } => mir::Instruction::TensorPad {
            destination: *destination,
            tensor: substitute(*tensor),
            arguments: substitute_arguments(*arguments),
            low_count: *low_count,
            high_count: *high_count,
            interior_count: *interior_count,
            value: substitute(*value),
        },
        mir::Instruction::TensorConcat {
            destination,
            tensors,
            axis,
        } => mir::Instruction::TensorConcat {
            destination: *destination,
            tensors: substitute_arguments(*tensors),
            axis: *axis,
        },
        mir::Instruction::TensorReduce {
            destination,
            operator,
            tensor,
            initial,
            axes,
        } => mir::Instruction::TensorReduce {
            destination: *destination,
            operator: *operator,
            tensor: substitute(*tensor),
            initial: substitute(*initial),
            axes: axes.clone(),
        },
        mir::Instruction::TensorIndexReduce {
            destination,
            operator,
            tensor,
            axis,
            tie_break,
        } => mir::Instruction::TensorIndexReduce {
            destination: *destination,
            operator: *operator,
            tensor: substitute(*tensor),
            axis: *axis,
            tie_break: *tie_break,
        },
        mir::Instruction::TensorDot {
            destination,
            left,
            right,
            dimensions,
        } => mir::Instruction::TensorDot {
            destination: *destination,
            left: substitute(*left),
            right: substitute(*right),
            dimensions: dimensions.clone(),
        },
        mir::Instruction::TensorConvolution {
            destination,
            input,
            kernel,
            dimensions,
            window,
            feature_group_count,
            batch_group_count,
        } => mir::Instruction::TensorConvolution {
            destination: *destination,
            input: substitute(*input),
            kernel: substitute(*kernel),
            dimensions: dimensions.clone(),
            window: window.clone(),
            feature_group_count: *feature_group_count,
            batch_group_count: *batch_group_count,
        },
        mir::Instruction::TensorGather {
            destination,
            operand,
            indices,
            dimensions,
            slice_sizes,
        } => mir::Instruction::TensorGather {
            destination: *destination,
            operand: substitute(*operand),
            indices: substitute(*indices),
            dimensions: dimensions.clone(),
            slice_sizes: slice_sizes.clone(),
        },
        mir::Instruction::TensorScatter {
            destination,
            operand,
            indices,
            updates,
            dimensions,
            mode,
        } => mir::Instruction::TensorScatter {
            destination: *destination,
            operand: substitute(*operand),
            indices: substitute(*indices),
            updates: substitute(*updates),
            dimensions: dimensions.clone(),
            mode: *mode,
        },
        mir::Instruction::TensorCompare {
            destination,
            operator,
            left,
            right,
        } => mir::Instruction::TensorCompare {
            destination: *destination,
            operator: *operator,
            left: substitute(*left),
            right: substitute(*right),
        },
        mir::Instruction::TensorSelect {
            destination,
            mask,
            then_value,
            else_value,
        } => mir::Instruction::TensorSelect {
            destination: *destination,
            mask: substitute(*mask),
            then_value: substitute(*then_value),
            else_value: substitute(*else_value),
        },
        mir::Instruction::TensorConvert {
            destination,
            mode,
            tensor,
        } => mir::Instruction::TensorConvert {
            destination: *destination,
            mode: *mode,
            tensor: substitute(*tensor),
        },
        mir::Instruction::Call {
            destination,
            function,
            call,
        } => mir::Instruction::Call {
            destination: *destination,
            function: *function,
            call: clone_call_with_arguments(call, substitute_arguments(call.arguments)),
        },
        mir::Instruction::CallClass {
            destination,
            receiver,
            call,
            declaring_type,
            slot,
            declared_target,
        } => mir::Instruction::CallClass {
            destination: *destination,
            receiver: substitute(*receiver),
            call: clone_call_with_arguments(call, substitute_arguments(call.arguments)),
            declaring_type: *declaring_type,
            slot: *slot,
            declared_target: *declared_target,
        },
        mir::Instruction::CallInterface {
            destination,
            receiver,
            call,
            declaring_type,
            slot,
        } => mir::Instruction::CallInterface {
            destination: *destination,
            receiver: substitute(*receiver),
            call: clone_call_with_arguments(call, substitute_arguments(call.arguments)),
            declaring_type: *declaring_type,
            slot: *slot,
        },
        mir::Instruction::CallIndirect {
            destination,
            callee,
            call,
        } => mir::Instruction::CallIndirect {
            destination: *destination,
            callee: substitute(*callee),
            call: clone_call_with_arguments(call, substitute_arguments(call.arguments)),
        },
        mir::Instruction::Intrinsic {
            destination,
            intrinsic,
            arguments,
        } => mir::Instruction::Intrinsic {
            destination: *destination,
            intrinsic: *intrinsic,
            arguments: substitute_arguments(*arguments),
        },
        mir::Instruction::AtomicLoad {
            destination,
            pointer,
            result_type,
            access,
        } => mir::Instruction::AtomicLoad {
            destination: *destination,
            pointer: substitute(*pointer),
            result_type: *result_type,
            access: *access,
        },
        mir::Instruction::AtomicStore {
            pointer,
            value,
            access,
        } => mir::Instruction::AtomicStore {
            pointer: substitute(*pointer),
            value: substitute(*value),
            access: *access,
        },
        mir::Instruction::AtomicCompareExchange {
            destination,
            pointer,
            expected,
            new_value,
            is_weak,
            access,
        } => mir::Instruction::AtomicCompareExchange {
            destination: *destination,
            pointer: substitute(*pointer),
            expected: substitute(*expected),
            new_value: substitute(*new_value),
            is_weak: *is_weak,
            access: *access,
        },
        mir::Instruction::AtomicRmw {
            destination,
            operator,
            pointer,
            value,
            access,
        } => mir::Instruction::AtomicRmw {
            destination: *destination,
            operator: *operator,
            pointer: substitute(*pointer),
            value: substitute(*value),
            access: *access,
        },
        mir::Instruction::AtomicFence { access } => {
            mir::Instruction::AtomicFence { access: *access }
        }
        mir::Instruction::BarrierWrite {
            object,
            offset,
            byte_len,
        } => mir::Instruction::BarrierWrite {
            object: substitute(*object),
            offset: substitute(*offset),
            byte_len: substitute(*byte_len),
        },
        _ => instruction_substitute_uses(instruction, substitutions),
    }
}

/// Substitute values in a slice using the provided mapping.
pub fn substitute_values(
    values: &[mir::ValueReference],
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> Vec<mir::ValueReference> {
    // fast path for empty substitutions
    if substitutions.is_empty() {
        return values.to_vec();
    }

    // apply substitutions to the value list
    values
        .iter()
        .map(|value| {
            let Some(value_id) = value.value() else {
                return *value;
            };

            substitutions
                .get(&value_id)
                .copied()
                .map(Into::into)
                .unwrap_or(*value)
        })
        .collect()
}

/// Apply substitutions and optional removals across a function.
///
/// Returns true when any instruction or terminator is updated or removed.
pub fn apply_substitutions_in_function(
    function: &mir::Function,
    tree: &mut mir::Tree,
    substitutions: &HashMap<mir::Value, mir::Value>,
    to_remove: Option<&HashSet<mir::LocalNodeId<mir::Instruction>>>,
) -> bool {
    // check if work is required
    let has_substitutions = !substitutions.is_empty();
    let has_removals = to_remove.is_some_and(|set| !set.is_empty());
    if !has_substitutions && !has_removals {
        return false;
    }

    // track whether any changes occur
    let mut changed = false;

    // rewrite instructions and terminators in each block
    for &block_id in &function.blocks {
        // snapshot block contents
        let block = tree.get(block_id).clone();
        let instruction_ids = block.instructions.clone();
        let terminator_id = block.terminator;
        let terminator = tree.get(terminator_id).clone();

        // rebuild instructions with substitutions and removals
        let mut new_instructions = Vec::with_capacity(instruction_ids.len());
        for instruction_id in instruction_ids {
            // skip instructions slated for removal
            if to_remove.is_some_and(|set| set.contains(&instruction_id)) {
                changed = true;
                continue;
            }

            // substitute instruction operands when requested
            if has_substitutions {
                let instruction = tree.get(instruction_id).clone();
                let updated =
                    instruction_substitute_uses_in_tree(&instruction, substitutions, tree);
                if updated != instruction {
                    tree.replace(instruction_id, updated);
                    remap_instruction_memory_accesses(tree, instruction_id, substitutions);
                    changed = true;
                }
            }

            new_instructions.push(instruction_id);
        }

        // rewrite terminator operands when requested
        let new_terminator = if has_substitutions {
            terminator_substitute_uses(&terminator, substitutions)
        } else {
            terminator.clone()
        };

        // update block when instructions or terminator changed
        if new_instructions.len() != block.instructions.len() || new_terminator != terminator {
            let mut new_block = block;
            new_block.instructions = new_instructions;
            tree.replace(block_id, new_block);
            tree.replace(terminator_id, new_terminator);
            changed = true;
        }
    }

    changed
}

/// Maps for tracking where values are used and defined.
#[derive(Debug)]
pub struct UseDefMaps {
    /// Maps each value to the blocks where it is used.
    pub use_blocks: HashMap<mir::Value, Vec<mir::LocalNodeId<mir::Block>>>,
    /// Maps each value to the block where it is defined.
    pub def_block: HashMap<mir::Value, mir::LocalNodeId<mir::Block>>,
}

/// Build maps from values to their use locations and definition blocks.
///
/// This is useful for sinking, code motion, and liveness analysis.
/// Function parameters are not included in `def_block` (they have no defining block).
pub fn build_use_def_maps(function: &mir::Function, tree: &mir::Tree) -> UseDefMaps {
    // initialize use and definition maps
    let mut use_blocks: HashMap<mir::Value, Vec<mir::LocalNodeId<mir::Block>>> = HashMap::new();
    let mut def_block: HashMap<mir::Value, mir::LocalNodeId<mir::Block>> = HashMap::new();

    // scan blocks for definitions and uses
    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        // block parameters are defined in this block
        for param in &block.parameters {
            let Some(value) = param.value.value() else {
                continue;
            };

            def_block.insert(value, block_id);
        }

        // instructions
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);

            // record definition
            if let Some(dest) = instruction.destination().and_then(|value| value.value()) {
                def_block.insert(dest, block_id);
            }

            // record uses
            for use_value in instruction
                .uses()
                .into_iter()
                .filter_map(|value| value.value())
            {
                use_blocks.entry(use_value).or_default().push(block_id);
            }

            // externalized arguments
            if let Some(args_slice) = instruction.argument_slice() {
                for arg in tree
                    .get_arguments(args_slice)
                    .iter()
                    .copied()
                    .filter_map(|value| value.value())
                {
                    use_blocks.entry(arg).or_default().push(block_id);
                }
            }
        }

        // terminator uses
        let terminator = tree.get(block.terminator);
        for use_value in terminator
            .uses()
            .into_iter()
            .filter_map(|value| value.value())
        {
            use_blocks.entry(use_value).or_default().push(block_id);
        }
    }

    UseDefMaps {
        use_blocks,
        def_block,
    }
}

/// Build a map from values to their use counts.
pub fn build_value_use_counts(
    function: &mir::Function,
    tree: &mir::Tree,
) -> HashMap<mir::Value, usize> {
    // reuse use def map and count occurrences
    let use_def = build_use_def_maps(function, tree);
    let mut counts = HashMap::new();

    // count uses per value
    for (value, blocks) in use_def.use_blocks {
        counts.insert(value, blocks.len());
    }

    // return the counts
    counts
}

/// Clone instruction metadata while remapping value references.
pub fn clone_instruction_metadata(
    tree: &mut mir::Tree,
    original: mir::LocalNodeId<mir::Instruction>,
    cloned: mir::LocalNodeId<mir::Instruction>,
    value_map: &HashMap<mir::Value, mir::Value>,
) {
    // preserve instruction provenance by default
    if let Some(provenance_id) = tree.get_provenance(original.id) {
        tree.set_provenance(cloned.id, provenance_id);
    }

    // clone memory access metadata
    if let Some(accesses) = tree.metadata.memory.memory_accesses(original) {
        let mut cloned_accesses = accesses.to_vec();
        for access in &mut cloned_accesses {
            if let mir::MemoryAccessTarget::Pointer(value) = access.target
                && let Some(&remapped) = value_map.get(&value)
            {
                access.target = mir::MemoryAccessTarget::Pointer(remapped);
            }
        }

        tree.metadata
            .memory
            .insert_memory_accesses(cloned, cloned_accesses);
    }
}

/// Remap instruction memory access metadata in place using a substitution map.
pub fn remap_instruction_memory_accesses(
    tree: &mut mir::Tree,
    instruction: mir::LocalNodeId<mir::Instruction>,
    substitutions: &HashMap<mir::Value, mir::Value>,
) {
    // skip when no substitutions are provided
    if substitutions.is_empty() {
        return;
    }

    // read existing memory access metadata
    let Some(accesses) = tree.metadata.memory.memory_accesses(instruction) else {
        return;
    };

    let mut updated = accesses.to_vec();
    for access in &mut updated {
        if let mir::MemoryAccessTarget::Pointer(value) = access.target
            && let Some(&remapped) = substitutions.get(&value)
        {
            access.target = mir::MemoryAccessTarget::Pointer(remapped);
        }
    }

    tree.metadata
        .memory
        .insert_memory_accesses(instruction, updated);
}

/// Definition metadata for instructions.
#[derive(Debug, Clone)]
pub struct InstructionRef {
    /// The instruction that defines the value.
    pub instruction: mir::Instruction,
    /// The block containing the instruction.
    pub block: mir::LocalNodeId<mir::Block>,
    /// The instruction index within the block.
    pub index: usize,
}

/// Build a map from values to the instructions that define them.
pub fn build_value_definition_map(
    function: &mir::Function,
    tree: &mir::Tree,
) -> HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>> {
    // collect instruction destinations
    let mut map = HashMap::new();

    // scan blocks for definitions
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if let Some(destination) = instruction.destination().and_then(|value| value.value()) {
                map.insert(destination, instruction_id);
            }
        }
    }

    map
}

/// Build a map from values to their defining blocks.
pub fn build_value_definition_blocks(
    function: &mir::Function,
    tree: &mir::Tree,
) -> HashMap<mir::Value, mir::LocalNodeId<mir::Block>> {
    // collect definition blocks
    let mut map = HashMap::new();

    // scan blocks for definitions
    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        // record block parameters as definitions
        for param in &block.parameters {
            if let Some(value) = param.value.value() {
                map.insert(value, block_id);
            }
        }

        // record instruction definitions
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if let Some(destination) = instruction.destination().and_then(|value| value.value()) {
                map.insert(destination, block_id);
            }
        }
    }

    map
}

/// Build a map from instruction ids to their containing blocks.
pub fn build_instruction_block_map(
    function: &mir::Function,
    tree: &mir::Tree,
) -> HashMap<mir::LocalNodeId<mir::Instruction>, mir::LocalNodeId<mir::Block>> {
    let mut map = HashMap::new();

    // scan blocks for instruction ownership
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            map.insert(instruction_id, block_id);
        }
    }

    map
}

/// Build a map from values to their defining instructions.
pub fn build_value_instruction_map(
    function: &mir::Function,
    tree: &mir::Tree,
) -> HashMap<mir::Value, mir::Instruction> {
    // collect instruction destinations
    let mut map = HashMap::new();

    // scan blocks for definitions
    for &block_id in &function.blocks {
        // read block instructions
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            // record instructions that define a value
            let instruction = tree.get(instruction_id);
            if let Some(destination) = instruction.destination().and_then(|value| value.value()) {
                map.insert(destination, instruction.clone());
            }
        }
    }

    map
}

/// Build a map from values to their defining instruction references.
pub fn build_value_instruction_refs(
    function: &mir::Function,
    tree: &mir::Tree,
) -> HashMap<mir::Value, InstructionRef> {
    // collect instruction references
    let mut map = HashMap::new();

    // scan blocks for definitions
    for &block_id in &function.blocks {
        // read block instructions
        let block = tree.get(block_id);
        for (index, instruction_id) in block.instructions.iter().enumerate() {
            // record instructions that define a value
            let instruction = tree.get(*instruction_id);
            if let Some(destination) = instruction.destination().and_then(|value| value.value()) {
                map.insert(
                    destination,
                    InstructionRef {
                        instruction: instruction.clone(),
                        block: block_id,
                        index,
                    },
                );
            }
        }
    }

    map
}

/// Remap all values in an instruction according to the given map.
///
/// Unlike `instruction_substitute_uses`, this also remaps the destination and
/// handles externalized arguments (Call, Intrinsic, etc.) by creating new
/// argument slices in the tree.
pub fn instruction_map(
    instruction: &mir::Instruction,
    value_map: &HashMap<mir::Value, mir::Value>,
    tree: &mut mir::Tree,
) -> mir::Instruction {
    // remap values through the provided map
    let remap = |value: mir::ValueReference| -> mir::ValueReference {
        let Some(value_id) = value.value() else {
            return value;
        };

        value_map
            .get(&value_id)
            .copied()
            .map(Into::into)
            .unwrap_or(value)
    };

    // rebuild argument slices with remapped values
    let mut remap_arguments = |slice: mir::ArgumentSlice| -> mir::ArgumentSlice {
        // remap argument values
        let new_args: Vec<_> = tree
            .get_arguments(slice)
            .iter()
            .copied()
            .map(remap)
            .collect();

        tree.add_arguments(&new_args)
    };

    // rebuild the instruction with remapped values
    match instruction {
        mir::Instruction::Error => {
            panic!("recovered MIR instruction reached optimizer");
        }
        mir::Instruction::Const { destination, value } => mir::Instruction::Const {
            destination: remap(*destination),
            value: value.clone(),
        },
        mir::Instruction::Binary {
            destination,
            operator,
            left,
            right,
        } => mir::Instruction::Binary {
            destination: remap(*destination),
            operator: *operator,
            left: remap(*left),
            right: remap(*right),
        },
        mir::Instruction::Unary {
            destination,
            operator,
            argument,
        } => mir::Instruction::Unary {
            destination: remap(*destination),
            operator: *operator,
            argument: remap(*argument),
        },
        mir::Instruction::Cast {
            destination,
            operator,
            argument,
            to_type,
        } => mir::Instruction::Cast {
            destination: remap(*destination),
            operator: *operator,
            argument: remap(*argument),
            to_type: *to_type,
        },
        mir::Instruction::Select {
            destination,
            condition,
            then_value,
            else_value,
        } => mir::Instruction::Select {
            destination: remap(*destination),
            condition: remap(*condition),
            then_value: remap(*then_value),
            else_value: remap(*else_value),
        },
        mir::Instruction::Load {
            destination,
            pointer,
            result_type,
        } => mir::Instruction::Load {
            destination: remap(*destination),
            pointer: remap(*pointer),
            result_type: *result_type,
        },
        mir::Instruction::Store { pointer, value } => mir::Instruction::Store {
            pointer: remap(*pointer),
            value: remap(*value),
        },
        mir::Instruction::Pin {
            destination,
            value,
            result_type,
        } => mir::Instruction::Pin {
            destination: remap(*destination),
            value: remap(*value),
            result_type: *result_type,
        },
        mir::Instruction::Unpin { value } => mir::Instruction::Unpin {
            value: remap(*value),
        },
        mir::Instruction::Free { value } => mir::Instruction::Free {
            value: remap(*value),
        },
        mir::Instruction::Drop { place } => mir::Instruction::Drop {
            place: place_substitute_uses(place, value_map),
        },
        mir::Instruction::FieldGet {
            destination,
            aggregate,
            index,
        } => mir::Instruction::FieldGet {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            index: *index,
        },
        mir::Instruction::FieldAddr {
            destination,
            aggregate,
            index,
            result_type,
        } => mir::Instruction::FieldAddr {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            index: *index,
            result_type: *result_type,
        },
        mir::Instruction::FieldSet {
            destination,
            aggregate,
            index,
            value,
        } => mir::Instruction::FieldSet {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            index: *index,
            value: remap(*value),
        },
        mir::Instruction::ElementGet {
            destination,
            array,
            index,
        } => mir::Instruction::ElementGet {
            destination: remap(*destination),
            array: remap(*array),
            index: *index,
        },
        mir::Instruction::ElementAddr {
            destination,
            array,
            index,
            result_type,
        } => mir::Instruction::ElementAddr {
            destination: remap(*destination),
            array: remap(*array),
            index: remap(*index),
            result_type: *result_type,
        },
        mir::Instruction::ElementSet {
            destination,
            array,
            index,
            value,
        } => mir::Instruction::ElementSet {
            destination: remap(*destination),
            array: remap(*array),
            index: *index,
            value: remap(*value),
        },
        mir::Instruction::Slice {
            destination,
            source,
            start,
            length,
            result_type,
        } => mir::Instruction::Slice {
            destination: remap(*destination),
            source: remap(*source),
            start: remap(*start),
            length: remap(*length),
            result_type: *result_type,
        },
        mir::Instruction::LocalGet { destination, local } => mir::Instruction::LocalGet {
            destination: remap(*destination),
            local: *local,
        },
        mir::Instruction::LocalSet { local, value } => mir::Instruction::LocalSet {
            local: *local,
            value: remap(*value),
        },
        mir::Instruction::Assume { condition } => mir::Instruction::Assume {
            condition: remap(*condition),
        },
        mir::Instruction::GlobalAddr {
            destination,
            global,
            result_type,
        } => mir::Instruction::GlobalAddr {
            destination: remap(*destination),
            global: *global,
            result_type: *result_type,
        },
        mir::Instruction::FunctionAddr {
            destination,
            function,
        } => mir::Instruction::FunctionAddr {
            destination: remap(*destination),
            function: *function,
        },
        mir::Instruction::CallableBind {
            destination,
            function,
            environment,
        } => mir::Instruction::CallableBind {
            destination: remap(*destination),
            function: *function,
            environment: remap(*environment),
        },
        mir::Instruction::CallableEnvironment { destination } => {
            mir::Instruction::CallableEnvironment {
                destination: remap(*destination),
            }
        }
        mir::Instruction::LocalAddr {
            destination,
            local,
            result_type,
        } => mir::Instruction::LocalAddr {
            destination: remap(*destination),
            local: *local,
            result_type: *result_type,
        },
        mir::Instruction::Struct {
            destination,
            ty,
            fields,
        } => mir::Instruction::Struct {
            destination: remap(*destination),
            ty: *ty,
            fields: remap_arguments(*fields),
        },
        mir::Instruction::Tuple {
            destination,
            ty,
            elements,
        } => mir::Instruction::Tuple {
            destination: remap(*destination),
            ty: *ty,
            elements: remap_arguments(*elements),
        },
        mir::Instruction::Array {
            destination,
            ty,
            elements,
        } => mir::Instruction::Array {
            destination: remap(*destination),
            ty: *ty,
            elements: remap_arguments(*elements),
        },
        mir::Instruction::VectorSplat { destination, value } => mir::Instruction::VectorSplat {
            destination: remap(*destination),
            value: remap(*value),
        },
        mir::Instruction::VectorExtract {
            destination,
            vector,
            index,
        } => mir::Instruction::VectorExtract {
            destination: remap(*destination),
            vector: remap(*vector),
            index: remap(*index),
        },
        mir::Instruction::VectorInsert {
            destination,
            vector,
            index,
            value,
        } => mir::Instruction::VectorInsert {
            destination: remap(*destination),
            vector: remap(*vector),
            index: remap(*index),
            value: remap(*value),
        },
        mir::Instruction::VectorShuffle {
            destination,
            left,
            right,
            mask,
        } => mir::Instruction::VectorShuffle {
            destination: remap(*destination),
            left: remap(*left),
            right: remap(*right),
            mask: mask.clone(),
        },
        mir::Instruction::VectorSelect {
            destination,
            mask,
            then_value,
            else_value,
        } => mir::Instruction::VectorSelect {
            destination: remap(*destination),
            mask: remap(*mask),
            then_value: remap(*then_value),
            else_value: remap(*else_value),
        },
        mir::Instruction::VectorReduce {
            destination,
            operator,
            vector,
        } => mir::Instruction::VectorReduce {
            destination: remap(*destination),
            operator: *operator,
            vector: remap(*vector),
        },
        mir::Instruction::VectorCompare {
            destination,
            operator,
            left,
            right,
        } => mir::Instruction::VectorCompare {
            destination: remap(*destination),
            operator: *operator,
            left: remap(*left),
            right: remap(*right),
        },
        mir::Instruction::VectorConvert {
            destination,
            mode,
            vector,
        } => mir::Instruction::VectorConvert {
            destination: remap(*destination),
            mode: *mode,
            vector: remap(*vector),
        },
        mir::Instruction::TensorSplat { destination, value } => mir::Instruction::TensorSplat {
            destination: remap(*destination),
            value: remap(*value),
        },
        mir::Instruction::TensorLoad {
            destination,
            view,
            indices,
        } => mir::Instruction::TensorLoad {
            destination: remap(*destination),
            view: remap(*view),
            indices: remap_arguments(*indices),
        },
        mir::Instruction::TensorExtract {
            destination,
            tensor,
            indices,
        } => mir::Instruction::TensorExtract {
            destination: remap(*destination),
            tensor: remap(*tensor),
            indices: remap_arguments(*indices),
        },
        mir::Instruction::TensorStore {
            view,
            indices,
            value,
        } => mir::Instruction::TensorStore {
            view: remap(*view),
            indices: remap_arguments(*indices),
            value: remap(*value),
        },
        mir::Instruction::TensorFill { view, value } => mir::Instruction::TensorFill {
            view: remap(*view),
            value: remap(*value),
        },
        mir::Instruction::TensorCopy { target, source } => mir::Instruction::TensorCopy {
            target: remap(*target),
            source: remap(*source),
        },
        mir::Instruction::TensorReshape {
            destination,
            tensor,
            shape,
        } => mir::Instruction::TensorReshape {
            destination: remap(*destination),
            tensor: remap(*tensor),
            shape: remap_arguments(*shape),
        },
        mir::Instruction::TensorBroadcast {
            destination,
            tensor,
            dimensions,
        } => mir::Instruction::TensorBroadcast {
            destination: remap(*destination),
            tensor: remap(*tensor),
            dimensions: dimensions.clone(),
        },
        mir::Instruction::TensorTranspose {
            destination,
            tensor,
            permutation,
        } => mir::Instruction::TensorTranspose {
            destination: remap(*destination),
            tensor: remap(*tensor),
            permutation: permutation.clone(),
        },
        mir::Instruction::TensorCast {
            destination,
            tensor,
        } => mir::Instruction::TensorCast {
            destination: remap(*destination),
            tensor: remap(*tensor),
        },
        mir::Instruction::TensorView {
            destination,
            view,
            arguments,
            offsets_count,
            sizes_count,
            strides_count,
        } => mir::Instruction::TensorView {
            destination: remap(*destination),
            view: remap(*view),
            arguments: remap_arguments(*arguments),
            offsets_count: *offsets_count,
            sizes_count: *sizes_count,
            strides_count: *strides_count,
        },
        mir::Instruction::TensorSlice {
            destination,
            tensor,
            arguments,
            offsets_count,
            sizes_count,
            strides_count,
        } => mir::Instruction::TensorSlice {
            destination: remap(*destination),
            tensor: remap(*tensor),
            arguments: remap_arguments(*arguments),
            offsets_count: *offsets_count,
            sizes_count: *sizes_count,
            strides_count: *strides_count,
        },
        mir::Instruction::TensorPad {
            destination,
            tensor,
            arguments,
            low_count,
            high_count,
            interior_count,
            value,
        } => mir::Instruction::TensorPad {
            destination: remap(*destination),
            tensor: remap(*tensor),
            arguments: remap_arguments(*arguments),
            low_count: *low_count,
            high_count: *high_count,
            interior_count: *interior_count,
            value: remap(*value),
        },
        mir::Instruction::TensorConcat {
            destination,
            tensors,
            axis,
        } => mir::Instruction::TensorConcat {
            destination: remap(*destination),
            tensors: remap_arguments(*tensors),
            axis: *axis,
        },
        mir::Instruction::TensorReduce {
            destination,
            operator,
            tensor,
            initial,
            axes,
        } => mir::Instruction::TensorReduce {
            destination: remap(*destination),
            operator: *operator,
            tensor: remap(*tensor),
            initial: remap(*initial),
            axes: axes.clone(),
        },
        mir::Instruction::TensorIndexReduce {
            destination,
            operator,
            tensor,
            axis,
            tie_break,
        } => mir::Instruction::TensorIndexReduce {
            destination: remap(*destination),
            operator: *operator,
            tensor: remap(*tensor),
            axis: *axis,
            tie_break: *tie_break,
        },
        mir::Instruction::TensorDot {
            destination,
            left,
            right,
            dimensions,
        } => mir::Instruction::TensorDot {
            destination: remap(*destination),
            left: remap(*left),
            right: remap(*right),
            dimensions: dimensions.clone(),
        },
        mir::Instruction::TensorConvolution {
            destination,
            input,
            kernel,
            dimensions,
            window,
            feature_group_count,
            batch_group_count,
        } => mir::Instruction::TensorConvolution {
            destination: remap(*destination),
            input: remap(*input),
            kernel: remap(*kernel),
            dimensions: dimensions.clone(),
            window: window.clone(),
            feature_group_count: *feature_group_count,
            batch_group_count: *batch_group_count,
        },
        mir::Instruction::TensorGather {
            destination,
            operand,
            indices,
            dimensions,
            slice_sizes,
        } => mir::Instruction::TensorGather {
            destination: remap(*destination),
            operand: remap(*operand),
            indices: remap(*indices),
            dimensions: dimensions.clone(),
            slice_sizes: slice_sizes.clone(),
        },
        mir::Instruction::TensorScatter {
            destination,
            operand,
            indices,
            updates,
            dimensions,
            mode,
        } => mir::Instruction::TensorScatter {
            destination: remap(*destination),
            operand: remap(*operand),
            indices: remap(*indices),
            updates: remap(*updates),
            dimensions: dimensions.clone(),
            mode: *mode,
        },
        mir::Instruction::TensorCompare {
            destination,
            operator,
            left,
            right,
        } => mir::Instruction::TensorCompare {
            destination: remap(*destination),
            operator: *operator,
            left: remap(*left),
            right: remap(*right),
        },
        mir::Instruction::TensorSelect {
            destination,
            mask,
            then_value,
            else_value,
        } => mir::Instruction::TensorSelect {
            destination: remap(*destination),
            mask: remap(*mask),
            then_value: remap(*then_value),
            else_value: remap(*else_value),
        },
        mir::Instruction::TensorConvert {
            destination,
            mode,
            tensor,
        } => mir::Instruction::TensorConvert {
            destination: remap(*destination),
            mode: *mode,
            tensor: remap(*tensor),
        },
        mir::Instruction::Call {
            destination,
            function,
            call,
        } => mir::Instruction::Call {
            destination: destination.map(remap),
            function: *function,
            call: clone_call_with_arguments(call, remap_arguments(call.arguments)),
        },
        mir::Instruction::CallClass {
            destination,
            receiver,
            call,
            declaring_type,
            slot,
            declared_target,
        } => mir::Instruction::CallClass {
            destination: destination.map(remap),
            receiver: remap(*receiver),
            call: clone_call_with_arguments(call, remap_arguments(call.arguments)),
            declaring_type: *declaring_type,
            slot: *slot,
            declared_target: *declared_target,
        },
        mir::Instruction::CallInterface {
            destination,
            receiver,
            call,
            declaring_type,
            slot,
        } => mir::Instruction::CallInterface {
            destination: destination.map(remap),
            receiver: remap(*receiver),
            call: clone_call_with_arguments(call, remap_arguments(call.arguments)),
            declaring_type: *declaring_type,
            slot: *slot,
        },
        mir::Instruction::CallIndirect {
            destination,
            callee,
            call,
        } => mir::Instruction::CallIndirect {
            destination: destination.map(remap),
            callee: remap(*callee),
            call: clone_call_with_arguments(call, remap_arguments(call.arguments)),
        },
        mir::Instruction::New {
            destination,
            layout,
            result_type,
        } => mir::Instruction::New {
            destination: remap(*destination),
            layout: *layout,
            result_type: *result_type,
        },
        mir::Instruction::NewSlice {
            destination,
            element,
            length,
            result_type,
        } => mir::Instruction::NewSlice {
            destination: remap(*destination),
            element: *element,
            length: remap(*length),
            result_type: *result_type,
        },
        mir::Instruction::RawAlloc {
            destination,
            layout,
            result_type,
        } => mir::Instruction::RawAlloc {
            destination: remap(*destination),
            layout: *layout,
            result_type: *result_type,
        },
        mir::Instruction::RawFree { pointer } => mir::Instruction::RawFree {
            pointer: remap(*pointer),
        },
        mir::Instruction::StackAlloc {
            destination,
            layout,
            result_type,
        } => mir::Instruction::StackAlloc {
            destination: remap(*destination),
            layout: *layout,
            result_type: *result_type,
        },
        mir::Instruction::Intrinsic {
            destination,
            intrinsic,
            arguments,
        } => mir::Instruction::Intrinsic {
            destination: destination.map(remap),
            intrinsic: *intrinsic,
            arguments: remap_arguments(*arguments),
        },
        mir::Instruction::AtomicLoad {
            destination,
            pointer,
            result_type,
            access,
        } => mir::Instruction::AtomicLoad {
            destination: remap(*destination),
            pointer: remap(*pointer),
            result_type: *result_type,
            access: *access,
        },
        mir::Instruction::AtomicStore {
            pointer,
            value,
            access,
        } => mir::Instruction::AtomicStore {
            pointer: remap(*pointer),
            value: remap(*value),
            access: *access,
        },
        mir::Instruction::AtomicCompareExchange {
            destination,
            pointer,
            expected,
            new_value,
            is_weak,
            access,
        } => mir::Instruction::AtomicCompareExchange {
            destination: remap(*destination),
            pointer: remap(*pointer),
            expected: remap(*expected),
            new_value: remap(*new_value),
            is_weak: *is_weak,
            access: *access,
        },
        mir::Instruction::AtomicRmw {
            destination,
            operator,
            pointer,
            value,
            access,
        } => mir::Instruction::AtomicRmw {
            destination: remap(*destination),
            operator: *operator,
            pointer: remap(*pointer),
            value: remap(*value),
            access: *access,
        },
        mir::Instruction::AtomicFence { access } => {
            mir::Instruction::AtomicFence { access: *access }
        }
        mir::Instruction::BarrierWrite {
            object,
            offset,
            byte_len,
        } => mir::Instruction::BarrierWrite {
            object: remap(*object),
            offset: remap(*offset),
            byte_len: remap(*byte_len),
        },
    }
}

/// Remap values and locals in an instruction.
///
/// This is similar to `instruction_map` but also remaps local ids.
pub fn instruction_map_with_locals(
    instruction: &mir::Instruction,
    value_map: &HashMap<mir::Value, mir::Value>,
    local_map: &HashMap<mir::LocalNodeId<mir::Local>, mir::LocalNodeId<mir::Local>>,
    tree: &mut mir::Tree,
) -> mir::Instruction {
    // create a value remapper for simple value uses
    let remap = |value: mir::ValueReference| -> mir::ValueReference {
        let Some(value_id) = value.value() else {
            return value;
        };

        value_map
            .get(&value_id)
            .copied()
            .map(Into::into)
            .unwrap_or(value)
    };

    // create a local remapper for direct local references
    let remap_local = |local: mir::LocalReference| -> mir::LocalReference {
        let Some(local_id) = local.local() else {
            return local;
        };

        local_map
            .get(&local_id)
            .copied()
            .map(Into::into)
            .unwrap_or(local)
    };

    // remap argument slices into a new argument buffer entry
    let mut remap_arguments = |slice: mir::ArgumentSlice| -> mir::ArgumentSlice {
        let new_args: Vec<_> = tree
            .get_arguments(slice)
            .iter()
            .copied()
            .map(remap)
            .collect();
        tree.add_arguments(&new_args)
    };

    // remap each instruction variant
    match instruction {
        mir::Instruction::Error => {
            panic!("recovered MIR instruction reached optimizer");
        }
        mir::Instruction::Const { destination, value } => mir::Instruction::Const {
            destination: remap(*destination),
            value: value.clone(),
        },
        mir::Instruction::Binary {
            destination,
            operator,
            left,
            right,
        } => mir::Instruction::Binary {
            destination: remap(*destination),
            operator: *operator,
            left: remap(*left),
            right: remap(*right),
        },
        mir::Instruction::Unary {
            destination,
            operator,
            argument,
        } => mir::Instruction::Unary {
            destination: remap(*destination),
            operator: *operator,
            argument: remap(*argument),
        },
        mir::Instruction::Cast {
            destination,
            operator,
            argument,
            to_type,
        } => mir::Instruction::Cast {
            destination: remap(*destination),
            operator: *operator,
            argument: remap(*argument),
            to_type: *to_type,
        },
        mir::Instruction::Select {
            destination,
            condition,
            then_value,
            else_value,
        } => mir::Instruction::Select {
            destination: remap(*destination),
            condition: remap(*condition),
            then_value: remap(*then_value),
            else_value: remap(*else_value),
        },
        mir::Instruction::Load {
            destination,
            pointer,
            result_type,
        } => mir::Instruction::Load {
            destination: remap(*destination),
            pointer: remap(*pointer),
            result_type: *result_type,
        },
        mir::Instruction::Store { pointer, value } => mir::Instruction::Store {
            pointer: remap(*pointer),
            value: remap(*value),
        },
        mir::Instruction::LocalGet { destination, local } => mir::Instruction::LocalGet {
            destination: remap(*destination),
            local: remap_local(*local),
        },
        mir::Instruction::LocalSet { local, value } => mir::Instruction::LocalSet {
            local: remap_local(*local),
            value: remap(*value),
        },
        mir::Instruction::Assume { condition } => mir::Instruction::Assume {
            condition: remap(*condition),
        },
        mir::Instruction::GlobalAddr {
            destination,
            global,
            result_type,
        } => mir::Instruction::GlobalAddr {
            destination: remap(*destination),
            global: *global,
            result_type: *result_type,
        },
        mir::Instruction::FunctionAddr {
            destination,
            function,
        } => mir::Instruction::FunctionAddr {
            destination: remap(*destination),
            function: *function,
        },
        mir::Instruction::CallableBind {
            destination,
            function,
            environment,
        } => mir::Instruction::CallableBind {
            destination: remap(*destination),
            function: *function,
            environment: remap(*environment),
        },
        mir::Instruction::CallableEnvironment { destination } => {
            mir::Instruction::CallableEnvironment {
                destination: remap(*destination),
            }
        }
        mir::Instruction::LocalAddr {
            destination,
            local,
            result_type,
        } => mir::Instruction::LocalAddr {
            destination: remap(*destination),
            local: *local,
            result_type: *result_type,
        },
        mir::Instruction::Struct {
            destination,
            ty,
            fields,
        } => mir::Instruction::Struct {
            destination: remap(*destination),
            ty: *ty,
            fields: remap_arguments(*fields),
        },
        mir::Instruction::Tuple {
            destination,
            ty,
            elements,
        } => mir::Instruction::Tuple {
            destination: remap(*destination),
            ty: *ty,
            elements: remap_arguments(*elements),
        },
        mir::Instruction::Array {
            destination,
            ty,
            elements,
        } => mir::Instruction::Array {
            destination: remap(*destination),
            ty: *ty,
            elements: remap_arguments(*elements),
        },
        mir::Instruction::VectorSplat { destination, value } => mir::Instruction::VectorSplat {
            destination: remap(*destination),
            value: remap(*value),
        },
        mir::Instruction::VectorExtract {
            destination,
            vector,
            index,
        } => mir::Instruction::VectorExtract {
            destination: remap(*destination),
            vector: remap(*vector),
            index: remap(*index),
        },
        mir::Instruction::VectorInsert {
            destination,
            vector,
            index,
            value,
        } => mir::Instruction::VectorInsert {
            destination: remap(*destination),
            vector: remap(*vector),
            index: remap(*index),
            value: remap(*value),
        },
        mir::Instruction::VectorShuffle {
            destination,
            left,
            right,
            mask,
        } => mir::Instruction::VectorShuffle {
            destination: remap(*destination),
            left: remap(*left),
            right: remap(*right),
            mask: mask.clone(),
        },
        mir::Instruction::VectorSelect {
            destination,
            mask,
            then_value,
            else_value,
        } => mir::Instruction::VectorSelect {
            destination: remap(*destination),
            mask: remap(*mask),
            then_value: remap(*then_value),
            else_value: remap(*else_value),
        },
        mir::Instruction::VectorReduce {
            destination,
            operator,
            vector,
        } => mir::Instruction::VectorReduce {
            destination: remap(*destination),
            operator: *operator,
            vector: remap(*vector),
        },
        mir::Instruction::VectorCompare {
            destination,
            operator,
            left,
            right,
        } => mir::Instruction::VectorCompare {
            destination: remap(*destination),
            operator: *operator,
            left: remap(*left),
            right: remap(*right),
        },
        mir::Instruction::VectorConvert {
            destination,
            mode,
            vector,
        } => mir::Instruction::VectorConvert {
            destination: remap(*destination),
            mode: *mode,
            vector: remap(*vector),
        },
        mir::Instruction::TensorSplat { destination, value } => mir::Instruction::TensorSplat {
            destination: remap(*destination),
            value: remap(*value),
        },
        mir::Instruction::TensorLoad {
            destination,
            view,
            indices,
        } => mir::Instruction::TensorLoad {
            destination: remap(*destination),
            view: remap(*view),
            indices: remap_arguments(*indices),
        },
        mir::Instruction::TensorExtract {
            destination,
            tensor,
            indices,
        } => mir::Instruction::TensorExtract {
            destination: remap(*destination),
            tensor: remap(*tensor),
            indices: remap_arguments(*indices),
        },
        mir::Instruction::TensorStore {
            view,
            indices,
            value,
        } => mir::Instruction::TensorStore {
            view: remap(*view),
            indices: remap_arguments(*indices),
            value: remap(*value),
        },
        mir::Instruction::TensorFill { view, value } => mir::Instruction::TensorFill {
            view: remap(*view),
            value: remap(*value),
        },
        mir::Instruction::TensorCopy { target, source } => mir::Instruction::TensorCopy {
            target: remap(*target),
            source: remap(*source),
        },
        mir::Instruction::TensorReshape {
            destination,
            tensor,
            shape,
        } => mir::Instruction::TensorReshape {
            destination: remap(*destination),
            tensor: remap(*tensor),
            shape: remap_arguments(*shape),
        },
        mir::Instruction::TensorBroadcast {
            destination,
            tensor,
            dimensions,
        } => mir::Instruction::TensorBroadcast {
            destination: remap(*destination),
            tensor: remap(*tensor),
            dimensions: dimensions.clone(),
        },
        mir::Instruction::TensorTranspose {
            destination,
            tensor,
            permutation,
        } => mir::Instruction::TensorTranspose {
            destination: remap(*destination),
            tensor: remap(*tensor),
            permutation: permutation.clone(),
        },
        mir::Instruction::TensorCast {
            destination,
            tensor,
        } => mir::Instruction::TensorCast {
            destination: remap(*destination),
            tensor: remap(*tensor),
        },
        mir::Instruction::TensorView {
            destination,
            view,
            arguments,
            offsets_count,
            sizes_count,
            strides_count,
        } => mir::Instruction::TensorView {
            destination: remap(*destination),
            view: remap(*view),
            arguments: remap_arguments(*arguments),
            offsets_count: *offsets_count,
            sizes_count: *sizes_count,
            strides_count: *strides_count,
        },
        mir::Instruction::TensorSlice {
            destination,
            tensor,
            arguments,
            offsets_count,
            sizes_count,
            strides_count,
        } => mir::Instruction::TensorSlice {
            destination: remap(*destination),
            tensor: remap(*tensor),
            arguments: remap_arguments(*arguments),
            offsets_count: *offsets_count,
            sizes_count: *sizes_count,
            strides_count: *strides_count,
        },
        mir::Instruction::TensorPad {
            destination,
            tensor,
            arguments,
            low_count,
            high_count,
            interior_count,
            value,
        } => mir::Instruction::TensorPad {
            destination: remap(*destination),
            tensor: remap(*tensor),
            arguments: remap_arguments(*arguments),
            low_count: *low_count,
            high_count: *high_count,
            interior_count: *interior_count,
            value: remap(*value),
        },
        mir::Instruction::TensorConcat {
            destination,
            tensors,
            axis,
        } => mir::Instruction::TensorConcat {
            destination: remap(*destination),
            tensors: remap_arguments(*tensors),
            axis: *axis,
        },
        mir::Instruction::TensorReduce {
            destination,
            operator,
            tensor,
            initial,
            axes,
        } => mir::Instruction::TensorReduce {
            destination: remap(*destination),
            operator: *operator,
            tensor: remap(*tensor),
            initial: remap(*initial),
            axes: axes.clone(),
        },
        mir::Instruction::TensorIndexReduce {
            destination,
            operator,
            tensor,
            axis,
            tie_break,
        } => mir::Instruction::TensorIndexReduce {
            destination: remap(*destination),
            operator: *operator,
            tensor: remap(*tensor),
            axis: *axis,
            tie_break: *tie_break,
        },
        mir::Instruction::TensorDot {
            destination,
            left,
            right,
            dimensions,
        } => mir::Instruction::TensorDot {
            destination: remap(*destination),
            left: remap(*left),
            right: remap(*right),
            dimensions: dimensions.clone(),
        },
        mir::Instruction::TensorConvolution {
            destination,
            input,
            kernel,
            dimensions,
            window,
            feature_group_count,
            batch_group_count,
        } => mir::Instruction::TensorConvolution {
            destination: remap(*destination),
            input: remap(*input),
            kernel: remap(*kernel),
            dimensions: dimensions.clone(),
            window: window.clone(),
            feature_group_count: *feature_group_count,
            batch_group_count: *batch_group_count,
        },
        mir::Instruction::TensorGather {
            destination,
            operand,
            indices,
            dimensions,
            slice_sizes,
        } => mir::Instruction::TensorGather {
            destination: remap(*destination),
            operand: remap(*operand),
            indices: remap(*indices),
            dimensions: dimensions.clone(),
            slice_sizes: slice_sizes.clone(),
        },
        mir::Instruction::TensorScatter {
            destination,
            operand,
            indices,
            updates,
            dimensions,
            mode,
        } => mir::Instruction::TensorScatter {
            destination: remap(*destination),
            operand: remap(*operand),
            indices: remap(*indices),
            updates: remap(*updates),
            dimensions: dimensions.clone(),
            mode: *mode,
        },
        mir::Instruction::TensorCompare {
            destination,
            operator,
            left,
            right,
        } => mir::Instruction::TensorCompare {
            destination: remap(*destination),
            operator: *operator,
            left: remap(*left),
            right: remap(*right),
        },
        mir::Instruction::TensorSelect {
            destination,
            mask,
            then_value,
            else_value,
        } => mir::Instruction::TensorSelect {
            destination: remap(*destination),
            mask: remap(*mask),
            then_value: remap(*then_value),
            else_value: remap(*else_value),
        },
        mir::Instruction::TensorConvert {
            destination,
            mode,
            tensor,
        } => mir::Instruction::TensorConvert {
            destination: remap(*destination),
            mode: *mode,
            tensor: remap(*tensor),
        },
        mir::Instruction::FieldGet {
            destination,
            aggregate,
            index,
        } => mir::Instruction::FieldGet {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            index: *index,
        },
        mir::Instruction::FieldAddr {
            destination,
            aggregate,
            index,
            result_type,
        } => mir::Instruction::FieldAddr {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            index: *index,
            result_type: *result_type,
        },
        mir::Instruction::FieldSet {
            destination,
            aggregate,
            index,
            value,
        } => mir::Instruction::FieldSet {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            index: *index,
            value: remap(*value),
        },
        mir::Instruction::ElementGet {
            destination,
            array,
            index,
        } => mir::Instruction::ElementGet {
            destination: remap(*destination),
            array: remap(*array),
            index: *index,
        },
        mir::Instruction::ElementAddr {
            destination,
            array,
            index,
            result_type,
        } => mir::Instruction::ElementAddr {
            destination: remap(*destination),
            array: remap(*array),
            index: remap(*index),
            result_type: *result_type,
        },
        mir::Instruction::ElementSet {
            destination,
            array,
            index,
            value,
        } => mir::Instruction::ElementSet {
            destination: remap(*destination),
            array: remap(*array),
            index: *index,
            value: remap(*value),
        },
        mir::Instruction::Slice {
            destination,
            source,
            start,
            length,
            result_type,
        } => mir::Instruction::Slice {
            destination: remap(*destination),
            source: remap(*source),
            start: remap(*start),
            length: remap(*length),
            result_type: *result_type,
        },
        mir::Instruction::New {
            destination,
            layout,
            result_type,
        } => mir::Instruction::New {
            destination: remap(*destination),
            layout: *layout,
            result_type: *result_type,
        },
        mir::Instruction::NewSlice {
            destination,
            element,
            length,
            result_type,
        } => mir::Instruction::NewSlice {
            destination: remap(*destination),
            element: *element,
            length: remap(*length),
            result_type: *result_type,
        },
        mir::Instruction::RawAlloc {
            destination,
            layout,
            result_type,
        } => mir::Instruction::RawAlloc {
            destination: remap(*destination),
            layout: *layout,
            result_type: *result_type,
        },
        mir::Instruction::RawFree { pointer } => mir::Instruction::RawFree {
            pointer: remap(*pointer),
        },
        mir::Instruction::Pin {
            destination,
            value,
            result_type,
        } => mir::Instruction::Pin {
            destination: remap(*destination),
            value: remap(*value),
            result_type: *result_type,
        },
        mir::Instruction::Unpin { value } => mir::Instruction::Unpin {
            value: remap(*value),
        },
        mir::Instruction::Free { value } => mir::Instruction::Free {
            value: remap(*value),
        },
        mir::Instruction::Drop { place } => mir::Instruction::Drop {
            place: place_map_values_and_locals(place, value_map, local_map),
        },
        mir::Instruction::StackAlloc {
            destination,
            layout,
            result_type,
        } => mir::Instruction::StackAlloc {
            destination: remap(*destination),
            layout: *layout,
            result_type: *result_type,
        },
        mir::Instruction::Call {
            destination,
            function,
            call,
        } => mir::Instruction::Call {
            destination: destination.map(remap),
            function: *function,
            call: clone_call_with_arguments(call, remap_arguments(call.arguments)),
        },
        mir::Instruction::CallClass {
            destination,
            receiver,
            call,
            declaring_type,
            slot,
            declared_target,
        } => mir::Instruction::CallClass {
            destination: destination.map(remap),
            receiver: remap(*receiver),
            call: clone_call_with_arguments(call, remap_arguments(call.arguments)),
            declaring_type: *declaring_type,
            slot: *slot,
            declared_target: *declared_target,
        },
        mir::Instruction::CallInterface {
            destination,
            receiver,
            call,
            declaring_type,
            slot,
        } => mir::Instruction::CallInterface {
            destination: destination.map(remap),
            receiver: remap(*receiver),
            call: clone_call_with_arguments(call, remap_arguments(call.arguments)),
            declaring_type: *declaring_type,
            slot: *slot,
        },
        mir::Instruction::CallIndirect {
            destination,
            callee,
            call,
        } => mir::Instruction::CallIndirect {
            destination: destination.map(remap),
            callee: remap(*callee),
            call: clone_call_with_arguments(call, remap_arguments(call.arguments)),
        },
        mir::Instruction::Intrinsic {
            destination,
            intrinsic,
            arguments,
        } => mir::Instruction::Intrinsic {
            destination: destination.map(remap),
            intrinsic: *intrinsic,
            arguments: remap_arguments(*arguments),
        },
        mir::Instruction::AtomicLoad {
            destination,
            pointer,
            result_type,
            access,
        } => mir::Instruction::AtomicLoad {
            destination: remap(*destination),
            pointer: remap(*pointer),
            result_type: *result_type,
            access: *access,
        },
        mir::Instruction::AtomicStore {
            pointer,
            value,
            access,
        } => mir::Instruction::AtomicStore {
            pointer: remap(*pointer),
            value: remap(*value),
            access: *access,
        },
        mir::Instruction::AtomicCompareExchange {
            destination,
            pointer,
            expected,
            new_value,
            is_weak,
            access,
        } => mir::Instruction::AtomicCompareExchange {
            destination: remap(*destination),
            pointer: remap(*pointer),
            expected: remap(*expected),
            new_value: remap(*new_value),
            is_weak: *is_weak,
            access: *access,
        },
        mir::Instruction::AtomicRmw {
            destination,
            operator,
            pointer,
            value,
            access,
        } => mir::Instruction::AtomicRmw {
            destination: remap(*destination),
            operator: *operator,
            pointer: remap(*pointer),
            value: remap(*value),
            access: *access,
        },
        mir::Instruction::AtomicFence { access } => {
            mir::Instruction::AtomicFence { access: *access }
        }
        mir::Instruction::BarrierWrite {
            object,
            offset,
            byte_len,
        } => mir::Instruction::BarrierWrite {
            object: remap(*object),
            offset: remap(*offset),
            byte_len: remap(*byte_len),
        },
    }
}

/// Remap block targets and values in a terminator.
///
/// Block targets are remapped according to `block_map`, and values are remapped
/// according to `value_map`. Values/blocks not in the maps are left unchanged.
pub fn terminator_remap(
    terminator: &mut mir::Terminator,
    block_map: &HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
    value_map: &HashMap<mir::Value, mir::Value>,
) {
    // remap one block target in place
    let remap_target = |target: &mut mir::BlockTarget| {
        let Some(block) = target.block.block() else {
            return;
        };

        if let Some(&remapped_block) = block_map.get(&block) {
            target.block = remapped_block.into();
        }
    };

    // remap one value reference in place
    let remap_value = |value: &mut mir::ValueReference| {
        let Some(concrete_value) = value.value() else {
            return;
        };

        if let Some(&remapped_value) = value_map.get(&concrete_value) {
            *value = remapped_value.into();
        }
    };

    // remap a list of block arguments
    let remap_args = |args: &mut Vec<mir::ValueReference>| {
        for arg in args.iter_mut() {
            remap_value(arg);
        }
    };

    // remap terminator fields
    match terminator {
        mir::Terminator::Error => {
            panic!("recovered MIR terminator reached optimizer");
        }
        mir::Terminator::Jump { target } => {
            remap_target(target);
            remap_args(&mut target.arguments);
        }
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => {
            remap_value(condition);
            remap_target(then_target);
            remap_args(&mut then_target.arguments);
            remap_target(else_target);
            remap_args(&mut else_target.arguments);
        }
        mir::Terminator::Check {
            constraint,
            success,
            failure,
        } => {
            remap_target(success);
            remap_args(&mut success.arguments);
            remap_target(failure);
            remap_args(&mut failure.arguments);
            match constraint {
                mir::CheckConstraint::Bounds {
                    index,
                    length,
                    collection,
                    ..
                } => {
                    remap_value(index);
                    remap_value(length);
                    remap_value(collection);
                }
                mir::CheckConstraint::Null { value } => {
                    remap_value(value);
                }
                mir::CheckConstraint::DivZero { divisor } => {
                    remap_value(divisor);
                }
                mir::CheckConstraint::ShiftRange { value, .. } => {
                    remap_value(value);
                }
                mir::CheckConstraint::Narrow { value, .. } => {
                    remap_value(value);
                }
                mir::CheckConstraint::Overflow { left, right, .. } => {
                    remap_value(left);
                    remap_value(right);
                }
                mir::CheckConstraint::Type { value, .. } => {
                    remap_value(value);
                }
                mir::CheckConstraint::Union { value, .. } => {
                    remap_value(value);
                }
                mir::CheckConstraint::ReceiverType { receiver, .. } => {
                    remap_value(receiver);
                }
                mir::CheckConstraint::Implements { receiver, .. } => {
                    remap_value(receiver);
                }
            }
        }
        mir::Terminator::Switch {
            value,
            default,
            cases,
        } => {
            remap_value(value);
            remap_target(default);
            remap_args(&mut default.arguments);
            for case in cases.iter_mut() {
                remap_target(&mut case.target);
                remap_args(&mut case.target.arguments);
            }
        }
        mir::Terminator::Return { value } => {
            if let Some(v) = value {
                remap_value(v);
            }
        }
        mir::Terminator::Yield { value, resume } => {
            remap_value(value);
            remap_target(resume);
            remap_args(&mut resume.arguments);
        }
        mir::Terminator::Call { call, target, .. } => {
            remap_args(&mut call.arguments);
            remap_target(target);
            remap_args(&mut target.arguments);
        }
        mir::Terminator::CallIndirect {
            callee,
            call,
            target,
            ..
        } => {
            remap_value(callee);
            remap_args(&mut call.arguments);
            remap_target(target);
            remap_args(&mut target.arguments);
        }
        mir::Terminator::CallClass {
            receiver,
            call,
            target,
            ..
        } => {
            remap_value(receiver);
            remap_args(&mut call.arguments);
            remap_target(target);
            remap_args(&mut target.arguments);
        }
        mir::Terminator::CallInterface {
            receiver,
            call,
            target,
            ..
        } => {
            remap_value(receiver);
            remap_args(&mut call.arguments);
            remap_target(target);
            remap_args(&mut target.arguments);
        }
        mir::Terminator::Trap { payload, .. } => {
            if let Some(payload) = payload {
                remap_value(payload);
            }
        }
        mir::Terminator::Unreachable => {}
        mir::Terminator::TailCall {
            function: _, call, ..
        } => {
            remap_args(&mut call.arguments);
        }
        mir::Terminator::TailCallClass { receiver, call, .. }
        | mir::Terminator::TailCallInterface { receiver, call, .. } => {
            remap_value(receiver);
            remap_args(&mut call.arguments);
        }
        mir::Terminator::TailCallIndirect { callee, call, .. } => {
            remap_value(callee);
            remap_args(&mut call.arguments);
        }
    }
}
