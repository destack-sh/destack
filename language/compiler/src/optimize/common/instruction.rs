use std::collections::{HashMap, HashSet};

use crate::optimize::common::terminator_substitute_uses;
use destack_mir as mir;
use mir::Instruction;

/// Check if an instruction is pure (result depends only on operands).
///
/// A pure instruction has no side effects AND does not read mutable state.
/// This is stricter than `!instruction_has_side_effects`.
/// `Load` and `LocalGet` have no side effects so they can be removed if unused.
/// They still read mutable state, so they cannot be hoisted out of a loop.
///
/// Use this for LICM, code motion, and speculation optimizations.
pub fn instruction_is_pure(instruction: &Instruction) -> bool {
    // classify instructions by purity
    match instruction {
        // pure computations
        Instruction::Const { .. }
        | Instruction::Binary { .. }
        | Instruction::Unary { .. }
        | Instruction::Cast { .. }
        | Instruction::Select { .. }
        | Instruction::Assume { .. } => true,

        // pure aggregate operations (value semantics)
        Instruction::Struct { .. }
        | Instruction::Tuple { .. }
        | Instruction::Array { .. }
        | Instruction::FieldGet { .. }
        | Instruction::FieldAddr { .. }
        | Instruction::FieldSet { .. }
        | Instruction::ElementGet { .. }
        | Instruction::ElementAddr { .. }
        | Instruction::ElementSet { .. } => true,

        // immutable global references
        Instruction::GlobalConst { .. } | Instruction::GlobalAddr { .. } => true,

        // reads mutable state, not speculatable
        Instruction::LocalGet { .. } | Instruction::Load { .. } => false,

        // writes have side effects
        Instruction::LocalSet { .. } | Instruction::Store { .. } => false,

        // drops run destructors / deallocate
        Instruction::RawDrop { .. } | Instruction::StackDrop { .. } => false,

        // calls may have side effects
        Instruction::Call { .. } | Instruction::CallIndirect { .. } => false,

        // allocations have side effects
        Instruction::ManagedAlloc { .. }
        | Instruction::ManagedAllocArray { .. }
        | Instruction::RawAlloc { .. }
        | Instruction::StackAlloc { .. } => false,

        // deallocation has side effects
        Instruction::RawFree { .. } => false,

        // intrinsics may have side effects
        Instruction::Intrinsic { .. } => false,
    }
}

/// Check if an instruction can be speculated without trapping.
///
/// This is a stricter predicate than purity: some pure operations may trap.
pub fn instruction_is_speculatable(instruction: &Instruction) -> bool {
    // classify instructions by speculative safety
    match instruction {
        // assumptions must not be speculated across control flow
        Instruction::Assume { .. } => false,
        // float to integer casts can trap on NaN or out of range inputs
        Instruction::Cast {
            operator: mir::CastOperator::FloatToSignedInt | mir::CastOperator::FloatToUnsignedInt,
            ..
        } => false,
        // integer division and remainder may trap
        Instruction::Binary {
            operator:
                mir::BinaryOperator::SignedDivide
                | mir::BinaryOperator::UnsignedDivide
                | mir::BinaryOperator::SignedRemainder
                | mir::BinaryOperator::UnsignedRemainder,
            ..
        } => false,
        _ => instruction_is_pure(instruction),
    }
}

/// Check if an instruction has side effects and cannot be removed even if unused.
///
/// Instructions with side effects must be preserved regardless of whether their
/// result is used. This includes stores, calls, allocations, and drops.
pub fn instruction_has_side_effects(instruction: &Instruction) -> bool {
    // classify instructions by side effects
    match instruction {
        // pure computations, no side effects
        Instruction::Const { .. }
        | Instruction::Binary { .. }
        | Instruction::Unary { .. }
        | Instruction::Cast { .. }
        | Instruction::Select { .. }
        | Instruction::Struct { .. }
        | Instruction::Tuple { .. }
        | Instruction::Array { .. }
        | Instruction::FieldGet { .. }
        | Instruction::FieldAddr { .. }
        | Instruction::ElementGet { .. }
        | Instruction::ElementAddr { .. }
        | Instruction::GlobalConst { .. }
        | Instruction::GlobalAddr { .. }
        | Instruction::Assume { .. } => false,

        // memory reads are pure (assuming no volatile)
        Instruction::LocalGet { .. } | Instruction::Load { .. } => false,

        // memory writes have side effects
        Instruction::LocalSet { .. } | Instruction::Store { .. } => true,

        // aggregate updates create new values, but FieldSet/ElementSet don't have
        // side effects if the result is unused (they produce new values, not mutate)
        Instruction::FieldSet { .. } | Instruction::ElementSet { .. } => false,

        // drops have side effects (deallocate, run destructors)
        Instruction::RawDrop { .. } | Instruction::StackDrop { .. } => true,

        // calls may have side effects
        Instruction::Call { .. } | Instruction::CallIndirect { .. } => true,

        // allocations have side effects (memory allocation)
        Instruction::ManagedAlloc { .. }
        | Instruction::ManagedAllocArray { .. }
        | Instruction::RawAlloc { .. }
        | Instruction::StackAlloc { .. } => true,

        // deallocation has side effects
        Instruction::RawFree { .. } => true,

        // intrinsics may have side effects (check purity for safe removal)
        Instruction::Intrinsic { intrinsic, .. } => {
            !intrinsic.is_pure() || matches!(intrinsic, mir::Intrinsic::BlackBox)
        }
    }
}

/// Check if an instruction reads from memory.
///
/// Memory reads include loads from pointers and gets from locals. These
/// instructions don't have side effects but read mutable state, so they
/// cannot be freely reordered past memory writes.
pub fn instruction_is_memory_read(instruction: &Instruction) -> bool {
    // identify instructions that read mutable memory
    matches!(
        instruction,
        Instruction::Load { .. } | Instruction::LocalGet { .. }
    )
}

/// Check if an instruction may write memory or have other side effects that could affect memory.
///
/// This is used to determine if it's safe to sink loads past an instruction.
/// Any instruction that writes memory, calls functions (which might write memory),
/// or performs allocations/deallocations is considered to affect memory.
pub fn instruction_may_affect_memory(instruction: &Instruction) -> bool {
    // identify instructions that can modify memory state
    matches!(
        instruction,
        Instruction::Store { .. }
            | Instruction::LocalSet { .. }
            | Instruction::Call { .. }
            | Instruction::CallIndirect { .. }
            | Instruction::Intrinsic { .. }
            | Instruction::ManagedAlloc { .. }
            | Instruction::ManagedAllocArray { .. }
            | Instruction::RawAlloc { .. }
            | Instruction::RawFree { .. }
            | Instruction::RawDrop { .. }
            | Instruction::StackAlloc { .. }
            | Instruction::StackDrop { .. }
    )
}

/// Collect all values that are used by instructions or terminators in a function.
///
/// This is useful for dead code elimination and other analyses that need to know
/// which values are live.
pub fn instruction_collect_used_values(
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> HashSet<mir::Value> {
    // seed the used value set
    let mut used = HashSet::new();

    // add function parameters as implicitly used (they're inputs)
    for param in &function.parameters {
        used.insert(param.value);
    }

    // scan blocks for instruction and terminator uses
    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        // collect uses from instructions
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);

            // add inline uses
            for value in instruction.uses() {
                used.insert(value);
            }

            // add externalized argument uses (for Call, CallIndirect, Intrinsic)
            if let Some(args_slice) = instruction.argument_slice() {
                for &arg in tree.get_arguments(args_slice) {
                    used.insert(arg);
                }
            }
        }

        // collect uses from terminator
        for value in block.terminator.uses() {
            used.insert(value);
        }
    }

    used
}

/// Substitute values in an instruction according to the given map.
///
/// Creates a new instruction with value references replaced according to the substitution map.
/// Values not in the map are left unchanged.
pub fn instruction_substitute_uses(
    instruction: &mir::Instruction,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> mir::Instruction {
    // skip when no substitutions are provided
    if substitutions.is_empty() {
        return instruction.clone();
    }

    // resolve a value through the substitution map
    let substitute = |v: &mir::Value| -> mir::Value { *substitutions.get(v).unwrap_or(v) };

    // rebuild the instruction with substituted operands
    match instruction {
        mir::Instruction::Binary {
            destination,
            operator,
            left,
            right,
        } => mir::Instruction::Binary {
            destination: *destination,
            operator: *operator,
            left: substitute(left),
            right: substitute(right),
        },
        mir::Instruction::Unary {
            destination,
            operator,
            argument,
        } => mir::Instruction::Unary {
            destination: *destination,
            operator: *operator,
            argument: substitute(argument),
        },
        mir::Instruction::Cast {
            destination,
            operator,
            argument,
            to_type,
        } => mir::Instruction::Cast {
            destination: *destination,
            operator: *operator,
            argument: substitute(argument),
            to_type: *to_type,
        },
        mir::Instruction::Select {
            destination,
            condition,
            then_value,
            else_value,
        } => mir::Instruction::Select {
            destination: *destination,
            condition: substitute(condition),
            then_value: substitute(then_value),
            else_value: substitute(else_value),
        },
        mir::Instruction::Load {
            destination,
            pointer,
            result_type,
        } => mir::Instruction::Load {
            destination: *destination,
            pointer: substitute(pointer),
            result_type: *result_type,
        },
        mir::Instruction::Store { pointer, value } => mir::Instruction::Store {
            pointer: substitute(pointer),
            value: substitute(value),
        },
        mir::Instruction::RawDrop { value } => mir::Instruction::RawDrop {
            value: substitute(value),
        },
        mir::Instruction::StackDrop { value } => mir::Instruction::StackDrop {
            value: substitute(value),
        },
        mir::Instruction::FieldGet {
            destination,
            aggregate,
            index,
        } => mir::Instruction::FieldGet {
            destination: *destination,
            aggregate: substitute(aggregate),
            index: *index,
        },
        mir::Instruction::FieldAddr {
            destination,
            aggregate,
            index,
            result_type,
        } => mir::Instruction::FieldAddr {
            destination: *destination,
            aggregate: substitute(aggregate),
            index: *index,
            result_type: *result_type,
        },
        mir::Instruction::FieldSet {
            destination,
            aggregate,
            index,
            value,
        } => mir::Instruction::FieldSet {
            destination: *destination,
            aggregate: substitute(aggregate),
            index: *index,
            value: substitute(value),
        },
        mir::Instruction::ElementGet {
            destination,
            array,
            index,
        } => mir::Instruction::ElementGet {
            destination: *destination,
            array: substitute(array),
            index: substitute(index),
        },
        mir::Instruction::ElementAddr {
            destination,
            array,
            index,
            result_type,
        } => mir::Instruction::ElementAddr {
            destination: *destination,
            array: substitute(array),
            index: substitute(index),
            result_type: *result_type,
        },
        mir::Instruction::ElementSet {
            destination,
            array,
            index,
            value,
        } => mir::Instruction::ElementSet {
            destination: *destination,
            array: substitute(array),
            index: substitute(index),
            value: substitute(value),
        },
        mir::Instruction::LocalSet { local, value } => mir::Instruction::LocalSet {
            local: *local,
            value: substitute(value),
        },
        mir::Instruction::Assume { condition } => mir::Instruction::Assume {
            condition: substitute(condition),
        },
        mir::Instruction::CallIndirect {
            destination,
            callee,
            arguments,
            signature,
        } => mir::Instruction::CallIndirect {
            destination: *destination,
            callee: substitute(callee),
            arguments: *arguments,
            signature: *signature,
        },
        mir::Instruction::ManagedAllocArray {
            destination,
            element,
            length,
            result_type,
        } => mir::Instruction::ManagedAllocArray {
            destination: *destination,
            element: *element,
            length: substitute(length),
            result_type: *result_type,
        },
        mir::Instruction::RawFree { pointer } => mir::Instruction::RawFree {
            pointer: substitute(pointer),
        },
        // instructions without value operands or with externalized arguments
        mir::Instruction::Const { .. }
        | mir::Instruction::LocalGet { .. }
        | mir::Instruction::GlobalAddr { .. }
        | mir::Instruction::GlobalConst { .. }
        | mir::Instruction::Struct { .. }
        | mir::Instruction::Tuple { .. }
        | mir::Instruction::Array { .. }
        | mir::Instruction::Call { .. }
        | mir::Instruction::ManagedAlloc { .. }
        | mir::Instruction::RawAlloc { .. }
        | mir::Instruction::StackAlloc { .. }
        | mir::Instruction::Intrinsic { .. } => instruction.clone(),
    }
}

/// Substitute values in an instruction, including externalized arguments.
///
/// Creates a new instruction with value references replaced according to the substitution map.
/// Values not in the map are left unchanged.
pub fn instruction_substitute_uses_in_tree(
    instruction: &mir::Instruction,
    substitutions: &HashMap<mir::Value, mir::Value>,
    tree: &mut mir::NodeTree,
) -> mir::Instruction {
    // skip when no substitutions are provided
    if substitutions.is_empty() {
        return instruction.clone();
    }

    // resolve values through the substitution map
    let substitute =
        |value: mir::Value| -> mir::Value { *substitutions.get(&value).unwrap_or(&value) };

    // rebuild argument slices when needed
    let mut substitute_arguments = |slice: mir::ArgumentSlice| -> mir::ArgumentSlice {
        // read existing arguments
        let arguments = tree.get_arguments(slice);

        // skip when no arguments are substituted
        if !arguments
            .iter()
            .any(|value| substitutions.contains_key(value))
        {
            return slice;
        }

        // build remapped arguments
        let new_arguments: Vec<_> = arguments.iter().map(|value| substitute(*value)).collect();

        tree.add_arguments(&new_arguments)
    };

    // rebuild the instruction using substituted operands
    match instruction {
        mir::Instruction::Struct {
            destination,
            ty,
            fields,
        } => mir::Instruction::Struct {
            destination: *destination,
            ty: *ty,
            fields: substitute_arguments(*fields),
        },
        mir::Instruction::Tuple {
            destination,
            ty,
            elements,
        } => mir::Instruction::Tuple {
            destination: *destination,
            ty: *ty,
            elements: substitute_arguments(*elements),
        },
        mir::Instruction::Array {
            destination,
            ty,
            elements,
        } => mir::Instruction::Array {
            destination: *destination,
            ty: *ty,
            elements: substitute_arguments(*elements),
        },
        mir::Instruction::Call {
            destination,
            function,
            arguments,
        } => mir::Instruction::Call {
            destination: *destination,
            function: *function,
            arguments: substitute_arguments(*arguments),
        },
        mir::Instruction::CallIndirect {
            destination,
            callee,
            arguments,
            signature,
        } => mir::Instruction::CallIndirect {
            destination: *destination,
            callee: substitute(*callee),
            arguments: substitute_arguments(*arguments),
            signature: *signature,
        },
        mir::Instruction::Intrinsic {
            destination,
            intrinsic,
            arguments,
            ordering,
        } => mir::Instruction::Intrinsic {
            destination: *destination,
            intrinsic: *intrinsic,
            arguments: substitute_arguments(*arguments),
            ordering: *ordering,
        },
        _ => instruction_substitute_uses(instruction, substitutions),
    }
}

/// Substitute values in a slice using the provided mapping.
pub fn substitute_values(
    values: &[mir::Value],
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> Vec<mir::Value> {
    // fast path for empty substitutions
    if substitutions.is_empty() {
        return values.to_vec();
    }

    // apply substitutions to the value list
    values
        .iter()
        .map(|value| substitutions.get(value).copied().unwrap_or(*value))
        .collect()
}

/// Apply substitutions and optional removals across a function.
///
/// Returns true when any instruction or terminator is updated or removed.
pub fn apply_substitutions_in_function(
    function: &mir::Function,
    tree: &mut mir::NodeTree,
    substitutions: &HashMap<mir::Value, mir::Value>,
    to_remove: Option<&HashSet<mir::LocalNodeId<mir::Instruction>>>,
) -> bool {
    // check if work is required
    let has_substitutions = !substitutions.is_empty();
    let has_removals = to_remove.is_some_and(|set| !set.is_empty());
    if !has_substitutions && !has_removals {
        return false;
    }

    // track whether any changes occur
    let mut changed = false;

    // rewrite instructions and terminators in each block
    for &block_id in &function.blocks {
        // snapshot block contents
        let block = tree.get(block_id).clone();
        let instruction_ids = block.instructions.clone();
        let terminator = block.terminator.clone();

        // rebuild instructions with substitutions and removals
        let mut new_instructions = Vec::with_capacity(instruction_ids.len());
        for instruction_id in instruction_ids {
            // skip instructions slated for removal
            if to_remove.is_some_and(|set| set.contains(&instruction_id)) {
                changed = true;
                continue;
            }

            // substitute instruction operands when requested
            if has_substitutions {
                let instruction = tree.get(instruction_id).clone();
                let updated =
                    instruction_substitute_uses_in_tree(&instruction, substitutions, tree);
                if updated != instruction {
                    tree.replace(instruction_id, updated);
                    changed = true;
                }
            }

            new_instructions.push(instruction_id);
        }

        // rewrite terminator operands when requested
        let new_terminator = if has_substitutions {
            terminator_substitute_uses(&terminator, substitutions)
        } else {
            terminator.clone()
        };

        // update block when instructions or terminator changed
        if new_instructions.len() != block.instructions.len() || new_terminator != terminator {
            let mut new_block = block;
            new_block.instructions = new_instructions;
            new_block.terminator = new_terminator;
            tree.replace(block_id, new_block);
            changed = true;
        }
    }

    changed
}

/// Maps for tracking where values are used and defined.
#[derive(Debug)]
pub struct UseDefMaps {
    /// Maps each value to the blocks where it is used.
    pub use_blocks: HashMap<mir::Value, Vec<mir::LocalNodeId<mir::Block>>>,
    /// Maps each value to the block where it is defined.
    pub def_block: HashMap<mir::Value, mir::LocalNodeId<mir::Block>>,
}

/// Build maps from values to their use locations and definition blocks.
///
/// This is useful for sinking, code motion, and liveness analysis.
/// Function parameters are not included in `def_block` (they have no defining block).
pub fn build_use_def_maps(function: &mir::Function, tree: &mir::NodeTree) -> UseDefMaps {
    // initialize use and definition maps
    let mut use_blocks: HashMap<mir::Value, Vec<mir::LocalNodeId<mir::Block>>> = HashMap::new();
    let mut def_block: HashMap<mir::Value, mir::LocalNodeId<mir::Block>> = HashMap::new();

    // scan blocks for definitions and uses
    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        // block parameters are defined in this block
        for param in &block.parameters {
            def_block.insert(param.value, block_id);
        }

        // instructions
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);

            // record definition
            if let Some(dest) = instruction.destination() {
                def_block.insert(dest, block_id);
            }

            // record uses
            for use_value in instruction.uses() {
                use_blocks.entry(use_value).or_default().push(block_id);
            }

            // externalized arguments
            if let Some(args_slice) = instruction.argument_slice() {
                for &arg in tree.get_arguments(args_slice) {
                    use_blocks.entry(arg).or_default().push(block_id);
                }
            }
        }

        // terminator uses
        for use_value in block.terminator.uses() {
            use_blocks.entry(use_value).or_default().push(block_id);
        }
    }

    UseDefMaps {
        use_blocks,
        def_block,
    }
}

/// Build a map from values to their use counts.
pub fn build_value_use_counts(
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> HashMap<mir::Value, usize> {
    // reuse use def map and count occurrences
    let use_def = build_use_def_maps(function, tree);
    let mut counts = HashMap::new();

    // count uses per value
    for (value, blocks) in use_def.use_blocks {
        counts.insert(value, blocks.len());
    }

    // return the counts
    counts
}

/// Definition metadata for instructions.
#[derive(Debug, Clone)]
pub struct InstructionRef {
    /// The instruction that defines the value.
    pub instruction: mir::Instruction,
    /// The block containing the instruction.
    pub block: mir::LocalNodeId<mir::Block>,
    /// The instruction index within the block.
    pub index: usize,
}

/// Build a map from values to the instructions that define them.
pub fn build_value_definition_map(
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>> {
    // collect instruction destinations
    let mut map = HashMap::new();

    // scan blocks for definitions
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if let Some(destination) = instruction.destination() {
                map.insert(destination, instruction_id);
            }
        }
    }

    map
}

/// Build a map from instruction ids to their containing blocks.
pub fn build_instruction_block_map(
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> HashMap<mir::LocalNodeId<mir::Instruction>, mir::LocalNodeId<mir::Block>> {
    let mut map = HashMap::new();

    // scan blocks for instruction ownership
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            map.insert(instruction_id, block_id);
        }
    }

    map
}

/// Build a map from values to their defining instructions.
pub fn build_value_instruction_map(
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> HashMap<mir::Value, mir::Instruction> {
    // collect instruction destinations
    let mut map = HashMap::new();

    // scan blocks for definitions
    for &block_id in &function.blocks {
        // read block instructions
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            // record instructions that define a value
            let instruction = tree.get(instruction_id);
            if let Some(destination) = instruction.destination() {
                map.insert(destination, instruction.clone());
            }
        }
    }

    map
}

/// Build a map from values to their defining instruction references.
pub fn build_value_instruction_refs(
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> HashMap<mir::Value, InstructionRef> {
    // collect instruction references
    let mut map = HashMap::new();

    // scan blocks for definitions
    for &block_id in &function.blocks {
        // read block instructions
        let block = tree.get(block_id);
        for (index, instruction_id) in block.instructions.iter().enumerate() {
            // record instructions that define a value
            let instruction = tree.get(*instruction_id);
            if let Some(destination) = instruction.destination() {
                map.insert(
                    destination,
                    InstructionRef {
                        instruction: instruction.clone(),
                        block: block_id,
                        index,
                    },
                );
            }
        }
    }

    map
}

/// Remap all values in an instruction according to the given map.
///
/// Unlike `instruction_substitute_uses`, this also remaps the destination and
/// handles externalized arguments (Call, Intrinsic, etc.) by creating new
/// argument slices in the tree.
pub fn instruction_map(
    instruction: &mir::Instruction,
    value_map: &HashMap<mir::Value, mir::Value>,
    tree: &mut mir::NodeTree,
) -> mir::Instruction {
    // remap values through the provided map
    let remap = |v: mir::Value| -> mir::Value { *value_map.get(&v).unwrap_or(&v) };

    // rebuild argument slices with remapped values
    let mut remap_arguments = |slice: mir::ArgumentSlice| -> mir::ArgumentSlice {
        // remap argument values
        let new_args: Vec<_> = tree
            .get_arguments(slice)
            .iter()
            .map(|&v| remap(v))
            .collect();

        tree.add_arguments(&new_args)
    };

    // rebuild the instruction with remapped values
    match instruction {
        mir::Instruction::Const { destination, value } => mir::Instruction::Const {
            destination: remap(*destination),
            value: value.clone(),
        },
        mir::Instruction::Binary {
            destination,
            operator,
            left,
            right,
        } => mir::Instruction::Binary {
            destination: remap(*destination),
            operator: *operator,
            left: remap(*left),
            right: remap(*right),
        },
        mir::Instruction::Unary {
            destination,
            operator,
            argument,
        } => mir::Instruction::Unary {
            destination: remap(*destination),
            operator: *operator,
            argument: remap(*argument),
        },
        mir::Instruction::Cast {
            destination,
            operator,
            argument,
            to_type,
        } => mir::Instruction::Cast {
            destination: remap(*destination),
            operator: *operator,
            argument: remap(*argument),
            to_type: *to_type,
        },
        mir::Instruction::Select {
            destination,
            condition,
            then_value,
            else_value,
        } => mir::Instruction::Select {
            destination: remap(*destination),
            condition: remap(*condition),
            then_value: remap(*then_value),
            else_value: remap(*else_value),
        },
        mir::Instruction::Load {
            destination,
            pointer,
            result_type,
        } => mir::Instruction::Load {
            destination: remap(*destination),
            pointer: remap(*pointer),
            result_type: *result_type,
        },
        mir::Instruction::Store { pointer, value } => mir::Instruction::Store {
            pointer: remap(*pointer),
            value: remap(*value),
        },
        mir::Instruction::RawDrop { value } => mir::Instruction::RawDrop {
            value: remap(*value),
        },
        mir::Instruction::StackDrop { value } => mir::Instruction::StackDrop {
            value: remap(*value),
        },
        mir::Instruction::FieldGet {
            destination,
            aggregate,
            index,
        } => mir::Instruction::FieldGet {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            index: *index,
        },
        mir::Instruction::FieldAddr {
            destination,
            aggregate,
            index,
            result_type,
        } => mir::Instruction::FieldAddr {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            index: *index,
            result_type: *result_type,
        },
        mir::Instruction::FieldSet {
            destination,
            aggregate,
            index,
            value,
        } => mir::Instruction::FieldSet {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            index: *index,
            value: remap(*value),
        },
        mir::Instruction::ElementGet {
            destination,
            array,
            index,
        } => mir::Instruction::ElementGet {
            destination: remap(*destination),
            array: remap(*array),
            index: remap(*index),
        },
        mir::Instruction::ElementAddr {
            destination,
            array,
            index,
            result_type,
        } => mir::Instruction::ElementAddr {
            destination: remap(*destination),
            array: remap(*array),
            index: remap(*index),
            result_type: *result_type,
        },
        mir::Instruction::ElementSet {
            destination,
            array,
            index,
            value,
        } => mir::Instruction::ElementSet {
            destination: remap(*destination),
            array: remap(*array),
            index: remap(*index),
            value: remap(*value),
        },
        mir::Instruction::LocalGet { destination, local } => mir::Instruction::LocalGet {
            destination: remap(*destination),
            local: *local,
        },
        mir::Instruction::LocalSet { local, value } => mir::Instruction::LocalSet {
            local: *local,
            value: remap(*value),
        },
        mir::Instruction::Assume { condition } => mir::Instruction::Assume {
            condition: remap(*condition),
        },
        mir::Instruction::GlobalAddr {
            destination,
            global,
            result_type,
        } => mir::Instruction::GlobalAddr {
            destination: remap(*destination),
            global: *global,
            result_type: *result_type,
        },
        mir::Instruction::GlobalConst {
            destination,
            global,
        } => mir::Instruction::GlobalConst {
            destination: remap(*destination),
            global: *global,
        },
        mir::Instruction::Struct {
            destination,
            ty,
            fields,
        } => mir::Instruction::Struct {
            destination: remap(*destination),
            ty: *ty,
            fields: remap_arguments(*fields),
        },
        mir::Instruction::Tuple {
            destination,
            ty,
            elements,
        } => mir::Instruction::Tuple {
            destination: remap(*destination),
            ty: *ty,
            elements: remap_arguments(*elements),
        },
        mir::Instruction::Array {
            destination,
            ty,
            elements,
        } => mir::Instruction::Array {
            destination: remap(*destination),
            ty: *ty,
            elements: remap_arguments(*elements),
        },
        mir::Instruction::Call {
            destination,
            function,
            arguments,
        } => mir::Instruction::Call {
            destination: destination.map(remap),
            function: *function,
            arguments: remap_arguments(*arguments),
        },
        mir::Instruction::CallIndirect {
            destination,
            callee,
            arguments,
            signature,
        } => mir::Instruction::CallIndirect {
            destination: destination.map(remap),
            callee: remap(*callee),
            arguments: remap_arguments(*arguments),
            signature: *signature,
        },
        mir::Instruction::ManagedAlloc {
            destination,
            layout,
            result_type,
        } => mir::Instruction::ManagedAlloc {
            destination: remap(*destination),
            layout: *layout,
            result_type: *result_type,
        },
        mir::Instruction::ManagedAllocArray {
            destination,
            element,
            length,
            result_type,
        } => mir::Instruction::ManagedAllocArray {
            destination: remap(*destination),
            element: *element,
            length: remap(*length),
            result_type: *result_type,
        },
        mir::Instruction::RawAlloc {
            destination,
            layout,
            result_type,
        } => mir::Instruction::RawAlloc {
            destination: remap(*destination),
            layout: *layout,
            result_type: *result_type,
        },
        mir::Instruction::RawFree { pointer } => mir::Instruction::RawFree {
            pointer: remap(*pointer),
        },
        mir::Instruction::StackAlloc {
            destination,
            layout,
            result_type,
        } => mir::Instruction::StackAlloc {
            destination: remap(*destination),
            layout: *layout,
            result_type: *result_type,
        },
        mir::Instruction::Intrinsic {
            destination,
            intrinsic,
            arguments,
            ordering,
        } => mir::Instruction::Intrinsic {
            destination: destination.map(remap),
            intrinsic: *intrinsic,
            arguments: remap_arguments(*arguments),
            ordering: *ordering,
        },
    }
}

/// Remap block targets and values in a terminator.
///
/// Block targets are remapped according to `block_map`, and values are remapped
/// according to `value_map`. Values/blocks not in the maps are left unchanged.
pub fn terminator_remap(
    terminator: &mut mir::Terminator,
    block_map: &HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
    value_map: &HashMap<mir::Value, mir::Value>,
) {
    // remap block targets in place
    let remap_target = |t: &mut mir::LocalNodeId<mir::Block>| {
        if let Some(&new_t) = block_map.get(t) {
            *t = new_t;
        }
    };

    // remap values in place
    let remap_value = |v: &mut mir::Value| {
        if let Some(&new_v) = value_map.get(v) {
            *v = new_v;
        }
    };

    // remap a list of arguments
    let remap_args = |args: &mut Vec<mir::Value>| {
        for arg in args.iter_mut() {
            remap_value(arg);
        }
    };

    // remap terminator fields
    match terminator {
        mir::Terminator::Jump { target, arguments } => {
            remap_target(target);
            remap_args(arguments);
        }
        mir::Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => {
            remap_value(condition);
            remap_target(then_target);
            remap_args(then_arguments);
            remap_target(else_target);
            remap_args(else_arguments);
        }
        mir::Terminator::Check {
            condition,
            constraint,
            success,
            failure,
        } => {
            remap_value(condition);
            remap_target(&mut success.target);
            remap_args(&mut success.arguments);
            remap_target(&mut failure.target);
            remap_args(&mut failure.arguments);
            match constraint {
                mir::CheckConstraint::Bounds {
                    index,
                    length,
                    collection,
                    ..
                } => {
                    remap_value(index);
                    remap_value(length);
                    remap_value(collection);
                }
                mir::CheckConstraint::Null { value } => {
                    remap_value(value);
                }
                mir::CheckConstraint::DivZero { divisor } => {
                    remap_value(divisor);
                }
                mir::CheckConstraint::ShiftRange { value, .. } => {
                    remap_value(value);
                }
                mir::CheckConstraint::Narrow { value, .. } => {
                    remap_value(value);
                }
                mir::CheckConstraint::Overflow { left, right, .. } => {
                    remap_value(left);
                    remap_value(right);
                }
            }
        }
        mir::Terminator::Switch {
            value,
            default,
            default_arguments,
            cases,
        } => {
            remap_value(value);
            remap_target(default);
            remap_args(default_arguments);
            for case in cases.iter_mut() {
                remap_target(&mut case.target);
                remap_args(&mut case.arguments);
            }
        }
        mir::Terminator::Return { value } => {
            if let Some(v) = value {
                remap_value(v);
            }
        }
        mir::Terminator::Yield {
            value,
            resume,
            resume_arguments,
        } => {
            remap_value(value);
            remap_target(resume);
            remap_args(resume_arguments);
        }
        mir::Terminator::Unreachable => {}
        mir::Terminator::TailCall {
            function: _,
            arguments,
        } => {
            remap_args(arguments);
        }
        mir::Terminator::TailCallIndirect {
            callee, arguments, ..
        } => {
            remap_value(callee);
            remap_args(arguments);
        }
    }
}
