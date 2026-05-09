use std::collections::HashMap;

use crate::declare_mir_pass;
use destack_mir as mir;

use crate::common::mir::analysis::{RangeAnalysis, RangeMap, ValueRange};
use crate::common::mir::{ValueTypeMap, is_comparison_operator};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_mir_pass! {
    /// Narrow integer operands for comparisons and bounds checks.
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
    #[pass(id = "narrow")]
    pub Narrow,
    "Narrow comparison operands using range information"
}

impl FunctionPass for Narrow {
    /// Run operand narrowing on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip imported functions
        if function.entry.is_none() {
            return AnalysisPreservation::all();
        }

        // gather analyses
        let analyses = ctx.function_analyses(function, tree);
        let ranges = analyses.get::<RangeAnalysis>().clone();
        let value_types = ValueTypeMap::new(function, tree);

        // apply narrowing
        let changed = run_narrow(function, tree, &ranges, &value_types);
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the display name for this pass.
    fn name(&self) -> &'static str {
        "Narrow"
    }

    /// Return the pipeline identifier for this pass.
    fn id(&self) -> &'static str {
        "narrow"
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
    value_types: &ValueTypeMap,
) -> bool {
    // refresh value ids before inserting casts
    function.recompute_next_value_id(tree);

    // track whether we rewrote any instructions
    let mut changed = false;

    // iterate blocks in function order
    let block_ids = function.blocks.clone();
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
                && is_comparison_operator(operator)
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
                    left: new_left.into(),
                    right: new_right.into(),
                };
                updated = true;
            }

            // commit instruction updates when changed
            if updated {
                tree.replace(instruction_id, instruction);
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
                    index: new_index.into(),
                    length: new_length.into(),
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
            let mut updated_block = block;
            updated_block.instructions = new_instructions;
            tree.replace(updated_block.terminator, new_terminator);
            tree.replace(block_id, updated_block);
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
        return Some(width.min(original_width));
    }

    // find the smallest signed width that contains the range
    for width in 1..=original_width {
        let min_bound = -(1i128 << (width - 1));
        let max_bound = (1i128 << (width - 1)) - 1;
        if min >= min_bound && max <= max_bound {
            return Some(width);
        }
    }

    None
}

/// Return integer range and type details needed for narrowing.
fn integer_info_for_value(
    value: mir::ValueReference,
    ranges: &RangeMap,
    value_types: &ValueTypeMap,
    tree: &mut mir::Tree,
) -> Option<IntegerInfo> {
    let value = value.value()?;

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
    let type_id = value_types.require_value_type(value);
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

/// Narrow a pair of operands when the range allows it.
#[allow(clippy::too_many_arguments)]
fn narrow_pair(
    left: mir::ValueReference,
    right: mir::ValueReference,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    new_instructions: &mut Vec<mir::LocalNodeId<mir::Instruction>>,
    cast_cache: &mut HashMap<(mir::Value, u16, bool), mir::Value>,
    type_cache: &mut HashMap<(u16, bool), mir::LocalNodeId<mir::Type>>,
    value_cast_width: &mut HashMap<mir::Value, u16>,
    ranges: &RangeMap,
    value_types: &ValueTypeMap,
) -> Option<(mir::Value, mir::Value)> {
    let left = left.value()?;
    let right = right.value()?;

    // compute range info for both operands
    let left_info = integer_info_for_value(left.into(), ranges, value_types, tree)?;
    let right_info = integer_info_for_value(right.into(), ranges, value_types, tree)?;

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
#[allow(clippy::too_many_arguments)]
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
        destination: destination.into(),
        operator: mir::CastOperator::Truncate,
        argument: value.into(),
        to_type: ty_id.into(),
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

    /// Narrowing inserts truncation casts before integer comparisons.
    #[test]
    fn test_narrow_comparison_operands() {
        let input = r#"
function test(): boolean {
b0:
    v0: uint32 = 3uint32
    v1: uint32 = 4uint32
    v2: boolean = int.lt.u v0, v1
    v3: boolean = int.lt.u v0, v0
    v4: boolean = int.and v2, v3
    return v4
}"#;

        let expected = r#"
function test(): boolean {
b0:
    v0: uint32 = 3uint32
    v1: uint32 = 4uint32
    v2: u3 = cast.truncate v0 -> u3
    v3: u3 = cast.truncate v1 -> u3
    v4: boolean = int.lt.u v2, v3
    v5: boolean = int.lt.u v2, v2
    v6: boolean = int.and v4, v5
    return v6
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Narrow);
        test.assert_output(expected);
    }

    /// Signed comparisons are narrowed with signed types.
    #[test]
    fn test_narrow_signed_comparison() {
        let input = r#"
function test(): boolean {
b0:
    v0: int32 = 0int32
    v1: int32 = 1int32
    v2: boolean = int.lt.s v0, v1
    return v2
}"#;

        let expected = r#"
function test(): boolean {
b0:
    v0: int32 = 0int32
    v1: int32 = 1int32
    v2: i2 = cast.truncate v0 -> i2
    v3: i2 = cast.truncate v1 -> i2
    v4: boolean = int.lt.s v2, v3
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Narrow);
        test.assert_output(expected);
    }

    /// Bounds checks are narrowed when indices fit within smaller widths.
    #[test]
    fn test_narrow_bounds_check_operands() {
        let input = r#"
function test(v0: uint8[8]): uint8 {
b0(v0: uint8[8]):
    v1: uint32 = 2uint32
    v2: uint32 = 4uint32
    v3: boolean = int.lt.u v1, v2
    check bounds.u v1, v2, v0 -> b1, b2
b1:
    v4: uint8 = element.get v0, v1
    return v4
b2:
    unreachable
}"#;

        let expected = r#"
function test(v0: uint8[8]): uint8 {
b0(v0: uint8[8]):
    v1: uint32 = 2uint32
    v2: uint32 = 4uint32
    v3: u3 = cast.truncate v1 -> u3
    v4: u3 = cast.truncate v2 -> u3
    v5: boolean = int.lt.u v3, v4
    check bounds.u v3, v4, v0 -> b1, b2
b1:
    v6: uint8 = element.get v0, v1
    return v6
b2:
    unreachable
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Narrow);
        test.assert_output(expected);
    }

    /// Comparisons with unknown ranges are left unchanged.
    #[test]
    fn test_narrow_skips_unknown_ranges() {
        let input = r#"
function test(v0: uint32, v1: uint32): boolean {
b0(v0: uint32, v1: uint32):
    v2: boolean = int.lt.u v0, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Narrow);
        test.assert_output(input);
    }

    /// Mismatched integer widths are not narrowed.
    #[test]
    fn test_narrow_skips_mismatched_widths() {
        let input = r#"
function test(v0: uint8[8]): void {
b0(v0: uint8[8]):
    v1: uint32 = 2uint32
    v2: uint64 = 4uint64
    v3: boolean = int.lt.u v1, v2
    check bounds.u v1, v2, v0 -> b1, b2
b1:
    return
b2:
    unreachable
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Narrow);
        test.assert_output(input);
    }

    /// Full range signed values are not narrowed.
    #[test]
    fn test_narrow_skips_full_range_signed() {
        let input = r#"
function test(): boolean {
b0:
    v0: int32 = -2147483648int32
    v1: int32 = 2147483647int32
    v2: boolean = int.lt.s v0, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Narrow);
        test.assert_output(input);
    }
}
