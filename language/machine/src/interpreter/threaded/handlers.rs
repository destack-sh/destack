#![allow(elided_lifetimes_in_paths)]

use destack_mir as mir;
use smallvec::SmallVec;

use crate::diagnostic::Error;
use crate::memory::{Value, ValueTag};

use super::types::{ControlFlow, InstData, ThreadedInst, ThreadedState};

// helper macro for common instruction pattern: do work, then become next handler
macro_rules! next {
    ($state:expr, $block:expr, $pc:expr) => {{
        let next_pc = $pc + 1;
        become ($block[next_pc].handler)($state, $block, next_pc)
    }};
}

/// Handle constant load.
pub fn handle_const(state: &mut ThreadedState, block: &[ThreadedInst], pc: usize) -> ControlFlow {
    let InstData::Const { dest, value } = &block[pc].data else {
        unreachable!()
    };
    state.set(*dest, *value);

    next!(state, block, pc)
}

/// Handle binary operation.
pub fn handle_binary(state: &mut ThreadedState, block: &[ThreadedInst], pc: usize) -> ControlFlow {
    let InstData::Binary { dest, op, left, right } = &block[pc].data else {
        unreachable!()
    };

    let lhs = state.get(*left);
    let rhs = state.get(*right);

    let result = match execute_binary_inline(*op, lhs, rhs) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    state.set(*dest, result);

    next!(state, block, pc)
}

/// Handle unary operation.
pub fn handle_unary(state: &mut ThreadedState, block: &[ThreadedInst], pc: usize) -> ControlFlow {
    let InstData::Unary { dest, op, arg } = &block[pc].data else {
        unreachable!()
    };

    let argument = state.get(*arg);

    let result = match execute_unary_inline(*op, argument) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    state.set(*dest, result);

    next!(state, block, pc)
}

/// Handle cast operation.
pub fn handle_cast(state: &mut ThreadedState, block: &[ThreadedInst], pc: usize) -> ControlFlow {
    let InstData::Cast { dest, op, arg, to_type } = &block[pc].data else {
        unreachable!()
    };

    let argument = state.get(*arg);
    let result = execute_cast_inline(state, *op, argument, *to_type);
    state.set(*dest, result);

    next!(state, block, pc)
}

/// Handle function call (returns to trampoline).
pub fn handle_call(state: &mut ThreadedState, block: &[ThreadedInst], pc: usize) -> ControlFlow {
    let InstData::Call { dest, function, arguments } = &block[pc].data else {
        unreachable!()
    };

    // resolve arguments
    let args: SmallVec<[Value; 4]> = arguments.iter().map(|v| state.get(*v)).collect();

    ControlFlow::Call {
        function: *function,
        destination: *dest,
        arguments: args,
        resume_pc: pc + 1,
    }
}

/// Handle indirect call (returns to trampoline).
pub fn handle_call_indirect(
    state: &mut ThreadedState,
    block: &[ThreadedInst],
    pc: usize,
) -> ControlFlow {
    let InstData::CallIndirect { dest, callee, arguments } = &block[pc].data else {
        unreachable!()
    };

    let callee_val = state.get(*callee);

    // extract function pointer
    let function = match callee_val.as_function_pointer() {
        Some(f) => f,
        None => {
            return ControlFlow::Error(Error::TypeMismatch {
                expected: "function_pointer".to_string(),
                actual: format!("{callee_val:?}"),
            })
        }
    };

    let args: SmallVec<[Value; 4]> = arguments.iter().map(|v| state.get(*v)).collect();

    ControlFlow::Call {
        function,
        destination: *dest,
        arguments: args,
        resume_pc: pc + 1,
    }
}

/// Handle local variable load.
pub fn handle_local_get(
    state: &mut ThreadedState,
    block: &[ThreadedInst],
    pc: usize,
) -> ControlFlow {
    let InstData::LocalGet { dest, local } = &block[pc].data else {
        unreachable!()
    };

    let value = state.get_local(*local);
    state.set(*dest, value);

    next!(state, block, pc)
}

/// Handle local variable store.
pub fn handle_local_set(
    state: &mut ThreadedState,
    block: &[ThreadedInst],
    pc: usize,
) -> ControlFlow {
    let InstData::LocalSet { local, value } = &block[pc].data else {
        unreachable!()
    };

    let val = state.get(*value);
    state.set_local(*local, val);

    next!(state, block, pc)
}

/// Handle global address.
pub fn handle_global_addr(
    state: &mut ThreadedState,
    block: &[ThreadedInst],
    pc: usize,
) -> ControlFlow {
    let InstData::GlobalAddr { dest, global } = &block[pc].data else {
        unreachable!()
    };

    state.set(*dest, Value::global_pointer(*global));

    next!(state, block, pc)
}

/// Handle global constant load.
pub fn handle_global_const(
    state: &mut ThreadedState,
    block: &[ThreadedInst],
    pc: usize,
) -> ControlFlow {
    let InstData::GlobalConst { dest, global } = &block[pc].data else {
        unreachable!()
    };

    let value = match state.interp.globals.get(*global).copied() {
        Some(v) => v,
        None => return ControlFlow::Error(Error::UndefinedGlobal { global: *global }),
    };

    state.set(*dest, value);

    next!(state, block, pc)
}

/// Handle pointer load.
pub fn handle_load(state: &mut ThreadedState, block: &[ThreadedInst], pc: usize) -> ControlFlow {
    let InstData::Load { dest, pointer } = &block[pc].data else {
        unreachable!()
    };

    let ptr = state.get(*pointer);

    let value = match load_from_pointer(state, ptr) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    state.set(*dest, value);

    next!(state, block, pc)
}

/// Handle pointer store.
pub fn handle_store(state: &mut ThreadedState, block: &[ThreadedInst], pc: usize) -> ControlFlow {
    let InstData::Store { pointer, value } = &block[pc].data else {
        unreachable!()
    };

    let ptr = state.get(*pointer);
    let val = state.get(*value);

    if let Err(e) = store_to_pointer(state, ptr, val) {
        return ControlFlow::Error(e);
    }

    next!(state, block, pc)
}

/// Handle field get.
pub fn handle_field_get(
    state: &mut ThreadedState,
    block: &[ThreadedInst],
    pc: usize,
) -> ControlFlow {
    let InstData::FieldGet { dest, aggregate, index } = &block[pc].data else {
        unreachable!()
    };

    let agg = state.get(*aggregate);

    let value = match get_field(state, agg, *index) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    state.set(*dest, value);

    next!(state, block, pc)
}

/// Handle field set.
pub fn handle_field_set(
    state: &mut ThreadedState,
    block: &[ThreadedInst],
    pc: usize,
) -> ControlFlow {
    let InstData::FieldSet { dest, aggregate, index, value } = &block[pc].data else {
        unreachable!()
    };

    let agg = state.get(*aggregate);
    let val = state.get(*value);

    let result = match set_field(state, agg, *index, val) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    state.set(*dest, result);

    next!(state, block, pc)
}

/// Handle element get.
pub fn handle_element_get(
    state: &mut ThreadedState,
    block: &[ThreadedInst],
    pc: usize,
) -> ControlFlow {
    let InstData::ElementGet { dest, array, index } = &block[pc].data else {
        unreachable!()
    };

    let arr = state.get(*array);
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    let value = match get_element(state, arr, idx_val) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    state.set(*dest, value);

    next!(state, block, pc)
}

/// Handle element set.
pub fn handle_element_set(
    state: &mut ThreadedState,
    block: &[ThreadedInst],
    pc: usize,
) -> ControlFlow {
    let InstData::ElementSet { dest, array, index, value } = &block[pc].data else {
        unreachable!()
    };

    let arr = state.get(*array);
    let idx = state.get(*index);
    let val = state.get(*value);
    let idx_val = idx.as_uint().unwrap_or(0);

    let result = match set_element(state, arr, idx_val, val) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    state.set(*dest, result);

    next!(state, block, pc)
}

/// Handle managed allocation.
pub fn handle_managed_alloc(
    state: &mut ThreadedState,
    block: &[ThreadedInst],
    pc: usize,
) -> ControlFlow {
    let InstData::ManagedAlloc { dest } = &block[pc].data else {
        unreachable!()
    };

    if state.interp.managed_heap.cell_count() >= state.interp.options.max_heap_cells {
        return ControlFlow::Error(Error::AllocationFailed);
    }

    let handle = state.interp.managed_heap.allocate();
    state.interp.statistics.heap_allocations += 1;
    state.set(*dest, Value::managed_reference(handle));

    next!(state, block, pc)
}

/// Handle managed array allocation.
pub fn handle_managed_alloc_array(
    state: &mut ThreadedState,
    block: &[ThreadedInst],
    pc: usize,
) -> ControlFlow {
    let InstData::ManagedAllocArray { dest, length } = &block[pc].data else {
        unreachable!()
    };

    if state.interp.managed_heap.cell_count() >= state.interp.options.max_heap_cells {
        return ControlFlow::Error(Error::AllocationFailed);
    }

    let len_val = state.get(*length);
    let length = len_val.as_uint().unwrap_or(0) as usize;

    let handle = state.interp.managed_heap.allocate_with_slots(length);
    state.interp.statistics.heap_allocations += 1;
    state.set(*dest, Value::managed_reference(handle));

    next!(state, block, pc)
}

/// Handle raw allocation.
pub fn handle_raw_alloc(
    state: &mut ThreadedState,
    block: &[ThreadedInst],
    pc: usize,
) -> ControlFlow {
    let InstData::RawAlloc { dest } = &block[pc].data else {
        unreachable!()
    };

    if state.interp.raw_heap.cell_count() >= state.interp.options.max_heap_cells {
        return ControlFlow::Error(Error::AllocationFailed);
    }

    let ptr = state.interp.raw_heap.allocate();
    state.interp.statistics.heap_allocations += 1;
    state.set(*dest, Value::raw_pointer(ptr));

    next!(state, block, pc)
}

/// Handle raw free.
pub fn handle_raw_free(state: &mut ThreadedState, block: &[ThreadedInst], pc: usize) -> ControlFlow {
    let InstData::RawFree { pointer } = &block[pc].data else {
        unreachable!()
    };

    let ptr = state.get(*pointer);

    if let Some(p) = ptr.as_raw_pointer() {
        if !state.interp.raw_heap.free(p) {
            return ControlFlow::Error(Error::InvalidHeapHandle);
        }
    } else {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "raw_pointer".to_string(),
            actual: format!("{ptr:?}"),
        });
    }

    next!(state, block, pc)
}

/// Handle stack allocation.
pub fn handle_stack_alloc(
    state: &mut ThreadedState,
    block: &[ThreadedInst],
    pc: usize,
) -> ControlFlow {
    let InstData::StackAlloc { dest } = &block[pc].data else {
        unreachable!()
    };

    // NOTE #Incomplete: stack allocation in threaded interpreter needs frame tracking
    // for now, use a placeholder that allocates from managed heap
    let handle = state.interp.managed_heap.allocate();
    state.set(*dest, Value::managed_reference(handle));

    next!(state, block, pc)
}

/// Handle intrinsic call.
pub fn handle_intrinsic(
    state: &mut ThreadedState<'_>,
    block: &[ThreadedInst],
    pc: usize,
) -> ControlFlow {
    let InstData::Intrinsic { dest, intrinsic, arguments, .. } = &block[pc].data else {
        unreachable!()
    };

    // resolve arguments
    let args: Vec<Value> = arguments.iter().map(|v| state.get(*v)).collect();

    // execute intrinsic
    match state.interp.execute_intrinsic_resolved(*intrinsic, &args) {
        Ok(result) => {
            if let Some(d) = dest {
                state.set(*d, result);
            }
            next!(state, block, pc)
        }
        Err(e) => ControlFlow::Error(e.error),
    }
}

/// Handle return (exits tail-call chain).
pub fn handle_return(state: &mut ThreadedState, block: &[ThreadedInst], pc: usize) -> ControlFlow {
    let InstData::Return { value } = &block[pc].data else {
        unreachable!()
    };

    let return_value = match value {
        Some(v) => state.get(*v),
        None => Value::VOID,
    };

    ControlFlow::Return(return_value)
}

/// Handle unconditional jump (exits tail-call chain).
pub fn handle_jump(state: &mut ThreadedState, block: &[ThreadedInst], pc: usize) -> ControlFlow {
    let InstData::Jump { target, arguments } = &block[pc].data else {
        unreachable!()
    };

    let args: SmallVec<[Value; 4]> = arguments.iter().map(|v| state.get(*v)).collect();

    ControlFlow::Jump {
        block: *target,
        arguments: args,
    }
}

/// Handle conditional branch (exits tail-call chain).
pub fn handle_branch(state: &mut ThreadedState, block: &[ThreadedInst], pc: usize) -> ControlFlow {
    let InstData::Branch {
        condition,
        then_target,
        then_arguments,
        else_target,
        else_arguments,
    } = &block[pc].data
    else {
        unreachable!()
    };

    let cond = state.get(*condition);
    let is_truthy = cond.is_truthy();

    state.interp.statistics.branches += 1;

    if is_truthy {
        let args: SmallVec<[Value; 4]> = then_arguments.iter().map(|v| state.get(*v)).collect();
        ControlFlow::Jump {
            block: *then_target,
            arguments: args,
        }
    } else {
        let args: SmallVec<[Value; 4]> = else_arguments.iter().map(|v| state.get(*v)).collect();
        ControlFlow::Jump {
            block: *else_target,
            arguments: args,
        }
    }
}

/// Handle switch (exits tail-call chain).
pub fn handle_switch(state: &mut ThreadedState, block: &[ThreadedInst], pc: usize) -> ControlFlow {
    let InstData::Switch { value, cases, default_target, default_arguments } = &block[pc].data
    else {
        unreachable!()
    };

    let switch_val = state.get(*value);
    let int_val = switch_val.as_int().unwrap_or(0);

    state.interp.statistics.branches += 1;

    // find matching case
    for case in cases {
        if case.value == int_val {
            let args: SmallVec<[Value; 4]> =
                case.arguments.iter().map(|v| state.get(*v)).collect();
            return ControlFlow::Jump {
                block: case.target,
                arguments: args,
            };
        }
    }

    // default case
    let args: SmallVec<[Value; 4]> = default_arguments.iter().map(|v| state.get(*v)).collect();

    ControlFlow::Jump {
        block: *default_target,
        arguments: args,
    }
}

/// Handle unreachable (errors).
pub fn handle_unreachable(
    _state: &mut ThreadedState,
    _block: &[ThreadedInst],
    _pc: usize,
) -> ControlFlow {
    ControlFlow::Error(Error::Unreachable)
}

// inline helper: execute binary operation
fn execute_binary_inline(
    op: mir::BinaryOperator,
    lhs: Value,
    rhs: Value,
) -> Result<Value, Error> {
    use mir::BinaryOperator::*;

    let lhs_tag = lhs.tag();
    let rhs_tag = rhs.tag();

    let result = match (lhs_tag, rhs_tag) {
        // signed integer
        (ValueTag::Int, ValueTag::Int) => {
            let a = lhs.raw_data() as i64;
            let b = rhs.raw_data() as i64;
            let width = lhs.width();
            match op {
                Add => Value::int(a.wrapping_add(b), width),
                Subtract => Value::int(a.wrapping_sub(b), width),
                Multiply => Value::int(a.wrapping_mul(b), width),
                SignedDivide => {
                    if b == 0 {
                        return Err(Error::DivisionByZero);
                    }
                    Value::int(a.wrapping_div(b), width)
                }
                SignedRemainder => {
                    if b == 0 {
                        return Err(Error::DivisionByZero);
                    }
                    Value::int(a.wrapping_rem(b), width)
                }
                Equal => Value::bool(a == b),
                NotEqual => Value::bool(a != b),
                SignedLessThan => Value::bool(a < b),
                SignedLessEqual => Value::bool(a <= b),
                SignedGreaterThan => Value::bool(a > b),
                SignedGreaterEqual => Value::bool(a >= b),
                And => Value::int(a & b, width),
                Or => Value::int(a | b, width),
                Xor => Value::int(a ^ b, width),
                ShiftLeft => Value::int(a.wrapping_shl(b as u32), width),
                ArithmeticShiftRight => Value::int(a.wrapping_shr(b as u32), width),
                _ => {
                    return Err(Error::TypeMismatch {
                        expected: format!("compatible types for {op:?}"),
                        actual: format!("{lhs:?}, {rhs:?}"),
                    })
                }
            }
        }

        // unsigned integer
        (ValueTag::UInt, ValueTag::UInt) => {
            let a = lhs.raw_data();
            let b = rhs.raw_data();
            let width = lhs.width();
            match op {
                Add => Value::uint(a.wrapping_add(b), width),
                Subtract => Value::uint(a.wrapping_sub(b), width),
                Multiply => Value::uint(a.wrapping_mul(b), width),
                UnsignedDivide => {
                    if b == 0 {
                        return Err(Error::DivisionByZero);
                    }
                    Value::uint(a.wrapping_div(b), width)
                }
                UnsignedRemainder => {
                    if b == 0 {
                        return Err(Error::DivisionByZero);
                    }
                    Value::uint(a.wrapping_rem(b), width)
                }
                UnsignedLessThan => Value::bool(a < b),
                UnsignedLessEqual => Value::bool(a <= b),
                UnsignedGreaterThan => Value::bool(a > b),
                UnsignedGreaterEqual => Value::bool(a >= b),
                And => Value::uint(a & b, width),
                Or => Value::uint(a | b, width),
                Xor => Value::uint(a ^ b, width),
                ShiftLeft => Value::uint(a.wrapping_shl(b as u32), width),
                LogicalShiftRight => Value::uint(a.wrapping_shr(b as u32), width),
                _ => {
                    return Err(Error::TypeMismatch {
                        expected: format!("compatible types for {op:?}"),
                        actual: format!("{lhs:?}, {rhs:?}"),
                    })
                }
            }
        }

        // f64
        (ValueTag::Float64, ValueTag::Float64) => {
            let a = f64::from_bits(lhs.raw_data());
            let b = f64::from_bits(rhs.raw_data());
            match op {
                FloatAdd => Value::float64(a + b),
                FloatSubtract => Value::float64(a - b),
                FloatMultiply => Value::float64(a * b),
                FloatDivide => Value::float64(a / b),
                FloatEqual => Value::bool(a == b),
                FloatNotEqual => Value::bool(a != b),
                FloatLessThan => Value::bool(a < b),
                FloatLessEqual => Value::bool(a <= b),
                FloatGreaterThan => Value::bool(a > b),
                FloatGreaterEqual => Value::bool(a >= b),
                _ => {
                    return Err(Error::TypeMismatch {
                        expected: format!("compatible types for {op:?}"),
                        actual: format!("{lhs:?}, {rhs:?}"),
                    })
                }
            }
        }

        // f32
        (ValueTag::Float32, ValueTag::Float32) => {
            let a = f32::from_bits(lhs.raw_data() as u32);
            let b = f32::from_bits(rhs.raw_data() as u32);
            match op {
                FloatAdd => Value::float32(a + b),
                FloatSubtract => Value::float32(a - b),
                FloatMultiply => Value::float32(a * b),
                FloatDivide => Value::float32(a / b),
                FloatEqual => Value::bool(a == b),
                FloatNotEqual => Value::bool(a != b),
                FloatLessThan => Value::bool(a < b),
                FloatLessEqual => Value::bool(a <= b),
                FloatGreaterThan => Value::bool(a > b),
                FloatGreaterEqual => Value::bool(a >= b),
                _ => {
                    return Err(Error::TypeMismatch {
                        expected: format!("compatible types for {op:?}"),
                        actual: format!("{lhs:?}, {rhs:?}"),
                    })
                }
            }
        }

        // boolean
        (ValueTag::Bool, ValueTag::Bool) => {
            let a = lhs.raw_data() != 0;
            let b = rhs.raw_data() != 0;
            match op {
                And => Value::bool(a && b),
                Or => Value::bool(a || b),
                Xor => Value::bool(a ^ b),
                _ => {
                    return Err(Error::TypeMismatch {
                        expected: format!("compatible types for {op:?}"),
                        actual: format!("{lhs:?}, {rhs:?}"),
                    })
                }
            }
        }

        // incompatible
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("compatible types for {op:?}"),
                actual: format!("{lhs:?}, {rhs:?}"),
            })
        }
    };

    Ok(result)
}

// inline helper: execute unary operation
fn execute_unary_inline(op: mir::UnaryOperator, arg: Value) -> Result<Value, Error> {
    use mir::UnaryOperator::*;

    let result = match (op, arg.tag()) {
        (Negate, ValueTag::Int) => {
            let value = arg.raw_data() as i64;
            Value::int(value.wrapping_neg(), arg.width())
        }
        (FloatNegate, ValueTag::Float64) => {
            let f = f64::from_bits(arg.raw_data());
            Value::float64(-f)
        }
        (FloatNegate, ValueTag::Float32) => {
            let f = f32::from_bits(arg.raw_data() as u32);
            Value::float32(-f)
        }
        (Not, ValueTag::Bool) => {
            let b = arg.raw_data() != 0;
            Value::bool(!b)
        }
        (Not, ValueTag::Int) => {
            let value = arg.raw_data() as i64;
            Value::int(!value, arg.width())
        }
        (Not, ValueTag::UInt) => {
            let value = arg.raw_data();
            Value::uint(!value, arg.width())
        }
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("compatible type for {op:?}"),
                actual: format!("{arg:?}"),
            })
        }
    };

    Ok(result)
}

// inline helper: execute cast
fn execute_cast_inline(
    state: &ThreadedState,
    operator: mir::CastOperator,
    argument: Value,
    to_type: mir::LocalNodeId<mir::Type>,
) -> Value {
    let target_type = state.interp.tree.get(to_type);

    match operator {
        mir::CastOperator::Bitcast => argument,

        mir::CastOperator::Truncate => {
            let target_width = match target_type {
                mir::Type::Int { width, .. } => *width as u8,
                _ => return argument,
            };
            match argument.tag() {
                ValueTag::Int => {
                    let value = argument.raw_data() as i64;
                    Value::int(truncate_signed(value, target_width), target_width)
                }
                ValueTag::UInt => {
                    let value = argument.raw_data();
                    Value::uint(truncate_unsigned(value, target_width), target_width)
                }
                _ => argument,
            }
        }

        mir::CastOperator::ZeroExtend => {
            let target_width = match target_type {
                mir::Type::Int { width, .. } => *width as u8,
                _ => return argument,
            };
            match argument.tag() {
                ValueTag::UInt => Value::uint(argument.raw_data(), target_width),
                ValueTag::Int => {
                    let value = argument.raw_data() as i64;
                    let width = argument.width();
                    let masked = truncate_unsigned(value as u64, width);
                    Value::uint(masked, target_width)
                }
                _ => argument,
            }
        }

        mir::CastOperator::SignExtend => {
            let target_width = match target_type {
                mir::Type::Int { width, .. } => *width as u8,
                _ => return argument,
            };
            match argument.tag() {
                ValueTag::Int => {
                    let value = argument.raw_data() as i64;
                    let width = argument.width();
                    let extended = sign_extend(value, width, target_width);
                    Value::int(extended, target_width)
                }
                ValueTag::UInt => {
                    let value = argument.raw_data();
                    let width = argument.width();
                    let as_signed = truncate_signed(value as i64, width);
                    let extended = sign_extend(as_signed, width, target_width);
                    Value::int(extended, target_width)
                }
                _ => argument,
            }
        }

        mir::CastOperator::FloatToSignedInt => {
            let target_width = match target_type {
                mir::Type::Int { width, .. } => *width as u8,
                _ => 64,
            };
            match argument.tag() {
                ValueTag::Float64 => {
                    let f = f64::from_bits(argument.raw_data());
                    Value::int(f as i64, target_width)
                }
                ValueTag::Float32 => {
                    let f = f32::from_bits(argument.raw_data() as u32);
                    Value::int(f as i64, target_width)
                }
                _ => argument,
            }
        }

        mir::CastOperator::FloatToUnsignedInt => {
            let target_width = match target_type {
                mir::Type::Int { width, .. } => *width as u8,
                _ => 64,
            };
            match argument.tag() {
                ValueTag::Float64 => {
                    let f = f64::from_bits(argument.raw_data());
                    Value::uint(f as u64, target_width)
                }
                ValueTag::Float32 => {
                    let f = f32::from_bits(argument.raw_data() as u32);
                    Value::uint(f as u64, target_width)
                }
                _ => argument,
            }
        }

        mir::CastOperator::SignedIntToFloat => {
            let target_width = match target_type {
                mir::Type::Float { width } => *width,
                _ => 64,
            };
            match argument.tag() {
                ValueTag::Int => {
                    let value = argument.raw_data() as i64;
                    if target_width == 32 {
                        Value::float32(value as f32)
                    } else {
                        Value::float64(value as f64)
                    }
                }
                _ => argument,
            }
        }

        mir::CastOperator::UnsignedIntToFloat => {
            let target_width = match target_type {
                mir::Type::Float { width } => *width,
                _ => 64,
            };
            match argument.tag() {
                ValueTag::UInt => {
                    let value = argument.raw_data();
                    if target_width == 32 {
                        Value::float32(value as f32)
                    } else {
                        Value::float64(value as f64)
                    }
                }
                _ => argument,
            }
        }

        mir::CastOperator::FloatExtend => {
            if argument.tag() == ValueTag::Float32 {
                let f = f32::from_bits(argument.raw_data() as u32);
                Value::float64(f as f64)
            } else {
                argument
            }
        }

        mir::CastOperator::FloatTruncate => {
            if argument.tag() == ValueTag::Float64 {
                let f = f64::from_bits(argument.raw_data());
                Value::float32(f as f32)
            } else {
                argument
            }
        }

        mir::CastOperator::PointerToInt => {
            let target_width = match target_type {
                mir::Type::Int { width, .. } => *width as u8,
                _ => 64,
            };
            match argument.tag() {
                ValueTag::RawPointer | ValueTag::ManagedReference => {
                    Value::uint(argument.raw_data(), target_width)
                }
                _ => argument,
            }
        }

        mir::CastOperator::IntToPointer => match argument.tag() {
            ValueTag::UInt | ValueTag::Int => {
                Value::raw_pointer(crate::memory::RawPointer::new(argument.raw_data()))
            }
            _ => argument,
        },
    }
}

// inline helper: load from pointer
fn load_from_pointer(state: &mut ThreadedState, ptr: Value) -> Result<Value, Error> {
    state.interp.statistics.loads += 1;

    match ptr.tag() {
        ValueTag::ManagedReference | ValueTag::Aggregate => {
            let handle = ptr.as_heap_handle().unwrap();
            if handle.is_null() {
                return Err(Error::NullPointerDereference);
            }
            if let Some(cell) = state.interp.managed_heap.get(handle) {
                Ok(cell.slots.first().copied().unwrap_or(Value::VOID))
            } else {
                Err(Error::InvalidHeapHandle)
            }
        }
        ValueTag::RawPointer => {
            let rp = ptr.as_raw_pointer().unwrap();
            if rp.is_null() {
                return Err(Error::NullPointerDereference);
            }
            if let Some(cell) = state.interp.raw_heap.get(rp) {
                Ok(cell.slots.first().copied().unwrap_or(Value::VOID))
            } else {
                Err(Error::InvalidHeapHandle)
            }
        }
        ValueTag::GlobalPointer => {
            let global = ptr.as_global_pointer().unwrap();
            state
                .interp
                .globals
                .get(global)
                .copied()
                .ok_or(Error::UndefinedGlobal { global })
        }
        _ => Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        }),
    }
}

// inline helper: store to pointer
fn store_to_pointer(state: &mut ThreadedState, ptr: Value, val: Value) -> Result<(), Error> {
    state.interp.statistics.stores += 1;

    match ptr.tag() {
        ValueTag::ManagedReference => {
            let handle = ptr.as_heap_handle().unwrap();
            if handle.is_null() {
                return Err(Error::NullPointerDereference);
            }
            if let Some(cell) = state.interp.managed_heap.get_mut(handle) {
                if cell.slots.is_empty() {
                    cell.slots.push(val);
                } else {
                    cell.slots[0] = val;
                }
                Ok(())
            } else {
                Err(Error::InvalidHeapHandle)
            }
        }
        ValueTag::RawPointer => {
            let rp = ptr.as_raw_pointer().unwrap();
            if rp.is_null() {
                return Err(Error::NullPointerDereference);
            }
            if let Some(cell) = state.interp.raw_heap.get_mut(rp) {
                if cell.slots.is_empty() {
                    cell.slots.push(val);
                } else {
                    cell.slots[0] = val;
                }
                Ok(())
            } else {
                Err(Error::InvalidHeapHandle)
            }
        }
        ValueTag::GlobalPointer => {
            let global = ptr.as_global_pointer().unwrap();
            let global_def = state.interp.tree.get(global);
            if !global_def.is_mutable() {
                return Err(Error::ImmutableGlobalWrite { global });
            }
            state.interp.globals.set(global, val);
            Ok(())
        }
        _ => Err(Error::InvalidPointerType {
            actual: format!("{ptr:?}"),
        }),
    }
}

// inline helper: get field from aggregate
fn get_field(state: &mut ThreadedState, agg: Value, index: u32) -> Result<Value, Error> {
    match agg.tag() {
        ValueTag::Aggregate | ValueTag::ManagedReference => {
            let handle = agg.as_heap_handle().unwrap();
            if handle.is_null() {
                return Err(Error::NullPointerDereference);
            }
            if let Some(cell) = state.interp.managed_heap.get(handle) {
                cell.slots.get(index as usize).copied().ok_or_else(|| {
                    Error::InvalidFieldAccess {
                        index,
                        field_count: cell.slots.len(),
                    }
                })
            } else {
                Err(Error::InvalidHeapHandle)
            }
        }
        ValueTag::RawPointer => {
            let rp = agg.as_raw_pointer().unwrap();
            if rp.is_null() {
                return Err(Error::NullPointerDereference);
            }
            if let Some(cell) = state.interp.raw_heap.get(rp) {
                cell.slots.get(index as usize).copied().ok_or_else(|| {
                    Error::InvalidFieldAccess {
                        index,
                        field_count: cell.slots.len(),
                    }
                })
            } else {
                Err(Error::InvalidHeapHandle)
            }
        }
        _ => Err(Error::TypeMismatch {
            expected: "aggregate".to_string(),
            actual: format!("{agg:?}"),
        }),
    }
}

// inline helper: set field on aggregate
fn set_field(
    state: &mut ThreadedState,
    agg: Value,
    index: u32,
    val: Value,
) -> Result<Value, Error> {
    match agg.tag() {
        ValueTag::Aggregate | ValueTag::ManagedReference => {
            let handle = agg.as_heap_handle().unwrap();
            if handle.is_null() {
                return Err(Error::NullPointerDereference);
            }
            let mut invalid_field_count = None;
            if let Some(cell) = state.interp.managed_heap.get_mut(handle) {
                if (index as usize) < cell.slots.len() {
                    cell.slots[index as usize] = val;
                } else {
                    invalid_field_count = Some(cell.slots.len());
                }
            } else {
                return Err(Error::InvalidHeapHandle);
            }
            if let Some(field_count) = invalid_field_count {
                return Err(Error::InvalidFieldAccess { index, field_count });
            }
            Ok(agg)
        }
        ValueTag::RawPointer => {
            let rp = agg.as_raw_pointer().unwrap();
            if rp.is_null() {
                return Err(Error::NullPointerDereference);
            }
            let mut invalid_field_count = None;
            if let Some(cell) = state.interp.raw_heap.get_mut(rp) {
                if (index as usize) < cell.slots.len() {
                    cell.slots[index as usize] = val;
                } else {
                    invalid_field_count = Some(cell.slots.len());
                }
            } else {
                return Err(Error::InvalidHeapHandle);
            }
            if let Some(field_count) = invalid_field_count {
                return Err(Error::InvalidFieldAccess { index, field_count });
            }
            Ok(Value::raw_pointer(rp))
        }
        _ => Err(Error::TypeMismatch {
            expected: "aggregate".to_string(),
            actual: format!("{agg:?}"),
        }),
    }
}

// inline helper: get element from array
fn get_element(state: &mut ThreadedState, arr: Value, index: u64) -> Result<Value, Error> {
    match arr.tag() {
        ValueTag::Aggregate | ValueTag::ManagedReference => {
            let handle = arr.as_heap_handle().unwrap();
            if handle.is_null() {
                return Err(Error::NullPointerDereference);
            }
            if let Some(cell) = state.interp.managed_heap.get(handle) {
                cell.slots.get(index as usize).copied().ok_or_else(|| {
                    Error::InvalidArrayAccess {
                        index,
                        length: cell.slots.len() as u64,
                    }
                })
            } else {
                Err(Error::InvalidHeapHandle)
            }
        }
        ValueTag::RawPointer => {
            let rp = arr.as_raw_pointer().unwrap();
            if rp.is_null() {
                return Err(Error::NullPointerDereference);
            }
            if let Some(cell) = state.interp.raw_heap.get(rp) {
                cell.slots.get(index as usize).copied().ok_or_else(|| {
                    Error::InvalidArrayAccess {
                        index,
                        length: cell.slots.len() as u64,
                    }
                })
            } else {
                Err(Error::InvalidHeapHandle)
            }
        }
        _ => Err(Error::TypeMismatch {
            expected: "array".to_string(),
            actual: format!("{arr:?}"),
        }),
    }
}

// inline helper: set element in array
fn set_element(
    state: &mut ThreadedState,
    arr: Value,
    index: u64,
    val: Value,
) -> Result<Value, Error> {
    match arr.tag() {
        ValueTag::Aggregate | ValueTag::ManagedReference => {
            let handle = arr.as_heap_handle().unwrap();
            if handle.is_null() {
                return Err(Error::NullPointerDereference);
            }
            let mut invalid_length = None;
            if let Some(cell) = state.interp.managed_heap.get_mut(handle) {
                if (index as usize) < cell.slots.len() {
                    cell.slots[index as usize] = val;
                } else {
                    invalid_length = Some(cell.slots.len() as u64);
                }
            } else {
                return Err(Error::InvalidHeapHandle);
            }
            if let Some(length) = invalid_length {
                return Err(Error::InvalidArrayAccess { index, length });
            }
            Ok(arr)
        }
        ValueTag::RawPointer => {
            let rp = arr.as_raw_pointer().unwrap();
            if rp.is_null() {
                return Err(Error::NullPointerDereference);
            }
            let mut invalid_length = None;
            if let Some(cell) = state.interp.raw_heap.get_mut(rp) {
                if (index as usize) < cell.slots.len() {
                    cell.slots[index as usize] = val;
                } else {
                    invalid_length = Some(cell.slots.len() as u64);
                }
            } else {
                return Err(Error::InvalidHeapHandle);
            }
            if let Some(length) = invalid_length {
                return Err(Error::InvalidArrayAccess { index, length });
            }
            Ok(Value::raw_pointer(rp))
        }
        _ => Err(Error::TypeMismatch {
            expected: "array".to_string(),
            actual: format!("{arr:?}"),
        }),
    }
}

// helper: truncate signed integer
fn truncate_signed(value: i64, width: u8) -> i64 {
    if width >= 64 {
        return value;
    }
    let mask = (1u64 << width) - 1;
    let masked = (value as u64) & mask;
    let sign_bit = 1u64 << (width - 1);
    if masked & sign_bit != 0 {
        (masked | !mask) as i64
    } else {
        masked as i64
    }
}

// helper: truncate unsigned integer
fn truncate_unsigned(value: u64, width: u8) -> u64 {
    if width >= 64 {
        return value;
    }
    let mask = (1u64 << width) - 1;
    value & mask
}

// helper: sign extend
fn sign_extend(value: i64, from_width: u8, to_width: u8) -> i64 {
    if from_width >= to_width || from_width >= 64 {
        return value;
    }
    truncate_signed(value, from_width)
}
