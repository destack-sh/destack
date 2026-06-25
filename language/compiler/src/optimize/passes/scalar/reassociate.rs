use std::collections::HashMap;

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, PipelineContext};
use destack_mir::{
    ConstantMap, ConstantPropagation, InstructionRef, Mutation, ValueTypeMap,
    build_value_instruction_refs, fold_binary,
};

declare_pass! {
    /// Reassociate associative expressions to expose constant folding.
    ///
    /// This pass combines constants across associative binary chains,
    /// enabling later constant folding and CSE.
    ///
    /// ```mir
    /// function before(v0: int32): int32 {
    /// b0(v0: int32):
    ///     v1 = 1int32
    ///     v2 = 2int32
    ///     v3 = int.add v0, v1
    ///     v4 = int.add v3, v2
    ///     return v4
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: int32): int32 {
    /// b0(v0: int32):
    ///     v1 = 1int32
    ///     v2 = 2int32
    ///     v3 = int.add v0, v1
    ///     v5 = 3int32
    ///     v4 = int.add v0, v5
    ///     return v4
    /// }
    /// ```
    ///
    /// Restrictions:
    /// - Only handles associative and commutative integer operators
    /// - Only reassociates when constants can be combined safely
    #[pass(id = "reassociate")]
    pub Reassociate,
    "Reassociate associative expressions"
}

impl FunctionPass for Reassociate {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        _ctx: &PipelineContext<'_>,
        analyses: &mir::FunctionAnalyses,
    ) -> Mutation {
        // collect constant propagation state
        let constants = { analyses.get::<ConstantPropagation>(function, tree).clone() };

        // run reassociation
        let changed = run_reassociate(function, tree, &constants);

        // report what this pass changed
        if changed {
            Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }

    fn name(&self) -> &'static str {
        "Reassociate"
    }

    fn id(&self) -> &'static str {
        "reassociate"
    }
}

/// Reassociate binary operations within each block.
fn run_reassociate(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    constants: &ConstantPropagation,
) -> bool {
    // build lookup for value definitions
    let mut value_to_instruction = build_value_instruction_refs(function, tree);

    // build value type lookup
    let value_types = ValueTypeMap::new(function, tree);

    // track whether any changes were made
    let mut changed = false;

    // walk blocks in order
    let block_ids = function.blocks.clone();
    for block_id in block_ids {
        // seed constant map for this block
        let mut block_constants = constants.entry(block_id).clone();

        // build a new instruction list when constants are inserted
        let block = tree.get(block_id).clone();
        let mut new_instructions = Vec::with_capacity(block.instructions.len());
        let instruction_ids = block.instructions.clone();

        // iterate in order so constant state is accurate
        for (instruction_index, instruction_id) in instruction_ids.iter().enumerate() {
            // read the current instruction
            let instruction = tree.get(*instruction_id).clone();

            // attempt reassociation for binary instructions
            if let mir::Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } = instruction
            {
                let (Some(destination), Some(left), Some(right)) =
                    (Some(destination), Some(left), Some(right))
                else {
                    new_instructions.push(*instruction_id);
                    continue;
                };

                // build a reassociation plan
                let plan = reassociate_binary(
                    operator,
                    left,
                    right,
                    block_id,
                    instruction_index,
                    &block_constants,
                    &value_to_instruction,
                );

                // apply reassociation when available
                let mut left_value = left;
                let mut right_value = right;

                if let Some(plan) = plan {
                    // resolve or insert the combined constant
                    let resolved = resolve_constant_value(
                        plan.constant.clone(),
                        value_types.expect_value_type(destination),
                        function,
                        tree,
                        &block_constants,
                    );
                    let combined_value = resolved.value;

                    // insert the constant before the current instruction when needed
                    if let Some(instruction_id) = resolved.instruction_id {
                        new_instructions.push(instruction_id);
                        block_constants.insert(combined_value, plan.constant.clone());
                    }

                    // build a new operand list ending with the combined constant
                    let mut operands = plan.operands;
                    operands.push(combined_value);

                    // rebuild the chain with fresh values when needed
                    let Some((base, last)) = rebuild_chain(
                        function,
                        tree,
                        operator,
                        &operands,
                        value_types.expect_value_type(destination),
                        &mut new_instructions,
                    ) else {
                        new_instructions.push(*instruction_id);
                        continue;
                    };

                    // update the binary instruction with the rebuilt operands
                    let new_instruction = mir::Instruction::Binary {
                        destination,
                        operator,
                        left: base,
                        right: last,
                    };
                    tree.set(*instruction_id, new_instruction.clone());
                    value_to_instruction.insert(
                        destination,
                        InstructionRef {
                            instruction: new_instruction,
                            block: block_id,
                            index: instruction_index,
                        },
                    );
                    changed = true;

                    // update operand tracking for constant propagation
                    left_value = base;
                    right_value = last;
                }

                // update constant state for this destination
                let left_const = block_constants.get(left_value);
                let right_const = block_constants.get(right_value);
                if let (Some(left_const), Some(right_const)) = (left_const, right_const)
                    && let Some(result) =
                        fold_binary(operator, left_const.clone(), right_const.clone())
                {
                    block_constants.insert(destination, result);
                } else {
                    block_constants.remove(destination);
                }

                // keep the instruction in the block
                new_instructions.push(*instruction_id);
                continue;
            }

            // update constant tracking for non binary instructions
            match instruction {
                mir::Instruction::Const { destination, value } => {
                    block_constants.insert(destination, value);
                }
                _ => {
                    if let Some(destination) = instruction.destination() {
                        block_constants.remove(destination);
                    }
                }
            }

            // keep the instruction in the block
            new_instructions.push(*instruction_id);
        }

        // update block instructions when needed
        if new_instructions != block.instructions {
            let mut updated_block = block.clone();
            updated_block.instructions = new_instructions;
            tree.set(block_id, updated_block);
            changed = true;
        }
    }

    changed
}

/// Plan for reassociating a binary instruction.
struct ReassociatePlan {
    /// Non constant operand values.
    operands: Vec<mir::Value>,
    /// Combined constant value.
    constant: mir::Constant,
}

/// Context for collecting associative operands.
struct CollectContext<'a> {
    /// Operator being reassociated.
    operator: mir::BinaryOperator,
    /// Block containing the current instruction.
    block_id: mir::LocalNodeId<mir::Block>,
    /// Instruction index within the block.
    instruction_index: usize,
    /// Constant information for the current block.
    constants: &'a ConstantMap,
    /// Definition lookup for values.
    value_to_instruction: &'a HashMap<mir::Value, InstructionRef>,
}

/// Decide whether a binary instruction can be reassociated.
fn reassociate_binary(
    operator: mir::BinaryOperator,
    left: mir::Value,
    right: mir::Value,
    block_id: mir::LocalNodeId<mir::Block>,
    instruction_index: usize,
    constants: &ConstantMap,
    value_to_instruction: &HashMap<mir::Value, InstructionRef>,
) -> Option<ReassociatePlan> {
    // only reassociate associative and commutative operators
    if !binary_operator_is_associative(operator) {
        return None;
    }

    // collect operands for the full associative chain
    let mut non_constants = Vec::new();
    let mut constant_values = Vec::new();
    let mut constant_cache = HashMap::new();
    let context = CollectContext {
        operator,
        block_id,
        instruction_index,
        constants,
        value_to_instruction,
    };
    collect_associative_operands(
        &context,
        left,
        &mut non_constants,
        &mut constant_values,
        &mut constant_cache,
    );
    collect_associative_operands(
        &context,
        right,
        &mut non_constants,
        &mut constant_values,
        &mut constant_cache,
    );

    // skip when there are not enough constants to combine
    if constant_values.len() < 2 {
        return None;
    }

    // require at least one non constant operand
    if non_constants.is_empty() {
        return None;
    }

    // fold constants into a single value
    let combined = combine_constants(operator, &constant_values)?;

    Some(ReassociatePlan {
        operands: non_constants,
        constant: combined,
    })
}

/// Collect associative operands into constant and non constant buckets.
fn collect_associative_operands(
    context: &CollectContext<'_>,
    value: mir::Value,
    non_constants: &mut Vec<mir::Value>,
    constant_values: &mut Vec<mir::Constant>,
    constant_cache: &mut HashMap<mir::Value, bool>,
) {
    // capture constants immediately
    if let Some(constant) = context.constants.get(value) {
        constant_values.push(constant.clone());
        return;
    }

    // avoid flattening pure non constant subtrees
    if !associative_subtree_contains_constant(context, value, constant_cache) {
        non_constants.push(value);
        return;
    }

    // walk nested associative instructions within the block
    let Some(definition) = context.value_to_instruction.get(&value) else {
        non_constants.push(value);
        return;
    };

    // stop when the definition is outside the current prefix
    if definition.block != context.block_id || definition.index >= context.instruction_index {
        non_constants.push(value);
        return;
    }

    // extract nested operands when the operator matches
    let mir::Instruction::Binary {
        operator: nested,
        left,
        right,
        ..
    } = &definition.instruction
    else {
        non_constants.push(value);
        return;
    };

    // stop when the operator is not associative
    if *nested != context.operator {
        non_constants.push(value);
        return;
    }

    // collect nested operands
    collect_associative_operands(
        context,
        *left,
        non_constants,
        constant_values,
        constant_cache,
    );

    collect_associative_operands(
        context,
        *right,
        non_constants,
        constant_values,
        constant_cache,
    );
}

/// Check whether an associative subtree contains any constants.
fn associative_subtree_contains_constant(
    context: &CollectContext<'_>,
    value: mir::Value,
    constant_cache: &mut HashMap<mir::Value, bool>,
) -> bool {
    // reuse cached results when possible
    if let Some(has_constant) = constant_cache.get(&value) {
        return *has_constant;
    }

    // mark constants immediately
    if context.constants.get(value).is_some() {
        constant_cache.insert(value, true);
        return true;
    }

    // require an in block definition before checking nested operands
    let Some(definition) = context.value_to_instruction.get(&value) else {
        constant_cache.insert(value, false);
        return false;
    };
    if definition.block != context.block_id || definition.index >= context.instruction_index {
        constant_cache.insert(value, false);
        return false;
    }

    // only recurse on matching associative operators
    let mir::Instruction::Binary {
        operator: nested,
        left,
        right,
        ..
    } = &definition.instruction
    else {
        constant_cache.insert(value, false);
        return false;
    };
    if *nested != context.operator {
        constant_cache.insert(value, false);
        return false;
    }

    // check whether either subtree contains constants
    let left_has = associative_subtree_contains_constant(context, *left, constant_cache);
    let right_has = associative_subtree_contains_constant(context, *right, constant_cache);
    let has_constant = left_has || right_has;

    constant_cache.insert(value, has_constant);
    has_constant
}

/// Rebuild a chain of binary operations for a list of operands.
fn rebuild_chain(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    operator: mir::BinaryOperator,
    operands: &[mir::Value],
    result_type: mir::LocalNodeId<mir::Type>,
    new_instructions: &mut Vec<mir::LocalNodeId<mir::Instruction>>,
) -> Option<(mir::Value, mir::Value)> {
    // require at least two operands
    if operands.len() < 2 {
        return None;
    }

    // fold operands left to right using fresh temporaries
    let mut current = operands[0];
    for operand in &operands[1..operands.len() - 1] {
        let destination = function.next_typed_value(result_type);
        let instruction = mir::Instruction::Binary {
            destination,
            operator,
            left: current,
            right: (*operand),
        };
        let instruction_id = tree.insert(instruction);
        new_instructions.push(instruction_id);
        current = destination;
    }

    // return the final pair for the last instruction
    let last = *operands.last()?;
    Some((current, last))
}

/// Fold a list of constants into a single value.
fn combine_constants(
    operator: mir::BinaryOperator,
    constants: &[mir::Constant],
) -> Option<mir::Constant> {
    // seed the combined constant
    let mut iter = constants.iter();
    let mut combined = iter.next()?.clone();

    // fold remaining constants
    for constant in iter {
        combined = fold_binary(operator, combined, constant.clone())?;
    }

    Some(combined)
}

/// Materialize a constant value or reuse an existing one.
struct ResolvedConstant {
    /// Constant value to use.
    value: mir::Value,
    /// Instruction id when a new constant is created.
    instruction_id: Option<mir::LocalNodeId<mir::Instruction>>,
}

/// Resolve a constant value or create a new instruction.
fn resolve_constant_value(
    constant: mir::Constant,
    result_type: mir::LocalNodeId<mir::Type>,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    block_constants: &ConstantMap,
) -> ResolvedConstant {
    // reuse an existing constant when available
    for (value, existing) in block_constants.iter() {
        if *existing == constant {
            return ResolvedConstant {
                value,
                instruction_id: None,
            };
        }
    }

    // insert a new constant instruction
    let destination = function.next_typed_value(result_type);
    let instruction = mir::Instruction::Const {
        destination,
        value: constant,
    };
    let instruction_id = tree.insert(instruction);

    ResolvedConstant {
        value: destination,
        instruction_id: Some(instruction_id),
    }
}

/// Check if a binary operator is associative and commutative.
fn binary_operator_is_associative(operator: mir::BinaryOperator) -> bool {
    // restrict to integer associative and commutative operators
    matches!(
        operator,
        mir::BinaryOperator::Add
            | mir::BinaryOperator::Multiply
            | mir::BinaryOperator::And
            | mir::BinaryOperator::Or
            | mir::BinaryOperator::Xor
    )
}

#[cfg(test)]
mod tests {
    use crate::optimize::common::tests::TestProgram;
    use crate::optimize::passes::Reassociate;

    /// Constant reassociation combines adjacent constants.
    #[test]
    fn test_reassociate_add_constants() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = 2
    v3: int32 = int.add v0, v1
    v4: int32 = int.add v3, v2
    return v4
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = 2
    v3: int32 = int.add v0, v1
    v5: int32 = 3
    v4: int32 = int.add v0, v5
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Reassociate);
        test.assert_output(expected);
    }

    /// Float arithmetic is not reassociated.
    #[test]
    fn test_reassociate_skips_float() {
        let input = r#"
function test(v0: float64): float64 {
entry(v0: float64):
    v1: float64 = 1
    v2: float64 = 2
    v3: float64 = float.add v0, v1
    v4: float64 = float.add v3, v2
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Reassociate);
        test.assert_output(input);
    }

    /// Reassociation does not fire without adjacent constants.
    #[test]
    fn test_reassociate_requires_constant() {
        let input = r#"
function test(v0: int32, v1: int32, v2: int32): int32 {
entry(v0: int32, v1: int32, v2: int32):
    v3: int32 = int.add v0, v1
    v4: int32 = int.add v3, v2
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Reassociate);
        test.assert_output(input);
    }

    /// Multiple constants are combined across associative chains.
    #[test]
    fn test_reassociate_combines_multiple_constants() {
        // source test
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = 2
    v3: int32 = 3
    v4: int32 = int.add v0, v1
    v5: int32 = int.add v4, v2
    v6: int32 = int.add v5, v3
    return v6
}
"#;
        // expected output
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = 2
    v3: int32 = 3
    v4: int32 = int.add v0, v1
    v5: int32 = int.add v0, v3
    v7: int32 = 6
    v6: int32 = int.add v0, v7
    return v6
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&Reassociate);
        test.assert_output(expected);
    }

    /// Constant folding combines around a non constant subtree.
    #[test]
    fn test_reassociate_combines_constants_with_nonconstant_subtree() {
        // source test
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = 4
    v3: int32 = 5
    v4: int32 = int.add v0, v1
    v5: int32 = int.add v4, v2
    v6: int32 = int.add v5, v3
    return v6
}
"#;
        // expected output
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = 4
    v3: int32 = 5
    v4: int32 = int.add v0, v1
    v5: int32 = int.add v4, v2
    v7: int32 = 9
    v6: int32 = int.add v4, v7
    return v6
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&Reassociate);
        test.assert_output(expected);
    }

    /// Constant folding rebuilds chains with multiple mixed operands.
    #[test]
    fn test_reassociate_rebuilds_chain_with_multiple_operands() {
        // source test
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = 1
    v3: int32 = 2
    v4: int32 = int.add v0, v2
    v5: int32 = int.add v1, v3
    v6: int32 = int.add v4, v5
    return v6
}
"#;
        // expected output
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = 1
    v3: int32 = 2
    v4: int32 = int.add v0, v2
    v5: int32 = int.add v1, v3
    v7: int32 = 3
    v8: int32 = int.add v0, v1
    v6: int32 = int.add v8, v7
    return v6
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&Reassociate);
        test.assert_output(expected);
    }

    /// Multiplication constants are reassociated for folding.
    #[test]
    fn test_reassociate_multiply_constants() {
        // source test
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 2
    v2: int32 = 3
    v3: int32 = int.mul v0, v1
    v4: int32 = int.mul v3, v2
    return v4
}
"#;
        // expected output
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 2
    v2: int32 = 3
    v3: int32 = int.mul v0, v1
    v5: int32 = 6
    v4: int32 = int.mul v0, v5
    return v4
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&Reassociate);
        test.assert_output(expected);
    }

    /// Bitwise constants are reassociated for folding.
    #[test]
    fn test_reassociate_bitwise_constants() {
        // source test
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = 2
    v3: int32 = int.and v0, v1
    v4: int32 = int.and v3, v2
    return v4
}
"#;
        // expected output
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = 2
    v3: int32 = int.and v0, v1
    v5: int32 = 0
    v4: int32 = int.and v0, v5
    return v4
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&Reassociate);
        test.assert_output(expected);
    }
}
