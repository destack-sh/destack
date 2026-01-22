use std::collections::HashMap;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;
use mir::{BinaryOperator, Constant, UnaryOperator};

use destack_workspace::FloatMathPolicy;

use crate::ConstantMap;
use crate::optimize::analyses::{ConstantPropagation, RangeAnalysis, RangeMap};
use crate::optimize::{
    AnalysisPreservation, FunctionPass, PipelineContext, TypeContext, constant_all_ones_like,
    constant_is_all_ones, constant_is_float_one, constant_is_float_zero, constant_is_one,
    constant_is_zero, constant_zero_like, evaluate_integer_range_comparison,
    instruction_substitute_uses, resolve_substitution_chains, terminator_substitute_uses,
};

/// Maximum recursion depth for chained field.set/element.set simplification.
const MAX_AGGREGATE_CHAIN_DEPTH: usize = 64;

declare_pass! {
    /// Algebraic simplification of instructions.
    ///
    /// Applies identity and annihilator rules to simplify expressions:
    /// - `x + 0 = x`, `x * 1 = x`, `x * 0 = 0`
    /// - `x & 0 = 0`, `x | 0 = x`, `x ^ 0 = x`
    /// - `x - x = 0`, `x ^ x = 0`, `x & x = x`
    /// - `x == x = true`, `x != x = false`
    /// - `!!x = x`
    /// - `field.get(struct/tuple/array(...), i)` = operand i
    /// - `field.get(field.set(..., i, v), i)` = v
    /// - `field.get(field.set(..., i, v), j)` = field.get(original, j) when i != j
    /// - `element.get(array(...), const_i)` = operand i
    /// - `element.get(element.set(..., i, v), i)` = v (when i is constant)
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

/// Tracks a field.set instruction for simplification.
struct FieldSetEntry {
    /// The original aggregate being modified.
    aggregate: mir::Value,
    /// The field index being updated.
    index: u32,
    /// The new value inserted at the field.
    value: mir::Value,
}

/// Tracks an element.set instruction for simplification.
struct ElementSetEntry {
    /// The original array being modified.
    array: mir::Value,
    /// The index value (may be constant or dynamic).
    index: mir::Value,
    /// The new value inserted at the index.
    value: mir::Value,
}

/// Tracks a field.get instruction for identity detection.
struct FieldGetEntry {
    /// The aggregate being extracted from.
    aggregate: mir::Value,
    /// The field index being extracted.
    index: u32,
}

/// Tracks an element.get instruction for identity detection.
struct ElementGetEntry {
    /// The array being extracted from.
    array: mir::Value,
    /// The index value (may be constant or dynamic).
    index: mir::Value,
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
    let mut value_to_instruction: HashMap<mir::Value, mir::Instruction> = HashMap::new();
    let mut aggregate_operands: HashMap<mir::Value, Vec<mir::Value>> = HashMap::new();
    let mut field_sets: HashMap<mir::Value, FieldSetEntry> = HashMap::new();
    let mut element_sets: HashMap<mir::Value, ElementSetEntry> = HashMap::new();
    let mut field_gets: HashMap<mir::Value, FieldGetEntry> = HashMap::new();
    let mut element_gets: HashMap<mir::Value, ElementGetEntry> = HashMap::new();

    // scan all blocks for aggregate definitions and value maps
    for &block_id in &function.blocks {
        // load the block instructions
        let block = tree.get(block_id);

        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
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
                mir::Instruction::FieldSet {
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
                mir::Instruction::ElementSet {
                    destination,
                    array,
                    index,
                    value,
                } => {
                    element_sets.insert(
                        *destination,
                        ElementSetEntry {
                            array: *array,
                            index: *index,
                            value: *value,
                        },
                    );
                }
                mir::Instruction::FieldGet {
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
                mir::Instruction::ElementGet {
                    destination,
                    array,
                    index,
                } => {
                    element_gets.insert(
                        *destination,
                        ElementGetEntry {
                            array: *array,
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
    for &block_id in &function.blocks {
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
                    } => simplify_field_get(*aggregate, *index, &aggregate_operands, &field_sets),

                    mir::Instruction::ElementGet { array, index, .. } => simplify_element_get(
                        *array,
                        *index,
                        &aggregate_operands,
                        &element_sets,
                        &constant_lookup,
                    ),

                    mir::Instruction::FieldSet {
                        aggregate,
                        index,
                        value,
                        ..
                    } => simplify_field_set(*aggregate, *index, *value, &field_gets),

                    mir::Instruction::ElementSet {
                        array,
                        index,
                        value,
                        ..
                    } => simplify_element_set(
                        *array,
                        *index,
                        *value,
                        &element_gets,
                        &constant_lookup,
                    ),

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
    if let Some(left_range) = ranges.get(left)
        && let Some(right_range) = ranges.get(right)
        && let Some(result) = evaluate_integer_range_comparison(operator, left_range, right_range)
    {
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
    // get type entry from constant if available for proper zero type
    let operand_const = constant;

    // simplify based on the operator
    match operator {
        // x - x = 0
        BinaryOperator::Subtract => {
            // need type entry to create proper zero constant
            let c = operand_const?;
            Some(Simplification::Constant(constant_zero_like(c)))
        }

        // x ^ x = 0
        BinaryOperator::Xor => {
            // need type entry to create proper zero constant
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
    block_constants: &mut ConstantMap,
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

/// Convert a constant to a non-negative index, rejecting negative values.
fn constant_to_index(constant: &Constant) -> Option<usize> {
    match constant {
        Constant::Int { value, .. } => {
            // reject negative indices
            if *value < 0 {
                return None;
            }
            usize::try_from(*value).ok()
        }
        Constant::UInt { value, .. } => usize::try_from(*value).ok(),
        _ => None,
    }
}

/// Simplify an element.get instruction.
///
/// Handles extraction from array constructions and from element.set operations
/// when indices are known non-negative constants.
/// Uses iterative traversal with depth limit to avoid stack overflow.
fn simplify_element_get(
    array: mir::Value,
    index: mir::Value,
    aggregate_operands: &HashMap<mir::Value, Vec<mir::Value>>,
    element_sets: &HashMap<mir::Value, ElementSetEntry>,
    constants: &ConstantLookup<'_>,
) -> Option<Simplification> {
    // resolve the index to a non-negative constant value
    let index_value = constants.get(index).as_ref().and_then(constant_to_index);

    let mut current = array;

    // iterate through chained element.set operations
    for _ in 0..MAX_AGGREGATE_CHAIN_DEPTH {
        // check array constructions
        if let Some(operands) = aggregate_operands.get(&current)
            && let Some(idx) = index_value
        {
            return operands.get(idx).map(|&op| Simplification::Substitute(op));
        }

        // check element.set: element.get(element.set(arr, i, val), j)
        if let Some(entry) = element_sets.get(&current) {
            // need both indices to be non-negative constants for comparison
            let set_index_value = constants
                .get(entry.index)
                .as_ref()
                .and_then(constant_to_index);

            if let (Some(get_idx), Some(set_idx)) = (index_value, set_index_value) {
                // same index: return the inserted value
                if get_idx == set_idx {
                    return Some(Simplification::Substitute(entry.value));
                }

                // different index: continue through original array
                current = entry.array;
                continue;
            }
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

/// Simplify an element.set instruction.
///
/// Detects identity pattern: `element.set(arr, i, element.get(arr, i))` → `arr`
/// Setting an element to its own current value is a no-op.
/// Requires constant indices for comparison.
fn simplify_element_set(
    array: mir::Value,
    index: mir::Value,
    value: mir::Value,
    element_gets: &HashMap<mir::Value, ElementGetEntry>,
    constants: &ConstantLookup<'_>,
) -> Option<Simplification> {
    // check if value comes from an element.get on the same array
    if let Some(get_entry) = element_gets.get(&value)
        && get_entry.array == array
    {
        // need both indices to be constants for comparison
        let set_idx = constants.get(index).as_ref().and_then(constant_to_index);
        let get_idx = constants
            .get(get_entry.index)
            .as_ref()
            .and_then(constant_to_index);

        if let (Some(s), Some(g)) = (set_idx, get_idx)
            && s == g
        {
            return Some(Simplification::Substitute(array));
        }
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
    }

    /// x - x simplifies to 0 (when type entry is available).
    #[test]
    fn test_simplify_sub_self() {
        // use a constant so we have type entry
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
    }

    /// x ^ x simplifies to 0 (when type entry is available).
    #[test]
    fn test_simplify_xor_self() {
        // use a constant so we have type entry
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
    }

    /// x - x without type entry is not simplified (conservative).
    #[test]
    fn test_preserve_sub_self_without_type_entry() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = isub v0, v0
    return v1
}"#;
        // no simplification because we don't know the type of v0

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_unchanged(input);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_unchanged(input);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(
            &InstructionCombine,
            PipelineOptions {
                float_math: FloatMathPolicy::Fast,
                ..PipelineOptions::default()
            },
        );
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(
            &InstructionCombine,
            PipelineOptions {
                float_math: FloatMathPolicy::Fast,
                ..PipelineOptions::default()
            },
        );
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(
            &InstructionCombine,
            PipelineOptions {
                float_math: FloatMathPolicy::Fast,
                ..PipelineOptions::default()
            },
        );
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(
            &InstructionCombine,
            PipelineOptions {
                float_math: FloatMathPolicy::Fast,
                ..PipelineOptions::default()
            },
        );
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
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
        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_unchanged(input);
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
        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_unchanged(input);
    }

    /// field.get(field.set(..., i, v), i) returns the inserted value.
    #[test]
    fn test_simplify_field_get_field_set_same_index() {
        let input = r#"type @Point = { i32, i32 }
function @test(v0: @Point, v1: i32) -> i32 {
block0(v0: @Point, v1: i32):
    v2 = field.set v0, 0, v1
    v3 = field.get v2, 0
    return v3
}"#;
        // field.get of the just-set field returns the inserted value
        let expected = r#"type @Point = { i32, i32 }
function @test(v0: @Point, v1: i32) -> i32 {
block0(v0: @Point, v1: i32):
    v2 = field.set v0, 0, v1
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
    }

    /// field.get(field.set(..., i, v), j) passes through to original when i != j.
    #[test]
    fn test_simplify_field_get_field_set_different_index() {
        let input = r#"type @Point = { i32, i32 }
function @test(v0: i32, v1: i32, v2: i32) -> i32 {
block0(v0: i32, v1: i32, v2: i32):
    v3 = struct @Point (v0, v1)
    v4 = field.set v3, 0, v2
    v5 = field.get v4, 1
    return v5
}"#;
        // field.get of different field passes through to original
        let expected = r#"type @Point = { i32, i32 }
function @test(v0: i32, v1: i32, v2: i32) -> i32 {
block0(v0: i32, v1: i32, v2: i32):
    v3 = struct @Point (v0, v1)
    v4 = field.set v3, 0, v2
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
    }

    /// Chained field.set operations are simplified correctly.
    #[test]
    fn test_simplify_field_get_chained_field_set() {
        let input = r#"type @Point = { i32, i32 }
function @test(v0: @Point, v1: i32, v2: i32) -> i32 {
block0(v0: @Point, v1: i32, v2: i32):
    v3 = field.set v0, 0, v1
    v4 = field.set v3, 1, v2
    v5 = field.get v4, 0
    v6 = field.get v4, 1
    v7 = iadd v5, v6
    return v7
}"#;
        // v5 -> v1 (from first set), v6 -> v2 (from second set)
        let expected = r#"type @Point = { i32, i32 }
function @test(v0: @Point, v1: i32, v2: i32) -> i32 {
block0(v0: @Point, v1: i32, v2: i32):
    v3 = field.set v0, 0, v1
    v4 = field.set v3, 1, v2
    v7 = iadd v1, v2
    return v7
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
    }

    /// element.get(element.set(..., i, v), i) returns the inserted value.
    #[test]
    fn test_simplify_element_get_element_set_same_index() {
        let input = r#"function @test(v0: [i32; 3], v1: i32) -> i32 {
block0(v0: [i32; 3], v1: i32):
    v2 = iconst 1i64
    v3 = element.set v0, v2, v1
    v4 = element.get v3, v2
    return v4
}"#;
        // element.get of the just-set element returns the inserted value
        let expected = r#"function @test(v0: [i32; 3], v1: i32) -> i32 {
block0(v0: [i32; 3], v1: i32):
    v2 = iconst 1i64
    v3 = element.set v0, v2, v1
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
    }

    /// element.get(element.set(..., i, v), j) with different constant indices.
    #[test]
    fn test_simplify_element_get_element_set_different_index() {
        let input = r#"function @test(v0: i32, v1: i32, v2: i32) -> i32 {
block0(v0: i32, v1: i32, v2: i32):
    v3 = array [i32; 2] (v0, v1)
    v4 = iconst 0i64
    v5 = element.set v3, v4, v2
    v6 = iconst 1i64
    v7 = element.get v5, v6
    return v7
}"#;
        // element.get of different index passes through to original
        let expected = r#"function @test(v0: i32, v1: i32, v2: i32) -> i32 {
block0(v0: i32, v1: i32, v2: i32):
    v3 = array [i32; 2] (v0, v1)
    v4 = iconst 0i64
    v5 = element.set v3, v4, v2
    v6 = iconst 1i64
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
    }

    /// element.get/set with non-constant indices is not simplified.
    #[test]
    fn test_preserve_element_get_element_set_dynamic_index() {
        let input = r#"function @test(v0: [i32; 3], v1: i32, v2: i64, v3: i64) -> i32 {
block0(v0: [i32; 3], v1: i32, v2: i64, v3: i64):
    v4 = element.set v0, v2, v1
    v5 = element.get v4, v3
    return v5
}"#;
        // dynamic indices cannot be compared
        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_unchanged(input);
    }

    /// Negative array index is not simplified (would be out-of-bounds).
    #[test]
    fn test_preserve_element_get_negative_index() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = array [i32; 2] (v0, v1)
    v3 = iconst -1i64
    v4 = element.get v2, v3
    return v4
}"#;
        // negative index must not be optimized
        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_unchanged(input);
    }

    /// Same field overwritten twice: get should return second value.
    #[test]
    fn test_simplify_field_set_overwrite_same_index() {
        let input = r#"type @Point = { i32, i32 }
function @test(v0: @Point, v1: i32, v2: i32) -> i32 {
block0(v0: @Point, v1: i32, v2: i32):
    v3 = field.set v0, 0, v1
    v4 = field.set v3, 0, v2
    v5 = field.get v4, 0
    return v5
}"#;
        // second set overwrites first, get returns v2
        let expected = r#"type @Point = { i32, i32 }
function @test(v0: @Point, v1: i32, v2: i32) -> i32 {
block0(v0: @Point, v1: i32, v2: i32):
    v3 = field.set v0, 0, v1
    v4 = field.set v3, 0, v2
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
    }

    /// Tuple with field.set also works.
    #[test]
    fn test_simplify_field_get_field_set_tuple() {
        let input = r#"function @test(v0: (i32, i64), v1: i32) -> i32 {
block0(v0: (i32, i64), v1: i32):
    v2 = field.set v0, 0, v1
    v3 = field.get v2, 0
    return v3
}"#;
        let expected = r#"function @test(v0: (i32, i64), v1: i32) -> i32 {
block0(v0: (i32, i64), v1: i32):
    v2 = field.set v0, 0, v1
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
    }

    /// Chained element.set with same index: second value wins.
    #[test]
    fn test_simplify_element_set_overwrite_same_index() {
        let input = r#"function @test(v0: [i32; 2], v1: i32, v2: i32) -> i32 {
block0(v0: [i32; 2], v1: i32, v2: i32):
    v3 = iconst 0i64
    v4 = element.set v0, v3, v1
    v5 = element.set v4, v3, v2
    v6 = element.get v5, v3
    return v6
}"#;
        // second set at index 0 overwrites first, get returns v2
        let expected = r#"function @test(v0: [i32; 2], v1: i32, v2: i32) -> i32 {
block0(v0: [i32; 2], v1: i32, v2: i32):
    v3 = iconst 0i64
    v4 = element.set v0, v3, v1
    v5 = element.set v4, v3, v2
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
    }

    /// element.get(element.set) with negative set index is not simplified.
    #[test]
    fn test_preserve_element_set_negative_index() {
        let input = r#"function @test(v0: [i32; 2], v1: i32) -> i32 {
block0(v0: [i32; 2], v1: i32):
    v2 = iconst -1i64
    v3 = element.set v0, v2, v1
    v4 = iconst 0i64
    v5 = element.get v3, v4
    return v5
}"#;
        // set index is negative, cannot safely compare
        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_unchanged(input);
    }

    /// Out-of-bounds field.get index is not simplified.
    #[test]
    fn test_preserve_field_get_out_of_bounds() {
        let input = r#"type @Point = { i32, i32 }
function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = struct @Point (v0, v1)
    v3 = field.get v2, 5
    return v3
}"#;
        // index 5 is out of bounds for 2-field struct
        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_unchanged(input);
    }

    /// Out-of-bounds element.get index is not simplified.
    #[test]
    fn test_preserve_element_get_out_of_bounds() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = array [i32; 2] (v0, v1)
    v3 = iconst 10i64
    v4 = element.get v2, v3
    return v4
}"#;
        // index 10 is out of bounds for 2-element array
        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_unchanged(input);
    }

    /// Identity field.set: field.set(agg, i, field.get(agg, i)) → agg
    #[test]
    fn test_identity_field_set() {
        let input = r#"type @Point = { i32, i32 }
function @test(v0: @Point) -> @Point {
block0(v0: @Point):
    v1 = field.get v0, 0
    v2 = field.set v0, 0, v1
    return v2
}"#;
        let expected = r#"type @Point = { i32, i32 }
function @test(v0: @Point) -> @Point {
block0(v0: @Point):
    v1 = field.get v0, 0
    return v0
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
    }

    /// Identity element.set: element.set(arr, i, element.get(arr, i)) → arr
    #[test]
    fn test_identity_element_set() {
        let input = r#"function @test(v0: [i32; 3]) -> [i32; 3] {
block0(v0: [i32; 3]):
    v1 = iconst 1i64
    v2 = element.get v0, v1
    v3 = element.set v0, v1, v2
    return v3
}"#;
        let expected = r#"function @test(v0: [i32; 3]) -> [i32; 3] {
block0(v0: [i32; 3]):
    v1 = iconst 1i64
    v2 = element.get v0, v1
    return v0
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_output(expected);
    }

    /// Non-identity field.set: different index, should not simplify.
    #[test]
    fn test_non_identity_field_set_different_index() {
        let input = r#"type @Point = { i32, i32 }
function @test(v0: @Point) -> @Point {
block0(v0: @Point):
    v1 = field.get v0, 0
    v2 = field.set v0, 1, v1
    return v2
}"#;
        // get from index 0, set at index 1 - not identity
        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_unchanged(input);
    }

    /// Non-identity field.set: different aggregate, should not simplify.
    #[test]
    fn test_non_identity_field_set_different_aggregate() {
        let input = r#"type @Point = { i32, i32 }
function @test(v0: @Point, v1: @Point) -> @Point {
block0(v0: @Point, v1: @Point):
    v2 = field.get v0, 0
    v3 = field.set v1, 0, v2
    return v3
}"#;
        // get from v0, set on v1 - not identity
        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_unchanged(input);
    }

    /// Non-identity element.set: different index, should not simplify.
    #[test]
    fn test_non_identity_element_set_different_index() {
        let input = r#"function @test(v0: [i32; 3]) -> [i32; 3] {
block0(v0: [i32; 3]):
    v1 = iconst 0i64
    v2 = iconst 1i64
    v3 = element.get v0, v1
    v4 = element.set v0, v2, v3
    return v4
}"#;
        // get from index 0, set at index 1 - not identity
        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_unchanged(input);
    }

    /// Non-identity element.set: different array, should not simplify.
    #[test]
    fn test_non_identity_element_set_different_array() {
        let input = r#"function @test(v0: [i32; 3], v1: [i32; 3]) -> [i32; 3] {
block0(v0: [i32; 3], v1: [i32; 3]):
    v2 = iconst 0i64
    v3 = element.get v0, v2
    v4 = element.set v1, v2, v3
    return v4
}"#;
        // get from v0, set on v1 - not identity
        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_unchanged(input);
    }

    /// Non-identity element.set: dynamic indices, cannot prove equal.
    #[test]
    fn test_non_identity_element_set_dynamic_index() {
        let input = r#"function @test(v0: [i32; 3], v1: i64, v2: i64) -> [i32; 3] {
block0(v0: [i32; 3], v1: i64, v2: i64):
    v3 = element.get v0, v1
    v4 = element.set v0, v2, v3
    return v4
}"#;
        // dynamic indices v1 and v2 - cannot prove equal
        let mut test = TestProgram::new(input);
        test.run_pass(&InstructionCombine);
        test.assert_unchanged(input);
    }
}
