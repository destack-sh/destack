use std::collections::{HashMap, HashSet};

use crate::declare_mir_pass;
use destack_mir as mir;

use crate::common::mir::analysis::{ConstantPropagation, constant_propagation_with_params};
use crate::common::mir::{
    SignatureKey, apply_constant_parameters, constant_arguments_for_parameters,
    constant_matches_type, constant_type_of,
};
use crate::optimize::passes::scalar::{SimplifyCfg, SparseConditionalConstantPropagation};
use crate::optimize::{AnalysisPreservation, ModulePass, PipelineContext, run_function_passes};

declare_mir_pass! {
    /// Propagate constants across call edges and prune dead paths.
    ///
    /// This pass discovers constant arguments for direct callsites and applies them to callees.
    /// It then runs SCCP locally to prune dead edges and replace constant returns.
    ///
    /// ```mir
    /// function callee(v0: int32): int32 {
    /// b0(v0: int32):
    ///     return v0
    /// }
    /// function root(): int32 {
    /// b0:
    ///     v0 = 7int32
    ///     v1 = call callee(v0)
    ///     return v1
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function callee(v0: int32): int32 {
    /// b0(v0: int32):
    ///     v1 = 7int32
    ///     return v1
    /// }
    /// function root(): int32 {
    /// b0:
    ///     v0 = 7int32
    ///     v1 = call callee(v0)
    ///     return v1
    /// }
    /// ```
    #[pass(id = "ip-sccp", requires(call_effects))]
    pub InterproceduralSccp,
    "Interprocedural sparse conditional constant propagation"
}

impl ModulePass for InterproceduralSccp {
    /// Run interprocedural SCCP for the module.
    fn run(&self, tree: &mut mir::Tree, ctx: &PipelineContext<'_>) -> AnalysisPreservation {
        // run the interprocedural pass
        let changed = run_interprocedural_sccp(tree, ctx);

        // report analysis preservation based on whether changes occurred
        if changed {
            ctx.strings.intern("ip-sccp");
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass display name.
    fn name(&self) -> &'static str {
        "InterproceduralSccp"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "ip-sccp"
    }
}

/// Lattice value for interprocedural constants.
#[derive(Debug, Clone, PartialEq)]
enum LatticeConstant {
    /// No information about the value yet.
    Unknown,
    /// A known constant value.
    Constant(mir::Constant),
    /// The value varies across callsites.
    Overdefined,
}

impl LatticeConstant {
    /// Return the constant value when known.
    fn constant(&self) -> Option<&mir::Constant> {
        match self {
            LatticeConstant::Constant(constant) => Some(constant),
            _ => None,
        }
    }
}

/// Interprocedural lattice state for a function.
#[derive(Debug, Clone)]
struct FunctionState {
    /// Parameter lattice values.
    param_states: Vec<LatticeConstant>,
    /// Return lattice value.
    return_state: LatticeConstant,
    /// True when the function is externally reachable.
    is_exposed: bool,
}

/// Direct callsite metadata for SCCP.
#[derive(Debug, Clone)]
struct DirectCallSite {
    /// The caller function id.
    caller: mir::LocalNodeId<mir::Function>,
    /// The callee function id.
    callee: mir::LocalNodeId<mir::Function>,
    /// The block containing the call.
    block: mir::LocalNodeId<mir::Block>,
    /// The call instruction when applicable.
    call_instruction: Option<mir::LocalNodeId<mir::Instruction>>,
    /// The arguments passed at the callsite.
    arguments: Vec<mir::ValueReference>,
}

/// Collected callsite data for IPSCCP.
#[derive(Debug, Default)]
struct CallData {
    /// Direct callsites in the module.
    callsites: Vec<DirectCallSite>,
    /// Signatures that may be targeted by indirect calls.
    indirect_signatures: HashSet<SignatureKey>,
}

/// Run interprocedural SCCP over the module.
fn run_interprocedural_sccp(tree: &mut mir::Tree, ctx: &PipelineContext<'_>) -> bool {
    // collect callsite information up front
    let call_data = collect_call_data(tree);

    // collect functions with bodies
    let function_ids: Vec<_> = tree
        .iter_nodes::<mir::Function>()
        .filter_map(|(id, function)| function.entry.is_some().then_some((id, function.linkage)))
        .collect();

    // seed lattice state for each function
    let mut states = seed_function_states(tree, &function_ids, &call_data);

    // reach a fixed point for parameter and return constants
    loop {
        // build constant propagation for each function using current parameter constants
        let constants_by_function = build_constant_maps(tree, &states, ctx.type_context());

        // update parameter lattice values
        let mut state_changed = update_parameter_states(
            tree,
            &function_ids,
            &call_data,
            &constants_by_function,
            &mut states,
            ctx.type_context(),
        );

        // update return lattice values
        state_changed |= update_return_states(
            tree,
            &function_ids,
            &constants_by_function,
            &mut states,
            ctx.type_context(),
        );

        // stop once the lattice is stable
        if !state_changed {
            break;
        }
    }

    // apply constant parameters to function bodies
    let mut changed = false;
    let mut cleanup_functions = HashSet::new();
    for (function_id, _) in &function_ids {
        // skip missing state entries
        let Some(state) = states.get(function_id) else {
            continue;
        };

        // apply constant substitutions and track updates
        let constants = state_constants(state);
        if apply_constant_parameters(*function_id, &constants, tree) {
            cleanup_functions.insert(*function_id);
            changed = true;
        }
    }

    // replace pure constant calls with literals
    if replace_constant_calls(tree, &call_data, &states, ctx.type_context()) {
        for callsite in &call_data.callsites {
            cleanup_functions.insert(callsite.caller);
        }
        changed = true;
    }

    // run cleanup passes for modified functions
    for function_id in cleanup_functions {
        let sccp = SparseConditionalConstantPropagation;
        let simplify = SimplifyCfg;
        if run_function_passes(function_id, tree, ctx, &[&sccp, &simplify]) {
            changed = true;
        }
    }

    // return whether changes occurred
    changed
}

/// Seed function lattice state for each defined function.
fn seed_function_states(
    tree: &mir::Tree,
    function_ids: &[(mir::LocalNodeId<mir::Function>, mir::Linkage)],
    call_data: &CallData,
) -> HashMap<mir::LocalNodeId<mir::Function>, FunctionState> {
    // build the state map
    let mut states = HashMap::new();

    for (function_id, linkage) in function_ids {
        // read the function signature
        let function = tree.get(*function_id);
        let signature = SignatureKey::from_function(tree, function);
        let is_indirect = signature
            .as_ref()
            .is_some_and(|signature| call_data.indirect_signatures.contains(signature));

        // mark functions reachable from outside or indirectly as exposed
        let is_exposed = linkage.is_exported() || signature.is_none() || is_indirect;

        // seed parameter lattices based on exposure
        let seed_state = if is_exposed {
            LatticeConstant::Overdefined
        } else {
            LatticeConstant::Unknown
        };
        let param_states = vec![seed_state; function.parameters.len()];

        // insert the initial state
        states.insert(
            *function_id,
            FunctionState {
                param_states,
                return_state: LatticeConstant::Unknown,
                is_exposed,
            },
        );
    }

    states
}

/// Update parameter lattice values using callsite constants.
fn update_parameter_states(
    tree: &mir::Tree,
    function_ids: &[(mir::LocalNodeId<mir::Function>, mir::Linkage)],
    call_data: &CallData,
    constants_by_function: &HashMap<mir::LocalNodeId<mir::Function>, ConstantPropagation>,
    states: &mut HashMap<mir::LocalNodeId<mir::Function>, FunctionState>,
    type_context: crate::common::mir::TypeContext,
) -> bool {
    // track whether any state changed
    let mut changed = false;

    // build a per function list of callsites
    let mut callsites_by_callee: HashMap<mir::LocalNodeId<mir::Function>, Vec<&DirectCallSite>> =
        HashMap::new();

    for callsite in &call_data.callsites {
        callsites_by_callee
            .entry(callsite.callee)
            .or_default()
            .push(callsite);
    }

    // recompute parameter states for each function
    for (function_id, _) in function_ids {
        // skip functions without state entries
        let Some(state) = states.get_mut(function_id) else {
            continue;
        };

        // skip exposed functions
        if state.is_exposed {
            continue;
        }

        // seed fresh parameter states
        let function = tree.get(*function_id);
        let mut next_states = vec![LatticeConstant::Unknown; function.parameters.len()];

        // merge constants from callsites
        if let Some(callsites) = callsites_by_callee.get(function_id) {
            for callsite in callsites {
                // skip callsites without constant maps
                let Some(constants) = constants_by_function.get(&callsite.caller) else {
                    continue;
                };

                // resolve constants at the callsite exit
                let block_constants = constants.exit(callsite.block);
                let Some(argument_constants) = constant_arguments_for_parameters(
                    &callsite.arguments,
                    &function.parameters,
                    block_constants,
                    type_context.pointer_width_bits,
                    tree,
                ) else {
                    continue;
                };

                merge_param_constants(&argument_constants, &mut next_states);
            }
        }

        // record changes
        if next_states != state.param_states {
            state.param_states = next_states;
            changed = true;
        }
    }

    changed
}

/// Merge callsite constants into parameter lattice values.
fn merge_param_constants(constants: &[Option<mir::Constant>], states: &mut [LatticeConstant]) {
    // merge each constant into the matching state
    for (index, constant) in constants.iter().enumerate() {
        let state = &mut states[index];
        match constant {
            Some(constant) => {
                // update the lattice state with the new constant
                match state {
                    LatticeConstant::Unknown => {
                        *state = LatticeConstant::Constant(constant.clone());
                    }
                    LatticeConstant::Constant(existing) => {
                        if existing != constant {
                            *state = LatticeConstant::Overdefined;
                        }
                    }
                    LatticeConstant::Overdefined => {}
                }
            }
            None => {
                // mark the parameter as varying for non constant arguments
                if !matches!(state, LatticeConstant::Overdefined) {
                    *state = LatticeConstant::Overdefined;
                }
            }
        }
    }
}

/// Update return lattice values using constant propagation.
fn update_return_states(
    tree: &mir::Tree,
    function_ids: &[(mir::LocalNodeId<mir::Function>, mir::Linkage)],
    constants_by_function: &HashMap<mir::LocalNodeId<mir::Function>, ConstantPropagation>,
    states: &mut HashMap<mir::LocalNodeId<mir::Function>, FunctionState>,
    type_context: crate::common::mir::TypeContext,
) -> bool {
    // track whether any state changed
    let mut changed = false;

    // update return states for each function
    for (function_id, _) in function_ids {
        // skip functions without constant maps
        let Some(constants) = constants_by_function.get(function_id) else {
            continue;
        };

        // read the function definition
        let function = tree.get(*function_id);

        // compute the merged return lattice state
        let next_state = return_state_for_function(function, constants, tree, type_context);

        // skip missing lattice state entries
        let Some(state) = states.get_mut(function_id) else {
            continue;
        };

        if next_state != state.return_state {
            state.return_state = next_state;
            changed = true;
        }
    }

    changed
}

/// Compute the return lattice value for a function.
fn return_state_for_function(
    function: &mir::Function,
    constants: &ConstantPropagation,
    tree: &mir::Tree,
    type_context: crate::common::mir::TypeContext,
) -> LatticeConstant {
    // require a concrete return type
    let Some(return_type) = function.return_type.ty() else {
        return LatticeConstant::Overdefined;
    };

    // skip void returns
    if matches!(tree.get(return_type), mir::Type::Void) {
        return LatticeConstant::Overdefined;
    }

    // track the merged return constant
    let mut merged = LatticeConstant::Unknown;

    // scan return terminators
    for block_id in &function.blocks {
        let block = tree.get(*block_id);
        let terminator = tree.get(block.terminator);

        let mir::Terminator::Return { value } = terminator else {
            continue;
        };

        // reject returns without a value
        let Some(value) = value.and_then(|value| value.value()) else {
            return LatticeConstant::Overdefined;
        };

        // resolve the return constant at the block exit
        let Some(constant) = constants.constant_at_exit(*block_id, value) else {
            return LatticeConstant::Overdefined;
        };

        // reject constants that do not match the return type
        let constant_type = constant_type_of(constant);
        if !constant_matches_type(
            constant_type,
            return_type,
            type_context.pointer_width_bits,
            tree,
        ) {
            return LatticeConstant::Overdefined;
        }

        // merge the return constant into the lattice
        merged = match merged {
            LatticeConstant::Unknown => LatticeConstant::Constant(constant.clone()),
            LatticeConstant::Constant(existing) => {
                if existing == *constant {
                    LatticeConstant::Constant(existing)
                } else {
                    return LatticeConstant::Overdefined;
                }
            }
            LatticeConstant::Overdefined => return LatticeConstant::Overdefined,
        };
    }

    merged
}

/// Build constant propagation results for each function.
fn build_constant_maps(
    tree: &mir::Tree,
    states: &HashMap<mir::LocalNodeId<mir::Function>, FunctionState>,
    type_context: crate::common::mir::TypeContext,
) -> HashMap<mir::LocalNodeId<mir::Function>, ConstantPropagation> {
    // prepare the result map
    let mut maps = HashMap::new();

    // build a constant propagation analysis per function
    for (function_id, state) in states {
        let function = tree.get(*function_id);
        let param_constants = param_constants_for_function(function, state);
        let constants =
            constant_propagation_with_params(function, tree, type_context, &param_constants);
        maps.insert(*function_id, constants);
    }

    maps
}

/// Build constant parameter maps from lattice state.
fn param_constants_for_function(
    function: &mir::Function,
    state: &FunctionState,
) -> HashMap<mir::Value, mir::Constant> {
    // collect constants for each parameter
    let mut constants = HashMap::new();
    for (param, param_state) in function.parameters.iter().zip(state.param_states.iter()) {
        // skip non constant parameter states
        if let LatticeConstant::Constant(constant) = param_state {
            let Some(value) = param.value.value() else {
                continue;
            };

            constants.insert(value, constant.clone());
        }
    }

    constants
}

/// Convert parameter lattice values into constant options.
fn state_constants(state: &FunctionState) -> Vec<Option<mir::Constant>> {
    // collect parameter constants in order
    let mut constants = Vec::with_capacity(state.param_states.len());
    for param_state in &state.param_states {
        constants.push(param_state.constant().cloned());
    }

    constants
}

/// Replace pure callsites with constant returns.
fn replace_constant_calls(
    tree: &mut mir::Tree,
    call_data: &CallData,
    states: &HashMap<mir::LocalNodeId<mir::Function>, FunctionState>,
    type_context: crate::common::mir::TypeContext,
) -> bool {
    // track whether any calls were replaced
    let mut changed = false;

    // replace direct call instructions when safe
    for callsite in &call_data.callsites {
        // skip callsites without direct call instructions
        let Some(call_instruction) = callsite.call_instruction else {
            continue;
        };

        // read the callee return lattice state
        let Some(state) = states.get(&callsite.callee) else {
            continue;
        };
        let Some(constant) = state.return_state.constant() else {
            continue;
        };

        // read the call instruction and destination
        let instruction = tree.get(call_instruction).clone();
        let mir::Instruction::Call {
            destination,
            function,
            ..
        } = instruction
        else {
            continue;
        };

        // skip calls that do not produce a value
        let Some(destination) = destination else {
            continue;
        };
        let Some(function) = function.function() else {
            continue;
        };

        // skip calls that are not pure
        if !call_is_pure(tree, call_instruction, function) {
            continue;
        }

        // skip return constants that do not match the return type
        let return_type = tree.get(function).return_type;
        let constant_type = constant_type_of(constant);
        if !constant_matches_type(
            constant_type,
            return_type,
            type_context.pointer_width_bits,
            tree,
        ) {
            continue;
        }

        // replace the call with a constant instruction
        tree.replace(
            call_instruction,
            mir::Instruction::Const {
                destination,
                value: constant.clone(),
            },
        );
        tree.metadata
            .memory
            .remove_memory_accesses(call_instruction);
        changed = true;
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
            let terminator = tree.get(block.terminator);

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
                    {
                        let Some(callee) = callee.function() else {
                            continue;
                        };
                        let arguments = tree.get_arguments(call.arguments).to_vec();

                        data.callsites.push(DirectCallSite {
                            caller: caller_id,
                            callee,
                            block: block_id,
                            call_instruction: Some(instruction_id),
                            arguments,
                        });
                        continue;
                    }

                    if let Some(signature) = instruction
                        .call_signature()
                        .and_then(|signature| signature.ty())
                        .and_then(|signature| SignatureKey::from_signature_type(tree, signature))
                    {
                        data.indirect_signatures.insert(signature);
                    }
                }
            }

            // record call terminators
            match terminator {
                mir::Terminator::Invoke {
                    function: callee,
                    call,
                    ..
                } => {
                    let Some(callee) = callee.function() else {
                        continue;
                    };
                    let arguments = call.arguments.clone();

                    data.callsites.push(DirectCallSite {
                        caller: caller_id,
                        callee,
                        block: block_id,
                        call_instruction: None,
                        arguments,
                    });
                }
                mir::Terminator::InvokeIndirect { call, .. }
                | mir::Terminator::InvokeVirtual { call, .. }
                | mir::Terminator::InvokeInterface { call, .. } => {
                    if let Some(signature) = call
                        .signature
                        .ty()
                        .and_then(|signature| SignatureKey::from_signature_type(tree, signature))
                    {
                        data.indirect_signatures.insert(signature);
                    }
                }
                mir::Terminator::TailCall {
                    function: callee,
                    call,
                    ..
                } => {
                    let Some(callee) = callee.function() else {
                        continue;
                    };
                    let arguments = call.arguments.clone();

                    data.callsites.push(DirectCallSite {
                        caller: caller_id,
                        callee,
                        block: block_id,
                        call_instruction: None,
                        arguments,
                    });
                }
                mir::Terminator::TailCallIndirect { call, .. }
                | mir::Terminator::TailCallVirtual { call, .. }
                | mir::Terminator::TailCallInterface { call, .. } => {
                    if let Some(signature) = call
                        .signature
                        .ty()
                        .and_then(|signature| SignatureKey::from_signature_type(tree, signature))
                    {
                        data.indirect_signatures.insert(signature);
                    }
                }
                _ => {}
            }
        }
    }

    data
}

/// Check whether a call is pure enough to replace with a constant.
fn call_is_pure(
    tree: &mir::Tree,
    call_instruction: mir::LocalNodeId<mir::Instruction>,
    callee: mir::LocalNodeId<mir::Function>,
) -> bool {
    let instruction = tree.get(call_instruction);
    // resolve callsite effects when present
    let effects = instruction
        .call_memory_effect()
        .cloned()
        .or_else(|| Some(tree.get(callee).memory_effect.clone()));
    let behavior = instruction
        .call_behavior()
        .cloned()
        .or_else(|| Some(tree.get(callee).call_behavior.clone()));

    // reject calls with no effect metadata
    let Some(effects) = effects else {
        return false;
    };
    let Some(behavior) = behavior else {
        return false;
    };

    // reject reads or writes
    if effects.reads || effects.writes {
        return false;
    }

    // reject calls with non local behavior
    if behavior.unwind.may_unwind()
        || behavior.return_behavior.is_no_return()
        || behavior.must_not_duplicate
        || behavior.allocation.allocate.is_some()
        || behavior.allocation.free.is_some()
    {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Constant arguments are propagated into the callee.
    #[test]
    fn test_ip_sccp_inserts_constants() {
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
    v1: int32 = 7int32
    return v1
}
function root(): int32 {
b0:
    v0: int32 = 7int32
    v1: int32 = call callee(v0): (int32) -> int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&InterproceduralSccp);
        test.assert_output(expected);
    }

    /// Pure constant returns replace call results.
    #[test]
    fn test_ip_sccp_replaces_pure_call() {
        let input = r#"
function pure(): int32 {
b0:
    v0: int32 = 9int32
    return v0
}
function root(): int32 {
b0:
    v0: int32 = call pure(): () -> int32
    return v0
}"#;

        let expected = r#"
function pure(): int32 {
b0:
    v0: int32 = 9int32
    return v0
}
function root(): int32 {
b0:
    v0: int32 = 9int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        let callee_id = test.function_id_by_name("pure");
        test.tree.get_mut(callee_id).memory_effect = mir::MemoryEffect::none();
        test.tree.get_mut(callee_id).call_behavior = mir::CallBehavior::none();

        test.run_module_pass(&InterproceduralSccp);
        test.assert_output(expected);
    }

    /// Mismatched constants do not propagate into the callee.
    #[test]
    fn test_ip_sccp_skips_mismatched_constants() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function first(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = call callee(v0): (int32) -> int32
    return v1
}
function second(): int32 {
b0:
    v0: int32 = 2int32
    v1: int32 = call callee(v0): (int32) -> int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&InterproceduralSccp);
        test.assert_output(input);
    }

    /// Non constant callsites prevent parameter propagation.
    #[test]
    fn test_ip_sccp_skips_non_constant_callsite() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function root(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 1int32
    v2: int32 = call callee(v1): (int32) -> int32
    v3: int32 = call callee(v0): (int32) -> int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&InterproceduralSccp);
        test.assert_output(input);
    }

    /// Tailcalls participate in interprocedural SCCP.
    #[test]
    fn test_ip_sccp_propagates_tailcall() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function root(): int32 {
b0:
    v0: int32 = 9int32
    tailCall callee(v0): (int32) -> int32
}"#;

        let expected = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 9int32
    return v1
}
function root(): int32 {
b0:
    v0: int32 = 9int32
    tailCall callee(v0): (int32) -> int32
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&InterproceduralSccp);
        test.assert_output(expected);
    }

    /// Exceptional direct call terminators participate in interprocedural SCCP.
    #[test]
    fn test_ip_sccp_propagates_call_terminator() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function root(): int32 {
b0:
    v0: int32 = 9int32
    invoke callee(v0): (int32) -> int32 -> b1, catch b2
b1(v1: int32):
    return v1
b2(v2: ref<int32, managed, readonly>):
    throw v2
}"#;

        let expected = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 9int32
    return v1
}
function root(): int32 {
b0:
    v0: int32 = 9int32
    invoke callee(v0): (int32) -> int32 -> b1, catch b2
b1(v1: int32):
    return v1
b2(v2: ref<int32, managed, readonly>):
    throw v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&InterproceduralSccp);
        test.assert_output(expected);
    }

    /// Calls without effect metadata are not replaced.
    #[test]
    fn test_ip_sccp_skips_call_without_metadata() {
        let input = r#"
function pure(): int32 {
b0:
    v0: int32 = 9int32
    return v0
}
function root(): int32 {
b0:
    v0: int32 = call pure(): () -> int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&InterproceduralSccp);
        test.assert_output(input);
    }

    /// Indirect signatures mark callees as exposed.
    #[test]
    fn test_ip_sccp_skips_exposed_by_indirect_signature() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function root(v0: fn(int32) -> int32): int32  {
b0(v0: fn(int32) -> int32) -> v1: int32 = 7int32
    v2: int32 = call callee(v1): (int32) -> int32
    v3: int32 = call.indirect v0(v1): (int32) -> int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&InterproceduralSccp);
        test.assert_output(input);
    }
}
