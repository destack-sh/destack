use std::collections::{HashMap, HashSet};

use crate::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    CallGraphScc, ConstantPropagation, constant_propagation_with_params,
};
use crate::optimize::common::{
    CallsiteHotness, ParameterRemap, SignatureKey, apply_constant_parameters, build_signature_type,
    callsite_hotness, clone_instruction_metadata, constant_arguments_for_parameters,
    instruction_map_with_locals, required_parameter_indices, run_function_passes_always,
    terminator_remap,
};
use crate::optimize::passes::scalar::{
    DeadCodeEliminate, SimplifyCfg, SparseConditionalConstantPropagation,
};
use crate::optimize::{AnalysisPreservation, ModulePass, PipelineContext};

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
    /// function callee(v0: int32, v1: int32): int32 {
    /// b0(v0: int32, v1: int32):
    ///     v2 = int.add v0, v1
    ///     return v2
    /// }
    /// function root(): int32 {
    /// b0:
    ///     v0 = 2int32
    ///     v1 = 3int32
    ///     v2 = call callee(v0, v1)
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function callee(v0: int32, v1: int32): int32 {
    /// b0(v0: int32, v1: int32):
    ///     v2 = int.add v0, v1
    ///     return v2
    /// }
    /// function callee$spec0(): int32 {
    /// b0:
    ///     v0 = 2int32
    ///     v1 = 3int32
    ///     v2 = int.add v0, v1
    ///     return v2
    /// }
    /// function root(): int32 {
    /// b0:
    ///     v0 = 2int32
    ///     v1 = 3int32
    ///     v2 = call callee$spec0()
    ///     return v2
    /// }
    /// ```
    #[pass(id = "argument-specialize", requires(call_effects))]
    pub ArgumentSpecialize,
    "Clone functions for constant argument callsites"
}

impl ModulePass for ArgumentSpecialize {
    /// Run argument specialization for the module.
    fn run(&self, tree: &mut mir::Tree, ctx: &PipelineContext<'_>) -> AnalysisPreservation {
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
    arguments: Vec<mir::ValueReference>,
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
    /// Null reference constant.
    Null,
    /// Boolean constant.
    Boolean(bool),
    /// Signed integer constant.
    Int {
        value: i128,
        width: u16,
        signed: bool,
    },
    /// Unsigned integer constant.
    UInt { value: u128, width: u16 },
    /// Floating point constant.
    Float { bits: u64, width: u8 },
    /// Character constant.
    Char(char),
}

/// Run argument specialization over the module.
fn run_argument_specialize(tree: &mut mir::Tree, ctx: &PipelineContext<'_>) -> bool {
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
    let hotness_policy = ctx.specialize_hotness_policy();

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
        let hotness = callsite_hotness(
            ctx.profile(),
            callsite.caller,
            callsite.call_instruction,
            hotness_policy,
        );
        if matches!(hotness, CallsiteHotness::Cold) {
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
fn collect_call_data(tree: &mir::Tree) -> CallData {
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

                // record callsites and signatures
                if let Some(dispatch) = instruction.call_dispatch_kind() {
                    if let mir::CallDispatchKind::Direct = dispatch
                        && let mir::Instruction::Call {
                            function: callee,
                            call,
                            ..
                        } = instruction
                        && let Some(callee) = callee.function()
                    {
                        let arguments = tree.get_arguments(call.arguments).to_vec();
                        data.callsites.push(DirectCallSite {
                            caller: caller_id,
                            callee,
                            block: block_id,
                            call_instruction: instruction_id,
                            arguments,
                        });
                        continue;
                    }

                    if let Some(signature) = instruction
                        .call_signature()
                        .and_then(|signature| SignatureKey::from_signature_type(tree, signature))
                    {
                        data.indirect_signatures.insert(signature);
                    }
                }
            }
        }
    }

    data
}

/// Build constant propagation data for each defined function.
fn build_constant_maps(
    tree: &mir::Tree,
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
    tree: &mir::Tree,
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
        mir::Constant::Null => ConstantKey::Null,
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
    }
}

/// Specialize a callee by cloning and substituting constants.
fn specialize_callee(
    callee: mir::LocalNodeId<mir::Function>,
    spec_index: usize,
    constants: &[Option<mir::Constant>],
    removal_indices: &[usize],
    tree: &mut mir::Tree,
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
    let sccp = SparseConditionalConstantPropagation;
    let simplify = SimplifyCfg;
    let dce = DeadCodeEliminate;
    run_function_passes_always(new_function_id, tree, ctx, &[&sccp, &simplify, &dce]);
    new_function_id
}

/// Create a suffix for specialized function names.
fn specialized_suffix(spec_index: usize) -> String {
    format!("$spec{spec_index}")
}

/// Clone a function body for specialization.
fn clone_function(
    function_id: mir::LocalNodeId<mir::Function>,
    name: destack_core::StringId,
    tree: &mut mir::Tree,
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
            name: None,
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
            let remapped = instruction_map_with_locals(&instruction, &value_map, &local_map, tree);
            let new_id = tree.insert(remapped);

            // preserve memory access metadata for the cloned instruction
            clone_instruction_metadata(tree, instruction_id, new_id, &value_map);

            // preserve debug locations for the cloned instruction
            if let Some(location) = tree
                .metadata
                .debug
                .instruction_locations
                .get(&instruction_id)
                .cloned()
            {
                tree.metadata
                    .debug
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
        let new_block = tree.get(new_block_id).clone();
        let mut terminator = tree.get(new_block.terminator).clone();
        terminator_remap(&mut terminator, &block_map, &value_map);
        tree.replace(new_block_id, new_block);
        tree.replace(tree.get(new_block_id).terminator, terminator);
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
    tree: &mut mir::Tree,
) {
    // update function parameters and attributes
    let entry_id = {
        let function = tree.get_mut(function_id);
        function.parameters = remap.filter_by_index(&function.parameters);
        function.parameter_attributes = remap.filter_by_index(&function.parameter_attributes);
        function.return_region = remap
            .remap_return_region(&function.return_region)
            .unwrap_or(mir::BorrowRegion::Inferred);
        function.allocation_size = remap.remap_allocation_size(function.allocation_size);
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
    tree: &mut mir::Tree,
) -> bool {
    // prepare removal remapping data
    let remap = ParameterRemap::new(removal_indices);

    // update the call instruction
    let (destination, slice) = match tree.get(callsite.call_instruction) {
        mir::Instruction::Call {
            destination, call, ..
        } => (*destination, call.arguments),
        _ => return false,
    };

    // filter the argument list to match the specialized signature
    let arguments = remap.filter_by_index(tree.get_arguments(slice));
    let new_slice = tree.add_arguments(&arguments);
    let call = match tree.get(callsite.call_instruction) {
        mir::Instruction::Call { call, .. } => call.clone(),
        _ => return false,
    };

    // build the updated signature type before borrowing the tree again
    let signature_type = if remap.removal_indices().is_empty() {
        None
    } else {
        Some(build_signature_type(new_callee, tree))
    };

    let mut call = call;
    call.argument_attributes = remap.filter_by_index(&call.argument_attributes);
    call.allocation_size = remap.remap_allocation_size(call.allocation_size);
    call.arguments = new_slice;
    call.signature = signature_type.map(Into::into).unwrap_or(call.signature);

    let updated = mir::Instruction::Call {
        destination,
        function: new_callee.into(),
        call,
    };
    tree.replace(callsite.call_instruction, updated);

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Constant callsites are specialized into clones.
    #[test]
    fn test_argument_specialize_clones_constant_call() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}
function root(): int32 {
b0:
    v0: int32 = 2int32
    v1: int32 = 3int32
    v2: int32 = call callee(v0, v1): (int32, int32) -> int32
    return v2
}"#;

        let expected = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}
function root(): int32 {
b0:
    v0: int32 = 2int32
    v1: int32 = 3int32
    v2: int32 = call callee$spec0(): () -> int32
    return v2
}
function callee$spec0(): int32 {
b0:
    v0: int32 = 5int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&ArgumentSpecialize);
        test.assert_output(expected);
    }

    /// Call metadata is remapped after specialization.
    #[test]
    fn test_argument_specialize_updates_call_metadata() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}
function root(): int32 {
b0:
    v0: int32 = 2int32
    v1: int32 = 3int32
    v2: int32 = call callee(v0, v1): (int32, int32) -> int32
    return v2
}"#;

        let expected = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}
function root(): int32 {
b0:
    v0: int32 = 2int32
    v1: int32 = 3int32
    v2: int32 = call callee$spec0(): () -> int32
    return v2
}
function callee$spec0(): int32 {
b0:
    v0: int32 = 5int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        let root_id = test.function_id_by_name("root");
        let (call_id, _callee_id) = test.first_call_in_entry(root_id);
        let instruction = test.tree.get_mut(call_id);
        let mir::Instruction::Call { call, .. } = instruction else {
            panic!("expected call instruction");
        };
        call.argument_attributes = vec![mir::ArgumentAttribute::default(); 2];

        test.run_module_pass(&ArgumentSpecialize);
        test.assert_output(expected);

        let (call_id, callee_id) = test.first_call_in_entry(root_id);
        let callee = test.tree.get(callee_id);
        let instruction = test.tree.get(call_id);
        let signature = test.tree.get(
            instruction
                .call_signature()
                .and_then(|signature| signature.ty())
                .expect("call signature should be concrete"),
        );
        let expected_signature = mir::Type::FunctionSignature {
            parameters: callee.parameters.iter().map(|param| param.ty).collect(),
            result: callee.return_type,
        };

        assert!(
            instruction
                .call_argument_attributes()
                .expect("missing call argument attributes")
                .is_empty()
        );
        assert_eq!(signature, &expected_signature);
    }

    /// Specialization remaps memory access metadata for cloned functions.
    #[test]
    fn test_argument_specialize_remaps_memory_access_metadata() {
        let input = r#"
function callee(v0: int32): int32 {
    local local0: int32, owned
b0(v0: int32):
    v1: ref<int32, borrowed, space(frame)> = local.address local0
    v2: int32 = load v1
    return v2
}
function root(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = call callee(v0): (int32) -> int32
    return v1
}"#;

        let mut test = TestProgram::new(input);

        let callee_id = test.function_id_by_name("callee");
        let callee = test.tree.get(callee_id);
        let mut callee_load = None;
        let mut callee_pointer = None;
        for block_id in &callee.blocks {
            let block = test.tree.get(*block_id);
            for instruction_id in &block.instructions {
                if let mir::Instruction::Load { pointer, .. } = test.tree.get(*instruction_id) {
                    callee_load = Some(*instruction_id);
                    callee_pointer = Some(*pointer);
                    break;
                }
            }
            if callee_load.is_some() {
                break;
            }
        }

        let callee_load = callee_load.expect("missing callee load");
        let callee_pointer = callee_pointer
            .expect("missing callee pointer")
            .value()
            .expect("callee pointer should be concrete");
        test.insert_pointer_access(
            callee_load,
            mir::MemoryAccessKind::Read,
            callee_pointer,
            None,
            Vec::new(),
            Vec::new(),
            None,
        );

        test.run_module_pass(&ArgumentSpecialize);

        let root_id = test.function_id_by_name("root");
        let specialized_ids: Vec<_> = test
            .tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .filter(|id| *id != callee_id && *id != root_id)
            .collect();
        assert_eq!(specialized_ids.len(), 1);
        let specialized_id = specialized_ids[0];
        let specialized = test.tree.get(specialized_id);
        let mut specialized_load = None;
        let mut specialized_pointer = None;
        for block_id in &specialized.blocks {
            let block = test.tree.get(*block_id);
            for instruction_id in &block.instructions {
                if let mir::Instruction::Load { pointer, .. } = test.tree.get(*instruction_id) {
                    specialized_load = Some(*instruction_id);
                    specialized_pointer = Some(*pointer);
                    break;
                }
            }
            if specialized_load.is_some() {
                break;
            }
        }

        let specialized_load = specialized_load.expect("missing specialized load");
        let specialized_pointer = specialized_pointer
            .expect("missing specialized pointer")
            .value()
            .expect("specialized pointer should be concrete");
        let accesses = test
            .tree
            .metadata
            .memory
            .memory_accesses(specialized_load)
            .expect("missing specialized access metadata");
        assert_eq!(accesses.len(), 1);
        match accesses[0].target {
            mir::MemoryAccessTarget::Pointer(value) => {
                assert_eq!(value, specialized_pointer);
            }
            _ => panic!("unexpected access target"),
        }
    }

    /// Cold callsites do not trigger specialization with profile data.
    #[test]
    fn test_argument_specialize_skips_cold_callsite() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}
function root(): int32 {
b0:
    v0: int32 = 2int32
    v1: int32 = 3int32
    v2: int32 = call callee(v0, v1): (int32, int32) -> int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        let root_id = test.function_id_by_name("root");
        let (call_id, _) = test.first_call_in_entry(root_id);

        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        test.record_function_profile(&mut profile, root_id, 100);
        test.record_callsite_profile(&mut profile, call_id, 5);

        test.run_module_pass_with_profile(&ArgumentSpecialize, profile);
        test.assert_output(input);
    }

    /// Missing callsite profiles prevent specialization.
    #[test]
    fn test_argument_specialize_skips_missing_callsite_profile() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}
function root(): int32 {
b0:
    v0: int32 = 2int32
    v1: int32 = 3int32
    v2: int32 = call callee(v0, v1): (int32, int32) -> int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        let root_id = test.function_id_by_name("root");

        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        test.record_function_profile(&mut profile, root_id, 100);

        test.run_module_pass_with_profile(&ArgumentSpecialize, profile);
        test.assert_output(input);
    }

    /// Missing function profiles prevent specialization below the hot threshold.
    #[test]
    fn test_argument_specialize_skips_missing_function_profile() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}
function root(): int32 {
b0:
    v0: int32 = 2int32
    v1: int32 = 3int32
    v2: int32 = call callee(v0, v1): (int32, int32) -> int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        let root_id = test.function_id_by_name("root");
        let (call_id, _) = test.first_call_in_entry(root_id);

        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        test.record_callsite_profile(&mut profile, call_id, 5);

        test.run_module_pass_with_profile(&ArgumentSpecialize, profile);
        test.assert_output(input);
    }

    /// Hot callsites specialize when profile data is present.
    #[test]
    fn test_argument_specialize_uses_hot_callsite() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}
function root(): int32 {
b0:
    v0: int32 = 2int32
    v1: int32 = 3int32
    v2: int32 = call callee(v0, v1): (int32, int32) -> int32
    return v2
}"#;

        let expected = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}
function root(): int32 {
b0:
    v0: int32 = 2int32
    v1: int32 = 3int32
    v2: int32 = call callee$spec0(): () -> int32
    return v2
}
function callee$spec0(): int32 {
b0:
    v0: int32 = 5int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        let root_id = test.function_id_by_name("root");
        let (call_id, _) = test.first_call_in_entry(root_id);

        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        test.record_function_profile(&mut profile, root_id, 100);
        test.record_callsite_profile(&mut profile, call_id, 25);

        test.run_module_pass_with_profile(&ArgumentSpecialize, profile);
        test.assert_output(expected);
    }

    /// Required alloc size parameters are not removed.
    #[test]
    fn test_argument_specialize_keeps_alloc_size_param() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function root(): int32 {
b0:
    v0: int32 = 7int32
    v1: int32 = call callee(v0): (int32) -> int32
    return v1
}"#;

        let expected = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function root(): int32 {
b0:
    v0: int32 = 7int32
    v1: int32 = call callee$spec0(v0): (int32) -> int32
    return v1
}
function callee$spec0(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 7int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        let callee_id = test.function_id_by_name("callee");
        test.tree.get_mut(callee_id).allocation_size = Some(mir::AllocationSize::new(0, None));

        test.run_module_pass(&ArgumentSpecialize);
        test.assert_output(expected);
    }

    /// Recursive callees are not specialized.
    #[test]
    fn test_argument_specialize_skips_recursive() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 1int32
    v2: boolean = int.lt.s v0, v1
    branch v2, b1, b2
b1:
    return v0
b2:
    v3: int32 = 1int32
    v4: int32 = int.sub v0, v3
    v5: int32 = call callee(v4): (int32) -> int32
    return v5
}
function root(): int32 {
b0:
    v0: int32 = 9int32
    v1: int32 = call callee(v0): (int32) -> int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&ArgumentSpecialize);
        test.assert_output(input);
    }

    /// Extern callees are not specialized.
    #[test]
    fn test_argument_specialize_skips_extern() {
        let input = r#"
extern function callee(int32): int32
function root(): int32 {
b0:
    v0: int32 = 2int32
    v1: int32 = call callee(v0): (int32) -> int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&ArgumentSpecialize);
        test.assert_output(input);
    }

    /// Specialization stops at the per function limit.
    #[test]
    fn test_argument_specialize_respects_function_limit() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function root(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = 2int32
    v2: int32 = 3int32
    v3: int32 = 4int32
    v4: int32 = 5int32
    v5: int32 = call callee(v0): (int32) -> int32
    v6: int32 = call callee(v1): (int32) -> int32
    v7: int32 = call callee(v2): (int32) -> int32
    v8: int32 = call callee(v3): (int32) -> int32
    v9: int32 = call callee(v4): (int32) -> int32
    return v9
}"#;

        let expected = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function root(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = 2int32
    v2: int32 = 3int32
    v3: int32 = 4int32
    v4: int32 = 5int32
    v5: int32 = call callee$spec0(): () -> int32
    v6: int32 = call callee$spec1(): () -> int32
    v7: int32 = call callee$spec2(): () -> int32
    v8: int32 = call callee$spec3(): () -> int32
    v9: int32 = call callee(v4): (int32) -> int32
    return v9
}
function callee$spec0(): int32 {
b0:
    v0: int32 = 1int32
    return v0
}
function callee$spec1(): int32 {
b0:
    v0: int32 = 2int32
    return v0
}
function callee$spec2(): int32 {
b0:
    v0: int32 = 3int32
    return v0
}
function callee$spec3(): int32 {
b0:
    v0: int32 = 4int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&ArgumentSpecialize);
        test.assert_output(expected);
    }
}
