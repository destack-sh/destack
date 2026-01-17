use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    CallGraphScc, ConstantPropagation, constant_propagation_with_params,
};
use crate::optimize::common::{
    ParameterRemap, SignatureKey, apply_constant_parameters, build_signature_type,
    constant_arguments_for_parameters, required_parameter_indices,
};
use crate::optimize::passes::scalar::{
    DeadCodeEliminate, SimplifyCfg, SparseConditionalConstantPropagation,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, ModulePass, PipelineContext};

/// Minimum callsite count to treat a call as hot.
const MIN_SPECIALIZE_CALL_COUNT: u64 = 20;
/// Minimum callsite ratio to treat a call as hot.
const MIN_SPECIALIZE_RATIO: f64 = 0.20;
/// Maximum specializations per function.
const MAX_SPECIALIZE_PER_FUNCTION: usize = 4;
/// Maximum specializations per module.
const MAX_SPECIALIZE_TOTAL: usize = 32;

declare_pass! {
    /// Clone functions for constant argument callsites.
    ///
    /// This pass clones a callee for callsites with constant arguments and
    /// rewrites those callsites to target the specialized clone. The clone
    /// substitutes constant parameters and drops removable parameters.
    ///
    /// ```mir
    /// function @callee(v0: i32, v1: i32) -> i32 {
    /// block0(v0: i32, v1: i32):
    ///     v2 = iadd v0, v1
    ///     return v2
    /// }
    /// function @root() -> i32 {
    /// block0:
    ///     v0 = iconst 2i32
    ///     v1 = iconst 3i32
    ///     v2 = call @callee(v0, v1)
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @callee(v0: i32, v1: i32) -> i32 {
    /// block0(v0: i32, v1: i32):
    ///     v2 = iadd v0, v1
    ///     return v2
    /// }
    /// function @callee$spec0() -> i32 {
    /// block0:
    ///     v0 = iconst 2i32
    ///     v1 = iconst 3i32
    ///     v2 = iadd v0, v1
    ///     return v2
    /// }
    /// function @root() -> i32 {
    /// block0:
    ///     v0 = iconst 2i32
    ///     v1 = iconst 3i32
    ///     v2 = call @callee$spec0()
    ///     return v2
    /// }
    /// ```
    #[pass(id = "argument-specialize")]
    pub ArgumentSpecialize,
    "Clone functions for constant argument callsites"
}

impl ModulePass for ArgumentSpecialize {
    /// Run argument specialization for the module.
    fn run(&self, tree: &mut mir::NodeTree, ctx: &PipelineContext<'_>) -> AnalysisPreservation {
        // run the specialization pass
        let changed = run_argument_specialize(tree, ctx);

        // report analysis preservation based on whether changes occurred
        if changed {
            ctx.strings.intern("argument-specialize");
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass display name.
    fn name(&self) -> &'static str {
        "ArgumentSpecialize"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "argument-specialize"
    }
}

/// Direct callsite metadata for specialization.
#[derive(Debug, Clone)]
struct DirectCallSite {
    /// The caller function id.
    caller: mir::LocalNodeId<mir::Function>,
    /// The callee function id.
    callee: mir::LocalNodeId<mir::Function>,
    /// The block containing the call.
    block: mir::LocalNodeId<mir::Block>,
    /// The call instruction.
    call_instruction: mir::LocalNodeId<mir::Instruction>,
    /// The arguments passed at the callsite.
    arguments: Vec<mir::Value>,
}

/// Collected callsite data for specialization.
#[derive(Debug, Default)]
struct CallData {
    /// Direct callsites in the module.
    callsites: Vec<DirectCallSite>,
    /// Signatures that may be targeted by indirect calls.
    indirect_signatures: HashSet<SignatureKey>,
}

/// Key used to deduplicate specializations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SpecializationKey {
    /// The callee function id.
    callee: mir::LocalNodeId<mir::Function>,
    /// Constant arguments by parameter index.
    constants: Vec<(usize, ConstantKey)>,
}

/// Hashable representation of a constant value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ConstantKey {
    /// Boolean constant.
    Boolean(bool),
    /// Signed integer constant.
    Int { value: i64, width: u8, signed: bool },
    /// Unsigned integer constant.
    UInt { value: u64, width: u8 },
    /// Floating point constant.
    Float { bits: u64, width: u8 },
    /// Character constant.
    Char(char),
    /// String constant.
    String(String),
}

/// Run argument specialization over the module.
fn run_argument_specialize(tree: &mut mir::NodeTree, ctx: &PipelineContext<'_>) -> bool {
    // collect callsite information
    let call_data = collect_call_data(tree);

    // collect call graph sccs for recursion checks
    let analyses = ctx.module_analyses(tree);
    let scc_map = analyses.get::<CallGraphScc>();

    // build constant propagation maps for callers
    let constants_by_function = build_constant_maps(tree, ctx.type_context());

    // track specialization state
    let mut changed = false;
    let mut specialization_cache: HashMap<SpecializationKey, mir::LocalNodeId<mir::Function>> =
        HashMap::new();
    let mut specialization_counts: HashMap<mir::LocalNodeId<mir::Function>, usize> = HashMap::new();
    let mut total_specializations = 0usize;

    // process callsites for specialization
    for callsite in &call_data.callsites {
        // skip recursive callees
        if scc_map.is_recursive_function(callsite.callee) {
            continue;
        }

        // skip extern callees
        if tree.get(callsite.callee).entry.is_none() {
            continue;
        }

        // skip callsites without constant arguments
        let Some(constants) = callsite_constants(callsite, &constants_by_function, tree, ctx)
        else {
            continue;
        };

        // skip callsites with no constant values
        if constants.iter().all(|constant| constant.is_none()) {
            continue;
        }

        // skip cold callsites when profile data is present
        if !callsite_is_hot(callsite, ctx.profile()) {
            continue;
        }

        // compute removal indices for constant parameters
        let removal_indices = removable_constant_parameters(tree.get(callsite.callee), &constants);

        // compute specialization key and check cache
        let key = specialization_key(callsite.callee, &constants);
        let new_callee = if let Some(existing) = specialization_cache.get(&key) {
            *existing
        } else {
            let count = specialization_counts.entry(callsite.callee).or_insert(0);
            if *count >= MAX_SPECIALIZE_PER_FUNCTION {
                continue;
            }
            if total_specializations >= MAX_SPECIALIZE_TOTAL {
                continue;
            }

            // clone and specialize the callee
            let new_callee = specialize_callee(
                callsite.callee,
                *count,
                &constants,
                &removal_indices,
                tree,
                ctx,
            );
            specialization_cache.insert(key, new_callee);
            *count += 1;
            total_specializations += 1;
            new_callee
        };

        // update the callsite to use the specialized clone
        if update_callsite(callsite, new_callee, &removal_indices, tree) {
            changed = true;
        }
    }

    changed
}

/// Collect direct callsites and indirect signatures for the module.
fn collect_call_data(tree: &mir::NodeTree) -> CallData {
    // prepare the callsite data container
    let mut data = CallData::default();

    // scan each function body for callsites
    for (caller_id, function) in tree.iter_nodes::<mir::Function>() {
        // skip extern functions
        if function.entry.is_none() {
            continue;
        }

        for &block_id in &function.blocks {
            let block = tree.get(block_id);

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);

                // record direct callsites
                if let mir::Instruction::Call {
                    function: callee,
                    arguments,
                    ..
                } = instruction
                {
                    let dispatch = tree
                        .call_table
                        .call_metadata(instruction_id)
                        .map(|meta| meta.dispatch)
                        .unwrap_or(mir::CallDispatchKind::Direct);

                    if matches!(dispatch, mir::CallDispatchKind::Direct) {
                        let arguments = tree.get_arguments(*arguments).to_vec();
                        data.callsites.push(DirectCallSite {
                            caller: caller_id,
                            callee: *callee,
                            block: block_id,
                            call_instruction: instruction_id,
                            arguments,
                        });
                        continue;
                    } else if let Some(signature) = tree
                        .call_table
                        .call_metadata(instruction_id)
                        .and_then(|meta| SignatureKey::from_signature_type(tree, meta.signature))
                    {
                        data.indirect_signatures.insert(signature);
                        continue;
                    }
                }

                // record signatures for call.indirect instructions
                if let mir::Instruction::CallIndirect { signature, .. } = instruction
                    && let Some(signature) = SignatureKey::from_signature_type(tree, *signature)
                {
                    data.indirect_signatures.insert(signature);
                }
            }
        }
    }

    data
}

/// Build constant propagation data for each defined function.
fn build_constant_maps(
    tree: &mir::NodeTree,
    type_context: crate::optimize::TypeContext,
) -> HashMap<mir::LocalNodeId<mir::Function>, ConstantPropagation> {
    // prepare the constants map
    let mut maps = HashMap::new();

    // build a constant propagation analysis per function
    for (function_id, function) in tree.iter_nodes::<mir::Function>() {
        if function.entry.is_none() {
            continue;
        }

        let constants =
            constant_propagation_with_params(function, tree, type_context, &HashMap::new());
        maps.insert(function_id, constants);
    }

    maps
}

/// Resolve constant arguments for a callsite.
fn callsite_constants(
    callsite: &DirectCallSite,
    constants_by_function: &HashMap<mir::LocalNodeId<mir::Function>, ConstantPropagation>,
    tree: &mir::NodeTree,
    ctx: &PipelineContext<'_>,
) -> Option<Vec<Option<mir::Constant>>> {
    // read the caller constant propagation state
    let constants = constants_by_function.get(&callsite.caller)?;
    let block_constants = constants.exit(callsite.block);

    // read the callee parameter list
    let callee = tree.get(callsite.callee);

    // resolve argument constants with type checks
    constant_arguments_for_parameters(
        &callsite.arguments,
        &callee.parameters,
        block_constants,
        ctx.type_context().pointer_width_bits,
        tree,
    )
}

/// Build a specialization key from constant arguments.
fn specialization_key(
    callee: mir::LocalNodeId<mir::Function>,
    constants: &[Option<mir::Constant>],
) -> SpecializationKey {
    // collect constants in index order
    let mut entries = Vec::new();
    for (index, constant) in constants.iter().enumerate() {
        // skip non constant arguments
        let Some(constant) = constant else {
            continue;
        };

        entries.push((index, constant_key(constant)));
    }

    SpecializationKey {
        callee,
        constants: entries,
    }
}

/// Convert a MIR constant into a hashable key.
fn constant_key(constant: &mir::Constant) -> ConstantKey {
    match constant {
        mir::Constant::Boolean { value } => ConstantKey::Boolean(*value),
        mir::Constant::Int {
            value,
            width,
            is_signed,
        } => ConstantKey::Int {
            value: *value,
            width: *width,
            signed: *is_signed,
        },
        mir::Constant::UInt { value, width } => ConstantKey::UInt {
            value: *value,
            width: *width,
        },
        mir::Constant::Float { bits, width } => ConstantKey::Float {
            bits: *bits,
            width: *width,
        },
        mir::Constant::Char { value } => ConstantKey::Char(*value),
        mir::Constant::String { value } => ConstantKey::String(value.clone()),
    }
}

/// Decide whether a callsite is hot enough to specialize.
fn callsite_is_hot(callsite: &DirectCallSite, profile: Option<&mir::ProfileTable>) -> bool {
    // allow specialization without profile data
    let Some(profile) = profile else {
        return true;
    };

    // read the callsite profile
    let Some(callsite_profile) = profile.callsite_profile(callsite.call_instruction) else {
        return false;
    };

    // accept highly executed callsites
    let total_count = callsite_profile.total_count.value;
    if total_count >= MIN_SPECIALIZE_CALL_COUNT {
        return true;
    }

    // compare callsite frequency against entry count when available
    let Some(function_profile) = profile.function_profile(callsite.caller) else {
        return false;
    };

    if function_profile.entry_count.value == 0 {
        return false;
    }

    let ratio = total_count as f64 / function_profile.entry_count.value as f64;
    ratio >= MIN_SPECIALIZE_RATIO
}

/// Specialize a callee by cloning and substituting constants.
fn specialize_callee(
    callee: mir::LocalNodeId<mir::Function>,
    spec_index: usize,
    constants: &[Option<mir::Constant>],
    removal_indices: &[usize],
    tree: &mut mir::NodeTree,
    ctx: &PipelineContext<'_>,
) -> mir::LocalNodeId<mir::Function> {
    // build the specialized function name
    let base_name = ctx.strings.get(tree.get(callee).name).to_string();
    let suffix = specialized_suffix(spec_index);
    let name = ctx.strings.intern(&format!("{base_name}{suffix}"));

    // clone the function body
    let new_function_id = clone_function(callee, name, tree);

    // insert constant parameters into the clone
    apply_constant_parameters(new_function_id, constants, tree);

    // remove parameters that are constant and not required
    if !removal_indices.is_empty() {
        let remap = ParameterRemap::new(removal_indices);
        apply_parameter_removals(new_function_id, &remap, tree);
    }

    // run cleanup passes on the specialized clone
    run_specialization_cleanup(new_function_id, tree, ctx);
    new_function_id
}

/// Create a suffix for specialized function names.
fn specialized_suffix(spec_index: usize) -> String {
    format!("$spec{spec_index}")
}

/// Clone a function body for specialization.
fn clone_function(
    function_id: mir::LocalNodeId<mir::Function>,
    name: destack_base::StringId,
    tree: &mut mir::NodeTree,
) -> mir::LocalNodeId<mir::Function> {
    // read the original function
    let original = tree.get(function_id).clone();

    // clone locals for the function
    let mut local_map = HashMap::new();
    let mut new_locals = Vec::new();
    for local_id in &original.locals {
        let local = tree.get(*local_id).clone();
        let new_local = tree.insert(local);
        local_map.insert(*local_id, new_local);
        new_locals.push(new_local);
    }

    // create block placeholders
    let mut block_map = HashMap::new();
    for block_id in &original.blocks {
        let block = tree.get(*block_id);
        let new_block = mir::Block {
            parameters: block.parameters.clone(),
            instructions: Vec::new(),
            terminator: block.terminator.clone(),
        };
        let new_block_id = tree.insert(new_block);
        block_map.insert(*block_id, new_block_id);
    }

    // clone instructions into new blocks
    let value_map = HashMap::new();
    for block_id in &original.blocks {
        let new_block_id = block_map[block_id];
        let instruction_ids = tree.get(*block_id).instructions.clone();
        let mut new_instructions = Vec::with_capacity(instruction_ids.len());

        for instruction_id in instruction_ids {
            // remap instruction operands to the cloned locals and values
            let instruction = tree.get(instruction_id).clone();
            let remapped = crate::optimize::common::instruction_map_with_locals(
                &instruction,
                &value_map,
                &local_map,
                tree,
            );
            let new_id = tree.insert(remapped);

            // preserve call metadata for the cloned instruction
            if let Some(metadata) = tree.call_table.call_metadata(instruction_id).cloned() {
                tree.call_table.insert_call_metadata(new_id, metadata);
            }

            // preserve memory access metadata for the cloned instruction
            if let Some(accesses) = tree
                .memory_table
                .memory_accesses(instruction_id)
                .map(|entries| entries.to_vec())
            {
                tree.memory_table.insert_memory_accesses(new_id, accesses);
            }

            // preserve debug locations for the cloned instruction
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

        // update the cloned block instruction list
        let mut new_block = tree.get(new_block_id).clone();
        new_block.instructions = new_instructions;
        tree.replace(new_block_id, new_block);
    }

    // remap terminators with new block ids
    for block_id in &original.blocks {
        let new_block_id = block_map[block_id];
        let mut new_block = tree.get(new_block_id).clone();
        let mut terminator = new_block.terminator.clone();
        crate::optimize::common::terminator_remap(&mut terminator, &block_map, &value_map);
        new_block.terminator = terminator;
        tree.replace(new_block_id, new_block);
    }

    // build the new function
    let mut new_function = original.clone();
    new_function.name = name;
    new_function.linkage = mir::Linkage::Local;
    new_function.locals = new_locals;
    new_function.blocks = original
        .blocks
        .iter()
        .map(|block_id| block_map[block_id])
        .collect();
    new_function.entry = original.entry.map(|entry| block_map[&entry]);

    // insert the specialized function
    let new_function_id = tree.insert(new_function);
    // recompute value id state for the clone
    let mut cloned_function = tree.get(new_function_id).clone();
    cloned_function.recompute_next_value_id(tree);
    *tree.get_mut(new_function_id) = cloned_function;
    new_function_id
}

/// Identify constant parameters that can be removed.
fn removable_constant_parameters(
    function: &mir::Function,
    constants: &[Option<mir::Constant>],
) -> Vec<usize> {
    // collect required parameter indices
    let required = required_parameter_indices(function);

    // collect removable indices
    let mut removable = Vec::new();
    for (index, constant) in constants.iter().enumerate() {
        // skip non constant parameters
        if constant.is_none() {
            continue;
        }

        // skip required parameters
        if required.contains(&index) {
            continue;
        }

        removable.push(index);
    }

    removable
}

/// Apply parameter removals to a specialized function.
fn apply_parameter_removals(
    function_id: mir::LocalNodeId<mir::Function>,
    remap: &ParameterRemap,
    tree: &mut mir::NodeTree,
) {
    // update function parameters and attributes
    let entry_id = {
        let function = tree.get_mut(function_id);
        function.parameters = remap.filter_by_index(&function.parameters);
        function.parameter_attributes = remap.filter_by_index(&function.parameter_attributes);
        function.return_lifetime = remap
            .remap_return_lifetime(&function.return_lifetime)
            .unwrap_or(mir::Lifetime::Inferred);
        function.alloc_size = remap.remap_alloc_size(function.alloc_size);
        function.entry.expect("defined function has entry block")
    };

    // update entry block parameters to match the new signature
    let entry = tree.get_mut(entry_id);
    entry.parameters = remap.filter_by_index(&entry.parameters);
}

/// Update a callsite to invoke a specialized clone.
fn update_callsite(
    callsite: &DirectCallSite,
    new_callee: mir::LocalNodeId<mir::Function>,
    removal_indices: &[usize],
    tree: &mut mir::NodeTree,
) -> bool {
    // prepare removal remapping data
    let remap = ParameterRemap::new(removal_indices);

    // update the call instruction
    let (destination, slice) = match tree.get(callsite.call_instruction) {
        mir::Instruction::Call {
            destination,
            arguments,
            ..
        } => (*destination, *arguments),
        _ => return false,
    };

    // filter the argument list to match the specialized signature
    let arguments = remap.filter_by_index(tree.get_arguments(slice));
    let new_slice = tree.add_arguments(&arguments);
    let updated = mir::Instruction::Call {
        destination,
        function: new_callee,
        arguments: new_slice,
    };
    tree.replace(callsite.call_instruction, updated);

    // build the updated signature type before borrowing call metadata
    let signature_type = if remap.removal_indices().is_empty() {
        None
    } else {
        Some(build_signature_type(new_callee, tree))
    };

    // update call metadata when present
    if let Some(metadata) = tree.call_table.call_metadata_mut(callsite.call_instruction) {
        metadata.declared_target = Some(new_callee);
        metadata.argument_metadata = remap.filter_by_index(&metadata.argument_metadata);
        metadata.alloc_size = remap.remap_alloc_size(metadata.alloc_size);

        // refresh the signature for removed arguments
        if let Some(signature_type) = signature_type {
            metadata.signature = signature_type;
        }
    }

    true
}

/// Run cleanup passes on a specialized function.
fn run_specialization_cleanup(
    function_id: mir::LocalNodeId<mir::Function>,
    tree: &mut mir::NodeTree,
    ctx: &PipelineContext<'_>,
) {
    // clone the function for mutation
    let mut function = tree.get(function_id).clone();

    // skip extern functions
    if function.entry.is_none() {
        return;
    }

    // run sccp
    let sccp = SparseConditionalConstantPropagation;
    function.recompute_next_value_id(tree);
    sccp.run(&mut function, tree, ctx);

    // run simplify cfg
    let simplify = SimplifyCfg;
    function.recompute_next_value_id(tree);
    simplify.run(&mut function, tree, ctx);

    // run dead code elimination
    let dce = DeadCodeEliminate;
    function.recompute_next_value_id(tree);
    dce.run(&mut function, tree, ctx);

    // write back the updated function
    *tree.get_mut(function_id) = function;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Constant callsites are specialized into clones.
    #[test]
    fn test_argument_specialize_clones_constant_call() {
        let input = r#"function @callee(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}
function @root() -> i32 {
block0:
    v0 = iconst 2i32
    v1 = iconst 3i32
    v2 = call @callee(v0, v1)
    return v2
}"#;

        let expected = r#"function @callee(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}
function @root() -> i32 {
block0:
    v0 = iconst 2i32
    v1 = iconst 3i32
    v2 = call @callee$spec0()
    return v2
}
function @callee$spec0() -> i32 {
block0:
    v2 = iconst 5i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&ArgumentSpecialize);
        program.assert_output(expected);
    }

    /// Call metadata is remapped after specialization.
    #[test]
    fn test_argument_specialize_updates_call_metadata() {
        let input = r#"function @callee(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}
function @root() -> i32 {
block0:
    v0 = iconst 2i32
    v1 = iconst 3i32
    v2 = call @callee(v0, v1)
    return v2
}"#;

        let expected = r#"function @callee(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}
function @root() -> i32 {
block0:
    v0 = iconst 2i32
    v1 = iconst 3i32
    v2 = call @callee$spec0()
    return v2
}
function @callee$spec0() -> i32 {
block0:
    v2 = iconst 5i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        let root_id = program.function_id_by_name("root");
        let (call_id, callee_id) = program.first_call_in_entry(root_id);
        let signature = program.call_signature_for_callee(callee_id);
        let argument_metadata = vec![mir::CallArgumentMetadata::default(); 2];
        let call_metadata = mir::CallMetadata::direct(callee_id, signature)
            .with_argument_metadata(argument_metadata);
        program
            .tree
            .call_table
            .insert_call_metadata(call_id, call_metadata);

        program.run_module_pass(&ArgumentSpecialize);
        program.assert_output(expected);

        let (call_id, callee_id) = program.first_call_in_entry(root_id);
        let callee = program.tree.get(callee_id);
        let call_metadata = program
            .tree
            .call_table
            .call_metadata(call_id)
            .expect("missing call metadata");
        let signature = program.tree.get(call_metadata.signature);
        let expected_signature = mir::Type::FunctionPointer {
            parameters: callee.parameters.iter().map(|param| param.ty).collect(),
            result: callee.return_type,
        };

        assert_eq!(call_metadata.declared_target, Some(callee_id));
        assert!(call_metadata.argument_metadata.is_empty());
        assert_eq!(signature, &expected_signature);
    }

    /// Cold callsites do not trigger specialization with profile data.
    #[test]
    fn test_argument_specialize_skips_cold_callsite() {
        let input = r#"function @callee(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}
function @root() -> i32 {
block0:
    v0 = iconst 2i32
    v1 = iconst 3i32
    v2 = call @callee(v0, v1)
    return v2
}"#;

        let mut program = TestProgram::new(input);
        let root_id = program.function_id_by_name("root");
        let (call_id, _) = program.first_call_in_entry(root_id);

        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        program.record_function_profile(&mut profile, root_id, 100);
        program.record_callsite_profile(&mut profile, call_id, 5);

        program.run_module_pass_with_profile(&ArgumentSpecialize, profile);
        program.assert_output(input);
    }

    /// Missing callsite profiles prevent specialization.
    #[test]
    fn test_argument_specialize_skips_missing_callsite_profile() {
        let input = r#"function @callee(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}
function @root() -> i32 {
block0:
    v0 = iconst 2i32
    v1 = iconst 3i32
    v2 = call @callee(v0, v1)
    return v2
}"#;

        let mut program = TestProgram::new(input);
        let root_id = program.function_id_by_name("root");

        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        program.record_function_profile(&mut profile, root_id, 100);

        program.run_module_pass_with_profile(&ArgumentSpecialize, profile);
        program.assert_output(input);
    }

    /// Missing function profiles prevent specialization below the hot threshold.
    #[test]
    fn test_argument_specialize_skips_missing_function_profile() {
        let input = r#"function @callee(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}
function @root() -> i32 {
block0:
    v0 = iconst 2i32
    v1 = iconst 3i32
    v2 = call @callee(v0, v1)
    return v2
}"#;

        let mut program = TestProgram::new(input);
        let root_id = program.function_id_by_name("root");
        let (call_id, _) = program.first_call_in_entry(root_id);

        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        program.record_callsite_profile(&mut profile, call_id, 5);

        program.run_module_pass_with_profile(&ArgumentSpecialize, profile);
        program.assert_output(input);
    }

    /// Hot callsites specialize when profile data is present.
    #[test]
    fn test_argument_specialize_uses_hot_callsite() {
        let input = r#"function @callee(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}
function @root() -> i32 {
block0:
    v0 = iconst 2i32
    v1 = iconst 3i32
    v2 = call @callee(v0, v1)
    return v2
}"#;

        let expected = r#"function @callee(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}
function @root() -> i32 {
block0:
    v0 = iconst 2i32
    v1 = iconst 3i32
    v2 = call @callee$spec0()
    return v2
}
function @callee$spec0() -> i32 {
block0:
    v2 = iconst 5i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        let root_id = program.function_id_by_name("root");
        let (call_id, _) = program.first_call_in_entry(root_id);

        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        program.record_function_profile(&mut profile, root_id, 100);
        program.record_callsite_profile(&mut profile, call_id, 25);

        program.run_module_pass_with_profile(&ArgumentSpecialize, profile);
        program.assert_output(expected);
    }

    /// Required alloc size parameters are not removed.
    #[test]
    fn test_argument_specialize_keeps_alloc_size_param() {
        let input = r#"function @callee(v0: i32) -> i32 {
block0(v0: i32):
    return v0
}
function @root() -> i32 {
block0:
    v0 = iconst 7i32
    v1 = call @callee(v0)
    return v1
}"#;

        let expected = r#"function @callee(v0: i32) -> i32 {
block0(v0: i32):
    return v0
}
function @root() -> i32 {
block0:
    v0 = iconst 7i32
    v1 = call @callee$spec0(v0)
    return v1
}
function @callee$spec0(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 7i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        let callee_id = program.function_id_by_name("callee");
        program.tree.get_mut(callee_id).alloc_size = Some(mir::AllocSize::new(0, None));

        program.run_module_pass(&ArgumentSpecialize);
        program.assert_output(expected);
    }

    /// Recursive callees are not specialized.
    #[test]
    fn test_argument_specialize_skips_recursive() {
        let input = r#"function @callee(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    v2 = icmp_slt v0, v1
    branch v2, block1, block2
block1:
    return v0
block2:
    v3 = iconst 1i32
    v4 = isub v0, v3
    v5 = call @callee(v4)
    return v5
}
function @root() -> i32 {
block0:
    v0 = iconst 9i32
    v1 = call @callee(v0)
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&ArgumentSpecialize);
        program.assert_output(input);
    }

    /// Extern callees are not specialized.
    #[test]
    fn test_argument_specialize_skips_extern() {
        let input = r#"extern function @callee(i32) -> i32
function @root() -> i32 {
block0:
    v0 = iconst 2i32
    v1 = call @callee(v0)
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&ArgumentSpecialize);
        program.assert_output(input);
    }

    /// Specialization stops at the per function limit.
    #[test]
    fn test_argument_specialize_respects_function_limit() {
        let input = r#"function @callee(v0: i32) -> i32 {
block0(v0: i32):
    return v0
}
function @root() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = iconst 3i32
    v3 = iconst 4i32
    v4 = iconst 5i32
    v5 = call @callee(v0)
    v6 = call @callee(v1)
    v7 = call @callee(v2)
    v8 = call @callee(v3)
    v9 = call @callee(v4)
    return v9
}"#;

        let expected = r#"function @callee(v0: i32) -> i32 {
block0(v0: i32):
    return v0
}
function @root() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = iconst 3i32
    v3 = iconst 4i32
    v4 = iconst 5i32
    v5 = call @callee$spec0()
    v6 = call @callee$spec1()
    v7 = call @callee$spec2()
    v8 = call @callee$spec3()
    v9 = call @callee(v4)
    return v9
}
function @callee$spec0() -> i32 {
block0:
    v1 = iconst 1i32
    return v1
}
function @callee$spec1() -> i32 {
block0:
    v1 = iconst 2i32
    return v1
}
function @callee$spec2() -> i32 {
block0:
    v1 = iconst 3i32
    return v1
}
function @callee$spec3() -> i32 {
block0:
    v1 = iconst 4i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&ArgumentSpecialize);
        program.assert_output(expected);
    }
}
