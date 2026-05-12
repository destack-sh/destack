use crate::Word;
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{
    BoundsCheck, Check, CheckId, Edge, EdgeId, Instruction, MoveRange, NarrowCheck, OverflowCheck,
    ShiftRangeCheck, SwitchCasesId, SwitchTableId, Transfer, UnionCheck,
};
use {destack_engine as engine, destack_mir as mir};

const SWITCH_SIGN_BIT: u32 = 1 << 16;
const SWITCH_WIDTH_MASK: u32 = SWITCH_SIGN_BIT - 1;

/// Return one pooled control edge.
#[inline(always)]
fn control_edge(machine: &Machine<'_, '_>, id: u32) -> Edge {
    let table = machine.side_table_ptr();

    unsafe { (*table).edge(EdgeId(id)) }
}

/// Return one branch jump based on the evaluated condition.
#[inline(always)]
fn branch_transfer(is_truthy: bool, then_edge: Edge, else_edge: Edge) -> Transfer {
    let edge = if is_truthy { then_edge } else { else_edge };

    Transfer::Jump {
        block: edge.target,
        moves: edge.moves,
    }
}

/// Load one fused comparison branch.
#[inline(always)]
fn compare_branch_words<'a>(
    machine: &Machine<'_, 'a>,
    instruction: &'a Instruction,
) -> (Word, Word, Edge, Edge) {
    let left = machine.load_word_at(instruction.a);
    let right = machine.load_word_at(instruction.b);
    let then_edge = control_edge(machine, instruction.c);
    let else_edge = control_edge(machine, instruction.d);

    (left, right, then_edge, else_edge)
}

/// Return one fused comparison branch transfer.
#[inline(always)]
fn compare_branch_transfer(then_edge: Edge, else_edge: Edge, is_truthy: bool) -> Transfer {
    branch_transfer(is_truthy, then_edge, else_edge)
}

macro_rules! fixed_compare_branch_executor {
    ($(#[$doc:meta] $name:ident => $ty:ty, $operation:tt,)+) => {
        $(
            #[$doc]
            #[inline(always)]
            pub(crate) fn $name(
                machine: &mut Machine<'_, '_>,
                instruction: &Instruction,
            ) -> Transfer {
                let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
                let left = left.bits() as $ty;
                let right = right.bits() as $ty;
                let is_truthy = left $operation right;

                compare_branch_transfer(then_edge, else_edge, is_truthy)
            }
        )+
    };
}

/// Return one default switch jump.
#[inline(always)]
fn default_switch_transfer(edge: Edge) -> Transfer {
    Transfer::Jump {
        block: edge.target,
        moves: edge.moves,
    }
}

/// Load one wide switch value as an integer case value.
#[inline(always)]
fn load_wide_switch_value<const IS_SIGNED: bool>(
    machine: &Machine<'_, '_>,
    offset: u32,
    width: u32,
) -> Result<Option<i128>, Error> {
    let width = width as u16;
    let byte_len = width.div_ceil(8) as usize;
    let address = machine.frame_pointer_at(offset).address() as *const u8;
    let bytes = unsafe { std::slice::from_raw_parts(address, byte_len) };

    integer_bytes_to_case_value::<IS_SIGNED>(bytes, width)
}

/// Load one word switch value as an integer case value.
#[inline(always)]
fn load_word_switch_value<const IS_SIGNED: bool>(machine: &Machine<'_, '_>, offset: u32) -> i128 {
    let value = machine.load_word_at(offset);

    if IS_SIGNED {
        return value.as_i64() as i128;
    }

    value.as_u64() as i128
}

/// Decode integer bytes into the switch case domain.
fn integer_bytes_to_case_value<const IS_SIGNED: bool>(
    bytes: &[u8],
    width: u16,
) -> Result<Option<i128>, Error> {
    let byte_len = usize::from(width.div_ceil(8));
    let copied = byte_len.min(bytes.len()).min(16);
    let mut value = [0u8; 16];

    value[..copied].copy_from_slice(&bytes[..copied]);

    if width == 0 {
        return Ok(Some(0));
    }

    let sign_bit = 1u8 << ((width - 1) % 8);
    let sign_byte = usize::from((width - 1) / 8);
    let is_negative = IS_SIGNED && sign_byte < value.len() && value[sign_byte] & sign_bit != 0;

    if is_negative {
        value[copied..].fill(0xff);
    }

    let excess_bits = byte_len * 8 - usize::from(width);
    if excess_bits > 0 && sign_byte < value.len() {
        let mask = 0xffu8 >> excess_bits;
        value[sign_byte] &= mask;
        if is_negative {
            value[sign_byte] |= !mask;
        }
    }

    if IS_SIGNED {
        return Ok(Some(i128::from_le_bytes(value)));
    }

    let value = u128::from_le_bytes(value);

    Ok(i128::try_from(value).ok())
}

/// Return one packed switch layout.
#[inline(always)]
fn switch_layout(field: u32) -> (u32, bool) {
    let width = field & SWITCH_WIDTH_MASK;
    let is_signed = field & SWITCH_SIGN_BIT != 0;

    (width, is_signed)
}

/// Load one word as a signed integer.
#[inline(always)]
fn load_signed_word(machine: &Machine<'_, '_>, offset: u32) -> i64 {
    machine.load_word_at(offset).as_i64()
}

/// Load one word as an unsigned integer.
#[inline(always)]
fn load_unsigned_word(machine: &Machine<'_, '_>, offset: u32) -> u64 {
    machine.load_word_at(offset).as_u64()
}

/// Load one unsigned word as a non-negative length.
#[inline(always)]
fn load_unsigned_length_word(machine: &Machine<'_, '_>, offset: u32) -> u64 {
    load_unsigned_word(machine, offset)
}

/// Load one signed word as a non-negative length.
#[inline(always)]
fn load_signed_length_word(machine: &Machine<'_, '_>, offset: u32) -> Result<u64, Error> {
    let value = load_signed_word(machine, offset);
    if value < 0 {
        return Err(Error::TypeMismatch {
            expected: "non negative integer".to_string(),
            actual: format!("{value:?}"),
        });
    }

    Ok(value as u64)
}

/// Evaluate one bounds check.
#[inline(always)]
fn bounds_check<const INDEX_SIGNED: bool, const LENGTH_SIGNED: bool>(
    machine: &Machine<'_, '_>,
    check: BoundsCheck,
) -> Result<bool, Error> {
    let length = if LENGTH_SIGNED {
        load_signed_length_word(machine, check.length)?
    } else {
        load_unsigned_length_word(machine, check.length)
    };

    if INDEX_SIGNED {
        let index = load_signed_word(machine, check.index);

        return Ok(index >= 0 && (index as u64) < length);
    }

    let index = load_unsigned_word(machine, check.index);

    Ok(index < length)
}

/// Evaluate one shift range check.
#[inline(always)]
fn shift_range_check<const IS_SIGNED: bool>(
    machine: &Machine<'_, '_>,
    check: ShiftRangeCheck,
) -> bool {
    let bit_width = u64::from(check.bit_width);

    if IS_SIGNED {
        let value = load_signed_word(machine, check.value);

        return value >= 0 && (value as u64) < bit_width;
    }

    let value = load_unsigned_word(machine, check.value);

    value < bit_width
}

/// Evaluate one narrow check.
#[inline(always)]
fn narrow_check<const IS_SIGNED: bool>(machine: &Machine<'_, '_>, check: NarrowCheck) -> bool {
    let target_width = u32::from(check.to_width);

    if IS_SIGNED {
        let value = load_signed_word(machine, check.value);
        let shift = target_width - 1;
        let min_value = -(1_i128 << shift);
        let max_value = (1_i128 << shift) - 1;
        let value = value as i128;

        return value >= min_value && value <= max_value;
    }

    let value = load_unsigned_word(machine, check.value);
    let max_value = if target_width >= 64 {
        u128::from(u64::MAX)
    } else {
        (1_u128 << target_width) - 1
    };

    u128::from(value) <= max_value
}

/// Evaluate one union tag check.
#[inline(always)]
fn union_check<const IS_SIGNED: bool>(machine: &Machine<'_, '_>, check: UnionCheck) -> bool {
    if IS_SIGNED {
        let actual = load_signed_word(machine, check.value);

        return actual >= 0 && actual as u64 == check.expected;
    }

    let actual = load_unsigned_word(machine, check.value);

    actual == check.expected
}

/// Return signed bounds for one integer width.
fn signed_bounds(width: u8) -> (i128, i128) {
    debug_assert!(width > 0);

    let shift = u32::from(width - 1);
    let min_value = -(1_i128 << shift);
    let max_value = (1_i128 << shift) - 1;

    (min_value, max_value)
}

/// Return unsigned max for one integer width.
fn unsigned_max(width: u8) -> u128 {
    if width >= 64 {
        return u128::from(u64::MAX);
    }

    (1_u128 << u32::from(width)) - 1
}

/// Load signed overflow inputs.
fn signed_overflow_inputs(machine: &Machine<'_, '_>, check: OverflowCheck) -> (i128, i128) {
    let left = load_signed_word(machine, check.left) as i128;
    let right = load_signed_word(machine, check.right) as i128;

    (left, right)
}

/// Load unsigned overflow inputs.
fn unsigned_overflow_inputs(machine: &Machine<'_, '_>, check: OverflowCheck) -> (u128, u128) {
    let left = u128::from(load_unsigned_word(machine, check.left));
    let right = u128::from(load_unsigned_word(machine, check.right));

    (left, right)
}

/// Evaluate signed add overflow.
fn overflow_add_int(machine: &Machine<'_, '_>, check: OverflowCheck) -> bool {
    let (left, right) = signed_overflow_inputs(machine, check);
    let (min_value, max_value) = signed_bounds(check.width);
    let result = left + right;

    result < min_value || result > max_value
}

/// Evaluate unsigned add overflow.
fn overflow_add_uint(machine: &Machine<'_, '_>, check: OverflowCheck) -> bool {
    let (left, right) = unsigned_overflow_inputs(machine, check);
    let max_value = unsigned_max(check.width);

    left + right > max_value
}

/// Evaluate signed subtract overflow.
fn overflow_sub_int(machine: &Machine<'_, '_>, check: OverflowCheck) -> bool {
    let (left, right) = signed_overflow_inputs(machine, check);
    let (min_value, max_value) = signed_bounds(check.width);
    let result = left - right;

    result < min_value || result > max_value
}

/// Evaluate unsigned subtract overflow.
fn overflow_sub_uint(machine: &Machine<'_, '_>, check: OverflowCheck) -> bool {
    let (left, right) = unsigned_overflow_inputs(machine, check);

    left < right
}

/// Evaluate signed multiply overflow.
fn overflow_mul_int(machine: &Machine<'_, '_>, check: OverflowCheck) -> bool {
    let (left, right) = signed_overflow_inputs(machine, check);
    let (min_value, max_value) = signed_bounds(check.width);
    let result = left * right;

    result < min_value || result > max_value
}

/// Evaluate unsigned multiply overflow.
fn overflow_mul_uint(machine: &Machine<'_, '_>, check: OverflowCheck) -> bool {
    let (left, right) = unsigned_overflow_inputs(machine, check);
    let max_value = unsigned_max(check.width);

    left != 0 && right > max_value / left
}

/// Evaluate signed divide or remainder overflow.
fn overflow_div_int(machine: &Machine<'_, '_>, check: OverflowCheck) -> Result<bool, Error> {
    let (left, right) = signed_overflow_inputs(machine, check);
    let (min_value, _) = signed_bounds(check.width);
    if right == 0 {
        return Err(Error::DivisionByZero);
    }

    Ok(left == min_value && right == -1)
}

/// Evaluate unsigned divide or remainder overflow.
fn overflow_div_uint(machine: &Machine<'_, '_>, check: OverflowCheck) -> Result<bool, Error> {
    let (_, right) = unsigned_overflow_inputs(machine, check);
    if right == 0 {
        return Err(Error::DivisionByZero);
    }

    Ok(false)
}

/// Evaluate one runtime check guard.
fn evaluate_check(machine: &Machine<'_, '_>, constraint: &Check) -> Result<bool, Error> {
    match constraint {
        Check::BoundsIntInt(check) => bounds_check::<true, true>(machine, *check),
        Check::BoundsIntUint(check) => bounds_check::<true, false>(machine, *check),
        Check::BoundsUintInt(check) => bounds_check::<false, true>(machine, *check),
        Check::BoundsUintUint(check) => bounds_check::<false, false>(machine, *check),
        Check::Null { value } => {
            let value = machine.load_word_at(*value);

            Ok(value.bits() != 0)
        }
        Check::DivZeroInt { divisor } => {
            let value = load_signed_word(machine, *divisor);

            Ok(value != 0)
        }
        Check::DivZeroUint { divisor } => {
            let value = load_unsigned_word(machine, *divisor);

            Ok(value != 0)
        }
        Check::ShiftRangeInt(check) => Ok(shift_range_check::<true>(machine, *check)),
        Check::ShiftRangeUint(check) => Ok(shift_range_check::<false>(machine, *check)),
        Check::NarrowInt(check) => Ok(narrow_check::<true>(machine, *check)),
        Check::NarrowUint(check) => Ok(narrow_check::<false>(machine, *check)),
        Check::OverflowAddInt(check) => Ok(overflow_add_int(machine, *check)),
        Check::OverflowAddUint(check) => Ok(overflow_add_uint(machine, *check)),
        Check::OverflowSubInt(check) => Ok(overflow_sub_int(machine, *check)),
        Check::OverflowSubUint(check) => Ok(overflow_sub_uint(machine, *check)),
        Check::OverflowMulInt(check) => Ok(overflow_mul_int(machine, *check)),
        Check::OverflowMulUint(check) => Ok(overflow_mul_uint(machine, *check)),
        Check::OverflowDivInt(check) => overflow_div_int(machine, *check),
        Check::OverflowDivUint(check) => overflow_div_uint(machine, *check),
        Check::Type { value, expected } => {
            let value = machine.load_word_at(*value);

            Ok(value.as_u64() == u64::from(*expected))
        }
        Check::UnionInt(check) => Ok(union_check::<true>(machine, *check)),
        Check::UnionUint(check) => Ok(union_check::<false>(machine, *check)),
    }
}

/// Execute assume (optimizer hint).
pub(crate) fn execute_assume(
    _machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let _ = instruction;

    Ok(())
}

/// Return an address from a lowered frame offset.
#[inline(always)]
fn frame_address(machine: &Machine<'_, '_>, offset: u32) -> Word {
    Word::frame_pointer(machine.frame_pointer_at(offset))
}

/// Execute word return.
pub(crate) fn execute_return_word(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let return_value = machine.load_word_at(instruction.a);

    Transfer::Return(return_value)
}

/// Execute address return.
pub(crate) fn execute_return_address(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let return_value = frame_address(machine, instruction.a);

    Transfer::Return(return_value)
}

/// Execute void return.
pub(crate) fn execute_return_void(
    _machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let _ = instruction;

    Transfer::Return(Word::VOID)
}

/// Execute word yield.
pub(crate) fn execute_yield_word(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let yield_value = machine.load_word_at(instruction.a);
    let source_type = mir::LocalNodeId::new(instruction.b);
    let frame_state = engine::FrameStateId(instruction.c);

    Transfer::Yield {
        value: yield_value,
        source_type,
        frame_state,
    }
}

/// Execute address yield.
pub(crate) fn execute_yield_address(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let yield_value = frame_address(machine, instruction.a);
    let source_type = mir::LocalNodeId::new(instruction.b);
    let frame_state = engine::FrameStateId(instruction.c);

    Transfer::Yield {
        value: yield_value,
        source_type,
        frame_state,
    }
}

/// Execute unconditional jump (exits tail-call chain).
#[inline(always)]
pub(crate) fn execute_jump(_machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let moves = MoveRange {
        start: instruction.b,
        len: instruction.c,
    };

    // return jump control
    Transfer::Jump {
        block: instruction.a,
        moves,
    }
}

/// Execute boolean branch (exits tail-call chain).
#[inline(always)]
pub(crate) fn execute_branch_bool(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let condition = instruction.a;
    let then_edge = control_edge(machine, instruction.b);
    let else_edge = control_edge(machine, instruction.c);

    // evaluate branch condition
    let condition = machine.load_word_at(condition);
    let is_truthy = condition.bits() != 0;

    branch_transfer(is_truthy, then_edge, else_edge)
}

fixed_compare_branch_executor! {
    /// Execute 32-bit integer equality branch.
    execute_branch_eq_32 => u32, ==,
    /// Execute 64-bit integer equality branch.
    execute_branch_eq_64 => u64, ==,
    /// Execute 32-bit integer inequality branch.
    execute_branch_ne_32 => u32, !=,
    /// Execute 64-bit integer inequality branch.
    execute_branch_ne_64 => u64, !=,
    /// Execute 32-bit signed integer less-than branch.
    execute_branch_lt_i32 => i32, <,
    /// Execute 32-bit unsigned integer less-than branch.
    execute_branch_lt_u32 => u32, <,
    /// Execute 64-bit signed integer less-than branch.
    execute_branch_lt_i64 => i64, <,
    /// Execute 64-bit unsigned integer less-than branch.
    execute_branch_lt_u64 => u64, <,
    /// Execute 32-bit signed integer less-or-equal branch.
    execute_branch_le_i32 => i32, <=,
    /// Execute 32-bit unsigned integer less-or-equal branch.
    execute_branch_le_u32 => u32, <=,
    /// Execute 64-bit signed integer less-or-equal branch.
    execute_branch_le_i64 => i64, <=,
    /// Execute 64-bit unsigned integer less-or-equal branch.
    execute_branch_le_u64 => u64, <=,
    /// Execute 32-bit signed integer greater-than branch.
    execute_branch_gt_i32 => i32, >,
    /// Execute 32-bit unsigned integer greater-than branch.
    execute_branch_gt_u32 => u32, >,
    /// Execute 64-bit signed integer greater-than branch.
    execute_branch_gt_i64 => i64, >,
    /// Execute 64-bit unsigned integer greater-than branch.
    execute_branch_gt_u64 => u64, >,
    /// Execute 32-bit signed integer greater-or-equal branch.
    execute_branch_ge_i32 => i32, >=,
    /// Execute 32-bit unsigned integer greater-or-equal branch.
    execute_branch_ge_u32 => u32, >=,
    /// Execute 64-bit signed integer greater-or-equal branch.
    execute_branch_ge_i64 => i64, >=,
    /// Execute 64-bit unsigned integer greater-or-equal branch.
    execute_branch_ge_u64 => u64, >=,
}

/// Execute runtime check (exits tail-call chain).
pub(crate) fn execute_check(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let table = machine.side_table_ptr();
    let constraint = unsafe { (*table).check(CheckId(instruction.a)) };
    let then_edge = control_edge(machine, instruction.b);
    let else_edge = control_edge(machine, instruction.c);

    // evaluate the runtime guard
    let is_truthy = match evaluate_check(machine, constraint) {
        Ok(is_truthy) => is_truthy,
        Err(error) => return Transfer::Error(error),
    };

    branch_transfer(is_truthy, then_edge, else_edge)
}

/// Execute integer equality branch.
#[inline(always)]
pub(crate) fn execute_branch_eq_word(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.bits() == right.bits();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute integer inequality branch.
#[inline(always)]
pub(crate) fn execute_branch_ne_word(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.bits() != right.bits();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute signed integer less-than branch.
#[inline(always)]
pub(crate) fn execute_branch_lt_word_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = (left.bits() as i64) < (right.bits() as i64);

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute signed integer less-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_le_word_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = (left.bits() as i64) <= (right.bits() as i64);

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute signed integer greater-than branch.
#[inline(always)]
pub(crate) fn execute_branch_gt_word_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = (left.bits() as i64) > (right.bits() as i64);

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute signed integer greater-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_ge_word_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = (left.bits() as i64) >= (right.bits() as i64);

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute unsigned integer less-than branch.
#[inline(always)]
pub(crate) fn execute_branch_lt_word_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.bits() < right.bits();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute unsigned integer less-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_le_word_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.bits() <= right.bits();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute unsigned integer greater-than branch.
#[inline(always)]
pub(crate) fn execute_branch_gt_word_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.bits() > right.bits();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute unsigned integer greater-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_ge_word_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.bits() >= right.bits();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float32 equality branch.
#[inline(always)]
pub(crate) fn execute_branch_eq_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f32() == right.as_f32();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float32 inequality branch.
#[inline(always)]
pub(crate) fn execute_branch_ne_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f32() != right.as_f32();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float32 less-than branch.
#[inline(always)]
pub(crate) fn execute_branch_lt_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f32() < right.as_f32();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float32 less-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_le_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f32() <= right.as_f32();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float32 greater-than branch.
#[inline(always)]
pub(crate) fn execute_branch_gt_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f32() > right.as_f32();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float32 greater-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_ge_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f32() >= right.as_f32();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float64 equality branch.
#[inline(always)]
pub(crate) fn execute_branch_eq_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f64() == right.as_f64();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float64 inequality branch.
#[inline(always)]
pub(crate) fn execute_branch_ne_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f64() != right.as_f64();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float64 less-than branch.
#[inline(always)]
pub(crate) fn execute_branch_lt_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f64() < right.as_f64();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float64 less-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_le_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f64() <= right.as_f64();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float64 greater-than branch.
#[inline(always)]
pub(crate) fn execute_branch_gt_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f64() > right.as_f64();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float64 greater-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_ge_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f64() >= right.as_f64();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute an integer switch.
pub(crate) fn execute_switch(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (width, is_signed) = switch_layout(instruction.d);

    if width <= u64::BITS && is_signed {
        return execute_switch_word::<true>(machine, instruction);
    }

    if width <= u64::BITS {
        return execute_switch_word::<false>(machine, instruction);
    }

    if is_signed {
        return execute_switch_wide::<true>(machine, instruction);
    }

    execute_switch_wide::<false>(machine, instruction)
}

/// Execute an integer switch via dense jump table.
pub(crate) fn execute_switch_table(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (_, is_signed) = switch_layout(instruction.d);

    if is_signed {
        return execute_switch_table_word::<true>(machine, instruction);
    }

    execute_switch_table_word::<false>(machine, instruction)
}

/// Execute a word switch.
fn execute_switch_word<const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let table = machine.side_table_ptr();
    let cases = unsafe { (*table).switch_cases(SwitchCasesId(instruction.b)) };
    let default_edge = control_edge(machine, instruction.c);
    let int_val = load_word_switch_value::<IS_SIGNED>(machine, instruction.a);

    // find matching case
    for case in cases {
        if case.value == int_val {
            return Transfer::Jump {
                block: case.target,
                moves: case.moves,
            };
        }
    }

    default_switch_transfer(default_edge)
}

/// Execute a wide integer switch.
fn execute_switch_wide<const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let table = machine.side_table_ptr();
    let cases = unsafe { (*table).switch_cases(SwitchCasesId(instruction.b)) };
    let default_edge = control_edge(machine, instruction.c);

    // load switch value
    let (width, _) = switch_layout(instruction.d);
    let int_val = match load_wide_switch_value::<IS_SIGNED>(machine, instruction.a, width) {
        Ok(Some(int_val)) => int_val,
        Ok(None) => return default_switch_transfer(default_edge),
        Err(error) => return Transfer::Error(error),
    };

    // find matching case
    for case in cases {
        if case.value == int_val {
            // forward case moves
            return Transfer::Jump {
                block: case.target,
                moves: case.moves,
            };
        }
    }

    // otherwise jump to the default target
    default_switch_transfer(default_edge)
}

/// Execute a word switch via dense jump table.
fn execute_switch_table_word<const IS_SIGNED: bool>(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let side_table = machine.side_table_ptr();
    let table = unsafe { (*side_table).switch_table(SwitchTableId(instruction.b)) };
    let default_edge = control_edge(machine, instruction.c);
    let int_val = load_word_switch_value::<IS_SIGNED>(machine, instruction.a);

    // resolve jump table entry
    if int_val < table.min {
        return default_switch_transfer(default_edge);
    }
    let offset = (int_val - table.min) as usize;
    let Some(case) = table.cases.get(offset) else {
        return default_switch_transfer(default_edge);
    };

    Transfer::Jump {
        block: case.target,
        moves: case.moves,
    }
}

/// Execute abort.
pub(crate) fn execute_abort(
    _machine: &mut Machine<'_, '_>,
    _instruction: &Instruction,
) -> Transfer {
    Transfer::Error(Error::Abort)
}

/// Execute panic.
pub(crate) fn execute_panic(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let payload = machine.load_word_at(instruction.a);
    let message = format!("panic payload: {payload:?}");

    Transfer::Error(Error::Panic { message })
}

/// Execute unreachable (errors).
pub(crate) fn execute_unreachable(
    _machine: &mut Machine<'_, '_>,
    _instruction: &Instruction,
) -> Transfer {
    // return unreachable error
    Transfer::Error(Error::Unreachable)
}
