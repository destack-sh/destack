use std::collections::HashMap;

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{
    ConstantState, ConstantTable, Mutation, RangeState, RangeTable, TargetLayout,
    constant_all_ones_like, constant_is_all_ones, constant_is_one, constant_is_zero,
    constant_zero_like, fold_binary, fold_cast, fold_unary, instruction_substitute_uses_in_tree,
    remap_instruction_memory_accesses, resolve_substitution_chains, terminator_substitute_uses,
};

/// Maximum recursion depth for chained field.set simplification.
const MAX_AGGREGATE_CHAIN_DEPTH: usize = 64;

declare_pass! {
    /// Algebraic simplification of instructions.
    ///
    /// ```mir
    /// function before(v0: int32): int32 {
    /// b0(v0: int32):
    ///     v1: int32 = 0
    ///     v2: int32 = add v0, v1
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: int32): int32 {
    /// b0(v0: int32):
    ///     return v0
    /// }
    /// ```
    #[pass(id = "combine-instructions")]
    pub CombineInstructions,
    "Combine and simplify instructions"
}

/// Result of simplifying an instruction.
enum Simplification {
    /// Replace with a constant value.
    Constant(mir::Constant),
    /// Replace all uses of destination with this value (identity).
    Substitute(mir::Value),
}

impl FunctionPass for CombineInstructions {
    fn run(
        &self,
        function: &mut mir::Function,
        optimized: &mut MirOptimized,
        ctx: &PipelineContext<'_>,
        analyses: &mut mir::FunctionCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;
        let accesses = &mut optimized.accesses;

        // collect analyses and options
        let constants = analyses.constant(function, tree).clone();
        let ranges = analyses.range(function, tree).clone();
        let changed = run_combine_instructions(
            function,
            tree,
            accesses,
            &constants,
            &ranges,
            ctx.target_layout(),
        );

        // report what this pass changed
        if changed {
            Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }
}

/// Tracks a field.set instruction for simplification.
struct FieldSetEntry {
    /// The original aggregate being modified.
    aggregate: mir::Value,
    /// The field index being updated.
    index: u32,
    /// The new value inserted at the field.
    value: mir::Value,
}

/// Tracks a field.get instruction for identity detection.
struct FieldGetEntry {
    /// The aggregate being extracted from.
    aggregate: mir::Value,
    /// The field index being extracted.
    index: u32,
}

/// Core instruction combine logic.
fn run_combine_instructions(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
    constants: &ConstantTable,
    ranges: &RangeTable,
    target_layout: TargetLayout,
) -> bool {
    let mut value_to_instruction: HashMap<mir::Value, mir::Instruction> = HashMap::new();
    let mut aggregate_operands: HashMap<mir::Value, Vec<mir::Value>> = HashMap::new();
    let mut field_sets: HashMap<mir::Value, FieldSetEntry> = HashMap::new();
    let mut field_gets: HashMap<mir::Value, FieldGetEntry> = HashMap::new();

    // scan all blocks for aggregate definitions and value maps
    for &block_id in function.blocks() {
        // load the block instructions
        let block = tree.get(block_id);

        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            match instruction {
                mir::Instruction::Aggregate {
                    destination,
                    values,
                } => {
                    let destination = *destination;

                    let args = tree.get_values(*values);
                    aggregate_operands.insert(destination, args.to_vec());
                }
                mir::Instruction::FieldSet {
                    destination,
                    aggregate,
                    field: index,
                    value,
                }
                | mir::Instruction::ElementSet {
                    destination,
                    aggregate,
                    index,
                    value,
                } => {
                    field_sets.insert(
                        *destination,
                        FieldSetEntry {
                            aggregate: *aggregate,
                            index: *index,
                            value: *value,
                        },
                    );
                }
                mir::Instruction::FieldGet {
                    destination,
                    aggregate,
                    field: index,
                }
                | mir::Instruction::ElementGet {
                    destination,
                    aggregate,
                    index,
                } => {
                    field_gets.insert(
                        *destination,
                        FieldGetEntry {
                            aggregate: *aggregate,
                            index: *index,
                        },
                    );
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
    for &block_id in function.blocks() {
        // seed constants and ranges for the block
        let mut block_constants = constants.entry(block_id).clone();
        let block_ranges = ranges.exit(block_id).clone();
        let instruction_ids: Vec<_> = tree.get(block_id).instructions.clone();

        // scan instructions in test order
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
                        function,
                        tree,
                        &constant_lookup,
                        &block_ranges,
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
                        aggregate,
                        field: index,
                        ..
                    } => simplify_field_get(*aggregate, *index, &aggregate_operands, &field_sets),

                    mir::Instruction::ElementGet {
                        aggregate, index, ..
                    } => simplify_field_get(*aggregate, *index, &aggregate_operands, &field_sets),

                    mir::Instruction::FieldSet {
                        aggregate,
                        field: index,
                        value,
                        ..
                    } => simplify_field_set(*aggregate, *index, *value, &field_gets),

                    mir::Instruction::ElementSet {
                        aggregate,
                        index,
                        value,
                        ..
                    } => simplify_field_set(*aggregate, *index, *value, &field_gets),

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
                        tree.set(instruction_id, new_instruction);
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
                    target_layout,
                );
            }
        }
    }

    // resolve transitive substitutions
    let substitutions = resolve_substitution_chains(substitutions);

    // apply substitutions
    if !substitutions.is_empty() {
        for &block_id in function.blocks() {
            let instruction_ids: Vec<_> = tree.get(block_id).instructions.clone();
            for instruction_id in instruction_ids {
                // skip removed instructions
                if to_remove.contains(&instruction_id) {
                    continue;
                }

                // rewrite instruction operands
                let instruction = tree.get(instruction_id).clone();
                let new_instruction =
                    instruction_substitute_uses_in_tree(&instruction, &substitutions, tree);
                if new_instruction != instruction {
                    tree.set(instruction_id, new_instruction);
                    remap_instruction_memory_accesses(accesses, instruction_id, &substitutions);
                }
            }
        }

        let block_ids = function.blocks().to_vec();
        for block_id in block_ids {
            let block = tree.get(block_id).clone();
            let terminator_id = block.terminator;
            let terminator = tree.get(terminator_id).clone();
            let new_terminator = terminator_substitute_uses(tree, &terminator, &substitutions);
            let filtered: Vec<_> = block
                .instructions
                .iter()
                .copied()
                .filter(|id| !to_remove.contains(id))
                .collect();

            // replace the block when terminators or instructions change
            if new_terminator != terminator || filtered.len() != block.instructions.len() {
                tree.set(terminator_id, new_terminator);
                function.replace_block_instructions(block_id, filtered, tree);
            }
        }
    }

    changed
}

/// Try to simplify a binary operation using algebraic identities.
fn simplify_binary_operator(
    destination: mir::Value,
    operator: mir::BinaryOperator,
    left: mir::Value,
    right: mir::Value,
    function: &mir::Function,
    tree: &mir::Tree,
    constants: &ConstantLookup<'_>,
    ranges: &RangeState,
) -> Option<Simplification> {
    // read constant operands
    let left_const = constants.get(left);
    let right_const = constants.get(right);
    let left_const_ref = left_const.as_ref();
    let right_const_ref = right_const.as_ref();
    let ty = function.expect_value_type(left);
    let is_float = tree.get(ty).is_float(tree);

    // fold comparisons using range evidence
    if let Some(left_range) = ranges.get(left)
        && let Some(right_range) = ranges.get(right)
        && let Some(result) = left_range.compare_integer(operator, right_range)
    {
        return Some(Simplification::Constant(mir::Constant::Boolean {
            value: result,
        }));
    }

    // same operand simplifications (x op x)
    if left == right && !is_float {
        return simplify_same_binary_operand(destination, operator, left, left_const_ref);
    }

    // preserve IEEE floating point behavior
    if is_float {
        return None;
    }

    // identity and annihilator rules with constants
    match operator {
        // x + 0 = x, 0 + x = x
        mir::BinaryOperator::Add => {
            if constant_is_zero(right_const_ref) {
                return Some(Simplification::Substitute(left));
            }
            if constant_is_zero(left_const_ref) {
                return Some(Simplification::Substitute(right));
            }
        }

        // x - 0 = x
        mir::BinaryOperator::Subtract => {
            if constant_is_zero(right_const_ref) {
                return Some(Simplification::Substitute(left));
            }
        }

        // x * 0 = 0, 0 * x = 0, x * 1 = x, 1 * x = x
        mir::BinaryOperator::Multiply => {
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

        // x / 1 = x
        mir::BinaryOperator::Divide => {
            if constant_is_one(right_const_ref) {
                return Some(Simplification::Substitute(left));
            }
        }

        // x % 1 = 0
        mir::BinaryOperator::Remainder => {
            if constant_is_one(right_const_ref) {
                return Some(Simplification::Constant(constant_zero_like(
                    right_const_ref.unwrap(),
                )));
            }
        }

        // x & 0 = 0, 0 & x = 0
        mir::BinaryOperator::And => {
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
        mir::BinaryOperator::Or => {
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
        mir::BinaryOperator::Xor => {
            if constant_is_zero(right_const_ref) {
                return Some(Simplification::Substitute(left));
            }
            if constant_is_zero(left_const_ref) {
                return Some(Simplification::Substitute(right));
            }
        }

        // x << 0 = x, x >> 0 = x
        mir::BinaryOperator::ShiftLeft
        | mir::BinaryOperator::ShiftRight
        | mir::BinaryOperator::UnsignedShiftRight => {
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

        _ => {}
    }

    None
}

/// Simplify operations where both operands are the same value.
fn simplify_same_binary_operand(
    _destination: mir::Value,
    operator: mir::BinaryOperator,
    operand: mir::Value,
    constant: Option<&mir::Constant>,
) -> Option<Simplification> {
    // get type entry from constant if available for proper zero type
    let operand_const = constant;

    // simplify based on the operator
    match operator {
        // x - x = 0
        mir::BinaryOperator::Subtract => {
            // need type entry to create proper zero constant
            let c = operand_const?;
            Some(Simplification::Constant(constant_zero_like(c)))
        }

        // x ^ x = 0
        mir::BinaryOperator::Xor => {
            // need type entry to create proper zero constant
            let c = operand_const?;
            Some(Simplification::Constant(constant_zero_like(c)))
        }

        // x & x = x, x | x = x
        mir::BinaryOperator::And | mir::BinaryOperator::Or => {
            Some(Simplification::Substitute(operand))
        }

        // x == x = true
        mir::BinaryOperator::Equal
        | mir::BinaryOperator::LessEqual
        | mir::BinaryOperator::GreaterEqual => {
            Some(Simplification::Constant(mir::Constant::Boolean {
                value: true,
            }))
        }

        // x != x = false, x < x = false, x > x = false
        mir::BinaryOperator::NotEqual
        | mir::BinaryOperator::LessThan
        | mir::BinaryOperator::GreaterThan => {
            Some(Simplification::Constant(mir::Constant::Boolean {
                value: false,
            }))
        }

        _ => None,
    }
}

/// Lookup for constants from propagation and range analysis.
struct ConstantLookup<'a> {
    /// Constants derived from propagation.
    block_constants: &'a mir::ConstantState,
    /// Ranges for the block.
    ranges: &'a RangeState,
}

impl<'a> ConstantLookup<'a> {
    /// Create a new lookup for a block.
    fn new(block_constants: &'a mir::ConstantState, ranges: &'a RangeState) -> Self {
        Self {
            block_constants,
            ranges,
        }
    }

    /// Get a constant value for a SSA value.
    fn get(&self, value: mir::Value) -> Option<mir::Constant> {
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
    tree: &mir::Tree,
    block_constants: &mut ConstantState,
    ranges: &RangeState,
    target_layout: TargetLayout,
) {
    // skip instructions without destinations
    let Some(destination) = instruction.destination() else {
        return;
    };

    // capture a local constant lookup
    let constant_for = |value: mir::Value| -> Option<mir::Constant> {
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
                && let Some(result) = fold_binary(*operator, left, right)
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
                && let Some(result) = fold_unary(*operator, argument)
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
                && let Some(result) = fold_cast(
                    *operator,
                    argument,
                    *to_type,
                    target_layout.pointer_bits(),
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
            if let Some(mir::Constant::Boolean { value }) = condition {
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

/// Try to simplify a unary operation.
fn simplify_unary_operator(
    _destination: mir::Value,
    operator: mir::UnaryOperator,
    argument: mir::Value,
    value_to_instruction: &HashMap<mir::Value, mir::Instruction>,
) -> Option<Simplification> {
    // look for double negation: !!x = x
    if operator == mir::UnaryOperator::Not
        && let Some(mir::Instruction::Unary {
            operator: mir::UnaryOperator::Not,
            argument: inner,
            ..
        }) = value_to_instruction.get(&argument)
    {
        // !!x = x
        return Some(Simplification::Substitute(*inner));
    }

    None
}

/// Simplify a field.get instruction.
///
/// Handles extraction from aggregate constructions (struct/tuple/array) and
/// from field.set operations where the index matches or differs.
/// Uses iterative traversal with depth limit to avoid stack overflow.
fn simplify_field_get(
    aggregate: mir::Value,
    index: u32,
    aggregate_operands: &HashMap<mir::Value, Vec<mir::Value>>,
    field_sets: &HashMap<mir::Value, FieldSetEntry>,
) -> Option<Simplification> {
    let mut current = aggregate;

    // iterate through chained field.set operations
    for _ in 0..MAX_AGGREGATE_CHAIN_DEPTH {
        // check aggregate constructions first
        if let Some(operands) = aggregate_operands.get(&current) {
            return operands
                .get(index as usize)
                .map(|&op| Simplification::Substitute(op));
        }

        // check field.set: field.get(field.set(agg, i, val), j)
        if let Some(entry) = field_sets.get(&current) {
            // same index: return the inserted value
            if entry.index == index {
                return Some(Simplification::Substitute(entry.value));
            }

            // different index: continue through original aggregate
            current = entry.aggregate;
            continue;
        }

        // no more simplifications possible
        break;
    }

    None
}

/// Simplify a field.set instruction.
///
/// Detects identity pattern: `field.set(agg, i, field.get(agg, i))` → `agg`
/// Setting a field to its own current value is a no-op.
fn simplify_field_set(
    aggregate: mir::Value,
    index: u32,
    value: mir::Value,
    field_gets: &HashMap<mir::Value, FieldGetEntry>,
) -> Option<Simplification> {
    // check if value comes from a field.get on the same aggregate with same index
    if let Some(get_entry) = field_gets.get(&value)
        && get_entry.aggregate == aggregate
        && get_entry.index == index
    {
        return Some(Simplification::Substitute(aggregate));
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
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = add v0, v1
    return v2
}
"#;
        // v2 is substituted with v0, the add is removed
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// 0 + x simplifies to x.
    #[test]
    fn test_simplify_zero_add() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = add v1, v0
    return v2
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x * 1 simplifies to x.
    #[test]
    fn test_simplify_mul_one() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = mul v0, v1
    return v2
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x * 0 simplifies to 0.
    #[test]
    fn test_simplify_mul_zero() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = mul v0, v1
    return v2
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = 0
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x & 0 simplifies to 0.
    #[test]
    fn test_simplify_and_zero() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = and v0, v1
    return v2
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = 0
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x | 0 simplifies to x.
    #[test]
    fn test_simplify_or_zero() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = or v0, v1
    return v2
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x ^ 0 simplifies to x.
    #[test]
    fn test_simplify_xor_zero() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = xor v0, v1
    return v2
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x - x simplifies to 0 (when type entry is available).
    #[test]
    fn test_simplify_sub_self() {
        // use a constant so we have type entry
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 42
    v1: int32 = sub v0, v0
    return v1
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: int32 = 42
    v1: int32 = 0
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x ^ x simplifies to 0 (when type entry is available).
    #[test]
    fn test_simplify_xor_self() {
        // use a constant so we have type entry
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 42
    v1: int32 = xor v0, v0
    return v1
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: int32 = 42
    v1: int32 = 0
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x - x without type entry is not simplified (conservative).
    #[test]
    fn test_preserve_sub_self_without_type_entry() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = sub v0, v0
    return v1
}
"#;
        // no simplification because we don't know the type of v0

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_unchanged(input);
    }

    /// x & x simplifies to x.
    #[test]
    fn test_simplify_and_self() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = and v0, v0
    return v1
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x | x simplifies to x.
    #[test]
    fn test_simplify_or_self() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = or v0, v0
    return v1
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x == x simplifies to true.
    #[test]
    fn test_simplify_eq_self() {
        let input = r#"
function test(v0: int32): boolean {
entry(v0: int32):
    v1: boolean = eq v0, v0
    return v1
}
"#;
        let expected = r#"
function test(v0: int32): boolean {
entry(v0: int32):
    v1: boolean = true
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x != x simplifies to false.
    #[test]
    fn test_simplify_ne_self() {
        let input = r#"
function test(v0: int32): boolean {
entry(v0: int32):
    v1: boolean = ne v0, v0
    return v1
}
"#;
        let expected = r#"
function test(v0: int32): boolean {
entry(v0: int32):
    v1: boolean = false
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x < x simplifies to false.
    #[test]
    fn test_simplify_lt_self() {
        let input = r#"
function test(v0: int32): boolean {
entry(v0: int32):
    v1: boolean = lt v0, v0
    return v1
}
"#;
        let expected = r#"
function test(v0: int32): boolean {
entry(v0: int32):
    v1: boolean = false
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x <= x simplifies to true.
    #[test]
    fn test_simplify_le_self() {
        let input = r#"
function test(v0: int32): boolean {
entry(v0: int32):
    v1: boolean = le v0, v0
    return v1
}
"#;
        let expected = r#"
function test(v0: int32): boolean {
entry(v0: int32):
    v1: boolean = true
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x << 0 simplifies to x.
    #[test]
    fn test_simplify_shl_zero() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = shl v0, v1
    return v2
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// 0 << x simplifies to 0.
    #[test]
    fn test_simplify_zero_shl() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = shl v1, v0
    return v2
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = 0
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x / 1 simplifies to x.
    #[test]
    fn test_simplify_div_one() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = div v0, v1
    return v2
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x % 1 simplifies to 0.
    #[test]
    fn test_simplify_rem_one() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = rem v0, v1
    return v2
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = 0
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// Non-simplifiable operations are preserved.
    #[test]
    fn test_preserve_non_simplifiable() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 5
    v2: int32 = add v0, v1
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_unchanged(input);
    }

    /// Chained simplifications work correctly.
    #[test]
    fn test_apply_chained_simplifications() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = add v0, v1
    v3: int32 = 1
    v4: int32 = mul v2, v3
    return v4
}
"#;
        // v2 substituted to v0, v4 substituted to v2 (which is v0)
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v3: int32 = 1
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// Substitutions propagate through uses.
    #[test]
    fn test_propagate_substitutions() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = add v0, v1
    v3: int32 = 5
    v4: int32 = add v2, v3
    return v4
}
"#;
        // v2 -> v0, so v4 = add v0, v3
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v3: int32 = 5
    v4: int32 = add v0, v3
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x & all_ones simplifies to x.
    #[test]
    fn test_simplify_and_all_ones() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = -1
    v2: int32 = and v0, v1
    return v2
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = -1
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x | all_ones simplifies to all_ones.
    #[test]
    fn test_simplify_or_all_ones() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = -1
    v2: int32 = or v0, v1
    return v2
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = -1
    v2: int32 = -1
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x >> 0 (arithmetic) simplifies to x.
    #[test]
    fn test_simplify_ashr_zero() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = shr v0, v1
    return v2
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// x >> 0 (logical) simplifies to x.
    #[test]
    fn test_simplify_lshr_zero() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = ushr v0, v1
    return v2
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// Unsigned x / 1 simplifies to x.
    #[test]
    fn test_simplify_udiv_one() {
        let input = r#"
function test(v0: uint32): uint32 {
entry(v0: uint32):
    v1: uint32 = 1
    v2: uint32 = div v0, v1
    return v2
}
"#;
        let expected = r#"
function test(v0: uint32): uint32 {
entry(v0: uint32):
    v1: uint32 = 1
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// Unsigned x % 1 simplifies to 0.
    #[test]
    fn test_simplify_urem_one() {
        let input = r#"
function test(v0: uint32): uint32 {
entry(v0: uint32):
    v1: uint32 = 1
    v2: uint32 = rem v0, v1
    return v2
}
"#;
        let expected = r#"
function test(v0: uint32): uint32 {
entry(v0: uint32):
    v1: uint32 = 1
    v2: uint32 = 0
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// Float identity expressions preserve strict semantics.
    #[test]
    fn test_preserve_float_add_zero() {
        let input = r#"
function test(v0: float32): float32 {
entry(v0: float32):
    v1: float32 = 0
    v2: float32 = add v0, v1
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(input);
    }

    /// Double negation !!x simplifies to x.
    #[test]
    fn test_simplify_double_not() {
        let input = r#"
function test(v0: boolean): boolean {
entry(v0: boolean):
    v1: boolean = not v0
    v2: boolean = not v1
    return v2
}
"#;
        let expected = r#"
function test(v0: boolean): boolean {
entry(v0: boolean):
    v1: boolean = not v0
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// field.get(tuple(...), i) simplifies to the i-th operand.
    #[test]
    fn test_simplify_field_get_tuple() {
        let input = r#"
function test(v0: int32, v1: int64): int32 {
entry(v0: int32, v1: int64):
    v2: (int32, int64) = aggregate (v0, v1)
    v3: int32 = field.get v2, 0
    return v3
}
"#;
        let expected = r#"
function test(v0: int32, v1: int64): int32 {
entry(v0: int32, v1: int64):
    v2: (int32, int64) = aggregate (v0, v1)
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// field.get(tuple(...), 1) simplifies to the second operand.
    #[test]
    fn test_simplify_field_get_tuple_second() {
        let input = r#"
function test(v0: int32, v1: int64): int64 {
entry(v0: int32, v1: int64):
    v2: (int32, int64) = aggregate (v0, v1)
    v3: int64 = field.get v2, 1
    return v3
}
"#;
        let expected = r#"
function test(v0: int32, v1: int64): int64 {
entry(v0: int32, v1: int64):
    v2: (int32, int64) = aggregate (v0, v1)
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// field.get(struct(...), i) simplifies to the i-th field value.
    #[test]
    fn test_simplify_field_get_struct() {
        let input = r#"
type Point {
    int32;
    int32;
}

function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: Point = aggregate (v0, v1)
    v3: int32 = field.get v2, 0
    v4: int32 = field.get v2, 1
    v5: int32 = add v3, v4
    return v5
}
"#;
        // both field.get replaced with direct operands
        let expected = r#"
type Point {
    int32;
    int32;
}

function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: Point = aggregate (v0, v1)
    v5: int32 = add v0, v1
    return v5
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// element.get(aggregate(...), i) simplifies to the selected array element.
    #[test]
    fn test_simplify_element_get_array() {
        let input = r#"
function test(v0: int32, v1: int32, v2: int32): int32 {
entry(v0: int32, v1: int32, v2: int32):
    v3: [int32; 3] = aggregate (v0, v1, v2)
    v4: int64 = 1
    v5: int32 = element.get v3, 1
    return v5
}
"#;
        // element.get with static index 1 is replaced with v1
        let expected = r#"
function test(v0: int32, v1: int32, v2: int32): int32 {
entry(v0: int32, v1: int32, v2: int32):
    v3: [int32; 3] = aggregate (v0, v1, v2)
    v4: int64 = 1
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// field.get from non-aggregate source is not simplified.
    #[test]
    fn test_preserve_field_get_unknown_source() {
        let input = r#"
function test(v0: (int32, int32)): int32 {
entry(v0: (int32, int32)):
    v1: int32 = field.get v0, 0
    return v1
}
"#;
        // v0 is a parameter, not from tuple/struct instruction
        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_unchanged(input);
    }

    /// field.get(field.set(..., i, v), i) returns the inserted value.
    #[test]
    fn test_simplify_field_get_field_set_same_index() {
        let input = r#"
type Point {
    int32;
    int32;
}

function test(v0: Point, v1: int32): int32 {
entry(v0: Point, v1: int32):
    v2: Point = field.set v0, 0, v1
    v3: int32 = field.get v2, 0
    return v3
}
"#;
        // field.get of the just-set field returns the inserted value
        let expected = r#"
type Point {
    int32;
    int32;
}

function test(v0: Point, v1: int32): int32 {
entry(v0: Point, v1: int32):
    v2: Point = field.set v0, 0, v1
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// field.get(field.set(..., i, v), j) passes through to original when i != j.
    #[test]
    fn test_simplify_field_get_field_set_different_index() {
        let input = r#"
type Point {
    int32;
    int32;
}

function test(v0: int32, v1: int32, v2: int32): int32 {
entry(v0: int32, v1: int32, v2: int32):
    v3: Point = aggregate (v0, v1)
    v4: Point = field.set v3, 0, v2
    v5: int32 = field.get v4, 1
    return v5
}
"#;
        // field.get of different field passes through to original
        let expected = r#"
type Point {
    int32;
    int32;
}

function test(v0: int32, v1: int32, v2: int32): int32 {
entry(v0: int32, v1: int32, v2: int32):
    v3: Point = aggregate (v0, v1)
    v4: Point = field.set v3, 0, v2
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// Chained field.set operations are simplified correctly.
    #[test]
    fn test_simplify_field_get_chained_field_set() {
        let input = r#"
type Point {
    int32;
    int32;
}

function test(v0: Point, v1: int32, v2: int32): int32 {
entry(v0: Point, v1: int32, v2: int32):
    v3: Point = field.set v0, 0, v1
    v4: Point = field.set v3, 1, v2
    v5: int32 = field.get v4, 0
    v6: int32 = field.get v4, 1
    v7: int32 = add v5, v6
    return v7
}
"#;
        // v5 -> v1 (from first set), v6 -> v2 (from second set)
        let expected = r#"
type Point {
    int32;
    int32;
}

function test(v0: Point, v1: int32, v2: int32): int32 {
entry(v0: Point, v1: int32, v2: int32):
    v3: Point = field.set v0, 0, v1
    v4: Point = field.set v3, 1, v2
    v7: int32 = add v1, v2
    return v7
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// Element get after element set returns the inserted value at the same index.
    #[test]
    fn test_simplify_element_get_after_element_set_same_index() {
        let input = r#"
function test(v0: [int32; 3], v1: int32): int32 {
entry(v0: [int32; 3], v1: int32):
    v2: int64 = 1
    v3: [int32; 3] = element.set v0, 1, v1
    v4: int32 = element.get v3, 1
    return v4
}
"#;
        // read the value inserted at the same element index
        let expected = r#"
function test(v0: [int32; 3], v1: int32): int32 {
entry(v0: [int32; 3], v1: int32):
    v2: int64 = 1
    v3: [int32; 3] = element.set v0, 1, v1
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// Element get after element set reads the original value at another index.
    #[test]
    fn test_simplify_element_get_after_element_set_different_index() {
        let input = r#"
function test(v0: int32, v1: int32, v2: int32): int32 {
entry(v0: int32, v1: int32, v2: int32):
    v3: [int32; 2] = aggregate (v0, v1)
    v4: int64 = 0
    v5: [int32; 2] = element.set v3, 0, v2
    v6: int64 = 1
    v7: int32 = element.get v5, 1
    return v7
}
"#;
        // read the original value from the untouched element index
        let expected = r#"
function test(v0: int32, v1: int32, v2: int32): int32 {
entry(v0: int32, v1: int32, v2: int32):
    v3: [int32; 2] = aggregate (v0, v1)
    v4: int64 = 0
    v5: [int32; 2] = element.set v3, 0, v2
    v6: int64 = 1
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// Same field overwritten twice: get should return second value.
    #[test]
    fn test_simplify_field_set_overwrite_same_index() {
        let input = r#"
type Point {
    int32;
    int32;
}

function test(v0: Point, v1: int32, v2: int32): int32 {
entry(v0: Point, v1: int32, v2: int32):
    v3: Point = field.set v0, 0, v1
    v4: Point = field.set v3, 0, v2
    v5: int32 = field.get v4, 0
    return v5
}
"#;
        // second set overwrites first, get returns v2
        let expected = r#"
type Point {
    int32;
    int32;
}

function test(v0: Point, v1: int32, v2: int32): int32 {
entry(v0: Point, v1: int32, v2: int32):
    v3: Point = field.set v0, 0, v1
    v4: Point = field.set v3, 0, v2
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// Tuple with field.set also works.
    #[test]
    fn test_simplify_field_get_field_set_tuple() {
        let input = r#"
function test(v0: (int32, int64), v1: int32): int32 {
entry(v0: (int32, int64), v1: int32):
    v2: (int32, int64) = field.set v0, 0, v1
    v3: int32 = field.get v2, 0
    return v3
}
"#;
        let expected = r#"
function test(v0: (int32, int64), v1: int32): int32 {
entry(v0: (int32, int64), v1: int32):
    v2: (int32, int64) = field.set v0, 0, v1
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// Chained element sets at one index retain the second value.
    #[test]
    fn test_simplify_element_set_overwrite_same_index() {
        let input = r#"
function test(v0: [int32; 2], v1: int32, v2: int32): int32 {
entry(v0: [int32; 2], v1: int32, v2: int32):
    v3: int64 = 0
    v4: [int32; 2] = element.set v0, 0, v1
    v5: [int32; 2] = element.set v4, 0, v2
    v6: int32 = element.get v5, 0
    return v6
}
"#;
        // the second update at index zero overwrites the first
        let expected = r#"
function test(v0: [int32; 2], v1: int32, v2: int32): int32 {
entry(v0: [int32; 2], v1: int32, v2: int32):
    v3: int64 = 0
    v4: [int32; 2] = element.set v0, 0, v1
    v5: [int32; 2] = element.set v4, 0, v2
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// field.get from a parameter source is not simplified.
    #[test]
    fn test_preserve_field_get_parameter_source() {
        let input = r#"
type Point {
    int32;
    int32;
}

function test(v0: Point): int32 {
entry(v0: Point):
    v1: int32 = field.get v0, 1
    return v1
}
"#;
        // aggregate source is a parameter, not a known constructor
        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_unchanged(input);
    }

    /// Out-of-bounds field.get index is not simplified.
    #[test]
    fn test_preserve_field_get_out_of_bounds() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: [int32; 2] = aggregate (v0, v1)
    v3: int64 = 10
    v4: int32 = field.get v2, 10
    return v4
}
"#;
        // index 10 is out of bounds for 2-element array
        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_unchanged(input);
    }

    /// Identity field.set: field.set(agg, i, field.get(agg, i)) → agg
    #[test]
    fn test_identity_field_set() {
        let input = r#"
type Point {
    int32;
    int32;
}

function test(v0: Point): Point {
entry(v0: Point):
    v1: int32 = field.get v0, 0
    v2: Point = field.set v0, 0, v1
    return v2
}
"#;
        let expected = r#"
type Point {
    int32;
    int32;
}

function test(v0: Point): Point {
entry(v0: Point):
    v1: int32 = field.get v0, 0
    return v0
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// Element set preserves an array when replacing an element with itself.
    #[test]
    fn test_identity_element_set() {
        let input = r#"
function test(v0: [int32; 3]): [int32; 3] {
entry(v0: [int32; 3]):
    v1: int64 = 1
    v2: int32 = element.get v0, 1
    v3: [int32; 3] = element.set v0, 1, v2
    return v3
}
"#;
        let expected = r#"
function test(v0: [int32; 3]): [int32; 3] {
entry(v0: [int32; 3]):
    v1: int64 = 1
    v2: int32 = element.get v0, 1
    return v0
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_output(expected);
    }

    /// Non-identity field.set: different index should not simplify.
    #[test]
    fn test_non_identity_field_set_different_index() {
        let input = r#"
type Point {
    int32;
    int32;
}

function test(v0: Point): Point {
entry(v0: Point):
    v1: int32 = field.get v0, 0
    v2: Point = field.set v0, 1, v1
    return v2
}
"#;
        // get from index 0, set at index 1 - not identity
        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_unchanged(input);
    }

    /// Non-identity field.set: different aggregate, should not simplify.
    #[test]
    fn test_non_identity_field_set_different_aggregate() {
        let input = r#"
type Point {
    int32;
    int32;
}

function test(v0: Point, v1: Point): Point {
entry(v0: Point, v1: Point):
    v2: int32 = field.get v0, 0
    v3: Point = field.set v1, 0, v2
    return v3
}
"#;
        // get from v0, set on v1 - not identity
        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_unchanged(input);
    }

    /// Element set from another index is not an identity update.
    #[test]
    fn test_non_identity_element_set_different_index() {
        let input = r#"
function test(v0: [int32; 3]): [int32; 3] {
entry(v0: [int32; 3]):
    v1: int32 = element.get v0, 0
    v2: [int32; 3] = element.set v0, 1, v1
    return v2
}
"#;
        // get from index 0, set at index 1 - not identity
        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_unchanged(input);
    }

    /// Element set from another array is not an identity update.
    #[test]
    fn test_non_identity_element_set_different_array() {
        let input = r#"
function test(v0: [int32; 3], v1: [int32; 3]): [int32; 3] {
entry(v0: [int32; 3], v1: [int32; 3]):
    v2: int64 = 0
    v3: int32 = element.get v0, 0
    v4: [int32; 3] = element.set v1, 0, v3
    return v4
}
"#;
        // get from v0, set on v1 - not identity
        let mut test = TestProgram::new(input);
        test.run_pass(&CombineInstructions);
        test.assert_unchanged(input);
    }
}
