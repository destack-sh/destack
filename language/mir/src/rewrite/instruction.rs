use tspp_core::FxIndexMap;

use crate as mir;

impl mir::Instruction {
    /// Map inline SSA operands, preserving the destination and external argument slices.
    pub fn map_uses(&mut self, mut map: impl FnMut(mir::Value) -> mir::Value) {
        match self {
            Self::Error
            | Self::Const { .. }
            | Self::FunctionAddr { .. }
            | Self::FunctionEnvironmentCurrent { .. }
            | Self::ContextCurrent { .. }
            | Self::NewZeroed { .. }
            | Self::NewUninit { .. }
            | Self::AtomicFence { .. }
            | Self::ProfileIncrement { .. }
            | Self::Poll
            | Self::Breakpoint
            | Self::Aggregate { .. }
            | Self::Intrinsic { .. } => {}

            Self::Copy { value, .. }
            | Self::Unary {
                argument: value, ..
            }
            | Self::Cast {
                argument: value, ..
            }
            | Self::FunctionBind {
                environment: value, ..
            }
            | Self::FunctionEnvironment {
                function: value, ..
            }
            | Self::ContextReplace { context: value, .. }
            | Self::FieldGet {
                aggregate: value, ..
            }
            | Self::ElementGet {
                aggregate: value, ..
            }
            | Self::VariantTag { variant: value, .. }
            | Self::VariantPayload { variant: value, .. }
            | Self::SliceLength { slice: value, .. }
            | Self::DynamicBind { payload: value, .. }
            | Self::DynamicPayload { dynamic: value, .. }
            | Self::DynamicType { dynamic: value, .. }
            | Self::DynamicRead { dynamic: value, .. }
            | Self::VectorSplat { value, .. }
            | Self::VectorReduce { vector: value, .. }
            | Self::VectorConvert { vector: value, .. }
            | Self::Drop { value }
            | Self::NewComplete { value, .. }
            | Self::NewSliceZeroed { length: value, .. }
            | Self::NewSliceUninit { length: value, .. }
            | Self::Release { value }
            | Self::Assume { condition: value }
            | Self::ProfileSample { value, .. } => {
                *value = map(*value);
            }

            Self::Binary { left, right, .. }
            | Self::VectorShuffle { left, right, .. }
            | Self::VectorCompare { left, right, .. }
            | Self::FieldSet {
                aggregate: left,
                value: right,
                ..
            }
            | Self::ElementSet {
                aggregate: left,
                value: right,
                ..
            }
            | Self::DynamicFind {
                dynamic: left,
                key: right,
                ..
            }
            | Self::VectorExtract {
                vector: left,
                index: right,
                ..
            } => {
                *left = map(*left);
                *right = map(*right);
            }

            Self::Select {
                condition: first,
                then_value: second,
                else_value: third,
                ..
            }
            | Self::VectorSelect {
                mask: first,
                then_value: second,
                else_value: third,
                ..
            }
            | Self::ContextBind {
                context: first,
                variable: second,
                value: third,
                ..
            }
            | Self::ContextGet {
                context: first,
                variable: second,
                default: third,
                ..
            }
            | Self::VectorInsert {
                vector: first,
                index: second,
                value: third,
                ..
            }
            | Self::BarrierWrite {
                object: first,
                offset: second,
                byte_len: third,
            } => {
                *first = map(*first);
                *second = map(*second);
                *third = map(*third);
            }

            Self::VariantNew { payload, .. } => {
                if let Some(value) = payload {
                    *value = map(*value);
                }
            }
            Self::Call { call, .. } => call.callee = call.callee.map_values(map),

            Self::VariantTagLoad { place, .. }
            | Self::Load { place, .. }
            | Self::Address { place, .. }
            | Self::AtomicLoad { place, .. } => place.map_values(map),
            Self::Store { place, value }
            | Self::AtomicStore { place, value, .. }
            | Self::AtomicRmw { place, value, .. } => {
                place.map_values(&mut map);
                *value = map(*value);
            }
            Self::AtomicCompareExchange {
                place,
                expected,
                new_value,
                ..
            } => {
                place.map_values(&mut map);
                *expected = map(*expected);
                *new_value = map(*new_value);
            }
        }
    }
}

/// Substitute mapped inline operands in one instruction.
pub fn instruction_substitute_uses(
    instruction: &mir::Instruction,
    substitutions: &FxIndexMap<mir::Value, mir::Value>,
) -> mir::Instruction {
    let mut instruction = instruction.clone();
    instruction.map_uses(|value| substitutions.get(&value).copied().unwrap_or(value));

    instruction
}

/// Substitute mapped operands and external arguments in one instruction.
pub fn instruction_substitute_uses_in_tree(
    instruction: &mir::Instruction,
    substitutions: &FxIndexMap<mir::Value, mir::Value>,
    tree: &mut mir::Tree,
) -> mir::Instruction {
    let mut instruction = instruction_substitute_uses(instruction, substitutions);

    // remap argument slices only when one of their values changes
    let arguments = match &mut instruction {
        mir::Instruction::Aggregate { values, .. } => Some(values),
        mir::Instruction::Call { call, .. } => Some(&mut call.arguments),
        mir::Instruction::Intrinsic { arguments, .. } => Some(arguments),
        _ => None,
    };
    if let Some(arguments) = arguments
        && tree
            .get_values(*arguments)
            .iter()
            .any(|value| substitutions.contains_key(value))
    {
        *arguments = remap_value_slice(tree, *arguments, substitutions);
    }

    instruction
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
