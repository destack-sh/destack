use std::collections::{HashMap, HashSet};

use crate::optimize::declare_pass;
use crate::optimize::passes::scalar::{
    EliminateDeadCode, FoldConstants, PropagateSparseConstants, SimplifyControlFlow,
};
use destack_mir as mir;

use crate::optimize::{
    FunctionPass, MirOptimized, ModulePass, PipelineContext, run_function_passes,
};
use destack_mir::{
    CallGraph, CallsiteHotness, ConstantPropagation, Mutation, ParameterRemap, SignatureKey,
    apply_constant_parameters, clone_instruction_tables, constant_arguments_for_parameters,
    instruction_map_with_locals, terminator_remap,
};

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
    /// function callee_spec0(): int32 {
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
    ///     v2 = call callee_spec0()
    ///     return v2
    /// }
    /// ```
    #[pass(id = "specialize-arguments")]
    pub SpecializeArguments,
    "Clone functions for constant argument callsites"
}

impl ModulePass for SpecializeArguments {
    /// Run argument specialization for the module.
    fn run(
        &self,
        optimized: &mut MirOptimized,
        ctx: &PipelineContext<'_>,
        analyses: &mir::TreeAnalysisCache,
    ) -> Mutation {
        // run the specialization pass
        let changed = run_specialize_arguments(optimized, ctx, analyses);

        // report what this pass changed
        if changed {
            ctx.strings.intern("specialize-arguments");
            Mutation::CONTROL | Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }

    /// Return the pass display name.
    fn name(&self) -> &'static str {
        "SpecializeArguments"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "specialize-arguments"
    }
}

/// Direct callsite tables for specialization.
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
    /// Null reference constant.
    Null,
    /// Undefined reference constant.
    Undefined,
    /// Uninitialized storage constant.
    Uninit,
    /// Zeroed storage constant.
    Zeroed,
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
    Float { bits: u64, format: mir::FloatType },
    /// Character constant.
    Char(char),
}

/// Run argument specialization over the module.
fn run_specialize_arguments(
    optimized: &mut MirOptimized,
    ctx: &PipelineContext<'_>,
    analyses: &mir::TreeAnalysisCache,
) -> bool {
    let mut specialized_functions = Vec::new();

    // specialize callsites while holding the mutable MIR tables
    let changed = {
        let MirOptimized {
            tree,
            layouts,
            memory,
            effects,
            ..
        } = optimized;

        let call_data = collect_call_data(tree);
        let callgraph = analyses.get::<CallGraph>(tree);
        let constants_by_function = build_constant_maps(tree, ctx.target_layout());

        let mut changed = false;
        let mut specialization_cache: HashMap<SpecializationKey, mir::LocalNodeId<mir::Function>> =
            HashMap::new();
        let mut specialization_counts: HashMap<mir::LocalNodeId<mir::Function>, usize> =
            HashMap::new();
        let mut total_specializations = 0usize;
        let mut function_analysis_cache: HashMap<mir::FunctionId, mir::FunctionAnalysisCache> =
            HashMap::new();
        let mut caller_counts: HashMap<mir::FunctionId, mir::ExecutionCounts> = HashMap::new();

        // process callsites for specialization
        for callsite in &call_data.callsites {
            // skip recursive callees
            if callgraph.is_recursive_function(callsite.callee) {
                continue;
            }

            // skip external callees
            if tree.get(callsite.callee).entry().is_none() {
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
            caller_counts.entry(callsite.caller).or_insert_with(|| {
                let analyses = function_analysis_cache
                    .entry(callsite.caller)
                    .or_insert_with(|| {
                        mir::FunctionAnalysisCache::with_options(
                            ctx.options.analysis,
                            memory,
                            effects,
                        )
                    });
                mir::ExecutionCounts::new(tree.get(callsite.caller), tree, ctx.profile(), analyses)
            });
            let entry_count = ctx
                .profile()
                .and_then(|profile| profile.function(tree.get(callsite.caller).symbol))
                .map(|function_profile| function_profile.entry.get())
                .unwrap_or(0);
            let block_count = caller_counts[&callsite.caller].block(callsite.block);
            if ctx.profile().is_some() {
                match ctx.hotness_thresholds().classify(block_count, entry_count) {
                    CallsiteHotness::Hot => {}
                    CallsiteHotness::Unknown | CallsiteHotness::Cold => continue,
                }
            }

            // compute removal indices for constant parameters
            let removal_indices =
                removable_constant_parameters(tree.get(callsite.callee), &constants, tree);

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
                    memory,
                    ctx,
                );
                specialization_cache.insert(key, new_callee);
                specialized_functions.push(new_callee);
                *count += 1;
                total_specializations += 1;
                changed = true;
                new_callee
            };

            // update the callsite to use the specialized clone
            if update_callsite(
                callsite,
                new_callee,
                &removal_indices,
                tree,
                layouts,
                effects,
            ) {
                changed = true;
            }
        }

        changed
    };
    let simplified = simplify_specialized_functions(&specialized_functions, optimized, ctx);

    changed || simplified
}

/// Collect direct callsites and indirect signatures for the module.
fn collect_call_data(tree: &mir::Tree) -> CallData {
    // prepare the callsite data container
    let mut data = CallData::default();

    // scan each function body for callsites
    for (caller_id, function) in tree.iter_nodes::<mir::Function>() {
        // skip external functions
        if function.entry().is_none() {
            continue;
        }

        for &block_id in function.blocks() {
            let block = tree.get(block_id);

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);

                // record callsites and signatures
                if let Some(dispatch) = instruction.call_dispatch() {
                    if let mir::CallDispatch::Direct = dispatch
                        && let mir::Instruction::Call { call, .. } = instruction
                        && let Some(callee) = call.callee.function()
                    {
                        let arguments = tree.get_values(call.arguments).to_vec();
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
                        .and_then(|signature| SignatureKey::from_signature_type(tree, &signature))
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
    target_layout: mir::TargetLayout,
) -> HashMap<mir::LocalNodeId<mir::Function>, ConstantPropagation> {
    // prepare the constants map
    let mut maps = HashMap::new();

    // build a constant propagation analysis per function
    for (function_id, function) in tree.iter_nodes::<mir::Function>() {
        if function.entry().is_none() {
            continue;
        }

        let constants = ConstantPropagation::with_parameter_constants(
            function,
            tree,
            target_layout,
            &HashMap::new(),
        );
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
        ctx.target_layout().pointer_bits(),
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
        mir::Constant::Undefined => ConstantKey::Undefined,
        mir::Constant::Uninit => ConstantKey::Uninit,
        mir::Constant::Zeroed => ConstantKey::Zeroed,
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
        mir::Constant::Float { bits, format } => ConstantKey::Float {
            bits: *bits,
            format: *format,
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
    memory: &mut mir::MemoryTable,
    ctx: &PipelineContext<'_>,
) -> mir::LocalNodeId<mir::Function> {
    // build the specialized function name
    let base_name = ctx.strings.get(tree.get(callee).name).to_string();
    let suffix = specialized_suffix(spec_index);
    let name = ctx.strings.intern(&format!("{base_name}{suffix}"));

    // clone the function body
    let new_function_id = clone_function(callee, name, tree, memory);

    // insert constant parameters into the clone
    apply_constant_parameters(new_function_id, constants, tree, memory);

    // remove parameters that are constant and not required
    if !removal_indices.is_empty() {
        let remap = ParameterRemap::new(removal_indices);
        apply_parameter_removals(new_function_id, &remap, tree);
    }

    new_function_id
}

/// Simplify newly specialized functions.
fn simplify_specialized_functions(
    function_ids: &[mir::LocalNodeId<mir::Function>],
    optimized: &mut MirOptimized,
    ctx: &PipelineContext<'_>,
) -> bool {
    let propagate = PropagateSparseConstants;
    let fold = FoldConstants;
    let simplify = SimplifyControlFlow;
    let eliminate = EliminateDeadCode;
    let passes: [&dyn FunctionPass; 4] = [&propagate, &fold, &simplify, &eliminate];

    let mut changed = false;
    for function_id in function_ids {
        changed |= run_function_passes(*function_id, optimized, ctx, &passes);
    }

    changed
}

/// Create a suffix for specialized function names.
fn specialized_suffix(spec_index: usize) -> String {
    format!("_spec{spec_index}")
}

/// Clone a function body for specialization.
fn clone_function(
    function_id: mir::LocalNodeId<mir::Function>,
    name: destack_core::StringId,
    tree: &mut mir::Tree,
    memory: &mut mir::MemoryTable,
) -> mir::LocalNodeId<mir::Function> {
    // read the original function
    let original = tree.get(function_id).clone();

    // clone locals for the function
    let mut local_map = HashMap::new();
    let mut new_locals = Vec::new();
    for local_id in original.locals() {
        let local = tree.get(*local_id).clone();
        let new_local = tree.insert(local);
        local_map.insert(*local_id, new_local);
        new_locals.push(new_local);
    }

    // create block placeholders
    let mut block_map = HashMap::new();
    for block_id in original.blocks() {
        let block = tree.get(*block_id);
        let parameters = block.parameters.clone();
        let terminator = tree.get(block.terminator).clone();
        let terminator = tree.insert(terminator);
        let new_block = mir::Block {
            parameters,
            instructions: Vec::new(),
            terminator,
        };
        let new_block_id = tree.insert(new_block);
        block_map.insert(*block_id, new_block_id);
    }

    // clone instructions into new blocks
    let value_map = HashMap::new();
    for block_id in original.blocks() {
        let new_block_id = block_map[block_id];
        let instruction_ids = tree.get(*block_id).instructions.clone();
        let mut new_instructions = Vec::with_capacity(instruction_ids.len());

        for instruction_id in instruction_ids {
            // remap instruction operands to the cloned locals and values
            let instruction = tree.get(instruction_id).clone();
            let remapped = instruction_map_with_locals(&instruction, &value_map, &local_map, tree);
            let new_id = tree.insert(remapped);

            // preserve memory access entries for the cloned instruction
            clone_instruction_tables(tree, memory, instruction_id, new_id, &value_map);

            new_instructions.push(new_id);
        }

        // update the cloned block instruction list
        let mut new_block = tree.get(new_block_id).clone();
        new_block.instructions = new_instructions;
        tree.set(new_block_id, new_block);
    }

    // remap terminators with new block ids
    for block_id in original.blocks() {
        let new_block_id = block_map[block_id];
        let terminator_id = tree.get(new_block_id).terminator;
        let mut terminator = tree.get(terminator_id).clone();
        terminator_remap(tree, &mut terminator, &block_map, &value_map);
        tree.set(terminator_id, terminator);
    }

    // build the new function
    let mut new_function = original.clone();
    new_function.name = name;
    new_function.linkage = mir::Linkage::Local;
    new_function.replace_locals(new_locals);
    new_function.replace_blocks(
        original
            .blocks()
            .iter()
            .map(|block_id| block_map[block_id])
            .collect(),
        tree,
    );
    if let Some(entry) = original.entry() {
        new_function.set_entry(block_map[&entry]);
    }

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
    tree: &mir::Tree,
) -> Vec<usize> {
    // collect required parameter indices
    let required = ParameterRemap::required_indices(function, tree);

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
        function.entry().expect("defined function has entry block")
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
    layouts: &mut mir::LayoutTable,
    effects: &mut mir::EffectTable,
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
    let arguments = remap.filter_by_index(tree.get_values(slice));
    let new_slice = tree.add_values(&arguments);
    let call = match tree.get(callsite.call_instruction) {
        mir::Instruction::Call { call, .. } => call.clone(),
        _ => return false,
    };

    // build the explicit specialized signature
    let callee = tree.get(new_callee);
    let signature = tree.intern_type(callee.signature());
    layouts.copy_type_entries(call.signature, signature);

    let mut call = call;
    call.callee = mir::Callee::Direct {
        function: new_callee,
    };
    call.arguments = new_slice;
    call.signature = signature;

    let updated = mir::Instruction::Call { destination, call };
    tree.set(callsite.call_instruction, updated);

    let callsite_id = mir::CallSite::Instruction(callsite.call_instruction);
    if let Some(tables) = effects.calls.get_mut(&callsite_id) {
        tables.arguments = remap.filter_by_index(&tables.arguments);
        tables.target = Some(new_callee);
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Constant callsites are specialized into clones.
    #[test]
    fn test_specialize_arguments_clones_constant_call() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}

function root(): int32 {
entry:
    v0: int32 = 2
    v1: int32 = 3
    v2: int32 = call callee(v0, v1): (int32, int32) => int32
    return v2
}
"#;

        let expected = r#"
function callee(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}

function root(): int32 {
entry:
    v0: int32 = 2
    v1: int32 = 3
    v2: int32 = call callee_spec0(): () => int32
    return v2
}

function callee_spec0(): int32 {
entry:
    v2: int32 = 5
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&SpecializeArguments);
        test.assert_output(expected);
    }

    /// Call effect entries are remapped after specialization.
    #[test]
    fn test_specialize_arguments_updates_call_entries() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}

function root(): int32 {
entry:
    v0: int32 = 2
    v1: int32 = 3
    v2: int32 = call callee(v0, v1): (int32, int32) => int32
    return v2
}
"#;

        let expected = r#"
function callee(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}

function root(): int32 {
entry:
    v0: int32 = 2
    v1: int32 = 3
    v2: int32 = call callee_spec0(): () => int32
    return v2
}

function callee_spec0(): int32 {
entry:
    v2: int32 = 5
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        let root_id = test.function_id_by_name("root");
        let (call_id, _callee_id) = test.first_call_in_entry(root_id);
        let callsite = mir::CallSite::Instruction(call_id);
        test.optimized.effects.call_mut(callsite).arguments =
            vec![mir::CallArgumentEffect::default(); 2];

        test.run_module_pass(&SpecializeArguments);
        test.assert_output(expected);

        let (call_id, callee_id) = test.first_call_in_entry(root_id);
        let callee = test.optimized.tree.get(callee_id);
        let callsite = mir::CallSite::Instruction(call_id);
        let tables = test
            .optimized
            .effects
            .call(callsite)
            .expect("missing call metadata");
        let instruction = test.optimized.tree.get(call_id);
        let signature = test.optimized.tree.get(
            instruction
                .call_signature()
                .expect("call signature should be concrete"),
        );
        let expected_signature = mir::Type::FunctionSignature {
            lifetimes: Vec::new(),
            parameters: callee
                .parameters
                .iter()
                .map(mir::FunctionParameter::signature_parameter)
                .collect(),
            result: callee.return_type,
        };

        assert!(tables.arguments.is_empty());
        assert_eq!(signature, &expected_signature);
    }

    /// Specialization remaps memory access entries for cloned functions.
    #[test]
    fn test_specialize_arguments_remaps_memory_access_metadata() {
        let input = r#"
function callee(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    v1: ref<int32, borrowed, mutable, space(frame)> = local.address l0
    v2: int32 = load v1
    return v2
}

function root(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = call callee(v0): (int32) => int32
    return v1
}
"#;

        let mut test = TestProgram::new(input);

        let callee_id = test.function_id_by_name("callee");
        let callee = test.optimized.tree.get(callee_id);
        let mut callee_load = None;
        let mut callee_pointer = None;
        for block_id in callee.blocks() {
            let block = test.optimized.tree.get(*block_id);
            for instruction_id in &block.instructions {
                if let mir::Instruction::Load { pointer, .. } =
                    test.optimized.tree.get(*instruction_id)
                {
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
        let callee_pointer = callee_pointer.expect("missing callee pointer");
        test.insert_pointer_access(
            callee_load,
            mir::MemoryOperation::Read,
            callee_pointer,
            None,
        );

        test.run_module_pass(&SpecializeArguments);

        let root_id = test.function_id_by_name("root");
        let specialized_ids: Vec<_> = test
            .optimized
            .tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .filter(|id| *id != callee_id && *id != root_id)
            .collect();
        assert_eq!(specialized_ids.len(), 1);
        let specialized_id = specialized_ids[0];
        let specialized = test.optimized.tree.get(specialized_id);
        let mut specialized_load = None;
        let mut specialized_pointer = None;
        for block_id in specialized.blocks() {
            let block = test.optimized.tree.get(*block_id);
            for instruction_id in &block.instructions {
                if let mir::Instruction::Load { pointer, .. } =
                    test.optimized.tree.get(*instruction_id)
                {
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
        let specialized_pointer = specialized_pointer.expect("missing specialized pointer");
        let accesses = test
            .optimized
            .memory
            .memory_accesses(specialized_load)
            .expect("missing specialized access entries");
        assert_eq!(accesses.len(), 1);
        match accesses[0].target {
            mir::MemoryTarget::Reference(value) => {
                assert_eq!(value, specialized_pointer);
            }
            _ => panic!("unexpected access target"),
        }
    }

    /// Cold callsites do not trigger specialization with profile data.
    #[test]
    fn test_specialize_arguments_skips_cold_callsite() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}

function root(): int32 {
entry:
    v0: int32 = 2
    v1: int32 = 3
    v2: int32 = call callee(v0, v1): (int32, int32) => int32
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        let root_id = test.function_id_by_name("root");

        // a cold caller leaves the callsite unspecialized
        let mut profile = mir::Profile::new();
        test.record_function_entry(&mut profile, root_id, 5);

        test.run_module_pass_with_profile(&SpecializeArguments, profile);
        test.assert_unchanged(input);
    }

    /// Missing function profiles prevent specialization below the hot threshold.
    #[test]
    fn test_specialize_arguments_skips_missing_function_count() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}

function root(): int32 {
entry:
    v0: int32 = 2
    v1: int32 = 3
    v2: int32 = call callee(v0, v1): (int32, int32) => int32
    return v2
}
"#;

        let mut test = TestProgram::new(input);

        // without a function entry count the caller hotness is unknown, so skip
        let profile = mir::Profile::new();

        test.run_module_pass_with_profile(&SpecializeArguments, profile);
        test.assert_unchanged(input);
    }

    /// Hot callsites specialize when profile data is present.
    #[test]
    fn test_specialize_arguments_uses_hot_callsite() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}

function root(): int32 {
entry:
    v0: int32 = 2
    v1: int32 = 3
    v2: int32 = call callee(v0, v1): (int32, int32) => int32
    return v2
}
"#;

        let expected = r#"
function callee(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}

function root(): int32 {
entry:
    v0: int32 = 2
    v1: int32 = 3
    v2: int32 = call callee(v0, v1): (int32, int32) => int32
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        let root_id = test.function_id_by_name("root");

        // a hot caller specializes the constant callsite
        let mut profile = mir::Profile::new();
        test.record_function_entry(&mut profile, root_id, 100);

        test.run_module_pass_with_profile(&SpecializeArguments, profile);
        test.assert_output(expected);
    }

    /// Recursive callees are not specialized.
    #[test]
    fn test_specialize_arguments_skips_recursive() {
        let input = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: boolean = int.lt.s v0, v1
    branch v2, b1, b2

b1:
    return v0

b2:
    v3: int32 = 1
    v4: int32 = int.sub v0, v3
    v5: int32 = call callee(v4): (int32) => int32
    return v5
}

function root(): int32 {
entry:
    v0: int32 = 9
    v1: int32 = call callee(v0): (int32) => int32
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&SpecializeArguments);
        test.assert_unchanged(input);
    }

    /// External callees are not specialized.
    #[test]
    fn test_specialize_arguments_skips_extern() {
        let input = r#"
external function callee(int32): int32

function root(): int32 {
entry:
    v0: int32 = 2
    v1: int32 = call callee(v0): (int32) => int32
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&SpecializeArguments);
        test.assert_unchanged(input);
    }

    /// Specialization stops at the per function limit.
    #[test]
    fn test_specialize_arguments_respects_function_limit() {
        let input = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function root(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = 3
    v3: int32 = 4
    v4: int32 = 5
    v5: int32 = call callee(v0): (int32) => int32
    v6: int32 = call callee(v1): (int32) => int32
    v7: int32 = call callee(v2): (int32) => int32
    v8: int32 = call callee(v3): (int32) => int32
    v9: int32 = call callee(v4): (int32) => int32
    return v9
}
"#;

        let expected = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function root(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = 3
    v3: int32 = 4
    v4: int32 = 5
    v5: int32 = call callee_spec0(): () => int32
    v6: int32 = call callee_spec1(): () => int32
    v7: int32 = call callee_spec2(): () => int32
    v8: int32 = call callee_spec3(): () => int32
    v9: int32 = call callee(v4): (int32) => int32
    return v9
}

function callee_spec0(): int32 {
entry:
    v1: int32 = 1
    return v1
}

function callee_spec1(): int32 {
entry:
    v1: int32 = 2
    return v1
}

function callee_spec2(): int32 {
entry:
    v1: int32 = 3
    return v1
}

function callee_spec3(): int32 {
entry:
    v1: int32 = 4
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&SpecializeArguments);
        test.assert_output(expected);
    }
}
