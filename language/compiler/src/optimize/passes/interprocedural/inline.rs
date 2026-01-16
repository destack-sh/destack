use std::collections::HashMap;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::CallGraphScc;
use crate::optimize::common::{
    instruction_substitute_uses_in_tree, terminator_remap, terminator_substitute_uses,
};
use crate::optimize::{AnalysisPreservation, ModuleAnalyses, ModulePass, PipelineContext};

declare_pass! {
    /// Inline direct calls into their callers when the callee is small.
    ///
    /// This pass clones callee blocks into the caller, rewires returns to a continuation block, and skips recursive SCCs and functions with tail calls.
    ///
    /// ```mir
    /// function @callee(v0: i32) -> i32 {
    /// block0(v0: i32):
    ///     v1 = iadd v0, v0
    ///     return v1
    /// }
    /// function @caller(v0: i32) -> i32 {
    /// block0(v0: i32):
    ///     v1 = call @callee(v0)
    ///     v2 = iadd v1, v0
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @caller(v0: i32) -> i32 {
    /// block0(v0: i32):
    ///     jump block1(v0)
    /// block1(v1: i32):
    ///     v2 = iadd v1, v1
    ///     jump block2(v2)
    /// block2(v3: i32):
    ///     v4 = iadd v3, v0
    ///     return v4
    /// }
    /// ```
    #[pass(id = "inline")]
    pub Inline,
    "Inline direct calls"
}

impl ModulePass for Inline {
    /// Run the inline pass over a module.
    fn run(&self, tree: &mut mir::NodeTree, ctx: &PipelineContext<'_>) -> AnalysisPreservation {
        let changed = run_inline(tree, ctx);

        // report analysis preservation based on whether changes occurred
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass display name.
    fn name(&self) -> &'static str {
        "Inline"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "inline"
    }
}

/// Maximum instructions allowed for inlining.
const INLINE_MAX_INSTRUCTIONS: usize = 48;
/// Maximum blocks allowed for inlining.
const INLINE_MAX_BLOCKS: usize = 6;
/// Maximum call sites within the callee for inlining.
const INLINE_MAX_CALLS: usize = 4;
/// Maximum instructions for leaf callees.
const INLINE_MAX_LEAF_INSTRUCTIONS: usize = 96;
/// Maximum function size after inlining.
const INLINE_MAX_FUNCTION_INSTRUCTIONS: usize = 2000;
/// Maximum inline sites per function.
const INLINE_MAX_SITES_PER_FUNCTION: usize = 16;

/// Inline pass main entry.
fn run_inline(tree: &mut mir::NodeTree, ctx: &PipelineContext<'_>) -> bool {
    // build analysis summaries for inlining
    let analyses = ModuleAnalyses::new(tree);
    let scc_map = analyses.get::<CallGraphScc>();

    // collect function ids for stable iteration
    let function_ids: Vec<_> = tree
        .iter_nodes::<mir::Function>()
        .map(|(id, _)| id)
        .collect();

    // track whether any inlining happened
    let mut changed = false;

    // walk each function for inline opportunities
    for function_id in function_ids {
        // skip functions without bodies
        let function = tree.get(function_id);
        if function.entry.is_none() {
            continue;
        }

        // clone the function for in place edits
        let mut function = function.clone();
        function.recompute_next_value_id(tree);

        // inline until no sites remain or budget is exhausted
        let mut inline_count = 0usize;
        // iterate inline sites until the budget is exhausted
        loop {
            // stop when the inline budget is exhausted
            if inline_count >= INLINE_MAX_SITES_PER_FUNCTION {
                break;
            }

            // find the next candidate callsite
            let site = find_inline_site(function_id, &function, tree, &scc_map);
            let Some(site) = site else {
                break;
            };

            // attempt to inline the selected callsite
            let did_inline = inline_callsite(&mut function, tree, &site);
            if !did_inline {
                break;
            }

            // record a successful inline for this function
            inline_count += 1;
            changed = true;
        }

        // commit the updated function back into the tree
        *tree.get_mut(function_id) = function;
    }

    // record pass activity for downstream diagnostics
    if changed {
        ctx.strings.intern("inline");
    }

    changed
}

/// Inline site information for a call instruction.
#[derive(Debug, Clone)]
struct InlineSite {
    /// The block containing the call.
    block_id: mir::LocalNodeId<mir::Block>,
    /// The index of the call instruction in the block.
    call_index: usize,
    /// The call instruction id.
    call_instruction_id: mir::LocalNodeId<mir::Instruction>,
    /// The resolved callee id.
    callee_id: mir::LocalNodeId<mir::Function>,
    /// The call argument values.
    arguments: Vec<mir::Value>,
    /// The call destination value when present.
    destination: Option<mir::Value>,
}

/// Find the next inline candidate in a function.
fn find_inline_site(
    function_id: mir::LocalNodeId<mir::Function>,
    function: &mir::Function,
    tree: &mir::NodeTree,
    scc_map: &CallGraphScc,
) -> Option<InlineSite> {
    // scan blocks in order for candidate callsites
    let block_ids = function.blocks.clone();
    for block_id in block_ids {
        // load the block and clone its instruction list
        let block = tree.get(block_id);
        let instruction_ids = block.instructions.clone();
        for (index, instruction_id) in instruction_ids.iter().enumerate() {
            // resolve a direct inline target for the instruction
            let instruction = tree.get(*instruction_id);
            let Some((callee_id, arguments, destination)) =
                resolve_inline_target(*instruction_id, instruction, tree)
            else {
                continue;
            };

            // reject callsites that do not meet heuristic thresholds
            let should_inline = should_inline(
                tree,
                function_id,
                function,
                callee_id,
                scc_map,
                arguments.len(),
            );
            if !should_inline {
                continue;
            }

            // return the first viable inline site
            return Some(InlineSite {
                block_id,
                call_index: index,
                call_instruction_id: *instruction_id,
                callee_id,
                arguments,
                destination,
            });
        }
    }

    // no inline sites were found
    None
}

/// Resolve a call instruction to a direct inline target.
fn resolve_inline_target(
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    instruction: &mir::Instruction,
    tree: &mir::NodeTree,
) -> Option<(
    mir::LocalNodeId<mir::Function>,
    Vec<mir::Value>,
    Option<mir::Value>,
)> {
    // inspect call instruction variants
    match instruction {
        mir::Instruction::Call {
            destination,
            function,
            arguments,
        } => {
            // capture call arguments for a direct call
            let args = tree.get_arguments(*arguments).to_vec();
            Some((*function, args, *destination))
        }
        mir::Instruction::CallIndirect {
            destination,
            arguments,
            ..
        } => {
            // resolve metadata to a direct target when available
            let args = tree.get_arguments(*arguments).to_vec();
            let metadata = tree.call_table.call_metadata(instruction_id)?;
            if metadata.dispatch != mir::CallDispatchKind::Direct {
                return None;
            }
            let target = metadata.declared_target?;
            Some((target, args, *destination))
        }
        _ => None,
    }
}

/// Check if a call should be inlined.
fn should_inline(
    tree: &mir::NodeTree,
    caller_id: mir::LocalNodeId<mir::Function>,
    caller: &mir::Function,
    callee_id: mir::LocalNodeId<mir::Function>,
    scc_map: &CallGraphScc,
    argument_count: usize,
) -> bool {
    // load the callee metadata
    let callee = tree.get(callee_id);

    // reject callees without bodies or with unsupported forms
    if callee.entry.is_none() {
        return false;
    }
    if callee.coroutine.is_some() {
        return false;
    }
    if callee.parameters.len() != argument_count {
        return false;
    }
    if has_tail_calls(tree, callee) {
        return false;
    }
    if is_recursive_call(caller_id, callee_id, scc_map) {
        return false;
    }

    // evaluate inline size thresholds
    let callee_size = function_size_for(tree, callee);
    let caller_size = function_size_for(tree, caller);
    let estimated = caller_size.instructions + callee_size.instructions;
    if estimated > INLINE_MAX_FUNCTION_INSTRUCTIONS {
        return false;
    }
    if callee_size.calls == 0 {
        return callee_size.instructions <= INLINE_MAX_LEAF_INSTRUCTIONS;
    }
    callee_size.instructions <= INLINE_MAX_INSTRUCTIONS
        && callee_size.blocks <= INLINE_MAX_BLOCKS
        && callee_size.calls <= INLINE_MAX_CALLS
}

/// Inline a direct callsite into the caller.
fn inline_callsite(
    caller: &mut mir::Function,
    tree: &mut mir::NodeTree,
    site: &InlineSite,
) -> bool {
    // load the callee and entry block
    let callee = tree.get(site.callee_id).clone();
    let Some(entry_block) = callee.entry else {
        return false;
    };

    // reject mismatched return handling
    if matches!(tree.get(callee.return_type), mir::Type::Void) && site.destination.is_some() {
        return false;
    }

    // validate the call instruction location
    let call_block = tree.get(site.block_id);
    if site.call_index >= call_block.instructions.len() {
        return false;
    }
    if call_block.instructions[site.call_index] != site.call_instruction_id {
        return false;
    }

    // build the parameter to argument mapping
    let mut argument_map = HashMap::new();
    for (param, arg) in callee.parameters.iter().zip(site.arguments.iter()) {
        argument_map.insert(param.value, *arg);
    }

    // ensure entry block parameters are sourced from arguments
    let entry_params = tree.get(entry_block).parameters.clone();
    for param in &entry_params {
        if !argument_map.contains_key(&param.value) {
            return false;
        }
    }

    // clone locals and blocks before rewriting the caller
    let local_map = clone_locals(caller, tree, &callee);
    let (block_map, value_map) = clone_callee_blocks(caller, tree, &callee, &argument_map);

    // split the caller block and jump into the inlined entry
    let inline_entry = block_map[&entry_block];
    let split = split_block_for_inline(
        caller,
        tree,
        site.block_id,
        site.call_index,
        site.call_instruction_id,
        inline_entry,
        callee.return_type,
        site.destination,
        &entry_params,
        &argument_map,
    );
    let Some(split) = split else {
        return false;
    };

    // substitute the call result in the continuation block
    if let (Some(destination), Some(result_value)) = (site.destination, split.result_value) {
        substitute_value_in_block(tree, split.continuation_id, destination, result_value);
    }

    // remap the inlined blocks and rewrite returns
    remap_inline_blocks(tree, &callee, &block_map, &value_map, &local_map);
    rewrite_inlined_returns(
        tree,
        &block_map,
        split.continuation_id,
        site.destination.is_some(),
    );

    // clean up metadata for the removed call instruction
    tree.call_table
        .remove_call_metadata(site.call_instruction_id);
    tree.memory_table
        .remove_memory_accesses(site.call_instruction_id);
    tree.debug_info
        .instruction_locations
        .remove(&site.call_instruction_id);

    true
}

/// Result of splitting a block around a call.
#[derive(Debug, Clone, Copy)]
struct InlineSplit {
    /// The continuation block id.
    continuation_id: mir::LocalNodeId<mir::Block>,
    /// The continuation parameter value when the call returns a value.
    result_value: Option<mir::Value>,
}

/// Clone locals from the callee into the caller.
fn clone_locals(
    caller: &mut mir::Function,
    tree: &mut mir::NodeTree,
    callee: &mir::Function,
) -> HashMap<mir::LocalNodeId<mir::Local>, mir::LocalNodeId<mir::Local>> {
    // allocate new locals in the caller
    let mut local_map = HashMap::new();
    for local_id in &callee.locals {
        let local = tree.get(*local_id).clone();
        let new_local = tree.insert(local);
        local_map.insert(*local_id, new_local);
        caller.locals.push(new_local);
    }

    local_map
}

/// Clone callee blocks and their value ids into the caller.
fn clone_callee_blocks(
    caller: &mut mir::Function,
    tree: &mut mir::NodeTree,
    callee: &mir::Function,
    argument_map: &HashMap<mir::Value, mir::Value>,
) -> (
    HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
    HashMap<mir::Value, mir::Value>,
) {
    // seed value mappings with function arguments
    let mut value_map = argument_map.clone();
    let mut block_map = HashMap::new();

    // clone each callee block and allocate new values
    for block_id in &callee.blocks {
        // load the original callee block
        let original = tree.get(*block_id);

        // allocate new values for block parameters
        let new_params: Vec<mir::TypedValue> = original
            .parameters
            .iter()
            .map(|param| {
                let new_value = caller.next_value();
                value_map.insert(param.value, new_value);
                mir::TypedValue {
                    value: new_value,
                    ty: param.ty,
                }
            })
            .collect();

        // allocate new values for instruction destinations
        for &instruction_id in &original.instructions {
            let instruction = tree.get(instruction_id);
            if let Some(destination) = instruction.destination() {
                let new_value = caller.next_value();
                value_map.insert(destination, new_value);
            }
        }

        // create the empty cloned block
        let new_block = mir::Block {
            parameters: new_params,
            instructions: Vec::new(),
            terminator: original.terminator.clone(),
        };
        let new_block_id = tree.insert(new_block);
        block_map.insert(*block_id, new_block_id);
        caller.blocks.push(new_block_id);
    }

    (block_map, value_map)
}

/// Split the caller block around a call and create the continuation block.
#[allow(clippy::too_many_arguments)]
fn split_block_for_inline(
    caller: &mut mir::Function,
    tree: &mut mir::NodeTree,
    block_id: mir::LocalNodeId<mir::Block>,
    call_index: usize,
    call_instruction_id: mir::LocalNodeId<mir::Instruction>,
    inline_entry: mir::LocalNodeId<mir::Block>,
    return_type: mir::LocalNodeId<mir::Type>,
    destination: Option<mir::Value>,
    entry_params: &[mir::TypedValue],
    argument_map: &HashMap<mir::Value, mir::Value>,
) -> Option<InlineSplit> {
    // load the call block for editing
    let mut block = tree.get(block_id).clone();
    if call_index >= block.instructions.len() {
        return None;
    }
    if block.instructions[call_index] != call_instruction_id {
        return None;
    }

    // build the continuation block
    let mut continuation_block = mir::Block::new();
    let mut result_value = None;

    // allocate a continuation parameter when a value is returned
    if destination.is_some() {
        let new_value = caller.next_value();
        continuation_block.parameters.push(mir::TypedValue {
            value: new_value,
            ty: return_type,
        });
        result_value = Some(new_value);
    }

    // split instructions around the call
    let after_instructions = block.instructions.split_off(call_index + 1);
    let removed = block.instructions.pop();
    if removed != Some(call_instruction_id) {
        return None;
    }
    // preserve the original terminator for the continuation
    let original_terminator = block.terminator.clone();

    // build jump arguments for the inlined entry block
    let mut entry_arguments = Vec::new();
    for param in entry_params {
        let argument = argument_map.get(&param.value).copied()?;
        entry_arguments.push(argument);
    }

    // replace the call with a jump to the inlined entry
    block.terminator = mir::Terminator::Jump {
        target: inline_entry,
        arguments: entry_arguments,
    };
    tree.replace(block_id, block);

    // finish the continuation block
    continuation_block.instructions = after_instructions;
    continuation_block.terminator = original_terminator;
    // insert the continuation block into the caller
    let continuation_id = tree.insert(continuation_block);
    caller.blocks.push(continuation_id);

    Some(InlineSplit {
        continuation_id,
        result_value,
    })
}

/// Substitute a value inside a single block.
fn substitute_value_in_block(
    tree: &mut mir::NodeTree,
    block_id: mir::LocalNodeId<mir::Block>,
    from: mir::Value,
    to: mir::Value,
) {
    // build substitution map for a single replacement
    let mut substitutions = HashMap::new();
    substitutions.insert(from, to);

    // update instructions inside the block
    let block = tree.get(block_id).clone();
    for instruction_id in &block.instructions {
        let instruction = tree.get(*instruction_id).clone();
        let updated = instruction_substitute_uses_in_tree(&instruction, &substitutions, tree);
        tree.replace(*instruction_id, updated);
    }

    // update the block terminator
    let mut updated_block = block.clone();
    updated_block.terminator = terminator_substitute_uses(&block.terminator, &substitutions);
    tree.replace(block_id, updated_block);
}

/// Remap values and locals in inlined blocks.
fn remap_inline_blocks(
    tree: &mut mir::NodeTree,
    callee: &mir::Function,
    block_map: &HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
    value_map: &HashMap<mir::Value, mir::Value>,
    local_map: &HashMap<mir::LocalNodeId<mir::Local>, mir::LocalNodeId<mir::Local>>,
) {
    // clone instruction bodies and remap terminators for each block
    for block_id in &callee.blocks {
        // load the original and cloned blocks
        let new_block_id = block_map[block_id];
        let original_block = tree.get(*block_id);
        let original_instructions = original_block.instructions.clone();
        let original_terminator = original_block.terminator.clone();
        let mut new_block = tree.get(new_block_id).clone();

        // clone instructions with remapped values
        let mut new_instructions = Vec::with_capacity(original_instructions.len());
        for instruction_id in original_instructions {
            // remap the instruction operands and destination
            let instruction = tree.get(instruction_id).clone();
            let remapped = instruction_inline_map(&instruction, value_map, local_map, tree);
            let new_id = tree.insert(remapped);

            // clone call metadata onto the new instruction
            if let Some(metadata) = tree.call_table.call_metadata(instruction_id).cloned() {
                tree.call_table.insert_call_metadata(new_id, metadata);
            }

            // clone memory access metadata onto the new instruction
            if let Some(accesses) = tree
                .memory_table
                .memory_accesses(instruction_id)
                .map(|entries| entries.to_vec())
            {
                tree.memory_table.insert_memory_accesses(new_id, accesses);
            }

            // clone debug locations onto the new instruction
            if let Some(location) = tree
                .debug_info
                .instruction_locations
                .get(&instruction_id)
                .cloned()
            {
                tree.debug_info
                    .instruction_locations
                    .insert(new_id, location);
            }
            new_instructions.push(new_id);
        }

        // remap the terminator and commit the new block body
        new_block.instructions = new_instructions;
        new_block.terminator = original_terminator;
        terminator_remap(&mut new_block.terminator, block_map, value_map);
        tree.replace(new_block_id, new_block);
    }
}

/// Rewrite return terminators in inlined blocks to jump to the continuation.
fn rewrite_inlined_returns(
    tree: &mut mir::NodeTree,
    block_map: &HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
    continuation: mir::LocalNodeId<mir::Block>,
    expects_value: bool,
) {
    // rewrite return terminators to jump to the continuation
    for &new_block_id in block_map.values() {
        // skip blocks that do not return
        let mut block = tree.get(new_block_id).clone();
        let mir::Terminator::Return { value } = block.terminator else {
            continue;
        };

        // forward return values when a result is expected
        let mut arguments = Vec::new();
        if expects_value && let Some(value) = value {
            arguments.push(value);
        }

        // replace the return with a jump to the continuation
        block.terminator = mir::Terminator::Jump {
            target: continuation,
            arguments,
        };
        tree.replace(new_block_id, block);
    }
}

/// Check whether a function contains tail call terminators.
fn has_tail_calls(tree: &mir::NodeTree, function: &mir::Function) -> bool {
    // scan terminators for tail call forms
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        if matches!(
            block.terminator,
            mir::Terminator::TailCall { .. } | mir::Terminator::TailCallIndirect { .. }
        ) {
            return true;
        }
    }

    false
}

/// Remap instruction values and locals for inlining.
fn instruction_inline_map(
    instruction: &mir::Instruction,
    value_map: &HashMap<mir::Value, mir::Value>,
    local_map: &HashMap<mir::LocalNodeId<mir::Local>, mir::LocalNodeId<mir::Local>>,
    tree: &mut mir::NodeTree,
) -> mir::Instruction {
    // create a value remapper for simple value uses
    let remap = |value: mir::Value| -> mir::Value { *value_map.get(&value).unwrap_or(&value) };

    // remap argument slices into a new argument buffer entry
    let mut remap_arguments = |slice: mir::ArgumentSlice| -> mir::ArgumentSlice {
        let new_args: Vec<_> = tree
            .get_arguments(slice)
            .iter()
            .map(|&v| remap(v))
            .collect();
        tree.add_arguments(&new_args)
    };

    // remap each instruction variant
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
        mir::Instruction::LocalGet { destination, local } => {
            let local = local_map.get(local).copied().unwrap_or(*local);
            mir::Instruction::LocalGet {
                destination: remap(*destination),
                local,
            }
        }
        mir::Instruction::LocalSet { local, value } => {
            let local = local_map.get(local).copied().unwrap_or(*local);
            mir::Instruction::LocalSet {
                local,
                value: remap(*value),
            }
        }
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
        mir::Instruction::RawDrop { value } => mir::Instruction::RawDrop {
            value: remap(*value),
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
        mir::Instruction::StackDrop { value } => mir::Instruction::StackDrop {
            value: remap(*value),
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

/// Check whether a call is recursive via SCC membership.
fn is_recursive_call(
    caller: mir::LocalNodeId<mir::Function>,
    callee: mir::LocalNodeId<mir::Function>,
    scc_map: &CallGraphScc,
) -> bool {
    // load the caller scc id
    let Some(caller_scc) = scc_map.scc_id(caller) else {
        return false;
    };

    // load the callee scc id
    let Some(callee_scc) = scc_map.scc_id(callee) else {
        return false;
    };

    // short circuit when the functions are in different sccs
    if caller_scc != callee_scc {
        return false;
    }

    // report recursion if the scc is marked recursive
    scc_map.is_recursive_scc(caller_scc)
}

/// Summary of function size for inlining decisions.
#[derive(Debug, Clone, Copy, Default)]
struct FunctionSize {
    /// Instruction count for the function.
    instructions: usize,
    /// Block count for the function.
    blocks: usize,
    /// Call count for the function.
    calls: usize,
}

/// Compute the size summary for a single function.
fn function_size_for(tree: &mir::NodeTree, function: &mir::Function) -> FunctionSize {
    // count blocks, instructions, and callsites
    let mut size = FunctionSize {
        blocks: function.blocks.len(),
        ..FunctionSize::default()
    };

    // scan all blocks for instruction and call counts
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        size.instructions += block.instructions.len();

        // count call instructions in the block
        for &instruction_id in &block.instructions {
            if matches!(
                tree.get(instruction_id),
                mir::Instruction::Call { .. } | mir::Instruction::CallIndirect { .. }
            ) {
                size.calls += 1;
            }
        }

        // count tail calls in the terminator
        if matches!(
            block.terminator,
            mir::Terminator::TailCall { .. } | mir::Terminator::TailCallIndirect { .. }
        ) {
            size.calls += 1;
        }
    }

    size
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Simple direct calls are inlined.
    #[test]
    fn test_inline_basic_call() {
        let input = r#"function @callee(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iadd v0, v0
    return v1
}
function @caller(v0: i32) -> i32 {
block0(v0: i32):
    v1 = call @callee(v0)
    v2 = iadd v1, v0
    return v2
}"#;

        let expected = r#"function @callee(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iadd v0, v0
    return v1
}
function @caller(v0: i32) -> i32 {
block0(v0: i32):
    jump block1(v0)
block1(v3: i32):
    v4 = iadd v3, v3
    jump block2(v4)
block2(v5: i32):
    v2 = iadd v5, v0
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&Inline);
        program.assert_output(expected);
    }

    /// Recursive calls are not inlined.
    #[test]
    fn test_inline_skips_recursive_call() {
        let input = r#"function @caller(v0: i32) -> i32 {
block0(v0: i32):
    v1 = call @caller(v0)
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&Inline);
        program.assert_output(input);
    }

    /// Tail call callees are not inlined.
    #[test]
    fn test_inline_skips_tailcall_callee() {
        let input = r#"function @callee(v0: i32) -> i32 {
block0(v0: i32):
    tailcall @callee(v0)
}
function @caller(v0: i32) -> i32 {
block0(v0: i32):
    v1 = call @callee(v0)
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&Inline);
        program.assert_output(input);
    }

    /// Locals are cloned during inlining.
    #[test]
    fn test_inline_clones_locals() {
        let input = r#"function @callee(v0: i32) -> i32 {
    local0: i32 ; owned
block0(v0: i32):
    v1 = local.get local0
    v2 = iadd v1, v0
    return v2
}
function @caller(v0: i32) -> i32 {
block0(v0: i32):
    v1 = call @callee(v0)
    return v1
}"#;

        let expected = r#"function @callee(v0: i32) -> i32 {
    local0: i32 ; owned
block0(v0: i32):
    v1 = local.get local0
    v2 = iadd v1, v0
    return v2
}
function @caller(v0: i32) -> i32 {
    local0: i32 ; owned
block0(v0: i32):
    jump block1(v0)
block1(v2: i32):
    v3 = local.get local0
    v4 = iadd v3, v2
    jump block2(v4)
block2(v5: i32):
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&Inline);
        program.assert_output(expected);
    }

    /// Large callees are not inlined.
    #[test]
    fn test_inline_skips_large_callee() {
        let mut input = String::from("function @callee(v0: i32) -> i32 {\n");
        input.push_str("block0(v0: i32):\n");
        input.push_str("    v1 = iadd v0, v0\n");
        for index in 2..=97 {
            input.push_str(&format!("    v{index} = iadd v{}, v0\n", index - 1));
        }
        input.push_str("    return v97\n");
        input.push_str("}\n");
        input.push_str("function @caller(v0: i32) -> i32 {\n");
        input.push_str("block0(v0: i32):\n");
        input.push_str("    v1 = call @callee(v0)\n");
        input.push_str("    return v1\n");
        input.push_str("}\n");

        let mut program = TestProgram::new(&input);
        program.run_module_pass(&Inline);
        program.assert_output(&input);
    }

    /// Calls with unused return values inline without continuation arguments.
    #[test]
    fn test_inline_unused_return() {
        let input = r#"function @callee(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iadd v0, v0
    return v1
}
function @caller(v0: i32) -> void {
block0(v0: i32):
    call @callee(v0)
    return
}"#;

        let expected = r#"function @callee(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iadd v0, v0
    return v1
}
function @caller(v0: i32) -> void {
block0(v0: i32):
    jump block1(v0)
block1(v1: i32):
    v2 = iadd v1, v1
    jump block2
block2:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&Inline);
        program.assert_output(expected);
    }

    /// Inline replaces multiple returns with a continuation.
    #[test]
    fn test_inline_multiple_returns() {
        let input = r#"function @callee(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    branch v2, block1, block2
block1:
    v3 = iadd v0, v1
    return v3
block2:
    v4 = isub v0, v1
    return v4
}
function @caller(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    v3 = call @callee(v0, v1, v2)
    return v3
}"#;

        let expected = r#"function @callee(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    branch v2, block1, block2
block1:
    v3 = iadd v0, v1
    return v3
block2:
    v4 = isub v0, v1
    return v4
}
function @caller(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    jump block1(v0, v1, v2)
block1(v4: i32, v5: i32, v6: bool):
    branch v6, block2, block3
block2:
    v7 = iadd v4, v5
    jump block4(v7)
block3:
    v8 = isub v4, v5
    jump block4(v8)
block4(v9: i32):
    return v9
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&Inline);
        program.assert_output(expected);
    }

    /// Inline forwards call results into continuation terminators.
    #[test]
    fn test_inline_continuation_argument() {
        let input = r#"function @callee(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iadd v0, v0
    return v1
}
function @caller(v0: i32) -> i32 {
block0(v0: i32):
    v1 = call @callee(v0)
    jump block1(v1)
block1(v2: i32):
    return v2
}"#;

        let expected = r#"function @callee(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iadd v0, v0
    return v1
}
function @caller(v0: i32) -> i32 {
block0(v0: i32):
    jump block2(v0)
block1(v2: i32):
    return v2
block2(v3: i32):
    v4 = iadd v3, v3
    jump block3(v4)
block3(v5: i32):
    jump block1(v5)
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&Inline);
        program.assert_output(expected);
    }

    /// Call indirect sites inline when metadata resolves a direct target.
    #[test]
    fn test_inline_indirect_with_metadata() {
        let input = r#"function @callee(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iadd v0, v0
    return v1
}
function @caller(v0: fn(i32) -> i32, v1: i32) -> i32 {
block0(v0: fn(i32) -> i32, v1: i32):
    v2 = call.indirect v0(v1) -> fn(i32) -> i32
    return v2
}"#;

        let expected = r#"function @callee(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iadd v0, v0
    return v1
}
function @caller(v0: fn(i32) -> i32, v1: i32) -> i32 {
block0(v0: fn(i32) -> i32, v1: i32):
    jump block1(v1)
block1(v3: i32):
    v4 = iadd v3, v3
    jump block2(v4)
block2(v5: i32):
    return v5
}"#;

        let mut program = TestProgram::new(input);
        let caller_id = program.function_id_by_name("caller");
        let callee_id = program.function_id_by_name("callee");
        let signature = program.call_signature_for_callee(callee_id);
        let entry_block = program.entry_block_id(caller_id);
        let call_instruction_id = program
            .instructions_in_block(entry_block)
            .into_iter()
            .find(|instruction_id| {
                matches!(
                    program.tree.get(*instruction_id),
                    mir::Instruction::CallIndirect { .. }
                )
            })
            .expect("missing call.indirect");

        program.tree.call_table.insert_call_metadata(
            call_instruction_id,
            mir::CallMetadata::direct(callee_id, signature),
        );

        program.run_module_pass(&Inline);
        program.assert_output(expected);
    }
}
