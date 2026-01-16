use std::collections::HashMap;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;
use mir::{BinaryOperator, Constant, UnaryOperator};

use destack_workspace::FloatMathPolicy;

use crate::optimize::analyses::{ConstantPropagation, RangeAnalysis, RangeMap};
use crate::optimize::{
    AnalysisPreservation, FunctionPass, PipelineContext, TypeContext, constant_all_ones_like,
    constant_is_all_ones, constant_is_float_one, constant_is_float_zero, constant_is_one,
    constant_is_zero, constant_zero_like, instruction_substitute_uses, resolve_substitution_chains,
    terminator_substitute_uses,
};

declare_pass! {
    /// Algebraic simplification of instructions.
    ///
    /// Applies identity and annihilator rules to simplify expressions:
    /// - `x + 0 = x`, `x * 1 = x`, `x * 0 = 0`
    /// - `x & 0 = 0`, `x | 0 = x`, `x ^ 0 = x`
    /// - `x - x = 0`, `x ^ x = 0`, `x & x = x`
    /// - `x == x = true`, `x != x = false`
    /// - `!!x = x`
    /// - `field.get(tuple/struct(...), i)` = operand i
    /// - `element.get(array(...), const_i)` = operand i
    ///
    /// ```mir
    /// function @before(v0: i32) -> i32 {
    /// block0(v0: i32):
    ///     v1 = iconst 0i32
    ///     v2 = iadd v0, v1
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: i32) -> i32 {
    /// block0(v0: i32):
    ///     return v0
    /// }
    /// ```
    #[pass(id = "instruction-combine")]
    pub InstructionCombine,
    "Combine and simplify instructions"
}

/// Result of simplifying an instruction.
enum Simplification {
    /// Replace with a constant value.
    Constant(Constant),
    /// Replace all uses of destination with this value (identity).
    Substitute(mir::Value),
}

impl FunctionPass for InstructionCombine {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // collect analyses and options
        let analyses = ctx.function_analyses(function, tree);
        let constants = analyses.get::<ConstantPropagation>().clone();
        let ranges = analyses.get::<RangeAnalysis>().clone();
        let float_math = ctx.options.float_math;

        let changed = run_instruction_combine(
            function,
            tree,
            &constants,
            &ranges,
            float_math,
            ctx.type_context(),
        );

        // preserve analyses when nothing changed
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    fn name(&self) -> &'static str {
        "InstructionCombine"
    }

    fn id(&self) -> &'static str {
        "instruction-combine"
    }
}

/// Core instruction combine logic.
fn run_instruction_combine(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    constants: &ConstantPropagation,
    ranges: &RangeAnalysis,
    float_math: FloatMathPolicy,
    type_context: TypeContext,
) -> bool {
    // build map of value to instruction for unary simplifications
    let mut value_to_instruction: HashMap<mir::Value, mir::Instruction> = HashMap::new();

    // track aggregate construction operands: dest -> operand list
    let mut aggregate_operands: HashMap<mir::Value, Vec<mir::Value>> = HashMap::new();

    // scan all blocks for aggregate definitions and value maps
    for &block_id in &function.blocks {
        // load the block instructions
        let block = tree.get(block_id);

        for &instruction_id in &block.instructions {
            // load the instruction for inspection
            let instruction = tree.get(instruction_id);

            // track aggregate constructions
            match instruction {
                mir::Instruction::Struct {
                    destination,
                    fields,
                    ..
                } => {
                    let args = tree.get_arguments(*fields);
                    aggregate_operands.insert(*destination, args.to_vec());
                }
                mir::Instruction::Tuple {
                    destination,
                    elements,
                    ..
                }
                | mir::Instruction::Array {
                    destination,
                    elements,
                    ..
                } => {
                    let args = tree.get_arguments(*elements);
                    aggregate_operands.insert(*destination, args.to_vec());
                }
                _ => {}
            }

            // track all instructions by destination
            if let Some(dest) = instruction.destination() {
                value_to_instruction.insert(dest, instruction.clone());
            }
        }
    }

    // track substitutions and removals
    let mut substitutions: HashMap<mir::Value, mir::Value> = HashMap::new();
    let mut to_remove: Vec<mir::LocalNodeId<mir::Instruction>> = Vec::new();
    let mut changed = false;

    // apply simplifications
    for &block_id in &function.blocks {
        // seed constants and ranges for the block
        let mut block_constants = constants.entry(block_id).clone();
        let block_ranges = ranges.exit(block_id).clone();
        let instruction_ids: Vec<_> = tree.get(block_id).instructions.clone();

        // scan instructions in program order
        for instruction_id in instruction_ids {
            // load the instruction for rewriting
            let instruction = tree.get(instruction_id).clone();

            // attempt to simplify the instruction
            let simplified = {
                let constant_lookup = ConstantLookup::new(&block_constants, &block_ranges);
                match &instruction {
                    mir::Instruction::Binary {
                        destination,
                        operator,
                        left,
                        right,
                    } => simplify_binary_operator(
                        *destination,
                        *operator,
                        *left,
                        *right,
                        &constant_lookup,
                        &block_ranges,
                        float_math,
                    ),

                    mir::Instruction::Unary {
                        destination,
                        operator,
                        argument,
                    } => simplify_unary_operator(
                        *destination,
                        *operator,
                        *argument,
                        &value_to_instruction,
                    ),

                    mir::Instruction::FieldGet {
                        aggregate, index, ..
                    } => {
                        // resolve field from known aggregate operands
                        if let Some(operands) = aggregate_operands.get(aggregate) {
                            operands
                                .get(*index as usize)
                                .map(|&op| Simplification::Substitute(op))
                        } else {
                            None
                        }
                    }

                    mir::Instruction::ElementGet { array, index, .. } => {
                        // resolve array element from known operands
                        if let Some(operands) = aggregate_operands.get(array) {
                            let index_constant = constant_lookup.get(*index);
                            let index_value = match index_constant {
                                Some(Constant::Int { value, .. }) => Some(value as usize),
                                Some(Constant::UInt { value, .. }) => Some(value as usize),
                                _ => None,
                            };

                            if let Some(index_value) = index_value {
                                operands
                                    .get(index_value)
                                    .map(|&op| Simplification::Substitute(op))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }

                    _ => None,
                }
            };

            // apply the simplification when available
            let mut applied_simplification = false;
            if let Some(simplification) = simplified {
                changed = true;
                applied_simplification = true;
                match simplification {
                    Simplification::Constant(value) => {
                        let dest = instruction.destination().unwrap();
                        let new_instruction = mir::Instruction::Const {
                            destination: dest,
                            value: value.clone(),
                        };
                        block_constants.insert(dest, value);
                        value_to_instruction.insert(dest, new_instruction.clone());
                        tree.replace(instruction_id, new_instruction);
                    }
                    Simplification::Substitute(replacement) => {
                        let dest = instruction.destination().unwrap();
                        substitutions.insert(dest, replacement);
                        to_remove.push(instruction_id);
                        let constant_lookup = ConstantLookup::new(&block_constants, &block_ranges);
                        if let Some(constant) = constant_lookup.get(replacement) {
                            block_constants.insert(dest, constant);
                        } else {
                            block_constants.remove(dest);
                        }
                    }
                }
            }

            // update constant tracking for instructions
            if !applied_simplification {
                update_constant_map(
                    &instruction,
                    tree,
                    &mut block_constants,
                    &block_ranges,
                    type_context,
                );
            }
        }
    }

    // resolve transitive substitutions
    let substitutions = resolve_substitution_chains(substitutions);

    // apply substitutions
    if !substitutions.is_empty() {
        for &block_id in &function.blocks {
            let instruction_ids: Vec<_> = tree.get(block_id).instructions.clone();
            for instruction_id in instruction_ids {
                // skip removed instructions
                if to_remove.contains(&instruction_id) {
                    continue;
                }

                // rewrite instruction operands
                let instruction = tree.get(instruction_id);
                let new_instruction = instruction_substitute_uses(instruction, &substitutions);
                if new_instruction != *instruction {
                    tree.replace(instruction_id, new_instruction);
                }
            }
        }

        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            let new_terminator = terminator_substitute_uses(&block.terminator, &substitutions);
            let filtered: Vec<_> = block
                .instructions
                .iter()
                .copied()
                .filter(|id| !to_remove.contains(id))
                .collect();

            // replace the block when terminators or instructions change
            if new_terminator != block.terminator || filtered.len() != block.instructions.len() {
                let mut new_block = block.clone();
                new_block.terminator = new_terminator;
                new_block.instructions = filtered;
                tree.replace(block_id, new_block);
            }
        }
    }

    changed
}

/// Try to simplify a binary operation using algebraic identities.
fn simplify_binary_operator(
    destination: mir::Value,
    operator: BinaryOperator,
    left: mir::Value,
    right: mir::Value,
    constants: &ConstantLookup<'_>,
    ranges: &RangeMap,
    float_math: FloatMathPolicy,
) -> Option<Simplification> {
    // read constant operands
    let left_const = constants.get(left);
    let right_const = constants.get(right);
    let left_const_ref = left_const.as_ref();
    let right_const_ref = right_const.as_ref();

    // fold comparisons using range evidence
    if let Some(result) = comparison_from_ranges(operator, left, right, ranges) {
        return Some(Simplification::Constant(Constant::Boolean {
            value: result,
        }));
    }

    // same operand simplifications (x op x)
    if left == right {
        return simplify_same_binary_operand(destination, operator, left, left_const_ref);
    }

    // identity and annihilator rules with constants
    match operator {
        // x + 0 = x, 0 + x = x
        BinaryOperator::Add => {
            if constant_is_zero(right_const_ref) {
                return Some(Simplification::Substitute(left));
            }
            if constant_is_zero(left_const_ref) {
                return Some(Simplification::Substitute(right));
            }
        }

        // x - 0 = x
        BinaryOperator::Subtract => {
            if constant_is_zero(right_const_ref) {
                return Some(Simplification::Substitute(left));
            }
        }

        // x * 0 = 0, 0 * x = 0, x * 1 = x, 1 * x = x
        BinaryOperator::Multiply => {
            if constant_is_zero(right_const_ref) {
                return Some(Simplification::Constant(constant_zero_like(
                    right_const_ref.unwrap(),
                )));
            }
            if constant_is_zero(left_const_ref) {
                return Some(Simplification::Constant(constant_zero_like(
                    left_const_ref.unwrap(),
                )));
            }
            if constant_is_one(right_const_ref) {
                return Some(Simplification::Substitute(left));
            }
            if constant_is_one(left_const_ref) {
                return Some(Simplification::Substitute(right));
            }
        }

        // x / 1 = x (signed)
        BinaryOperator::SignedDivide | BinaryOperator::UnsignedDivide => {
            if constant_is_one(right_const_ref) {
                return Some(Simplification::Substitute(left));
            }
        }

        // x % 1 = 0 (signed and unsigned)
        BinaryOperator::SignedRemainder | BinaryOperator::UnsignedRemainder => {
            if constant_is_one(right_const_ref) {
                return Some(Simplification::Constant(constant_zero_like(
                    right_const_ref.unwrap(),
                )));
            }
        }

        // x & 0 = 0, 0 & x = 0
        BinaryOperator::And => {
            if constant_is_zero(right_const_ref) {
                return Some(Simplification::Constant(constant_zero_like(
                    right_const_ref.unwrap(),
                )));
            }
            if constant_is_zero(left_const_ref) {
                return Some(Simplification::Constant(constant_zero_like(
                    left_const_ref.unwrap(),
                )));
            }
            // x & all_ones = x
            if constant_is_all_ones(right_const_ref) {
                return Some(Simplification::Substitute(left));
            }
            if constant_is_all_ones(left_const_ref) {
                return Some(Simplification::Substitute(right));
            }
        }

        // x | 0 = x, 0 | x = x
        BinaryOperator::Or => {
            if constant_is_zero(right_const_ref) {
                return Some(Simplification::Substitute(left));
            }
            if constant_is_zero(left_const_ref) {
                return Some(Simplification::Substitute(right));
            }
            // x | all_ones = all_ones
            if constant_is_all_ones(right_const_ref) {
                return Some(Simplification::Constant(constant_all_ones_like(
                    right_const_ref.unwrap(),
                )));
            }
            if constant_is_all_ones(left_const_ref) {
                return Some(Simplification::Constant(constant_all_ones_like(
                    left_const_ref.unwrap(),
                )));
            }
        }

        // x ^ 0 = x, 0 ^ x = x
        BinaryOperator::Xor => {
            if constant_is_zero(right_const_ref) {
                return Some(Simplification::Substitute(left));
            }
            if constant_is_zero(left_const_ref) {
                return Some(Simplification::Substitute(right));
            }
        }

        // x << 0 = x, x >> 0 = x
        BinaryOperator::ShiftLeft
        | BinaryOperator::ArithmeticShiftRight
        | BinaryOperator::LogicalShiftRight => {
            if constant_is_zero(right_const_ref) {
                return Some(Simplification::Substitute(left));
            }
            // 0 << x = 0, 0 >> x = 0
            if constant_is_zero(left_const_ref) {
                return Some(Simplification::Constant(constant_zero_like(
                    left_const_ref.unwrap(),
                )));
            }
        }

        // float: x + 0.0 = x (not for -0.0, but we simplify for 0.0)
        BinaryOperator::FloatAdd => {
            if allow_float_identities(float_math) && constant_is_float_zero(right_const_ref) {
                return Some(Simplification::Substitute(left));
            }
            if allow_float_identities(float_math) && constant_is_float_zero(left_const_ref) {
                return Some(Simplification::Substitute(right));
            }
        }

        // float: x - 0.0 = x
        BinaryOperator::FloatSubtract => {
            if allow_float_identities(float_math) && constant_is_float_zero(right_const_ref) {
                return Some(Simplification::Substitute(left));
            }
        }

        // float: x * 1.0 = x
        BinaryOperator::FloatMultiply => {
            if allow_float_identities(float_math) && constant_is_float_one(right_const_ref) {
                return Some(Simplification::Substitute(left));
            }
            if allow_float_identities(float_math) && constant_is_float_one(left_const_ref) {
                return Some(Simplification::Substitute(right));
            }
        }

        // float: x / 1.0 = x
        BinaryOperator::FloatDivide => {
            if allow_float_identities(float_math) && constant_is_float_one(right_const_ref) {
                return Some(Simplification::Substitute(left));
            }
        }

        _ => {}
    }

    None
}

/// Simplify operations where both operands are the same value.
fn simplify_same_binary_operand(
    _destination: mir::Value,
    operator: BinaryOperator,
    operand: mir::Value,
    constant: Option<&Constant>,
) -> Option<Simplification> {
    // get type info from constant if available for proper zero type
    let operand_const = constant;

    // simplify based on the operator
    match operator {
        // x - x = 0
        BinaryOperator::Subtract => {
            // need type info to create proper zero constant
            let c = operand_const?;
            Some(Simplification::Constant(constant_zero_like(c)))
        }

        // x ^ x = 0
        BinaryOperator::Xor => {
            // need type info to create proper zero constant
            let c = operand_const?;
            Some(Simplification::Constant(constant_zero_like(c)))
        }

        // x & x = x, x | x = x
        BinaryOperator::And | BinaryOperator::Or => Some(Simplification::Substitute(operand)),

        // x == x = true
        BinaryOperator::Equal
        | BinaryOperator::SignedLessEqual
        | BinaryOperator::SignedGreaterEqual
        | BinaryOperator::UnsignedLessEqual
        | BinaryOperator::UnsignedGreaterEqual => {
            Some(Simplification::Constant(Constant::Boolean { value: true }))
        }

        // x != x = false, x < x = false, x > x = false
        BinaryOperator::NotEqual
        | BinaryOperator::SignedLessThan
        | BinaryOperator::SignedGreaterThan
        | BinaryOperator::UnsignedLessThan
        | BinaryOperator::UnsignedGreaterThan => {
            Some(Simplification::Constant(Constant::Boolean { value: false }))
        }

        _ => None,
    }
}

/// Lookup for constants from propagation and range analysis.
struct ConstantLookup<'a> {
    /// Constants derived from propagation.
    block_constants: &'a crate::optimize::analyses::ConstantMap,
    /// Ranges for the block.
    ranges: &'a RangeMap,
}

impl<'a> ConstantLookup<'a> {
    /// Create a new lookup for a block.
    fn new(
        block_constants: &'a crate::optimize::analyses::ConstantMap,
        ranges: &'a RangeMap,
    ) -> Self {
        Self {
            block_constants,
            ranges,
        }
    }

    /// Get a constant value for a SSA value.
    fn get(&self, value: mir::Value) -> Option<Constant> {
        // check propagation constants first
        if let Some(constant) = self.block_constants.get(value) {
            return Some(constant.clone());
        }

        // fall back to range derived constants
        let range = self.ranges.get(value)?;
        range.as_constant()
    }
}

/// Update a constant map with instruction effects.
fn update_constant_map(
    instruction: &mir::Instruction,
    tree: &mir::NodeTree,
    block_constants: &mut crate::optimize::analyses::ConstantMap,
    ranges: &RangeMap,
    type_context: TypeContext,
) {
    // skip instructions without destinations
    let Some(destination) = instruction.destination() else {
        return;
    };

    // capture a local constant lookup
    let constant_for = |value: mir::Value| -> Option<Constant> {
        if let Some(constant) = block_constants.get(value) {
            return Some(constant.clone());
        }

        let range = ranges.get(value)?;
        range.as_constant()
    };

    // update the constant map based on instruction semantics
    match instruction {
        mir::Instruction::Const { value, .. } => {
            block_constants.insert(destination, value.clone());
        }
        mir::Instruction::GlobalConst { global, .. } => {
            if let Some(constant) = crate::optimize::common::constant_from_global(*global, tree) {
                block_constants.insert(destination, constant);
            } else {
                block_constants.remove(destination);
            }
        }
        mir::Instruction::Binary {
            operator,
            left,
            right,
            ..
        } => {
            // fold binary constants when possible
            let left = constant_for(*left);
            let right = constant_for(*right);
            if let (Some(left), Some(right)) = (left, right)
                && let Some(result) = crate::optimize::common::fold_binary(*operator, left, right)
            {
                block_constants.insert(destination, result);
            } else {
                block_constants.remove(destination);
            }
        }
        mir::Instruction::Unary {
            operator, argument, ..
        } => {
            // fold unary constants when possible
            let argument = constant_for(*argument);
            if let Some(argument) = argument
                && let Some(result) = crate::optimize::common::fold_unary(*operator, argument)
            {
                block_constants.insert(destination, result);
            } else {
                block_constants.remove(destination);
            }
        }
        mir::Instruction::Cast {
            operator,
            argument,
            to_type,
            ..
        } => {
            // fold casts when possible
            let argument = constant_for(*argument);
            if let Some(argument) = argument
                && let Some(result) = crate::optimize::common::fold_cast(
                    *operator,
                    argument,
                    *to_type,
                    type_context.pointer_width_bits,
                    tree,
                )
            {
                block_constants.insert(destination, result);
            } else {
                block_constants.remove(destination);
            }
        }
        mir::Instruction::Select {
            condition,
            then_value,
            else_value,
            ..
        } => {
            // fold selects with constant conditions
            let condition = constant_for(*condition);
            if let Some(Constant::Boolean { value }) = condition {
                let selected = if value { *then_value } else { *else_value };
                if let Some(constant) = constant_for(selected) {
                    block_constants.insert(destination, constant);
                } else {
                    block_constants.remove(destination);
                }
            } else {
                block_constants.remove(destination);
            }
        }
        _ => {
            // clear constants for unhandled instructions
            block_constants.remove(destination);
        }
    }
}

/// Fold comparisons using range analysis when possible.
fn comparison_from_ranges(
    operator: mir::BinaryOperator,
    left: mir::Value,
    right: mir::Value,
    ranges: &RangeMap,
) -> Option<bool> {
    // read operand ranges
    let left_range = ranges.get(left)?;
    let right_range = ranges.get(right)?;

    // fold comparisons when both operands are integer ranges
    match (left_range, right_range) {
        (
            crate::optimize::analyses::ValueRange::Integer {
                min: left_min,
                max: left_max,
                ..
            },
            crate::optimize::analyses::ValueRange::Integer {
                min: right_min,
                max: right_max,
                ..
            },
        ) => match operator {
            BinaryOperator::SignedLessThan | BinaryOperator::UnsignedLessThan => {
                if left_max < right_min {
                    Some(true)
                } else if left_min >= right_max {
                    Some(false)
                } else {
                    None
                }
            }
            BinaryOperator::SignedLessEqual | BinaryOperator::UnsignedLessEqual => {
                if left_max <= right_min {
                    Some(true)
                } else if left_min > right_max {
                    Some(false)
                } else {
                    None
                }
            }
            BinaryOperator::SignedGreaterThan | BinaryOperator::UnsignedGreaterThan => {
                if left_min > right_max {
                    Some(true)
                } else if left_max <= right_min {
                    Some(false)
                } else {
                    None
                }
            }
            BinaryOperator::SignedGreaterEqual | BinaryOperator::UnsignedGreaterEqual => {
                if left_min >= right_max {
                    Some(true)
                } else if left_max < right_min {
                    Some(false)
                } else {
                    None
                }
            }
            BinaryOperator::Equal => {
                if left_min == left_max && left_min == right_min && right_min == right_max {
                    Some(true)
                } else if left_max < right_min || right_max < left_min {
                    Some(false)
                } else {
                    None
                }
            }
            BinaryOperator::NotEqual => {
                if left_min == left_max && left_min == right_min && right_min == right_max {
                    Some(false)
                } else if left_max < right_min || right_max < left_min {
                    Some(true)
                } else {
                    None
                }
            }
            _ => None,
        },
        _ => None,
    }
}

/// Check if float identity simplifications are allowed.
fn allow_float_identities(policy: FloatMathPolicy) -> bool {
    matches!(policy, FloatMathPolicy::Fast)
}

/// Try to simplify a unary operation.
fn simplify_unary_operator(
    _destination: mir::Value,
    operator: UnaryOperator,
    argument: mir::Value,
    value_to_instruction: &HashMap<mir::Value, mir::Instruction>,
) -> Option<Simplification> {
    // look for double negation: !!x = x
    if operator == UnaryOperator::Not
        && let Some(mir::Instruction::Unary {
            operator: UnaryOperator::Not,
            argument: inner,
            ..
        }) = value_to_instruction.get(&argument)
    {
        // !!x = x
        return Some(Simplification::Substitute(*inner));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::PipelineOptions;
    use crate::optimize::common::tests::TestProgram;
    use destack_workspace::FloatMathPolicy;

    /// x + 0 simplifies to x (instruction removed, uses substituted).
    #[test]
    fn test_simplify_add_zero() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = iadd v0, v1
    return v2
}"#;
        // v2 is substituted with v0, the iadd is removed
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// 0 + x simplifies to x.
    #[test]
    fn test_simplify_zero_add() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = iadd v1, v0
    return v2
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// x * 1 simplifies to x.
    #[test]
    fn test_simplify_mul_one() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    v2 = imul v0, v1
    return v2
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// x * 0 simplifies to 0.
    #[test]
    fn test_simplify_mul_zero() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = imul v0, v1
    return v2
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = iconst 0i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// x & 0 simplifies to 0.
    #[test]
    fn test_simplify_and_zero() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = band v0, v1
    return v2
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = iconst 0i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// x | 0 simplifies to x.
    #[test]
    fn test_simplify_or_zero() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = bor v0, v1
    return v2
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// x ^ 0 simplifies to x.
    #[test]
    fn test_simplify_xor_zero() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = bxor v0, v1
    return v2
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// x - x simplifies to 0 (when type info is available).
    #[test]
    fn test_simplify_sub_self() {
        // use a constant so we have type info
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    v1 = isub v0, v0
    return v1
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    v1 = iconst 0i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// x ^ x simplifies to 0 (when type info is available).
    #[test]
    fn test_simplify_xor_self() {
        // use a constant so we have type info
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    v1 = bxor v0, v0
    return v1
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    v1 = iconst 0i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// x - x without type info is not simplified (conservative).
    #[test]
    fn test_preserve_sub_self_without_type_info() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = isub v0, v0
    return v1
}"#;
        // no simplification because we don't know the type of v0

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_unchanged(input);
    }

    /// x & x simplifies to x.
    #[test]
    fn test_simplify_and_self() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = band v0, v0
    return v1
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// x | x simplifies to x.
    #[test]
    fn test_simplify_or_self() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = bor v0, v0
    return v1
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// x == x simplifies to true.
    #[test]
    fn test_simplify_eq_self() {
        let input = r#"function @test(v0: i32) -> bool {
block0(v0: i32):
    v1 = icmp_eq v0, v0
    return v1
}"#;
        let expected = r#"function @test(v0: i32) -> bool {
block0(v0: i32):
    v1 = iconst true
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// x != x simplifies to false.
    #[test]
    fn test_simplify_ne_self() {
        let input = r#"function @test(v0: i32) -> bool {
block0(v0: i32):
    v1 = icmp_ne v0, v0
    return v1
}"#;
        let expected = r#"function @test(v0: i32) -> bool {
block0(v0: i32):
    v1 = iconst false
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// x < x simplifies to false.
    #[test]
    fn test_simplify_lt_self() {
        let input = r#"function @test(v0: i32) -> bool {
block0(v0: i32):
    v1 = icmp_slt v0, v0
    return v1
}"#;
        let expected = r#"function @test(v0: i32) -> bool {
block0(v0: i32):
    v1 = iconst false
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// x <= x simplifies to true.
    #[test]
    fn test_simplify_le_self() {
        let input = r#"function @test(v0: i32) -> bool {
block0(v0: i32):
    v1 = icmp_sle v0, v0
    return v1
}"#;
        let expected = r#"function @test(v0: i32) -> bool {
block0(v0: i32):
    v1 = iconst true
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// Range evidence folds comparisons.
    #[test]
    fn test_fold_comparison_from_range() {
        let left = mir::Value::new(1);
        let right = mir::Value::new(2);
        let mut ranges = RangeMap::new();
        ranges.insert(
            left,
            crate::optimize::analyses::ValueRange::Integer {
                min: 0,
                max: 4,
                width: 32,
                is_signed: true,
            },
        );
        ranges.insert(
            right,
            crate::optimize::analyses::ValueRange::Integer {
                min: 10,
                max: 12,
                width: 32,
                is_signed: true,
            },
        );

        assert_eq!(
            comparison_from_ranges(BinaryOperator::SignedLessThan, left, right, &ranges),
            Some(true)
        );
        assert_eq!(
            comparison_from_ranges(BinaryOperator::SignedGreaterEqual, left, right, &ranges),
            Some(false)
        );
    }

    /// x << 0 simplifies to x.
    #[test]
    fn test_simplify_shl_zero() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = ishl v0, v1
    return v2
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// 0 << x simplifies to 0.
    #[test]
    fn test_simplify_zero_shl() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = ishl v1, v0
    return v2
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = iconst 0i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// x / 1 simplifies to x.
    #[test]
    fn test_simplify_div_one() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    v2 = sdiv v0, v1
    return v2
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// x % 1 simplifies to 0.
    #[test]
    fn test_simplify_rem_one() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    v2 = srem v0, v1
    return v2
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    v2 = iconst 0i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// Non-simplifiable operations are preserved.
    #[test]
    fn test_preserve_non_simplifiable() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 5i32
    v2 = iadd v0, v1
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_unchanged(input);
    }

    /// Chained simplifications work correctly.
    #[test]
    fn test_apply_chained_simplifications() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = iadd v0, v1
    v3 = iconst 1i32
    v4 = imul v2, v3
    return v4
}"#;
        // v2 substituted to v0, v4 substituted to v2 (which is v0)
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v3 = iconst 1i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// Substitutions propagate through uses.
    #[test]
    fn test_propagate_substitutions() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = iadd v0, v1
    v3 = iconst 5i32
    v4 = iadd v2, v3
    return v4
}"#;
        // v2 -> v0, so v4 = iadd v0, v3
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v3 = iconst 5i32
    v4 = iadd v0, v3
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// x & all_ones simplifies to x.
    #[test]
    fn test_simplify_and_all_ones() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst -1i32
    v2 = band v0, v1
    return v2
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst -1i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// x | all_ones simplifies to all_ones.
    #[test]
    fn test_simplify_or_all_ones() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst -1i32
    v2 = bor v0, v1
    return v2
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst -1i32
    v2 = iconst -1i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// x >> 0 (arithmetic) simplifies to x.
    #[test]
    fn test_simplify_ashr_zero() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = sshr v0, v1
    return v2
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// x >> 0 (logical) simplifies to x.
    #[test]
    fn test_simplify_lshr_zero() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = ushr v0, v1
    return v2
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// Unsigned x / 1 simplifies to x.
    #[test]
    fn test_simplify_udiv_one() {
        let input = r#"function @test(v0: u32) -> u32 {
block0(v0: u32):
    v1 = iconst 1u32
    v2 = udiv v0, v1
    return v2
}"#;
        let expected = r#"function @test(v0: u32) -> u32 {
block0(v0: u32):
    v1 = iconst 1u32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// Unsigned x % 1 simplifies to 0.
    #[test]
    fn test_simplify_urem_one() {
        let input = r#"function @test(v0: u32) -> u32 {
block0(v0: u32):
    v1 = iconst 1u32
    v2 = urem v0, v1
    return v2
}"#;
        let expected = r#"function @test(v0: u32) -> u32 {
block0(v0: u32):
    v1 = iconst 1u32
    v2 = iconst 0u32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// Float x + 0.0 simplifies to x.
    #[test]
    fn test_simplify_fadd_zero() {
        let input = r#"function @test(v0: f32) -> f32 {
block0(v0: f32):
    v1 = iconst 0.0f32
    v2 = fadd v0, v1
    return v2
}"#;
        // 0.0f32 becomes 0f32 after roundtrip
        let expected = r#"function @test(v0: f32) -> f32 {
block0(v0: f32):
    v1 = iconst 0f32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass_with_options(
            &InstructionCombine,
            PipelineOptions {
                float_math: FloatMathPolicy::Fast,
                ..PipelineOptions::default()
            },
        );
        program.assert_output(expected);
    }

    /// Float x * 1.0 simplifies to x.
    #[test]
    fn test_simplify_fmul_one() {
        let input = r#"function @test(v0: f32) -> f32 {
block0(v0: f32):
    v1 = iconst 1.0f32
    v2 = fmul v0, v1
    return v2
}"#;
        // 1.0f32 becomes 1f32 after roundtrip
        let expected = r#"function @test(v0: f32) -> f32 {
block0(v0: f32):
    v1 = iconst 1f32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass_with_options(
            &InstructionCombine,
            PipelineOptions {
                float_math: FloatMathPolicy::Fast,
                ..PipelineOptions::default()
            },
        );
        program.assert_output(expected);
    }

    /// Float x / 1.0 simplifies to x.
    #[test]
    fn test_simplify_fdiv_one() {
        let input = r#"function @test(v0: f32) -> f32 {
block0(v0: f32):
    v1 = iconst 1.0f32
    v2 = fdiv v0, v1
    return v2
}"#;
        // 1.0f32 becomes 1f32 after roundtrip
        let expected = r#"function @test(v0: f32) -> f32 {
block0(v0: f32):
    v1 = iconst 1f32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass_with_options(
            &InstructionCombine,
            PipelineOptions {
                float_math: FloatMathPolicy::Fast,
                ..PipelineOptions::default()
            },
        );
        program.assert_output(expected);
    }

    /// Float x - 0.0 simplifies to x.
    #[test]
    fn test_simplify_fsub_zero() {
        let input = r#"function @test(v0: f32) -> f32 {
block0(v0: f32):
    v1 = iconst 0.0f32
    v2 = fsub v0, v1
    return v2
}"#;
        // 0.0f32 becomes 0f32 after roundtrip
        let expected = r#"function @test(v0: f32) -> f32 {
block0(v0: f32):
    v1 = iconst 0f32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass_with_options(
            &InstructionCombine,
            PipelineOptions {
                float_math: FloatMathPolicy::Fast,
                ..PipelineOptions::default()
            },
        );
        program.assert_output(expected);
    }

    /// Double negation !!x simplifies to x.
    #[test]
    fn test_simplify_double_not() {
        let input = r#"function @test(v0: bool) -> bool {
block0(v0: bool):
    v1 = bnot v0
    v2 = bnot v1
    return v2
}"#;
        let expected = r#"function @test(v0: bool) -> bool {
block0(v0: bool):
    v1 = bnot v0
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// field.get(tuple(...), i) simplifies to the i-th operand.
    #[test]
    fn test_simplify_field_get_tuple() {
        let input = r#"function @test(v0: i32, v1: i64) -> i32 {
block0(v0: i32, v1: i64):
    v2 = tuple (i32, i64) (v0, v1)
    v3 = field.get v2, 0
    return v3
}"#;
        let expected = r#"function @test(v0: i32, v1: i64) -> i32 {
block0(v0: i32, v1: i64):
    v2 = tuple (i32, i64) (v0, v1)
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// field.get(tuple(...), 1) simplifies to the second operand.
    #[test]
    fn test_simplify_field_get_tuple_second() {
        let input = r#"function @test(v0: i32, v1: i64) -> i64 {
block0(v0: i32, v1: i64):
    v2 = tuple (i32, i64) (v0, v1)
    v3 = field.get v2, 1
    return v3
}"#;
        let expected = r#"function @test(v0: i32, v1: i64) -> i64 {
block0(v0: i32, v1: i64):
    v2 = tuple (i32, i64) (v0, v1)
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// field.get(struct(...), i) simplifies to the i-th field value.
    #[test]
    fn test_simplify_field_get_struct() {
        let input = r#"type @Point = { i32, i32 }
function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = struct @Point (v0, v1)
    v3 = field.get v2, 0
    v4 = field.get v2, 1
    v5 = iadd v3, v4
    return v5
}"#;
        // both field.get replaced with direct operands
        let expected = r#"type @Point = { i32, i32 }
function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = struct @Point (v0, v1)
    v5 = iadd v0, v1
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// element.get(array(...), const_i) simplifies to the i-th element.
    #[test]
    fn test_simplify_element_get_array() {
        let input = r#"function @test(v0: i32, v1: i32, v2: i32) -> i32 {
block0(v0: i32, v1: i32, v2: i32):
    v3 = array [i32; 3] (v0, v1, v2)
    v4 = iconst 1i64
    v5 = element.get v3, v4
    return v5
}"#;
        // element.get with constant index 1 replaced with v1
        let expected = r#"function @test(v0: i32, v1: i32, v2: i32) -> i32 {
block0(v0: i32, v1: i32, v2: i32):
    v3 = array [i32; 3] (v0, v1, v2)
    v4 = iconst 1i64
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_output(expected);
    }

    /// element.get with non-constant index is not simplified.
    #[test]
    fn test_preserve_element_get_non_constant_index() {
        let input = r#"function @test(v0: i32, v1: i32, v2: i64) -> i32 {
block0(v0: i32, v1: i32, v2: i64):
    v3 = array [i32; 2] (v0, v1)
    v4 = element.get v3, v2
    return v4
}"#;
        // v2 is not a constant, cannot simplify
        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_unchanged(input);
    }

    /// field.get from non-aggregate source is not simplified.
    #[test]
    fn test_preserve_field_get_unknown_source() {
        let input = r#"function @test(v0: (i32, i32)) -> i32 {
block0(v0: (i32, i32)):
    v1 = field.get v0, 0
    return v1
}"#;
        // v0 is a parameter, not from tuple/struct instruction
        let mut program = TestProgram::new(input);
        program.run_pass(&InstructionCombine);
        program.assert_unchanged(input);
    }
}
