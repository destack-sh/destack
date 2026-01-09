use std::collections::{HashMap, HashSet};

use destack_mir as mir;
use mir::Instruction;

/// Check if an instruction is pure (result depends only on operands).
///
/// A pure instruction has no side effects AND does not read mutable state.
/// This is stricter than `!instruction_has_side_effects`:
/// - `Load`/`LocalGet` have no side effects (can be removed if unused)
/// - But they read mutable state (cannot be hoisted out of a loop)
///
/// Use this for LICM, code motion, and speculation optimizations.
pub fn instruction_is_pure(instruction: &Instruction) -> bool {
    match instruction {
        // pure computations
        Instruction::Const { .. }
        | Instruction::Binary { .. }
        | Instruction::Unary { .. }
        | Instruction::Cast { .. } => true,

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

/// Check if an instruction has side effects and cannot be removed even if unused.
///
/// Instructions with side effects must be preserved regardless of whether their
/// result is used. This includes stores, calls, allocations, and drops.
pub fn instruction_has_side_effects(instruction: &Instruction) -> bool {
    match instruction {
        // pure computations, no side effects
        Instruction::Const { .. }
        | Instruction::Binary { .. }
        | Instruction::Unary { .. }
        | Instruction::Cast { .. }
        | Instruction::Struct { .. }
        | Instruction::Tuple { .. }
        | Instruction::Array { .. }
        | Instruction::FieldGet { .. }
        | Instruction::FieldAddr { .. }
        | Instruction::ElementGet { .. }
        | Instruction::ElementAddr { .. }
        | Instruction::GlobalConst { .. }
        | Instruction::GlobalAddr { .. } => false,

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

        // intrinsics may have side effects (conservative)
        Instruction::Intrinsic { .. } => true,
    }
}

/// Collect all values that are used by instructions or terminators in a function.
///
/// This is useful for dead code elimination and other analyses that need to know
/// which values are live.
pub fn instruction_collect_used_values(
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> HashSet<mir::Value> {
    let mut used = HashSet::new();

    // add function parameters as implicitly used (they're inputs)
    for param in &function.parameters {
        used.insert(param.value);
    }

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
    if substitutions.is_empty() {
        return instruction.clone();
    }

    let substitute = |v: &mir::Value| -> mir::Value { *substitutions.get(v).unwrap_or(v) };

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
        mir::Instruction::Load {
            destination,
            pointer,
        } => mir::Instruction::Load {
            destination: *destination,
            pointer: substitute(pointer),
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
        } => mir::Instruction::FieldAddr {
            destination: *destination,
            aggregate: substitute(aggregate),
            index: *index,
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
        } => mir::Instruction::ElementAddr {
            destination: *destination,
            array: substitute(array),
            index: substitute(index),
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
        mir::Instruction::CallIndirect {
            destination,
            callee,
            arguments,
        } => mir::Instruction::CallIndirect {
            destination: *destination,
            callee: substitute(callee),
            arguments: *arguments,
        },
        mir::Instruction::ManagedAllocArray {
            destination,
            element,
            length,
        } => mir::Instruction::ManagedAllocArray {
            destination: *destination,
            element: *element,
            length: substitute(length),
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
    let mut use_blocks: HashMap<mir::Value, Vec<mir::LocalNodeId<mir::Block>>> = HashMap::new();
    let mut def_block: HashMap<mir::Value, mir::LocalNodeId<mir::Block>> = HashMap::new();

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
    let remap = |v: mir::Value| -> mir::Value { *value_map.get(&v).unwrap_or(&v) };

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
        mir::Instruction::Load {
            destination,
            pointer,
        } => mir::Instruction::Load {
            destination: remap(*destination),
            pointer: remap(*pointer),
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
        } => mir::Instruction::FieldAddr {
            destination: remap(*destination),
            aggregate: remap(*aggregate),
            index: *index,
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
        } => mir::Instruction::ElementAddr {
            destination: remap(*destination),
            array: remap(*array),
            index: remap(*index),
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
        mir::Instruction::GlobalAddr {
            destination,
            global,
        } => mir::Instruction::GlobalAddr {
            destination: remap(*destination),
            global: *global,
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
        } => {
            let new_args: Vec<mir::Value> = tree
                .get_arguments(*fields)
                .iter()
                .map(|&v| remap(v))
                .collect();
            let new_slice = tree.add_arguments(&new_args);
            mir::Instruction::Struct {
                destination: remap(*destination),
                ty: *ty,
                fields: new_slice,
            }
        }
        mir::Instruction::Tuple {
            destination,
            ty,
            elements,
        } => {
            let new_args: Vec<mir::Value> = tree
                .get_arguments(*elements)
                .iter()
                .map(|&v| remap(v))
                .collect();
            let new_slice = tree.add_arguments(&new_args);
            mir::Instruction::Tuple {
                destination: remap(*destination),
                ty: *ty,
                elements: new_slice,
            }
        }
        mir::Instruction::Array {
            destination,
            ty,
            elements,
        } => {
            let new_args: Vec<mir::Value> = tree
                .get_arguments(*elements)
                .iter()
                .map(|&v| remap(v))
                .collect();
            let new_slice = tree.add_arguments(&new_args);
            mir::Instruction::Array {
                destination: remap(*destination),
                ty: *ty,
                elements: new_slice,
            }
        }
        mir::Instruction::Call {
            destination,
            function,
            arguments,
        } => {
            let new_args: Vec<mir::Value> = tree
                .get_arguments(*arguments)
                .iter()
                .map(|&v| remap(v))
                .collect();
            let new_slice = tree.add_arguments(&new_args);
            mir::Instruction::Call {
                destination: destination.map(remap),
                function: *function,
                arguments: new_slice,
            }
        }
        mir::Instruction::CallIndirect {
            destination,
            callee,
            arguments,
        } => {
            let new_args: Vec<mir::Value> = tree
                .get_arguments(*arguments)
                .iter()
                .map(|&v| remap(v))
                .collect();
            let new_slice = tree.add_arguments(&new_args);
            mir::Instruction::CallIndirect {
                destination: destination.map(remap),
                callee: remap(*callee),
                arguments: new_slice,
            }
        }
        mir::Instruction::ManagedAlloc {
            destination,
            layout,
        } => mir::Instruction::ManagedAlloc {
            destination: remap(*destination),
            layout: *layout,
        },
        mir::Instruction::ManagedAllocArray {
            destination,
            element,
            length,
        } => mir::Instruction::ManagedAllocArray {
            destination: remap(*destination),
            element: *element,
            length: remap(*length),
        },
        mir::Instruction::RawAlloc {
            destination,
            layout,
        } => mir::Instruction::RawAlloc {
            destination: remap(*destination),
            layout: *layout,
        },
        mir::Instruction::RawFree { pointer } => mir::Instruction::RawFree {
            pointer: remap(*pointer),
        },
        mir::Instruction::StackAlloc {
            destination,
            layout,
        } => mir::Instruction::StackAlloc {
            destination: remap(*destination),
            layout: *layout,
        },
        mir::Instruction::Intrinsic {
            destination,
            intrinsic,
            arguments,
            ordering,
        } => {
            let new_args: Vec<mir::Value> = tree
                .get_arguments(*arguments)
                .iter()
                .map(|&v| remap(v))
                .collect();
            let new_slice = tree.add_arguments(&new_args);
            mir::Instruction::Intrinsic {
                destination: destination.map(remap),
                intrinsic: *intrinsic,
                arguments: new_slice,
                ordering: *ordering,
            }
        }
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
    let remap_target = |t: &mut mir::LocalNodeId<mir::Block>| {
        if let Some(&new_t) = block_map.get(t) {
            *t = new_t;
        }
    };

    let remap_value = |v: &mut mir::Value| {
        if let Some(&new_v) = value_map.get(v) {
            *v = new_v;
        }
    };

    let remap_args = |args: &mut Vec<mir::Value>| {
        for arg in args.iter_mut() {
            remap_value(arg);
        }
    };

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
    }
}
