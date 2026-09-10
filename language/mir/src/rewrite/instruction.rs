use destack_core::FxIndexMap;

use crate as mir;

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
        mir::Instruction::Copy { destination, value } => mir::Instruction::Copy {
            destination: *destination,
            value: substitute(value),
        },
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
            copy,
            destination,
            pointer,
            result_type,
        } => mir::Instruction::Load {
            copy: *copy,
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
            copy,
            destination,
            aggregate,
            field,
        } => mir::Instruction::FieldGet {
            copy: *copy,
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
            copy,
            destination,
            aggregate,
            index,
        } => mir::Instruction::ElementGet {
            copy: *copy,
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
            copy,
            destination,
            variant,
            case,
        } => mir::Instruction::VariantPayload {
            copy: *copy,
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
        // preserve instructions with unchanged or external arguments
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
