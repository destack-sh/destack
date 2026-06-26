use std::collections::HashMap;

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, PipelineContext};
use destack_mir::{Mutation, RangeAnalysis, RangeMap, ValueRange, ValueTypes};

/// Integer widths supported by the textual MIR primitive type grammar.
const SUPPORTED_INTEGER_WIDTHS: [u16; 6] = [8, 16, 32, 64, 128, 256];

declare_pass! {
    /// NarrowValues integer operands for comparisons and bounds checks.
    ///
    /// This pass inserts truncating casts where the upper bits are provably unused.
    /// This reduces comparison operand widths without changing observable semantics.
    ///
    /// ```mir
    /// function before(v0: uint32, v1: uint32): boolean {
    /// b0(v0: uint32, v1: uint32):
    ///     v2 = int.lt.u v0, v1
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: uint32, v1: uint32): boolean {
    /// b0(v0: uint32, v1: uint32):
    ///     v2 = cast.truncate v0 -> uint8
    ///     v3 = cast.truncate v1 -> uint8
    ///     v4 = int.lt.u v2, v3
    ///     return v4
    /// }
    /// ```
    #[pass(id = "narrow-values")]
    pub NarrowValues,
    "NarrowValues comparison operands using range information"
}

impl FunctionPass for NarrowValues {
    /// Run operand narrowing on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        _ctx: &PipelineContext<'_>,
        analyses: &mir::FunctionAnalyses,
    ) -> Mutation {
        // skip imported functions
        if function.entry().is_none() {
            return Mutation::NONE;
        }

        // gather analyses
        let ranges = analyses.get::<RangeAnalysis>(function, tree).clone();
        let value_types = analyses.get::<ValueTypes>(function, tree);

        // apply narrowing
        let changed = run_narrow(function, tree, &ranges, &value_types);
        if changed {
            Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }

    /// Return the display name for this pass.
    fn name(&self) -> &'static str {
        "NarrowValues"
    }

    /// Return the pipeline identifier for this pass.
    fn id(&self) -> &'static str {
        "narrow-values"
    }
}

/// Holds integer width details needed for narrowing.
#[derive(Debug, Clone, Copy)]
struct IntegerInfo {
    /// The original integer width.
    original_width: u16,
    /// The minimal required width for the value range.
    required_width: u16,
    /// Whether the value is signed.
    signed: bool,
}

/// Run narrowing on a single function and report whether it changed.
fn run_narrow(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    ranges: &RangeAnalysis,
    value_types: &ValueTypes,
) -> bool {
    // refresh value ids before inserting casts
    function.recompute_next_value_id(tree);

    // track whether we rewrote any instructions
    let mut changed = false;

    // iterate blocks in function order
    let block_ids = function.blocks().to_vec();
    for block_id in block_ids {
        // snapshot block state and range info
        let block = tree.get(block_id).clone();
        let block_ranges = ranges.exit(block_id);

        // initialize per block caches
        let mut new_instructions = Vec::new();
        let mut cast_cache: HashMap<(mir::Value, u16, bool), mir::Value> = HashMap::new();
        let mut type_cache: HashMap<(u16, bool), mir::LocalNodeId<mir::Type>> = HashMap::new();

        // reuse the widest cast per value within this block
        let mut value_cast_width: HashMap<mir::Value, u16> = HashMap::new();

        // rewrite instructions with narrower operands
        for &instruction_id in &block.instructions {
            let mut instruction = tree.get(instruction_id).clone();
            let mut updated = false;

            // narrow comparison operands when ranges permit
            if let mir::Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } = instruction
                && operator.is_comparison()
                && let Some((new_left, new_right)) = narrow_pair(
                    left,
                    right,
                    function,
                    tree,
                    &mut new_instructions,
                    &mut cast_cache,
                    &mut type_cache,
                    &mut value_cast_width,
                    block_ranges,
                    value_types,
                )
            {
                instruction = mir::Instruction::Binary {
                    destination,
                    operator,
                    left: new_left,
                    right: new_right,
                };
                updated = true;
            }

            // commit instruction updates when changed
            if updated {
                tree.set(instruction_id, instruction);
                changed = true;
            }
            new_instructions.push(instruction_id);
        }

        // narrow terminator operands for bounds checks
        let terminator = tree.get(block.terminator).clone();
        let mut new_terminator = terminator.clone();
        if let mir::Terminator::Check {
            constraint,
            success,
            failure,
        } = &terminator
        {
            let mut updated_constraint = constraint.clone();
            let mut updated = false;

            // narrow bounds check operands when ranges permit
            if let mir::CheckConstraint::Bounds {
                index,
                length,
                collection,
                is_signed,
            } = &updated_constraint
                && let Some((new_index, new_length)) = narrow_pair(
                    *index,
                    *length,
                    function,
                    tree,
                    &mut new_instructions,
                    &mut cast_cache,
                    &mut type_cache,
                    &mut value_cast_width,
                    block_ranges,
                    value_types,
                )
            {
                updated_constraint = mir::CheckConstraint::Bounds {
                    index: new_index,
                    length: new_length,
                    collection: *collection,
                    is_signed: *is_signed,
                };
                updated = true;
            }

            // replace the terminator when a constraint changed
            if updated {
                new_terminator = mir::Terminator::Check {
                    constraint: updated_constraint,
                    success: success.clone(),
                    failure: failure.clone(),
                };
                changed = true;
            }
        }

        // update the block when instruction or terminator changed
        if new_instructions != block.instructions || new_terminator != terminator {
            let terminator_id = block.terminator;
            function.replace_block_instructions(block_id, new_instructions, tree);
            tree.set(terminator_id, new_terminator);
            changed = true;
        }
    }

    changed
}

/// Compute the minimal integer width that contains the range.
fn required_integer_width(
    min: i128,
    max: i128,
    is_signed: bool,
    original_width: u16,
) -> Option<u16> {
    // reject invalid widths
    if original_width == 0 {
        return None;
    }

    // handle unsigned widths from the maximum value
    if !is_signed {
        if min < 0 {
            return None;
        }

        let max = max as u128;
        let width = if max == 0 {
            1
        } else {
            (128 - max.leading_zeros()) as u16
        };
        return supported_integer_width(width, original_width);
    }

    // find the smallest signed width that contains the range
    for width in 1..=original_width {
        let min_bound = -(1i128 << (width - 1));
        let max_bound = (1i128 << (width - 1)) - 1;
        if min >= min_bound && max <= max_bound {
            return supported_integer_width(width, original_width);
        }
    }

    None
}

/// Return the smallest MIR-supported integer width for one required width.
fn supported_integer_width(required_width: u16, original_width: u16) -> Option<u16> {
    SUPPORTED_INTEGER_WIDTHS
        .into_iter()
        .find(|width| *width >= required_width && *width <= original_width)
}

/// Return integer range and type details needed for narrowing.
fn integer_info_for_value(
    value: mir::Value,
    ranges: &RangeMap,
    value_types: &ValueTypes,
    tree: &mut mir::Tree,
) -> Option<IntegerInfo> {
    // fetch the integer range for this value
    let range = ranges.get(value)?;
    let ValueRange::Integer {
        min,
        max,
        width,
        is_signed,
    } = range
    else {
        return None;
    };

    // require a matching integer type
    let type_id = value_types.expect_value_type(value);
    let original = tree.get(type_id);
    let mir::Type::Int {
        width: original_width,
        is_signed: signed,
    } = original
    else {
        return None;
    };
    let (original_width, original_signed) = (*original_width, *signed);

    if original_signed != *is_signed || original_width != *width {
        return None;
    }

    // compute the smallest width that preserves the range
    let required_width = required_integer_width(*min, *max, *is_signed, *width)?;
    if required_width >= original_width {
        return None;
    }

    Some(IntegerInfo {
        original_width,
        required_width,
        signed: *is_signed,
    })
}

/// NarrowValues a pair of operands when the range allows it.
fn narrow_pair(
    left: mir::Value,
    right: mir::Value,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    new_instructions: &mut Vec<mir::LocalNodeId<mir::Instruction>>,
    cast_cache: &mut HashMap<(mir::Value, u16, bool), mir::Value>,
    type_cache: &mut HashMap<(u16, bool), mir::LocalNodeId<mir::Type>>,
    value_cast_width: &mut HashMap<mir::Value, u16>,
    ranges: &RangeMap,
    value_types: &ValueTypes,
) -> Option<(mir::Value, mir::Value)> {
    // compute range info for both operands
    let left_info = integer_info_for_value(left, ranges, value_types, tree)?;
    let right_info = integer_info_for_value(right, ranges, value_types, tree)?;

    // require compatible operand types
    if left_info.signed != right_info.signed
        || left_info.original_width != right_info.original_width
    {
        return None;
    }

    // compute the widest narrow width required by either operand
    let mut target_width = left_info.required_width.max(right_info.required_width);
    if let Some(existing) = value_cast_width.get(&left) {
        target_width = target_width.max(*existing);
    }
    if let Some(existing) = value_cast_width.get(&right) {
        target_width = target_width.max(*existing);
    }
    if target_width >= left_info.original_width {
        return None;
    }

    let left_cast = narrow_value_to_width(
        left,
        target_width,
        left_info.signed,
        function,
        tree,
        new_instructions,
        cast_cache,
        type_cache,
    );
    let right_cast = narrow_value_to_width(
        right,
        target_width,
        right_info.signed,
        function,
        tree,
        new_instructions,
        cast_cache,
        type_cache,
    );

    value_cast_width.insert(left, target_width);
    value_cast_width.insert(right, target_width);

    Some((left_cast, right_cast))
}

/// Insert or reuse a truncation cast for a narrower width.
fn narrow_value_to_width(
    value: mir::Value,
    width: u16,
    signed: bool,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    new_instructions: &mut Vec<mir::LocalNodeId<mir::Instruction>>,
    cast_cache: &mut HashMap<(mir::Value, u16, bool), mir::Value>,
    type_cache: &mut HashMap<(u16, bool), mir::LocalNodeId<mir::Type>>,
) -> mir::Value {
    // reuse any existing cast for this value and width
    let cache_key = (value, width, signed);
    if let Some(cached) = cast_cache.get(&cache_key).copied() {
        return cached;
    }

    // cache or create the narrower integer type
    let ty_id = *type_cache.entry((width, signed)).or_insert_with(|| {
        tree.insert_type(mir::Type::Int {
            width,
            is_signed: signed,
        })
    });

    // insert a truncating cast before the use
    let destination = function.next_typed_value(ty_id);
    let cast = mir::Instruction::Cast {
        destination,
        operator: mir::CastOperator::Truncate,
        argument: value,
        to_type: ty_id,
    };
    let cast_id = tree.insert(cast);
    new_instructions.push(cast_id);
    cast_cache.insert(cache_key, destination);

    destination
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// NarrowValuesing inserts truncation casts before integer comparisons.
    #[test]
    fn test_narrow_comparison_operands() {
        let input = r#"
function test(): boolean {
entry:
    v0: uint32 = 3
    v1: uint32 = 4
    v2: boolean = int.lt.u v0, v1
    v3: boolean = int.lt.u v0, v0
    v4: boolean = int.and v2, v3
    return v4
}
"#;

        let expected = r#"
function test(): boolean {
entry:
    v0: uint32 = 3
    v1: uint32 = 4
    v5: uint8 = cast.truncate v0 -> uint8
    v6: uint8 = cast.truncate v1 -> uint8
    v2: boolean = int.lt.u v5, v6
    v3: boolean = int.lt.u v5, v5
    v4: boolean = int.and v2, v3
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&NarrowValues);
        test.assert_output(expected);
    }

    /// Signed comparisons are narrowed with signed types.
    #[test]
    fn test_narrow_signed_comparison() {
        let input = r#"
function test(): boolean {
entry:
    v0: int32 = 0
    v1: int32 = 1
    v2: boolean = int.lt.s v0, v1
    return v2
}
"#;

        let expected = r#"
function test(): boolean {
entry:
    v0: int32 = 0
    v1: int32 = 1
    v3: int8 = cast.truncate v0 -> int8
    v4: int8 = cast.truncate v1 -> int8
    v2: boolean = int.lt.s v3, v4
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&NarrowValues);
        test.assert_output(expected);
    }

    /// Bounds checks are narrowed when indices fit within smaller widths.
    #[test]
    fn test_narrow_bounds_check_operands() {
        let input = r#"
function test(v0: [uint8; 8]): uint8 {
entry(v0: [uint8; 8]):
    v1: uint32 = 2
    v2: uint32 = 4
    v3: boolean = int.lt.u v1, v2
    check bounds.u v1, v2, v0 => b1, b2

b1:
    v4: uint8 = field.get v0, 0
    return v4

b2:
    unreachable
}
"#;

        let expected = r#"
function test(v0: [uint8; 8]): uint8 {
entry(v0: [uint8; 8]):
    v1: uint32 = 2
    v2: uint32 = 4
    v5: uint8 = cast.truncate v1 -> uint8
    v6: uint8 = cast.truncate v2 -> uint8
    v3: boolean = int.lt.u v5, v6
    check bounds.u v5, v6, v0 => b1, b2

b1:
    v4: uint8 = field.get v0, 0
    return v4

b2:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&NarrowValues);
        test.assert_output(expected);
    }

    /// Comparisons with unknown ranges are left unchanged.
    #[test]
    fn test_narrow_skips_unknown_ranges() {
        let input = r#"
function test(v0: uint32, v1: uint32): boolean {
entry(v0: uint32, v1: uint32):
    v2: boolean = int.lt.u v0, v1
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&NarrowValues);
        test.assert_output(input);
    }

    /// Mismatched integer widths are not narrowed.
    #[test]
    fn test_narrow_skips_mismatched_widths() {
        let input = r#"
function test(v0: [uint8; 8]): void {
entry(v0: [uint8; 8]):
    v1: uint32 = 2
    v2: uint64 = 4
    v3: boolean = int.lt.u v1, v2
    check bounds.u v1, v2, v0 => b1, b2

b1:
    return

b2:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&NarrowValues);
        test.assert_output(input);
    }

    /// Full range signed values are not narrowed.
    #[test]
    fn test_narrow_skips_full_range_signed() {
        let input = r#"
function test(): boolean {
entry:
    v0: int32 = -2147483648
    v1: int32 = 2147483647
    v2: boolean = int.lt.s v0, v1
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&NarrowValues);
        test.assert_output(input);
    }
}
