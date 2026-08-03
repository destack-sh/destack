use std::collections::{HashMap, HashSet};

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{
    ConstantPropagation, Mutation, TargetLayout, fold_binary, fold_cast, fold_intrinsic,
    fold_unary, instruction_substitute_uses_in_tree, remap_instruction_memory_accesses,
    resolve_substitution_chains, terminator_substitute_uses,
};

declare_pass! {
    /// Fold constant expressions at compile time.
    ///
    /// Evaluates operations on constant values and replaces them with the
    /// computed result. This includes arithmetic, comparisons, and logical
    /// operations where all operands are known constants.
    ///
    /// ```mir
    /// function before(): int32 {
    /// b0:
    ///     v0 = 2int32
    ///     v1 = 3int32
    ///     v2 = int.add v0, v1
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(): int32 {
    /// b0:
    ///     v0 = 5int32
    ///     return v0
    /// }
    /// ```
    #[pass(id = "fold-constants")]
    pub FoldConstants,
    "Fold constant expressions"
}

impl FunctionPass for FoldConstants {
    fn run(
        &self,
        function: &mut mir::Function,
        optimized: &mut MirOptimized,
        ctx: &PipelineContext<'_>,
        analyses: &mir::FunctionAnalysisCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;
        let memory = &mut optimized.memory;

        // get constant propagation analysis
        let constants = { analyses.get::<ConstantPropagation>(function, tree).clone() };

        // run constant folding
        let changed = run_fold_constants(function, tree, memory, &constants, ctx.target_layout());

        // report what this pass changed
        if changed {
            Mutation::CONTROL | Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }

    fn name(&self) -> &'static str {
        "FoldConstants"
    }

    fn id(&self) -> &'static str {
        "fold-constants"
    }
}

/// Core constant folding logic. Returns true if changes were made.
fn run_fold_constants(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    memory: &mut mir::MemoryTable,
    constants: &ConstantPropagation,
    target_layout: TargetLayout,
) -> bool {
    // track pass state and pending rewrites
    let mut changed = false;
    let mut substitutions: HashMap<mir::Value, mir::Value> = HashMap::new();
    let mut to_remove: HashSet<mir::LocalNodeId<mir::Instruction>> = HashSet::new();

    // fold instructions with local constants
    for &block_id in function.blocks() {
        // seed constants for this block
        let mut block_constants = constants.entry(block_id).clone();
        let block = tree.get(block_id);
        let instruction_ids: Vec<_> = block.instructions.clone();

        // walk instructions in order
        for instruction_id in instruction_ids {
            let instruction = tree.get(instruction_id);
            let destination = instruction.destination();

            // fold instruction when possible
            match instruction {
                mir::Instruction::Const { destination, value } => {
                    block_constants.insert(*destination, value.clone());
                }

                mir::Instruction::Binary {
                    destination,
                    operator,
                    left,
                    right,
                } => {
                    // fold binary ops with constant operands
                    let destination = *destination;
                    let left = *left;
                    let right = *right;

                    let left_const = block_constants.get(left);
                    let right_const = block_constants.get(right);

                    if let (Some(left_val), Some(right_val)) = (left_const, right_const)
                        && let Some(result) =
                            fold_binary(*operator, left_val.clone(), right_val.clone())
                    {
                        let new_instruction = mir::Instruction::Const {
                            destination,
                            value: result.clone(),
                        };
                        tree.set(instruction_id, new_instruction);
                        block_constants.insert(destination, result);
                        changed = true;
                    } else {
                        block_constants.remove(destination);
                    }
                }

                mir::Instruction::Unary {
                    destination,
                    operator,
                    argument,
                } => {
                    // fold unary ops with constant operands
                    let destination = *destination;
                    let argument = *argument;

                    if let Some(arg_const) = block_constants.get(argument)
                        && let Some(result) = fold_unary(*operator, arg_const.clone())
                    {
                        let new_instruction = mir::Instruction::Const {
                            destination,
                            value: result.clone(),
                        };
                        tree.set(instruction_id, new_instruction);
                        block_constants.insert(destination, result);
                        changed = true;
                    } else {
                        block_constants.remove(destination);
                    }
                }

                mir::Instruction::Select {
                    destination,
                    condition,
                    then_value,
                    else_value,
                } => {
                    // fold select with constant condition
                    let destination = *destination;
                    let condition = *condition;
                    let then_value = *then_value;
                    let else_value = *else_value;

                    if let Some(mir::Constant::Boolean { value: cond_val }) =
                        block_constants.get(condition)
                    {
                        let selected = if *cond_val { then_value } else { else_value };

                        // if selected value is constant, fold to constant
                        if let Some(result) = block_constants.get(selected) {
                            let new_instruction = mir::Instruction::Const {
                                destination,
                                value: result.clone(),
                            };
                            tree.set(instruction_id, new_instruction);
                            block_constants.insert(destination, result.clone());
                            changed = true;
                        } else {
                            // condition is constant but selected value isn't
                            substitutions.insert(destination, selected);
                            to_remove.insert(instruction_id);
                            block_constants.remove(destination);
                            changed = true;
                        }
                    } else {
                        block_constants.remove(destination);
                    }
                }
                mir::Instruction::Cast {
                    destination,
                    operator,
                    argument,
                    to_type,
                } => {
                    // fold casts with constant operands
                    let destination = *destination;
                    let argument = *argument;
                    let to_type = *to_type;

                    if let Some(arg_const) = block_constants.get(argument)
                        && let Some(result) = fold_cast(
                            *operator,
                            arg_const.clone(),
                            to_type,
                            target_layout.pointer_bits(),
                            tree,
                        )
                    {
                        let new_instruction = mir::Instruction::Const {
                            destination,
                            value: result.clone(),
                        };
                        tree.set(instruction_id, new_instruction);
                        block_constants.insert(destination, result);
                        changed = true;
                    } else {
                        block_constants.remove(destination);
                    }
                }
                mir::Instruction::Intrinsic {
                    destination: Some(destination),
                    intrinsic,
                    arguments,
                    ..
                } => {
                    // fold pure intrinsics with constant arguments
                    let mut constant_arguments = Vec::new();
                    let destination = *destination;

                    for &argument in tree.get_values(*arguments) {
                        let Some(constant) = block_constants.get(argument) else {
                            constant_arguments.clear();
                            break;
                        };
                        constant_arguments.push(constant.clone());
                    }

                    if !constant_arguments.is_empty() && intrinsic.is_pure() {
                        if let Some(result) = fold_intrinsic(*intrinsic, &constant_arguments) {
                            let new_instruction = mir::Instruction::Const {
                                destination,
                                value: result.clone(),
                            };
                            tree.set(instruction_id, new_instruction);
                            block_constants.insert(destination, result);
                            changed = true;
                        } else {
                            block_constants.remove(destination);
                        }
                    } else {
                        block_constants.remove(destination);
                    }
                }
                _ => {
                    // clear destinations for unknown instructions
                    if let Some(dest) = destination {
                        block_constants.remove(dest);
                    }
                }
            }
        }
    }

    // apply substitutions and remove redundant instructions
    if !substitutions.is_empty() || !to_remove.is_empty() {
        // resolve transitive substitutions
        let substitutions = resolve_substitution_chains(substitutions);

        for &block_id in function.blocks() {
            let instruction_ids: Vec<_> = tree.get(block_id).instructions.clone();
            for instruction_id in instruction_ids {
                // skip instructions that will be removed
                if to_remove.contains(&instruction_id) {
                    continue;
                }

                let instruction = tree.get(instruction_id).clone();
                let updated =
                    instruction_substitute_uses_in_tree(&instruction, &substitutions, tree);

                // replace instructions when substitutions apply
                if updated != instruction {
                    tree.set(instruction_id, updated);
                    remap_instruction_memory_accesses(memory, instruction_id, &substitutions);
                }
            }
        }

        let block_ids = function.blocks().to_vec();
        for block_id in block_ids {
            let (terminator_id, instructions) = {
                let block = tree.get(block_id);
                (block.terminator, block.instructions.clone())
            };
            let terminator = tree.get(terminator_id).clone();
            let new_terminator = terminator_substitute_uses(tree, &terminator, &substitutions);
            let new_instructions: Vec<_> = instructions
                .iter()
                .copied()
                .filter(|id| !to_remove.contains(id))
                .collect();

            // rewrite blocks when instructions or terminators change
            if new_terminator != terminator || new_instructions.len() != instructions.len() {
                tree.set(terminator_id, new_terminator);
                function.replace_block_instructions(block_id, new_instructions, tree);
            }
        }

        changed = true;
    }

    // fold terminators with constant conditions
    changed |= fold_terminators(function, tree, constants);

    changed
}

/// Fold branch, check, and switch terminators when conditions are constant.
fn fold_terminators(
    function: &mir::Function,
    tree: &mut mir::Tree,
    constants: &ConstantPropagation,
) -> bool {
    // track whether any terminators change
    let mut changed = false;

    // scan blocks for foldable terminators
    for &block_id in function.blocks() {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        let exit_constants = constants.exit(block_id);

        let new_terminator = match terminator {
            mir::Terminator::Branch {
                condition,
                then_target,
                else_target,
            } => {
                let condition = *condition;
                let condition_constant = exit_constants.get(condition);
                let condition_value = match condition_constant {
                    Some(mir::Constant::Boolean { value }) => Some(*value),
                    _ => None,
                };

                condition_value.map(|is_true| {
                    let target = if is_true {
                        then_target.clone()
                    } else {
                        else_target.clone()
                    };
                    mir::Terminator::Jump { target }
                })
            }
            mir::Terminator::Switch {
                value,
                default,
                cases,
            } => {
                let value = *value;
                let constant_value = exit_constants.get(value);
                let selected = match constant_value {
                    Some(mir::Constant::Int { value, .. }) => Some(*value),
                    Some(mir::Constant::UInt { value, .. }) => i128::try_from(*value).ok(),
                    _ => None,
                };

                selected.map(|value| {
                    let mut target = default.clone();
                    let cases = tree.get_switch_cases(*cases);
                    for case in cases {
                        if case.value == value {
                            target = case.target.clone();
                            break;
                        }
                    }

                    mir::Terminator::Jump { target }
                })
            }
            _ => None,
        };

        // update the terminator when a constant fold applies
        if let Some(new_terminator) = new_terminator {
            tree.set(block.terminator, new_terminator);
            changed = true;
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use destack_mir::fold_binary_signed;

    use crate::optimize::common::tests::TestProgram;

    /// Signed integer addition folds to result, division by zero returns None.
    #[test]
    fn test_fold_binary_signed() {
        assert_eq!(
            fold_binary_signed(1, 2, 32, mir::BinaryOperator::Add),
            Some(mir::Constant::Int {
                value: 3,
                width: 32,
                is_signed: true
            })
        );

        assert_eq!(
            fold_binary_signed(10, 0, 32, mir::BinaryOperator::SignedDivide),
            None
        );
    }

    /// Unary negation folds signed integer to its negated value.
    #[test]
    fn test_fold_unary_negation() {
        assert_eq!(
            fold_unary(
                mir::UnaryOperator::Negate,
                mir::Constant::Int {
                    value: 5,
                    width: 32,
                    is_signed: true
                }
            ),
            Some(mir::Constant::Int {
                value: -5,
                width: 32,
                is_signed: true
            })
        );
    }

    /// Binary add of two constants folds to their sum.
    #[test]
    fn test_fold_binary_add() {
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = int.add v0, v1
    return v2
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = 3
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_output(expected);
    }

    /// Chained arithmetic operations fold through intermediate results.
    #[test]
    fn test_fold_chained_operations() {
        // 2 * 3 = 6, then 6 + 4 = 10
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 2
    v1: int32 = 3
    v2: int32 = int.mul v0, v1
    v3: int32 = 4
    v4: int32 = int.add v2, v3
    return v4
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: int32 = 2
    v1: int32 = 3
    v2: int32 = 6
    v3: int32 = 4
    v4: int32 = 10
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_output(expected);
    }

    /// Comparison of constants folds to boolean result.
    #[test]
    fn test_fold_comparison() {
        let input = r#"
function test(): boolean {
entry:
    v0: int32 = 5
    v1: int32 = 3
    v2: boolean = int.gt.s v0, v1
    return v2
}
"#;
        let expected = r#"
function test(): boolean {
entry:
    v0: int32 = 5
    v1: int32 = 3
    v2: boolean = true
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_output(expected);
    }

    /// Operations with non-constant operands are not folded.
    #[test]
    fn test_preserve_non_constant_operands() {
        // v0 is a parameter, not a constant
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 2
    v2: int32 = int.add v0, v1
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_unchanged(input);
    }

    /// Unary negation instruction folds constant to negated value.
    #[test]
    fn test_fold_unary_negation_instruction() {
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 42
    v1: int32 = int.negate v0
    return v1
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: int32 = 42
    v1: int32 = -42
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_output(expected);
    }

    /// Boolean not folds true to false.
    #[test]
    fn test_fold_boolean_not() {
        let input = r#"
function test(): boolean {
entry:
    v0: boolean = true
    v1: boolean = int.not v0
    return v1
}
"#;
        let expected = r#"
function test(): boolean {
entry:
    v0: boolean = true
    v1: boolean = false
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_output(expected);
    }

    /// Unsigned integer division folds correctly.
    #[test]
    fn test_fold_unsigned_division() {
        let input = r#"
function test(): uint32 {
entry:
    v0: uint32 = 10
    v1: uint32 = 3
    v2: uint32 = int.div.u v0, v1
    return v2
}
"#;
        let expected = r#"
function test(): uint32 {
entry:
    v0: uint32 = 10
    v1: uint32 = 3
    v2: uint32 = 3
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_output(expected);
    }

    /// Division by zero is not folded to avoid compile-time UB.
    #[test]
    fn test_preserve_division_by_zero() {
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 10
    v1: int32 = 0
    v2: int32 = int.div.s v0, v1
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_unchanged(input);
    }

    /// Constants from earlier blocks are available for folding in later blocks.
    #[test]
    fn test_fold_across_blocks() {
        let input = r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 5
    v2: int32 = 3
    branch v0 => b1 | b2

b1:
    v3: int32 = int.add v1, v2
    return v3

b2:
    v4: int32 = int.mul v1, v2
    return v4
}
"#;
        let expected = r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 5
    v2: int32 = 3
    branch v0 => b1 | b2

b1:
    v3: int32 = 8
    return v3

b2:
    v4: int32 = 15
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_output(expected);
    }

    /// Readonly global loads do not fold through unary operations.
    #[test]
    fn test_preserve_readonly_global_load_boolean() {
        let input = r#"
readonly global flag: boolean = true

function test(): boolean {
entry:
    v0: ref<boolean, raw, readonly> = global.address flag
    v1: boolean = load v0
    v2: boolean = int.not v1
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_unchanged(input);
    }

    /// Mutable globals do not fold through constant operations.
    #[test]
    fn test_preserve_mutable_global_load_boolean() {
        let input = r#"
global flag: boolean = true

function test(): boolean {
entry:
    v0: ref<boolean, raw, mutable> = global.address flag
    v1: boolean = load v0
    v2: boolean = int.not v1
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_unchanged(input);
    }

    /// Block parameter constants fold within successor blocks.
    #[test]
    fn test_fold_block_param_constant() {
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 3
    jump b1(v0)

b1(v1: int32):
    v2: int32 = int.add v1, v1
    return v2
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: int32 = 3
    jump b1(v0)

b1(v1: int32):
    v2: int32 = 6
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_output(expected);
    }

    /// Sign extend casts fold to constants.
    #[test]
    fn test_fold_cast_sign_extend() {
        let input = r#"
function test(): int64 {
entry:
    v0: int32 = -1
    v1: int64 = cast.extend.s v0 -> int64
    return v1
}
"#;
        let expected = r#"
function test(): int64 {
entry:
    v0: int32 = -1
    v1: int64 = -1
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_output(expected);
    }

    /// Truncate casts fold to constants.
    #[test]
    fn test_fold_cast_truncate() {
        let input = r#"
function test(): uint8 {
entry:
    v0: uint16 = 257
    v1: uint8 = cast.truncate v0 -> uint8
    return v1
}
"#;
        let expected = r#"
function test(): uint8 {
entry:
    v0: uint16 = 257
    v1: uint8 = 1
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_output(expected);
    }

    /// Bitwise and, or, xor fold correctly on integer constants.
    #[test]
    fn test_fold_bitwise_operations() {
        // 10 & 12 = 8, 10 | 12 = 14, 10 ^ 12 = 6
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 10
    v1: int32 = 12
    v2: int32 = int.and v0, v1
    v3: int32 = int.or v0, v1
    v4: int32 = int.xor v0, v1
    return v2
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: int32 = 10
    v1: int32 = 12
    v2: int32 = 8
    v3: int32 = 14
    v4: int32 = 6
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_output(expected);
    }

    /// Shift left and arithmetic shift right fold correctly.
    #[test]
    fn test_fold_shift_operations() {
        // 8 << 2 = 32, 8 >> 2 = 2 (signed/arithmetic)
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 8
    v1: int32 = 2
    v2: int32 = int.shl v0, v1
    v3: int32 = int.shr.s v0, v1
    return v2
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: int32 = 8
    v1: int32 = 2
    v2: int32 = 32
    v3: int32 = 2
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_output(expected);
    }

    /// Boolean and/or operations fold correctly.
    #[test]
    fn test_fold_boolean_and_or() {
        // true && false = false, true || false = true
        let input = r#"
function test(): boolean {
entry:
    v0: boolean = true
    v1: boolean = false
    v2: boolean = int.and v0, v1
    v3: boolean = int.or v0, v1
    return v2
}
"#;
        let expected = r#"
function test(): boolean {
entry:
    v0: boolean = true
    v1: boolean = false
    v2: boolean = false
    v3: boolean = true
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_output(expected);
    }

    /// Select with constant true condition folds to then_value.
    #[test]
    fn test_fold_select_true_condition() {
        let input = r#"
function test(): int32 {
entry:
    v0: boolean = true
    v1: int32 = 42
    v2: int32 = 0
    v3: int32 = select v0, v1, v2
    return v3
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: boolean = true
    v1: int32 = 42
    v2: int32 = 0
    v3: int32 = 42
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_output(expected);
    }

    /// Select with constant false condition folds to else_value.
    #[test]
    fn test_fold_select_false_condition() {
        let input = r#"
function test(): int32 {
entry:
    v0: boolean = false
    v1: int32 = 42
    v2: int32 = 0
    v3: int32 = select v0, v1, v2
    return v3
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: boolean = false
    v1: int32 = 42
    v2: int32 = 0
    v3: int32 = 0
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_output(expected);
    }

    /// Select with non-constant condition is preserved.
    #[test]
    fn test_preserve_select_non_constant_condition() {
        let input = r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 42
    v2: int32 = 0
    v3: int32 = select v0, v1, v2
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_unchanged(input);
    }

    /// Select with constant condition forwards non constant values.
    #[test]
    fn test_fold_select_constant_to_copy() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: boolean = true
    v2: int32 = int.add v0, v0
    v3: int32 = select v1, v2, v0
    return v3
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: boolean = true
    v2: int32 = int.add v0, v0
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_output(expected);
    }

    /// Branches with constant conditions are folded to jumps.
    #[test]
    fn test_fold_constant_branch() {
        let input = r#"
function test(): int32 {
entry:
    v0: boolean = true
    branch v0 => b1 | b2

b1:
    v1: int32 = 1
    return v1

b2:
    v2: int32 = 2
    return v2
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: boolean = true
    jump b1

b1:
    v1: int32 = 1
    return v1

b2:
    v2: int32 = 2
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_output(expected);
    }

    /// Pure intrinsics with constant arguments fold to constants.
    #[test]
    fn test_fold_intrinsic_clz() {
        let input = r#"
function test(): uint32 {
entry:
    v0: uint32 = 8
    v1: uint32 = intrinsic.math.bits.leadingZeroCount(v0)
    return v1
}
"#;
        let expected = r#"
function test(): uint32 {
entry:
    v0: uint32 = 8
    v1: uint32 = 28
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&FoldConstants);
        test.assert_output(expected);
    }
}
