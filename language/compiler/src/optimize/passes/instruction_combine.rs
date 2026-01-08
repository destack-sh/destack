use std::collections::HashMap;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;
use mir::{BinaryOperator, Constant, UnaryOperator};

use crate::AnalysisKind;
use crate::optimize::{
    AnalysisPreservation, FunctionPass, OptimizationContext, Pass, PassMetadata,
    constant_all_ones_like, constant_is_all_ones, constant_is_float_one, constant_is_float_zero,
    constant_is_one, constant_is_zero, constant_zero_like, instruction_substitute_uses,
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

impl Pass for InstructionCombine {
    fn metadata(&self) -> &'static PassMetadata {
        InstructionCombine::metadata()
    }
}

/// Result of simplifying an instruction.
enum Simplification {
    /// Replace with a constant value.
    Constant(Constant),
    /// Replace all uses of destination with this value (identity).
    Substitute(mir::Value),
}

impl FunctionPass for InstructionCombine {
    fn run_on_function(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        _context: &OptimizationContext<'_>,
    ) -> AnalysisPreservation {
        // build map of value -> constant for known constants
        let mut constants: HashMap<mir::Value, mir::Constant> = HashMap::new();

        // build map of value -> instruction for unary simplifications
        let mut value_to_instruction: HashMap<mir::Value, mir::Instruction> = HashMap::new();

        // scan all blocks for constant definitions and build value map
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);

                // track constants
                if let mir::Instruction::Const { destination, value } = instruction {
                    constants.insert(*destination, value.clone());
                }

                // track all instructions by destination
                if let Some(dest) = instruction.destination() {
                    value_to_instruction.insert(dest, instruction.clone());
                }
            }
        }

        // track substitutions: dest -> replacement value
        let mut substitutions: HashMap<mir::Value, mir::Value> = HashMap::new();

        // track instructions to remove (those that became substitutions)
        let mut to_remove: Vec<mir::LocalNodeId<mir::Instruction>> = Vec::new();

        // track whether we made any changes
        let mut changed = false;

        // apply simplifications
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            let instruction_ids: Vec<_> = block.instructions.clone();

            for instruction_id in instruction_ids {
                let instruction = tree.get(instruction_id);

                // try to simplify the instruction
                let simplified = match instruction {
                    mir::Instruction::Binary {
                        destination,
                        operator,
                        left,
                        right,
                    } => {
                        simplify_binary_operator(*destination, *operator, *left, *right, &constants)
                    }

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

                    _ => None,
                };

                // handle simplification result
                if let Some(simplification) = simplified {
                    changed = true;
                    match simplification {
                        Simplification::Constant(value) => {
                            let dest = instruction.destination().unwrap();
                            let new_instruction = mir::Instruction::Const {
                                destination: dest,
                                value: value.clone(),
                            };
                            constants.insert(dest, value);
                            value_to_instruction.insert(dest, new_instruction.clone());
                            tree.replace(instruction_id, new_instruction);
                        }
                        Simplification::Substitute(replacement) => {
                            let dest = instruction.destination().unwrap();
                            substitutions.insert(dest, replacement);
                            to_remove.push(instruction_id);
                        }
                    }
                }
            }
        }

        // resolve transitive substitutions (v4 -> v2 -> v0 becomes v4 -> v0)
        let substitutions = resolve_substitution_chains(substitutions);

        // apply substitutions to all instructions and terminators
        if !substitutions.is_empty() {
            // substitute in instructions
            for &block_id in &function.blocks {
                let instruction_ids: Vec<_> = tree.get(block_id).instructions.clone();
                for instruction_id in instruction_ids {
                    if to_remove.contains(&instruction_id) {
                        continue;
                    }
                    let instruction = tree.get(instruction_id);
                    let new_instruction = instruction_substitute_uses(instruction, &substitutions);
                    if new_instruction != *instruction {
                        tree.replace(instruction_id, new_instruction);
                    }
                }
            }

            // substitute in terminators and remove substituted instructions
            for &block_id in &function.blocks {
                let block = tree.get(block_id);
                let new_terminator = terminator_substitute_uses(&block.terminator, &substitutions);
                let filtered: Vec<_> = block
                    .instructions
                    .iter()
                    .copied()
                    .filter(|id| !to_remove.contains(id))
                    .collect();

                // update block if anything changed
                if new_terminator != block.terminator || filtered.len() != block.instructions.len()
                {
                    let mut new_block = block.clone();
                    new_block.terminator = new_terminator;
                    new_block.instructions = filtered;
                    tree.replace(block_id, new_block);
                }
            }
        }

        // instruction combine doesn't change the CFG, only instruction contents
        if changed {
            AnalysisPreservation::Some(vec![AnalysisKind::ControlFlowGraph])
        } else {
            AnalysisPreservation::all()
        }
    }
}

/// Try to simplify a binary operation using algebraic identities.
fn simplify_binary_operator(
    destination: mir::Value,
    operator: BinaryOperator,
    left: mir::Value,
    right: mir::Value,
    constants: &HashMap<mir::Value, Constant>,
) -> Option<Simplification> {
    let left_const = constants.get(&left);
    let right_const = constants.get(&right);

    // same operand simplifications (x op x)
    if left == right {
        return simplify_same_binary_operand(destination, operator, left, constants);
    }

    // identity and annihilator rules with constants
    match operator {
        // x + 0 = x, 0 + x = x
        BinaryOperator::Add => {
            if constant_is_zero(right_const) {
                return Some(Simplification::Substitute(left));
            }
            if constant_is_zero(left_const) {
                return Some(Simplification::Substitute(right));
            }
        }

        // x - 0 = x
        BinaryOperator::Subtract => {
            if constant_is_zero(right_const) {
                return Some(Simplification::Substitute(left));
            }
        }

        // x * 0 = 0, 0 * x = 0, x * 1 = x, 1 * x = x
        BinaryOperator::Multiply => {
            if constant_is_zero(right_const) {
                return Some(Simplification::Constant(constant_zero_like(
                    right_const.unwrap(),
                )));
            }
            if constant_is_zero(left_const) {
                return Some(Simplification::Constant(constant_zero_like(
                    left_const.unwrap(),
                )));
            }
            if constant_is_one(right_const) {
                return Some(Simplification::Substitute(left));
            }
            if constant_is_one(left_const) {
                return Some(Simplification::Substitute(right));
            }
        }

        // x / 1 = x (signed)
        BinaryOperator::SignedDivide | BinaryOperator::UnsignedDivide => {
            if constant_is_one(right_const) {
                return Some(Simplification::Substitute(left));
            }
        }

        // x % 1 = 0 (signed and unsigned)
        BinaryOperator::SignedRemainder | BinaryOperator::UnsignedRemainder => {
            if constant_is_one(right_const) {
                return Some(Simplification::Constant(constant_zero_like(
                    right_const.unwrap(),
                )));
            }
        }

        // x & 0 = 0, 0 & x = 0
        BinaryOperator::And => {
            if constant_is_zero(right_const) {
                return Some(Simplification::Constant(constant_zero_like(
                    right_const.unwrap(),
                )));
            }
            if constant_is_zero(left_const) {
                return Some(Simplification::Constant(constant_zero_like(
                    left_const.unwrap(),
                )));
            }
            // x & all_ones = x
            if constant_is_all_ones(right_const) {
                return Some(Simplification::Substitute(left));
            }
            if constant_is_all_ones(left_const) {
                return Some(Simplification::Substitute(right));
            }
        }

        // x | 0 = x, 0 | x = x
        BinaryOperator::Or => {
            if constant_is_zero(right_const) {
                return Some(Simplification::Substitute(left));
            }
            if constant_is_zero(left_const) {
                return Some(Simplification::Substitute(right));
            }
            // x | all_ones = all_ones
            if constant_is_all_ones(right_const) {
                return Some(Simplification::Constant(constant_all_ones_like(
                    right_const.unwrap(),
                )));
            }
            if constant_is_all_ones(left_const) {
                return Some(Simplification::Constant(constant_all_ones_like(
                    left_const.unwrap(),
                )));
            }
        }

        // x ^ 0 = x, 0 ^ x = x
        BinaryOperator::Xor => {
            if constant_is_zero(right_const) {
                return Some(Simplification::Substitute(left));
            }
            if constant_is_zero(left_const) {
                return Some(Simplification::Substitute(right));
            }
        }

        // x << 0 = x, x >> 0 = x
        BinaryOperator::ShiftLeft
        | BinaryOperator::ArithmeticShiftRight
        | BinaryOperator::LogicalShiftRight => {
            if constant_is_zero(right_const) {
                return Some(Simplification::Substitute(left));
            }
            // 0 << x = 0, 0 >> x = 0
            if constant_is_zero(left_const) {
                return Some(Simplification::Constant(constant_zero_like(
                    left_const.unwrap(),
                )));
            }
        }

        // float: x + 0.0 = x (not for -0.0, but we simplify for 0.0)
        BinaryOperator::FloatAdd => {
            if constant_is_float_zero(right_const) {
                return Some(Simplification::Substitute(left));
            }
            if constant_is_float_zero(left_const) {
                return Some(Simplification::Substitute(right));
            }
        }

        // float: x - 0.0 = x
        BinaryOperator::FloatSubtract => {
            if constant_is_float_zero(right_const) {
                return Some(Simplification::Substitute(left));
            }
        }

        // float: x * 1.0 = x
        BinaryOperator::FloatMultiply => {
            if constant_is_float_one(right_const) {
                return Some(Simplification::Substitute(left));
            }
            if constant_is_float_one(left_const) {
                return Some(Simplification::Substitute(right));
            }
        }

        // float: x / 1.0 = x
        BinaryOperator::FloatDivide => {
            if constant_is_float_one(right_const) {
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
    constants: &HashMap<mir::Value, Constant>,
) -> Option<Simplification> {
    // get type info from constant if available for proper zero type
    let operand_const = constants.get(&operand);

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

/// Resolve transitive substitution chains.
///
/// If we have v4 -> v2 and v2 -> v0, this produces v4 -> v0 and v2 -> v0.
fn resolve_substitution_chains(
    mut substitutions: HashMap<mir::Value, mir::Value>,
) -> HashMap<mir::Value, mir::Value> {
    let keys: Vec<_> = substitutions.keys().copied().collect();
    for key in keys {
        let mut current = substitutions[&key];

        // follow the chain
        while let Some(&next) = substitutions.get(&current) {
            if next == current {
                break; // avoid infinite loop
            }
            current = next;
        }

        substitutions.insert(key, current);
    }
    substitutions
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
    use crate::optimize::common::tests::TestProgram;

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
        program.run_pass(&InstructionCombine);
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
        program.run_pass(&InstructionCombine);
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
        program.run_pass(&InstructionCombine);
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
        program.run_pass(&InstructionCombine);
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
}
