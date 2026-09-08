use destack_core::{FxIndexMap, FxIndexSet};

use crate as mir;
use crate::{MemoryNode, MemoryTable, terminator_substitute_uses};

/// Return whether an instruction depends only on its operands.
pub fn instruction_is_pure(instruction: &mir::Instruction) -> bool {
    // classify instructions by purity
    match instruction {
        mir::Instruction::Error => {
            panic!("recovered MIR instruction reached optimizer");
        }

        // pure computations
        mir::Instruction::Const { .. }
        | mir::Instruction::Binary { .. }
        | mir::Instruction::Unary { .. }
        | mir::Instruction::Cast { .. }
        | mir::Instruction::Select { .. }
        | mir::Instruction::NewComplete { .. }
        | mir::Instruction::Assume { .. } => true,

        // pure aggregate operations
        mir::Instruction::Aggregate { .. }
        | mir::Instruction::FieldGet { .. }
        | mir::Instruction::FieldSet { .. }
        | mir::Instruction::ElementGet { .. }
        | mir::Instruction::ElementSet { .. }
        | mir::Instruction::VariantNew { .. }
        | mir::Instruction::VariantTag { .. }
        | mir::Instruction::VariantPayload { .. }
        | mir::Instruction::SliceView { .. }
        | mir::Instruction::SliceLength { .. }
        | mir::Instruction::DynamicBind { .. }
        | mir::Instruction::DynamicPayload { .. }
        | mir::Instruction::DynamicType { .. }
        | mir::Instruction::DynamicRead { .. }
        | mir::Instruction::DynamicFind { .. }
        | mir::Instruction::VectorSplat { .. }
        | mir::Instruction::VectorExtract { .. }
        | mir::Instruction::VectorInsert { .. }
        | mir::Instruction::VectorShuffle { .. }
        | mir::Instruction::VectorSelect { .. }
        | mir::Instruction::VectorReduce { .. }
        | mir::Instruction::VectorCompare { .. }
        | mir::Instruction::VectorConvert { .. } => true,

        // pure descriptor operations
        mir::Instruction::GlobalAddr { .. }
        | mir::Instruction::FunctionAddr { .. }
        | mir::Instruction::FunctionBind { .. }
        | mir::Instruction::FunctionEnvironment { .. }
        | mir::Instruction::FunctionEnvironmentCurrent { .. }
        | mir::Instruction::ContextGet { .. } => true,

        // borrow producing address computations are not speculatable
        mir::Instruction::FieldAddr { .. }
        | mir::Instruction::ElementAddr { .. }
        | mir::Instruction::VariantPayloadAddr { .. }
        | mir::Instruction::LocalAddr { .. } => false,

        // reads mutable state, not speculatable
        mir::Instruction::LocalGet { .. }
        | mir::Instruction::Load { .. }
        | mir::Instruction::VariantTagLoad { .. }
        | mir::Instruction::AtomicLoad { .. }
        | mir::Instruction::AtomicCompareExchange { .. }
        | mir::Instruction::AtomicRmw { .. } => false,

        // writes have side effects
        mir::Instruction::LocalSet { .. }
        | mir::Instruction::Store { .. }
        | mir::Instruction::AtomicStore { .. }
        | mir::Instruction::AtomicFence { .. }
        | mir::Instruction::BarrierWrite { .. } => false,

        // calls may have side effects
        mir::Instruction::Call { .. }
        | mir::Instruction::ContextCurrent { .. }
        | mir::Instruction::ContextReplace { .. }
        | mir::Instruction::ContextBind { .. }
        | mir::Instruction::Drop { .. } => false,

        // allocations have side effects
        mir::Instruction::NewZeroed { .. }
        | mir::Instruction::NewUninit { .. }
        | mir::Instruction::NewSliceZeroed { .. }
        | mir::Instruction::NewSliceUninit { .. } => false,

        // returning storage has side effects
        mir::Instruction::Release { .. } => false,

        // instrumentation and intrinsics may have side effects
        mir::Instruction::ProfileIncrement { .. } | mir::Instruction::ProfileSample { .. } => false,
        mir::Instruction::Poll | mir::Instruction::Breakpoint => false,
        mir::Instruction::Intrinsic { .. } => false,
    }
}

/// Return whether an instruction can execute speculatively without trapping.
pub fn instruction_is_speculatable(
    instruction: &mir::Instruction,
    function: &mir::Function,
    tree: &mir::Tree,
) -> bool {
    // classify instructions by speculative safety
    match instruction {
        // address computations recompute freely once verify sealed their borrows
        mir::Instruction::FieldAddr { result_type, .. }
        | mir::Instruction::ElementAddr { result_type, .. }
        | mir::Instruction::VariantPayloadAddr { result_type, .. }
        | mir::Instruction::LocalAddr { result_type, .. } => {
            let result_type = *result_type;

            let ty = tree.get(result_type);
            matches!(
                ty,
                mir::Type::Pointer { .. }
                    | mir::Type::Reference {
                        kind: mir::ReferenceKind::Borrowed,
                        ..
                    }
            )
        }

        // assumptions and linear transitions must not cross control flow
        mir::Instruction::Assume { .. } | mir::Instruction::NewComplete { .. } => false,

        // non-saturating float to integer casts can trap on NaN or out of range inputs
        mir::Instruction::Cast {
            operator: mir::CastOperator::FloatToSignedInt | mir::CastOperator::FloatToUnsignedInt,
            ..
        } => false,

        // float division and remainder do not trap
        mir::Instruction::Binary {
            operator: mir::BinaryOperator::Divide | mir::BinaryOperator::Remainder,
            left,
            ..
        } => {
            let ty = function.expect_value_type(*left);

            tree.get(ty).is_float(tree)
        }

        _ => instruction_is_pure(instruction),
    }
}

/// Return whether an instruction computes an address.
pub fn instruction_is_borrow_address(instruction: &mir::Instruction) -> bool {
    matches!(
        instruction,
        mir::Instruction::FieldAddr { .. }
            | mir::Instruction::ElementAddr { .. }
            | mir::Instruction::VariantPayloadAddr { .. }
            | mir::Instruction::LocalAddr { .. }
    )
}

/// Return whether an instruction must remain when its result is unused.
pub fn instruction_has_side_effects(instruction: &mir::Instruction) -> bool {
    // classify instructions by side effects
    match instruction {
        mir::Instruction::Error => {
            panic!("recovered MIR instruction reached optimizer");
        }

        // pure computations, no side effects
        mir::Instruction::Const { .. }
        | mir::Instruction::Binary { .. }
        | mir::Instruction::Unary { .. }
        | mir::Instruction::Cast { .. }
        | mir::Instruction::Select { .. }
        | mir::Instruction::Aggregate { .. }
        | mir::Instruction::FieldGet { .. }
        | mir::Instruction::FieldSet { .. }
        | mir::Instruction::ElementGet { .. }
        | mir::Instruction::ElementSet { .. }
        | mir::Instruction::VariantNew { .. }
        | mir::Instruction::VariantTag { .. }
        | mir::Instruction::VariantTagLoad { .. }
        | mir::Instruction::VariantPayload { .. }
        | mir::Instruction::FieldAddr { .. }
        | mir::Instruction::ElementAddr { .. }
        | mir::Instruction::VariantPayloadAddr { .. }
        | mir::Instruction::SliceView { .. }
        | mir::Instruction::SliceLength { .. }
        | mir::Instruction::DynamicBind { .. }
        | mir::Instruction::DynamicPayload { .. }
        | mir::Instruction::DynamicType { .. }
        | mir::Instruction::DynamicRead { .. }
        | mir::Instruction::DynamicFind { .. }
        | mir::Instruction::VectorSplat { .. }
        | mir::Instruction::VectorExtract { .. }
        | mir::Instruction::VectorInsert { .. }
        | mir::Instruction::VectorShuffle { .. }
        | mir::Instruction::VectorSelect { .. }
        | mir::Instruction::VectorReduce { .. }
        | mir::Instruction::VectorCompare { .. }
        | mir::Instruction::VectorConvert { .. }
        | mir::Instruction::GlobalAddr { .. }
        | mir::Instruction::FunctionAddr { .. }
        | mir::Instruction::FunctionBind { .. }
        | mir::Instruction::FunctionEnvironment { .. }
        | mir::Instruction::FunctionEnvironmentCurrent { .. }
        | mir::Instruction::ContextCurrent { .. }
        | mir::Instruction::ContextGet { .. }
        | mir::Instruction::LocalAddr { .. }
        | mir::Instruction::NewComplete { .. }
        | mir::Instruction::Assume { .. } => false,

        // memory reads are pure when nonvolatile
        mir::Instruction::LocalGet { .. } | mir::Instruction::Load { .. } => false,

        // memory writes have side effects
        mir::Instruction::LocalSet { .. }
        | mir::Instruction::Store { .. }
        | mir::Instruction::AtomicLoad { .. }
        | mir::Instruction::AtomicStore { .. }
        | mir::Instruction::AtomicCompareExchange { .. }
        | mir::Instruction::AtomicRmw { .. }
        | mir::Instruction::AtomicFence { .. }
        | mir::Instruction::BarrierWrite { .. } => true,

        // calls may have side effects
        mir::Instruction::Call { .. }
        | mir::Instruction::ContextReplace { .. }
        | mir::Instruction::ContextBind { .. }
        | mir::Instruction::Drop { .. } => true,

        // allocations have side effects (memory allocation)
        mir::Instruction::NewZeroed { .. }
        | mir::Instruction::NewUninit { .. }
        | mir::Instruction::NewSliceZeroed { .. }
        | mir::Instruction::NewSliceUninit { .. } => true,

        // returning storage has side effects
        mir::Instruction::Release { .. } => true,

        // profile instrumentation must be preserved
        mir::Instruction::ProfileIncrement { .. } | mir::Instruction::ProfileSample { .. } => true,

        // runtime and debugger control must be preserved
        mir::Instruction::Poll | mir::Instruction::Breakpoint => true,

        // intrinsics may have side effects, check purity for safe removal
        mir::Instruction::Intrinsic { intrinsic, .. } => {
            !intrinsic.is_pure() || matches!(intrinsic, mir::Intrinsic::BlackBox)
        }
    }
}

/// Return whether an instruction reads memory.
pub fn instruction_is_memory_read(instruction: &mir::Instruction) -> bool {
    // identify instructions that read mutable memory
    matches!(
        instruction,
        mir::Instruction::Load { .. }
            | mir::Instruction::VariantTagLoad { .. }
            | mir::Instruction::LocalGet { .. }
            | mir::Instruction::ContextCurrent { .. }
            | mir::Instruction::AtomicLoad { .. }
            | mir::Instruction::AtomicCompareExchange { .. }
            | mir::Instruction::AtomicRmw { .. }
    )
}

/// Return whether an instruction may change observable memory state.
pub fn instruction_may_affect_memory(instruction: &mir::Instruction) -> bool {
    // identify instructions that can modify memory state
    matches!(
        instruction,
        mir::Instruction::Store { .. }
            | mir::Instruction::LocalSet { .. }
            | mir::Instruction::Call { .. }
            | mir::Instruction::Drop { .. }
            | mir::Instruction::ProfileIncrement { .. }
            | mir::Instruction::ProfileSample { .. }
            | mir::Instruction::Intrinsic { .. }
            | mir::Instruction::AtomicLoad { .. }
            | mir::Instruction::AtomicStore { .. }
            | mir::Instruction::AtomicCompareExchange { .. }
            | mir::Instruction::AtomicRmw { .. }
            | mir::Instruction::AtomicFence { .. }
            | mir::Instruction::BarrierWrite { .. }
            | mir::Instruction::NewZeroed { .. }
            | mir::Instruction::NewUninit { .. }
            | mir::Instruction::NewSliceZeroed { .. }
            | mir::Instruction::NewSliceUninit { .. }
            | mir::Instruction::Release { .. }
    )
}

/// Check if an instruction is a read only memory access under MemoryTable.
pub fn instruction_is_read_only_access(
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    memory: &MemoryTable,
) -> bool {
    // load memory accesses for this instruction
    let Some(accesses) = memory.instruction_accesses(instruction_id) else {
        return false;
    };

    // require read only effects across all accesses
    let mut reads = false;
    for access_id in accesses {
        // read the access effect
        let effect = match memory.access(*access_id) {
            MemoryNode::Use(use_access) => &use_access.effect,
            MemoryNode::Def(def_access) => &def_access.effect,
            MemoryNode::Phi(_) | MemoryNode::LiveOnEntry => continue,
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
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    instruction: &mir::Instruction,
    effects: &mir::EffectTable,
) -> bool {
    // accept non call instructions
    let is_call = matches!(instruction, mir::Instruction::Call { .. });
    if !is_call {
        return true;
    }

    // require call tables to be present
    let callsite = mir::Point::Instruction(instruction_id);
    let Some(tables) = effects.call(callsite) else {
        return false;
    };
    if tables.behavior.must_preserve_execution
        || tables.behavior.park.may_park()
        || tables.behavior.return_behavior.is_no_return()
        || tables.behavior.allocates
        || tables.behavior.frees
    {
        return false;
    }

    true
}

/// Collect values read by one function.
pub fn instruction_collect_used_values(
    function: &mir::Function,
    tree: &mir::Tree,
) -> FxIndexSet<mir::Value> {
    // seed the used value set
    let mut used = FxIndexSet::default();

    // add function parameters as implicitly used (they're inputs)
    for param in &function.parameters {
        used.insert(param.value);
    }

    // scan blocks for instruction and terminator uses
    for &block_id in function.blocks() {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);

        // collect uses from instructions
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);

            // add inline uses
            for value in instruction.uses() {
                used.insert(value);
            }

            // add externalized argument uses for calls and intrinsics
            if let Some(args_slice) = instruction.argument_slice() {
                for &arg in tree.get_values(args_slice) {
                    used.insert(arg);
                }
            }
        }

        // collect uses from terminator
        for value in terminator.uses(tree) {
            used.insert(value);
        }
    }

    used
}

/// Substitute mapped operands in one instruction.
pub fn instruction_substitute_uses(
    instruction: &mir::Instruction,
    substitutions: &FxIndexMap<mir::Value, mir::Value>,
) -> mir::Instruction {
    // skip when no substitutions are provided
    if substitutions.is_empty() {
        return instruction.clone();
    }

    // resolve a value through the substitution map
    let substitute =
        |value: &mir::Value| -> mir::Value { substitutions.get(value).copied().unwrap_or(*value) };

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
        mir::Instruction::FunctionBind {
            destination,
            function,
            arguments,
            environment,
        } => mir::Instruction::FunctionBind {
            destination: *destination,
            function: *function,
            arguments: arguments.clone(),
            environment: substitute(environment),
        },
        mir::Instruction::FunctionEnvironment {
            destination,
            function,
        } => mir::Instruction::FunctionEnvironment {
            destination: *destination,
            function: substitute(function),
        },
        mir::Instruction::ContextReplace {
            destination,
            context,
        } => mir::Instruction::ContextReplace {
            destination: *destination,
            context: substitute(context),
        },
        mir::Instruction::ContextBind {
            destination,
            context,
            variable,
            value,
            node_type,
            result_type,
        } => mir::Instruction::ContextBind {
            destination: *destination,
            context: substitute(context),
            variable: substitute(variable),
            value: substitute(value),
            node_type: *node_type,
            result_type: *result_type,
        },
        mir::Instruction::ContextGet {
            destination,
            context,
            variable,
            default,
            node_type,
            result_type,
        } => mir::Instruction::ContextGet {
            destination: *destination,
            context: substitute(context),
            variable: substitute(variable),
            default: substitute(default),
            node_type: *node_type,
            result_type: *result_type,
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
        mir::Instruction::Release { value } => mir::Instruction::Release {
            value: substitute(value),
        },
        mir::Instruction::FieldGet {
            destination,
            aggregate,
            field,
        } => mir::Instruction::FieldGet {
            destination: *destination,
            aggregate: substitute(aggregate),
            field: *field,
        },
        mir::Instruction::FieldSet {
            destination,
            aggregate,
            field,
            value,
        } => mir::Instruction::FieldSet {
            destination: *destination,
            aggregate: substitute(aggregate),
            field: *field,
            value: substitute(value),
        },
        mir::Instruction::ElementGet {
            destination,
            aggregate,
            index,
        } => mir::Instruction::ElementGet {
            destination: *destination,
            aggregate: substitute(aggregate),
            index: *index,
        },
        mir::Instruction::ElementSet {
            destination,
            aggregate,
            index,
            value,
        } => mir::Instruction::ElementSet {
            destination: *destination,
            aggregate: substitute(aggregate),
            index: *index,
            value: substitute(value),
        },
        mir::Instruction::VariantNew {
            destination,
            case,
            payload,
            result_type,
        } => mir::Instruction::VariantNew {
            destination: *destination,
            case: *case,
            payload: payload.as_ref().map(substitute),
            result_type: *result_type,
        },
        mir::Instruction::VariantTag {
            destination,
            variant,
        } => mir::Instruction::VariantTag {
            destination: *destination,
            variant: substitute(variant),
        },
        mir::Instruction::VariantTagLoad {
            destination,
            variant,
        } => mir::Instruction::VariantTagLoad {
            destination: *destination,
            variant: substitute(variant),
        },
        mir::Instruction::VariantPayload {
            destination,
            variant,
            case,
        } => mir::Instruction::VariantPayload {
            destination: *destination,
            variant: substitute(variant),
            case: *case,
        },
        mir::Instruction::VariantPayloadAddr {
            destination,
            variant,
            case,
            result_type,
            kind,
        } => mir::Instruction::VariantPayloadAddr {
            destination: *destination,
            variant: substitute(variant),
            case: *case,
            result_type: *result_type,
            kind: *kind,
        },
        mir::Instruction::FieldAddr {
            destination,
            aggregate,
            field,
            result_type,
            kind,
        } => mir::Instruction::FieldAddr {
            destination: *destination,
            aggregate: substitute(aggregate),
            field: *field,
            result_type: *result_type,
            kind: *kind,
        },
        mir::Instruction::ElementAddr {
            destination,
            base,
            index,
            result_type,
            kind,
        } => mir::Instruction::ElementAddr {
            destination: *destination,
            base: substitute(base),
            index: substitute(index),
            result_type: *result_type,
            kind: *kind,
        },
        mir::Instruction::SliceView {
            destination,
            source,
            start,
            length,
            result_type,
        } => mir::Instruction::SliceView {
            destination: *destination,
            source: substitute(source),
            start: substitute(start),
            length: substitute(length),
            result_type: *result_type,
        },
        mir::Instruction::SliceLength { destination, slice } => mir::Instruction::SliceLength {
            destination: *destination,
            slice: substitute(slice),
        },
        mir::Instruction::DynamicBind {
            destination,
            payload,
            concrete,
        } => mir::Instruction::DynamicBind {
            destination: *destination,
            payload: substitute(payload),
            concrete: *concrete,
        },
        mir::Instruction::DynamicPayload {
            destination,
            dynamic,
            result_type,
        } => mir::Instruction::DynamicPayload {
            destination: *destination,
            dynamic: substitute(dynamic),
            result_type: *result_type,
        },
        mir::Instruction::DynamicType {
            destination,
            dynamic,
        } => mir::Instruction::DynamicType {
            destination: *destination,
            dynamic: substitute(dynamic),
        },
        mir::Instruction::DynamicRead {
            destination,
            dynamic,
            slot,
            result_type,
        } => mir::Instruction::DynamicRead {
            destination: *destination,
            dynamic: substitute(dynamic),
            slot: *slot,
            result_type: *result_type,
        },
        mir::Instruction::DynamicFind {
            destination,
            dynamic,
            key,
            result_type,
        } => mir::Instruction::DynamicFind {
            destination: *destination,
            dynamic: substitute(dynamic),
            key: substitute(key),
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
            mask: *mask,
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
        mir::Instruction::Call { destination, call } => mir::Instruction::Call {
            destination: *destination,
            call: call.remap(
                call.callee.map_values(|value| substitute(&value)),
                call.arguments,
            ),
        },
        mir::Instruction::Drop { value } => mir::Instruction::Drop {
            value: substitute(value),
        },
        mir::Instruction::NewSliceZeroed {
            destination,
            element,
            length,
            result_type,
        } => mir::Instruction::NewSliceZeroed {
            destination: *destination,
            element: *element,
            length: substitute(length),
            result_type: *result_type,
        },
        mir::Instruction::NewSliceUninit {
            destination,
            element,
            length,
            result_type,
        } => mir::Instruction::NewSliceUninit {
            destination: *destination,
            element: *element,
            length: substitute(length),
            result_type: *result_type,
        },
        mir::Instruction::NewComplete {
            destination,
            value,
            result_type,
        } => mir::Instruction::NewComplete {
            destination: *destination,
            value: substitute(value),
            result_type: *result_type,
        },
        mir::Instruction::LocalSet { local, value } => mir::Instruction::LocalSet {
            local: *local,
            value: substitute(value),
        },
        mir::Instruction::Assume { condition } => mir::Instruction::Assume {
            condition: substitute(condition),
        },
        mir::Instruction::ProfileSample { sampler, value } => mir::Instruction::ProfileSample {
            sampler: *sampler,
            value: substitute(value),
        },
        // instructions without value operands or with externalized arguments
        mir::Instruction::Const { .. }
        | mir::Instruction::LocalGet { .. }
        | mir::Instruction::GlobalAddr { .. }
        | mir::Instruction::FunctionAddr { .. }
        | mir::Instruction::LocalAddr { .. }
        | mir::Instruction::Aggregate { .. }
        | mir::Instruction::FunctionEnvironmentCurrent { .. }
        | mir::Instruction::ContextCurrent { .. }
        | mir::Instruction::NewZeroed { .. }
        | mir::Instruction::NewUninit { .. }
        | mir::Instruction::ProfileIncrement { .. }
        | mir::Instruction::Poll
        | mir::Instruction::Breakpoint
        | mir::Instruction::Intrinsic { .. } => instruction.clone(),
    }
}

/// Substitute mapped operands and externalized arguments in one instruction.
pub fn instruction_substitute_uses_in_tree(
    instruction: &mir::Instruction,
    substitutions: &FxIndexMap<mir::Value, mir::Value>,
    tree: &mut mir::Tree,
) -> mir::Instruction {
    // skip when no substitutions are provided
    if substitutions.is_empty() {
        return instruction.clone();
    }

    // resolve values through the substitution map
    let substitute =
        |value: mir::Value| -> mir::Value { substitutions.get(&value).copied().unwrap_or(value) };

    // rebuild argument slices when needed
    let mut substitute_arguments = |slice: mir::ValueSlice| -> mir::ValueSlice {
        // read existing arguments
        let arguments = tree.get_values(slice);

        // skip when no arguments are substituted
        if !arguments
            .iter()
            .any(|value| substitutions.contains_key(value))
        {
            return slice;
        }

        // build remapped arguments
        let new_arguments: Vec<_> = arguments.iter().map(|value| substitute(*value)).collect();

        tree.add_values(&new_arguments)
    };

    // rebuild the instruction using substituted operands
    match instruction {
        mir::Instruction::Aggregate {
            destination,
            values,
        } => mir::Instruction::Aggregate {
            destination: *destination,
            values: substitute_arguments(*values),
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
            mask: *mask,
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
        mir::Instruction::Call { destination, call } => mir::Instruction::Call {
            destination: *destination,
            call: call.remap(
                call.callee.map_values(substitute),
                substitute_arguments(call.arguments),
            ),
        },
        mir::Instruction::Drop { value } => mir::Instruction::Drop {
            value: substitute(*value),
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
    values: &[mir::Value],
    substitutions: &FxIndexMap<mir::Value, mir::Value>,
) -> Vec<mir::Value> {
    // fast path for empty substitutions
    if substitutions.is_empty() {
        return values.to_vec();
    }

    // apply substitutions to the value list
    values
        .iter()
        .map(|value| substitutions.get(value).copied().unwrap_or(*value))
        .collect()
}

/// Resolve transitive value substitutions.
pub fn resolve_substitution_chains(
    mut substitutions: FxIndexMap<mir::Value, mir::Value>,
) -> FxIndexMap<mir::Value, mir::Value> {
    for value in substitutions.keys().copied().collect::<Vec<_>>() {
        let mut replacement = substitutions[&value];

        // follow substitutions to their terminal value
        while let Some(&next) = substitutions.get(&replacement) {
            if next == replacement {
                break;
            }
            replacement = next;
        }

        substitutions.insert(value, replacement);
    }

    substitutions
}

/// Apply substitutions and removals across one function.
pub fn apply_substitutions_in_function(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
    substitutions: &FxIndexMap<mir::Value, mir::Value>,
    to_remove: Option<&FxIndexSet<mir::LocalNodeId<mir::Instruction>>>,
) -> bool {
    // check if work is required
    let has_substitutions = !substitutions.is_empty();
    let has_removals = to_remove.is_some_and(|set| !set.is_empty());
    if !has_substitutions && !has_removals {
        return false;
    }

    // track whether any changes occur
    let mut changed = false;

    let blocks = function.blocks().to_vec();

    // rewrite instructions and terminators in each block
    for block_id in blocks {
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
                    tree.set(instruction_id, updated);
                    remap_instruction_memory_accesses(accesses, instruction_id, substitutions);
                    changed = true;
                }
            }

            new_instructions.push(instruction_id);
        }

        // rewrite terminator operands when requested
        let new_terminator = if has_substitutions {
            terminator_substitute_uses(tree, &terminator, substitutions)
        } else {
            terminator.clone()
        };

        // update block when instructions or terminator changed
        if new_instructions.len() != block.instructions.len() || new_terminator != terminator {
            function.replace_block_instructions(block_id, new_instructions, tree);
            tree.set(terminator_id, new_terminator);
            changed = true;
        }
    }

    changed
}

/// Clone instruction tables while remapping value references.
pub fn clone_instruction_tables(
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
    original: mir::LocalNodeId<mir::Instruction>,
    cloned: mir::LocalNodeId<mir::Instruction>,
    value_map: &FxIndexMap<mir::Value, mir::Value>,
) {
    // preserve instruction source by default
    if let Some(source_id) = tree.get_source(original.id) {
        tree.set_source(cloned.id, source_id);
    }

    // clone memory access entries
    if let Some(original_accesses) = accesses.get(original) {
        let mut cloned_accesses = original_accesses.to_vec();
        for access in &mut cloned_accesses {
            if let mir::MemoryTarget::Address(value) = access.target
                && let Some(&remapped) = value_map.get(&value)
            {
                access.target = mir::MemoryTarget::Address(remapped);
            }
        }

        accesses.insert(cloned, cloned_accesses);
    }
}

/// Remap instruction memory access entries in place using a substitution map.
pub fn remap_instruction_memory_accesses(
    accesses: &mut mir::AccessTable,
    instruction: mir::LocalNodeId<mir::Instruction>,
    substitutions: &FxIndexMap<mir::Value, mir::Value>,
) {
    // skip when no substitutions are provided
    if substitutions.is_empty() {
        return;
    }

    // read existing memory access entries
    let Some(original_accesses) = accesses.get(instruction) else {
        return;
    };

    let mut updated = original_accesses.to_vec();
    for access in &mut updated {
        if let mir::MemoryTarget::Address(value) = access.target
            && let Some(&remapped) = substitutions.get(&value)
        {
            access.target = mir::MemoryTarget::Address(remapped);
        }
    }

    accesses.insert(instruction, updated);
}

/// Remap every value in one instruction.
pub fn instruction_map(
    instruction: &mir::Instruction,
    value_map: &FxIndexMap<mir::Value, mir::Value>,
    tree: &mut mir::Tree,
) -> mir::Instruction {
    // remap values through the provided map
    let remap =
        |value: mir::Value| -> mir::Value { value_map.get(&value).copied().unwrap_or(value) };

    // rebuild argument slices with remapped values
    let mut remap_arguments = |slice: mir::ValueSlice| -> mir::ValueSlice {
        // remap argument values
        let new_args: Vec<_> = tree.get_values(slice).iter().copied().map(remap).collect();

        tree.add_values(&new_args)
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
        mir::Instruction::Release { value } => mir::Instruction::Release {
            value: remap(*value),
        },
        mir::Instruction::FieldGet {
            destination,
            aggregate,
            field,
        } => mir::Instruction::FieldGet {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            field: *field,
        },
        mir::Instruction::FieldSet {
            destination,
            aggregate,
            field,
            value,
        } => mir::Instruction::FieldSet {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            field: *field,
            value: remap(*value),
        },
        mir::Instruction::ElementGet {
            destination,
            aggregate,
            index,
        } => mir::Instruction::ElementGet {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            index: *index,
        },
        mir::Instruction::ElementSet {
            destination,
            aggregate,
            index,
            value,
        } => mir::Instruction::ElementSet {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            index: *index,
            value: remap(*value),
        },
        mir::Instruction::VariantNew {
            destination,
            case,
            payload,
            result_type,
        } => mir::Instruction::VariantNew {
            destination: remap(*destination),
            case: *case,
            payload: payload.map(remap),
            result_type: *result_type,
        },
        mir::Instruction::VariantTag {
            destination,
            variant,
        } => mir::Instruction::VariantTag {
            destination: remap(*destination),
            variant: remap(*variant),
        },
        mir::Instruction::VariantTagLoad {
            destination,
            variant,
        } => mir::Instruction::VariantTagLoad {
            destination: remap(*destination),
            variant: remap(*variant),
        },
        mir::Instruction::VariantPayload {
            destination,
            variant,
            case,
        } => mir::Instruction::VariantPayload {
            destination: remap(*destination),
            variant: remap(*variant),
            case: *case,
        },
        mir::Instruction::VariantPayloadAddr {
            destination,
            variant,
            case,
            result_type,
            kind,
        } => mir::Instruction::VariantPayloadAddr {
            destination: remap(*destination),
            variant: remap(*variant),
            case: *case,
            result_type: *result_type,
            kind: *kind,
        },
        mir::Instruction::FieldAddr {
            destination,
            aggregate,
            field,
            result_type,
            kind,
        } => mir::Instruction::FieldAddr {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            field: *field,
            result_type: *result_type,
            kind: *kind,
        },
        mir::Instruction::ElementAddr {
            destination,
            base,
            index,
            result_type,
            kind,
        } => mir::Instruction::ElementAddr {
            destination: remap(*destination),
            base: remap(*base),
            index: remap(*index),
            result_type: *result_type,
            kind: *kind,
        },
        mir::Instruction::SliceView {
            destination,
            source,
            start,
            length,
            result_type,
        } => mir::Instruction::SliceView {
            destination: remap(*destination),
            source: remap(*source),
            start: remap(*start),
            length: remap(*length),
            result_type: *result_type,
        },
        mir::Instruction::SliceLength { destination, slice } => mir::Instruction::SliceLength {
            destination: remap(*destination),
            slice: remap(*slice),
        },
        mir::Instruction::DynamicBind {
            destination,
            payload,
            concrete,
        } => mir::Instruction::DynamicBind {
            destination: remap(*destination),
            payload: remap(*payload),
            concrete: *concrete,
        },
        mir::Instruction::DynamicPayload {
            destination,
            dynamic,
            result_type,
        } => mir::Instruction::DynamicPayload {
            destination: remap(*destination),
            dynamic: remap(*dynamic),
            result_type: *result_type,
        },
        mir::Instruction::DynamicType {
            destination,
            dynamic,
        } => mir::Instruction::DynamicType {
            destination: remap(*destination),
            dynamic: remap(*dynamic),
        },
        mir::Instruction::DynamicRead {
            destination,
            dynamic,
            slot,
            result_type,
        } => mir::Instruction::DynamicRead {
            destination: remap(*destination),
            dynamic: remap(*dynamic),
            slot: *slot,
            result_type: *result_type,
        },
        mir::Instruction::DynamicFind {
            destination,
            dynamic,
            key,
            result_type,
        } => mir::Instruction::DynamicFind {
            destination: remap(*destination),
            dynamic: remap(*dynamic),
            key: remap(*key),
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
            kind,
        } => mir::Instruction::GlobalAddr {
            destination: remap(*destination),
            global: *global,
            result_type: *result_type,
            kind: *kind,
        },
        mir::Instruction::FunctionAddr {
            destination,
            function,
            arguments,
        } => mir::Instruction::FunctionAddr {
            destination: remap(*destination),
            function: *function,
            arguments: arguments.clone(),
        },
        mir::Instruction::FunctionBind {
            destination,
            function,
            arguments,
            environment,
        } => mir::Instruction::FunctionBind {
            destination: remap(*destination),
            function: *function,
            arguments: arguments.clone(),
            environment: remap(*environment),
        },
        mir::Instruction::FunctionEnvironment {
            destination,
            function,
        } => mir::Instruction::FunctionEnvironment {
            destination: remap(*destination),
            function: remap(*function),
        },
        mir::Instruction::FunctionEnvironmentCurrent { destination } => {
            mir::Instruction::FunctionEnvironmentCurrent {
                destination: remap(*destination),
            }
        }
        mir::Instruction::ContextCurrent { destination } => mir::Instruction::ContextCurrent {
            destination: remap(*destination),
        },
        mir::Instruction::ContextReplace {
            destination,
            context,
        } => mir::Instruction::ContextReplace {
            destination: remap(*destination),
            context: remap(*context),
        },
        mir::Instruction::ContextBind {
            destination,
            context,
            variable,
            value,
            node_type,
            result_type,
        } => mir::Instruction::ContextBind {
            destination: remap(*destination),
            context: remap(*context),
            variable: remap(*variable),
            value: remap(*value),
            node_type: *node_type,
            result_type: *result_type,
        },
        mir::Instruction::ContextGet {
            destination,
            context,
            variable,
            default,
            node_type,
            result_type,
        } => mir::Instruction::ContextGet {
            destination: remap(*destination),
            context: remap(*context),
            variable: remap(*variable),
            default: remap(*default),
            node_type: *node_type,
            result_type: *result_type,
        },
        mir::Instruction::LocalAddr {
            destination,
            local,
            result_type,
            kind,
        } => mir::Instruction::LocalAddr {
            destination: remap(*destination),
            local: *local,
            result_type: *result_type,
            kind: *kind,
        },
        mir::Instruction::Aggregate {
            destination,
            values,
        } => mir::Instruction::Aggregate {
            destination: remap(*destination),
            values: remap_arguments(*values),
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
            mask: *mask,
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
        mir::Instruction::Call { destination, call } => mir::Instruction::Call {
            destination: destination.map(remap),
            call: call.remap(
                call.callee.map_values(remap),
                remap_arguments(call.arguments),
            ),
        },
        mir::Instruction::Drop { value } => mir::Instruction::Drop {
            value: remap(*value),
        },
        mir::Instruction::NewZeroed {
            destination,
            storage_type,
            result_type,
        } => mir::Instruction::NewZeroed {
            destination: remap(*destination),
            storage_type: *storage_type,
            result_type: *result_type,
        },
        mir::Instruction::NewUninit {
            destination,
            storage_type,
            result_type,
        } => mir::Instruction::NewUninit {
            destination: remap(*destination),
            storage_type: *storage_type,
            result_type: *result_type,
        },
        mir::Instruction::NewComplete {
            destination,
            value,
            result_type,
        } => mir::Instruction::NewComplete {
            destination: remap(*destination),
            value: remap(*value),
            result_type: *result_type,
        },
        mir::Instruction::NewSliceZeroed {
            destination,
            element,
            length,
            result_type,
        } => mir::Instruction::NewSliceZeroed {
            destination: remap(*destination),
            element: *element,
            length: remap(*length),
            result_type: *result_type,
        },
        mir::Instruction::NewSliceUninit {
            destination,
            element,
            length,
            result_type,
        } => mir::Instruction::NewSliceUninit {
            destination: remap(*destination),
            element: *element,
            length: remap(*length),
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
        mir::Instruction::ProfileIncrement { counter } => {
            mir::Instruction::ProfileIncrement { counter: *counter }
        }
        mir::Instruction::ProfileSample { sampler, value } => mir::Instruction::ProfileSample {
            sampler: *sampler,
            value: remap(*value),
        },
        mir::Instruction::Poll => mir::Instruction::Poll,
        mir::Instruction::Breakpoint => mir::Instruction::Breakpoint,
    }
}

/// Remap values and locals in one instruction.
pub fn instruction_map_with_locals(
    instruction: &mir::Instruction,
    value_map: &FxIndexMap<mir::Value, mir::Value>,
    local_map: &FxIndexMap<mir::LocalNodeId<mir::Local>, mir::LocalNodeId<mir::Local>>,
    tree: &mut mir::Tree,
) -> mir::Instruction {
    // create a value remapper for simple value uses
    let remap =
        |value: mir::Value| -> mir::Value { value_map.get(&value).copied().unwrap_or(value) };

    // create a local remapper for direct local references
    let remap_local =
        |local: mir::LocalId| -> mir::LocalId { local_map.get(&local).copied().unwrap_or(local) };

    // remap argument slices into a new argument buffer entry
    let mut remap_arguments = |slice: mir::ValueSlice| -> mir::ValueSlice {
        let new_args: Vec<_> = tree.get_values(slice).iter().copied().map(remap).collect();
        tree.add_values(&new_args)
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
            kind,
        } => mir::Instruction::GlobalAddr {
            destination: remap(*destination),
            global: *global,
            result_type: *result_type,
            kind: *kind,
        },
        mir::Instruction::FunctionAddr {
            destination,
            function,
            arguments,
        } => mir::Instruction::FunctionAddr {
            destination: remap(*destination),
            function: *function,
            arguments: arguments.clone(),
        },
        mir::Instruction::FunctionBind {
            destination,
            function,
            arguments,
            environment,
        } => mir::Instruction::FunctionBind {
            destination: remap(*destination),
            function: *function,
            arguments: arguments.clone(),
            environment: remap(*environment),
        },
        mir::Instruction::FunctionEnvironment {
            destination,
            function,
        } => mir::Instruction::FunctionEnvironment {
            destination: remap(*destination),
            function: remap(*function),
        },
        mir::Instruction::FunctionEnvironmentCurrent { destination } => {
            mir::Instruction::FunctionEnvironmentCurrent {
                destination: remap(*destination),
            }
        }
        mir::Instruction::ContextCurrent { destination } => mir::Instruction::ContextCurrent {
            destination: remap(*destination),
        },
        mir::Instruction::ContextReplace {
            destination,
            context,
        } => mir::Instruction::ContextReplace {
            destination: remap(*destination),
            context: remap(*context),
        },
        mir::Instruction::ContextBind {
            destination,
            context,
            variable,
            value,
            node_type,
            result_type,
        } => mir::Instruction::ContextBind {
            destination: remap(*destination),
            context: remap(*context),
            variable: remap(*variable),
            value: remap(*value),
            node_type: *node_type,
            result_type: *result_type,
        },
        mir::Instruction::ContextGet {
            destination,
            context,
            variable,
            default,
            node_type,
            result_type,
        } => mir::Instruction::ContextGet {
            destination: remap(*destination),
            context: remap(*context),
            variable: remap(*variable),
            default: remap(*default),
            node_type: *node_type,
            result_type: *result_type,
        },
        mir::Instruction::LocalAddr {
            destination,
            local,
            result_type,
            kind,
        } => mir::Instruction::LocalAddr {
            destination: remap(*destination),
            local: *local,
            result_type: *result_type,
            kind: *kind,
        },
        mir::Instruction::Aggregate {
            destination,
            values,
        } => mir::Instruction::Aggregate {
            destination: remap(*destination),
            values: remap_arguments(*values),
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
            mask: *mask,
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
        mir::Instruction::FieldGet {
            destination,
            aggregate,
            field,
        } => mir::Instruction::FieldGet {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            field: *field,
        },
        mir::Instruction::FieldSet {
            destination,
            aggregate,
            field,
            value,
        } => mir::Instruction::FieldSet {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            field: *field,
            value: remap(*value),
        },
        mir::Instruction::ElementGet {
            destination,
            aggregate,
            index,
        } => mir::Instruction::ElementGet {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            index: *index,
        },
        mir::Instruction::ElementSet {
            destination,
            aggregate,
            index,
            value,
        } => mir::Instruction::ElementSet {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            index: *index,
            value: remap(*value),
        },
        mir::Instruction::VariantNew {
            destination,
            case,
            payload,
            result_type,
        } => mir::Instruction::VariantNew {
            destination: remap(*destination),
            case: *case,
            payload: payload.map(remap),
            result_type: *result_type,
        },
        mir::Instruction::VariantTag {
            destination,
            variant,
        } => mir::Instruction::VariantTag {
            destination: remap(*destination),
            variant: remap(*variant),
        },
        mir::Instruction::VariantTagLoad {
            destination,
            variant,
        } => mir::Instruction::VariantTagLoad {
            destination: remap(*destination),
            variant: remap(*variant),
        },
        mir::Instruction::VariantPayload {
            destination,
            variant,
            case,
        } => mir::Instruction::VariantPayload {
            destination: remap(*destination),
            variant: remap(*variant),
            case: *case,
        },
        mir::Instruction::VariantPayloadAddr {
            destination,
            variant,
            case,
            result_type,
            kind,
        } => mir::Instruction::VariantPayloadAddr {
            destination: remap(*destination),
            variant: remap(*variant),
            case: *case,
            result_type: *result_type,
            kind: *kind,
        },
        mir::Instruction::FieldAddr {
            destination,
            aggregate,
            field,
            result_type,
            kind,
        } => mir::Instruction::FieldAddr {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            field: *field,
            result_type: *result_type,
            kind: *kind,
        },
        mir::Instruction::ElementAddr {
            destination,
            base,
            index,
            result_type,
            kind,
        } => mir::Instruction::ElementAddr {
            destination: remap(*destination),
            base: remap(*base),
            index: remap(*index),
            result_type: *result_type,
            kind: *kind,
        },
        mir::Instruction::SliceView {
            destination,
            source,
            start,
            length,
            result_type,
        } => mir::Instruction::SliceView {
            destination: remap(*destination),
            source: remap(*source),
            start: remap(*start),
            length: remap(*length),
            result_type: *result_type,
        },
        mir::Instruction::SliceLength { destination, slice } => mir::Instruction::SliceLength {
            destination: remap(*destination),
            slice: remap(*slice),
        },
        mir::Instruction::DynamicBind {
            destination,
            payload,
            concrete,
        } => mir::Instruction::DynamicBind {
            destination: remap(*destination),
            payload: remap(*payload),
            concrete: *concrete,
        },
        mir::Instruction::DynamicPayload {
            destination,
            dynamic,
            result_type,
        } => mir::Instruction::DynamicPayload {
            destination: remap(*destination),
            dynamic: remap(*dynamic),
            result_type: *result_type,
        },
        mir::Instruction::DynamicType {
            destination,
            dynamic,
        } => mir::Instruction::DynamicType {
            destination: remap(*destination),
            dynamic: remap(*dynamic),
        },
        mir::Instruction::DynamicRead {
            destination,
            dynamic,
            slot,
            result_type,
        } => mir::Instruction::DynamicRead {
            destination: remap(*destination),
            dynamic: remap(*dynamic),
            slot: *slot,
            result_type: *result_type,
        },
        mir::Instruction::DynamicFind {
            destination,
            dynamic,
            key,
            result_type,
        } => mir::Instruction::DynamicFind {
            destination: remap(*destination),
            dynamic: remap(*dynamic),
            key: remap(*key),
            result_type: *result_type,
        },
        mir::Instruction::NewZeroed {
            destination,
            storage_type,
            result_type,
        } => mir::Instruction::NewZeroed {
            destination: remap(*destination),
            storage_type: *storage_type,
            result_type: *result_type,
        },
        mir::Instruction::NewUninit {
            destination,
            storage_type,
            result_type,
        } => mir::Instruction::NewUninit {
            destination: remap(*destination),
            storage_type: *storage_type,
            result_type: *result_type,
        },
        mir::Instruction::NewComplete {
            destination,
            value,
            result_type,
        } => mir::Instruction::NewComplete {
            destination: remap(*destination),
            value: remap(*value),
            result_type: *result_type,
        },
        mir::Instruction::NewSliceZeroed {
            destination,
            element,
            length,
            result_type,
        } => mir::Instruction::NewSliceZeroed {
            destination: remap(*destination),
            element: *element,
            length: remap(*length),
            result_type: *result_type,
        },
        mir::Instruction::NewSliceUninit {
            destination,
            element,
            length,
            result_type,
        } => mir::Instruction::NewSliceUninit {
            destination: remap(*destination),
            element: *element,
            length: remap(*length),
            result_type: *result_type,
        },
        mir::Instruction::Release { value } => mir::Instruction::Release {
            value: remap(*value),
        },
        mir::Instruction::Call { destination, call } => mir::Instruction::Call {
            destination: destination.map(remap),
            call: call.remap(
                call.callee.map_values(remap),
                remap_arguments(call.arguments),
            ),
        },
        mir::Instruction::Drop { value } => mir::Instruction::Drop {
            value: remap(*value),
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
        mir::Instruction::ProfileIncrement { counter } => {
            mir::Instruction::ProfileIncrement { counter: *counter }
        }
        mir::Instruction::ProfileSample { sampler, value } => mir::Instruction::ProfileSample {
            sampler: *sampler,
            value: remap(*value),
        },
        mir::Instruction::Poll => mir::Instruction::Poll,
        mir::Instruction::Breakpoint => mir::Instruction::Breakpoint,
    }
}

/// Remap block targets and values in one terminator.
pub fn terminator_remap(
    tree: &mut mir::Tree,
    terminator: &mut mir::Terminator,
    block_map: &FxIndexMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
    value_map: &FxIndexMap<mir::Value, mir::Value>,
) {
    // remap one block target in place
    let remap_target = |target: &mut mir::BlockTarget| {
        let block = target.block;

        if let Some(&remapped_block) = block_map.get(&block) {
            target.block = remapped_block;
        }
    };

    // remap one value reference in place
    let remap_value = |value: &mut mir::Value| {
        if let Some(&remapped_value) = value_map.get(value) {
            *value = remapped_value;
        }
    };

    // remap terminator fields
    match terminator {
        mir::Terminator::Error => {
            panic!("recovered MIR terminator reached optimizer");
        }
        mir::Terminator::Jump { target } => {
            remap_target(target);
            target.arguments = remap_value_slice(tree, target.arguments, value_map);
        }
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => {
            remap_value(condition);
            remap_target(then_target);
            then_target.arguments = remap_value_slice(tree, then_target.arguments, value_map);
            remap_target(else_target);
            else_target.arguments = remap_value_slice(tree, else_target.arguments, value_map);
        }
        mir::Terminator::Check {
            constraint,
            success,
            failure,
        } => {
            remap_target(success);
            success.arguments = remap_value_slice(tree, success.arguments, value_map);
            remap_target(failure);
            failure.arguments = remap_value_slice(tree, failure.arguments, value_map);
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
                mir::CheckConstraint::IsType { value, .. } => {
                    remap_value(value);
                }
                mir::CheckConstraint::IsSubtype { value, .. } => {
                    remap_value(value);
                }
            }
        }
        mir::Terminator::NewZeroedTry {
            success, failure, ..
        }
        | mir::Terminator::NewUninitTry {
            success, failure, ..
        } => {
            remap_target(success);
            success.arguments = remap_value_slice(tree, success.arguments, value_map);
            remap_target(failure);
            failure.arguments = remap_value_slice(tree, failure.arguments, value_map);
        }
        mir::Terminator::NewSliceZeroedTry {
            length,
            success,
            failure,
            ..
        }
        | mir::Terminator::NewSliceUninitTry {
            length,
            success,
            failure,
            ..
        } => {
            remap_value(length);
            remap_target(success);
            success.arguments = remap_value_slice(tree, success.arguments, value_map);
            remap_target(failure);
            failure.arguments = remap_value_slice(tree, failure.arguments, value_map);
        }
        mir::Terminator::Switch {
            value,
            default,
            cases,
        } => {
            remap_value(value);
            remap_target(default);
            default.arguments = remap_value_slice(tree, default.arguments, value_map);

            let mut new_cases = tree.get_switch_cases(*cases).to_vec();
            for case in &mut new_cases {
                remap_target(&mut case.target);
                case.target.arguments = remap_value_slice(tree, case.target.arguments, value_map);
            }
            *cases = tree.add_switch_cases(&new_cases);
        }
        mir::Terminator::VariantSwitch {
            value,
            default,
            cases,
        } => {
            remap_value(value);
            if let Some(default) = default {
                remap_target(default);
                default.arguments = remap_value_slice(tree, default.arguments, value_map);
            }

            let mut new_cases = tree.get_switch_cases(*cases).to_vec();
            for case in &mut new_cases {
                remap_target(&mut case.target);
                case.target.arguments = remap_value_slice(tree, case.target.arguments, value_map);
            }
            *cases = tree.add_switch_cases(&new_cases);
        }
        mir::Terminator::Return { value } => {
            if let Some(v) = value {
                remap_value(v);
            }
        }
        mir::Terminator::Invoke {
            call,
            target,
            unwind,
        } => {
            call.callee = call
                .callee
                .map_values(|value| value_map.get(&value).copied().unwrap_or(value));
            call.arguments = remap_value_slice(tree, call.arguments, value_map);
            remap_target(target);
            target.arguments = remap_value_slice(tree, target.arguments, value_map);
            remap_target(unwind);
            unwind.arguments = remap_value_slice(tree, unwind.arguments, value_map);
        }
        mir::Terminator::Panic { payload } => {
            if let Some(payload) = payload {
                remap_value(payload);
            }
        }
        mir::Terminator::UnwindResume => {}
        mir::Terminator::Abort { payload } => {
            if let Some(payload) = payload {
                remap_value(payload);
            }
        }
        mir::Terminator::Unreachable => {}
        mir::Terminator::TailCall { call } => {
            call.callee = call
                .callee
                .map_values(|value| value_map.get(&value).copied().unwrap_or(value));
            call.arguments = remap_value_slice(tree, call.arguments, value_map);
        }
    }
}

/// Remap one tree-owned value slice.
fn remap_value_slice(
    tree: &mut mir::Tree,
    slice: mir::ValueSlice,
    value_map: &FxIndexMap<mir::Value, mir::Value>,
) -> mir::ValueSlice {
    let values = tree
        .get_values(slice)
        .iter()
        .map(|value| value_map.get(value).copied().unwrap_or(*value))
        .collect::<Vec<_>>();

    tree.add_values(&values)
}
