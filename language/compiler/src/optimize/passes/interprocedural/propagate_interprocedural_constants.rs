use std::collections::{HashMap, HashSet};

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{MirOptimized, ModulePass, PipelineContext};
use destack_mir::{
    Mutation, SignatureKey, ValueDefinitions, apply_constant_parameters, constant_for_value,
    constant_matches_type, constant_type_of,
};

declare_pass! {
    /// Propagate constants across direct callsites.
    ///
    /// This pass substitutes parameters in a callee when all direct callsites
    /// pass the same constant, then leaves dead argument removal to later passes.
    ///
    /// ```mir
    /// function callee(v0: int32, v1: int32): int32 {
    /// b0(v0: int32, v1: int32):
    ///     v2 = int.add v0, v1
    ///     return v2
    /// }
    /// function root(): int32 {
    /// b0:
    ///     v0 = 40int32
    ///     v1 = 2int32
    ///     v2 = call callee(v0, v1)
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function callee(v0: int32, v1: int32): int32 {
    /// b0(v0: int32, v1: int32):
    ///     v3 = 40int32
    ///     v4 = 2int32
    ///     v2 = int.add v3, v4
    ///     return v2
    /// }
    /// function root(): int32 {
    /// b0:
    ///     v0 = 40int32
    ///     v1 = 2int32
    ///     v2 = call callee(v0, v1)
    ///     return v2
    /// }
    /// ```
    #[pass(id = "propagate-interprocedural-constants")]
    pub PropagateInterproceduralConstants,
    "Propagate constants across callsites"
}

impl ModulePass for PropagateInterproceduralConstants {
    /// Run interprocedural constant propagation for the module.
    fn run(
        &self,
        optimized: &mut MirOptimized,
        ctx: &PipelineContext<'_>,
        _analyses: &mir::TreeAnalysisCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;
        let memory = &mut optimized.memory;

        let pointer_width_bits = ctx.options.target_layout().pointer_bits();
        let changed = run_interprocedural_constant_prop(tree, memory, pointer_width_bits);

        // report what this pass changed
        if changed {
            ctx.strings.intern("propagate-interprocedural-constants");
            Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }

    /// Return the pass display name.
    fn name(&self) -> &'static str {
        "PropagateInterproceduralConstants"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "propagate-interprocedural-constants"
    }
}

/// Direct call arguments tracked by callee.
#[derive(Debug, Clone)]
struct DirectCallArgs {
    /// The caller function id.
    caller: mir::LocalNodeId<mir::Function>,
    /// The arguments passed at the callsite.
    arguments: Vec<mir::Value>,
}

/// Collected callsite data for interprocedural constant propagation.
#[derive(Debug, Default)]
struct CallData {
    /// Direct call arguments keyed by callee.
    direct_calls: HashMap<mir::LocalNodeId<mir::Function>, Vec<DirectCallArgs>>,
    /// Signatures that may be targeted by indirect calls.
    indirect_signatures: HashSet<SignatureKey>,
}

/// Run interprocedural constant propagation over the module.
fn run_interprocedural_constant_prop(
    tree: &mut mir::Tree,
    memory: &mut mir::MemoryTable,
    pointer_width_bits: u16,
) -> bool {
    // collect callsites up front
    let call_data = collect_call_data(tree);

    // cache value definitions by caller for fast constant lookups
    let definitions_by_function = build_definition_cache(tree);

    // track whether anything changed
    let mut changed = false;

    // scan candidate callees
    let function_ids: Vec<_> = tree
        .iter_nodes::<mir::Function>()
        .filter_map(|(id, function)| function.entry().is_some().then_some((id, function.linkage)))
        .collect();

    for (function_id, linkage) in function_ids {
        // skip imported and exported functions
        if linkage.is_exported() || linkage.is_import() {
            continue;
        }

        let function = tree.get(function_id);
        let signature = SignatureKey::from_function(function);

        // skip functions reachable through indirect calls
        if call_data.indirect_signatures.contains(&signature) {
            continue;
        }

        // require at least one direct callsite
        let Some(callsites) = call_data.direct_calls.get(&function_id) else {
            continue;
        };

        // compute constant arguments for each parameter
        let constants = constant_parameters(
            function,
            callsites,
            &definitions_by_function,
            tree,
            pointer_width_bits,
        );
        if constants.iter().all(|entry| entry.is_none()) {
            continue;
        }

        // insert constants and rewrite uses inside the callee
        if apply_constant_parameters(function_id, &constants, tree, memory) {
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
        if function.entry().is_none() {
            continue;
        }

        for &block_id in function.blocks() {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);

                if let Some(dispatch) = instruction.call_dispatch() {
                    if let mir::CallDispatch::Direct = dispatch
                        && let mir::Instruction::Call { call, .. } = instruction
                        && let Some(function) = call.callee.function()
                    {
                        let arguments = tree.get_values(call.arguments).to_vec();

                        data.direct_calls
                            .entry(function)
                            .or_default()
                            .push(DirectCallArgs {
                                caller: caller_id,
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

            match terminator {
                mir::Terminator::Invoke { call, .. } | mir::Terminator::TailCall { call } => {
                    if let Some(function) = call.callee.function() {
                        let arguments = tree.get_values(call.arguments).to_vec();

                        data.direct_calls
                            .entry(function)
                            .or_default()
                            .push(DirectCallArgs {
                                caller: caller_id,
                                arguments,
                            });
                    } else if let Some(signature) =
                        SignatureKey::from_signature_type(tree, &call.signature)
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

/// Build definition maps for each function with a body.
fn build_definition_cache(
    tree: &mir::Tree,
) -> HashMap<mir::LocalNodeId<mir::Function>, HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>>
{
    // prepare the cache container
    let mut cache = HashMap::new();

    // build definition maps per function
    for (function_id, function) in tree.iter_nodes::<mir::Function>() {
        if function.entry().is_none() {
            continue;
        }

        cache.insert(
            function_id,
            ValueDefinitions::build(function, tree).instruction_map(),
        );
    }

    cache
}

/// Compute constant parameters for a callee when all direct callsites agree.
fn constant_parameters(
    function: &mir::Function,
    callsites: &[DirectCallArgs],
    definitions_by_function: &HashMap<
        mir::LocalNodeId<mir::Function>,
        HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    >,
    tree: &mir::Tree,
    pointer_width_bits: u16,
) -> Vec<Option<mir::Constant>> {
    // allocate constant slots for each parameter
    let mut constants = vec![None; function.parameters.len()];

    // evaluate each parameter position independently
    for (index, param) in function.parameters.iter().enumerate() {
        let mut candidate: Option<mir::Constant> = None;

        for callsite in callsites {
            // ensure the callsite has the argument position
            let Some(argument) = callsite.arguments.get(index) else {
                candidate = None;
                break;
            };

            // resolve constant from the caller's definitions
            let Some(definitions) = definitions_by_function.get(&callsite.caller) else {
                candidate = None;
                break;
            };
            let Some(constant) = constant_for_value(*argument, definitions, tree) else {
                candidate = None;
                break;
            };

            // validate the constant matches the parameter type
            let constant_type = constant_type_of(&constant);
            let parameter_type = param.ty;
            if !constant_matches_type(constant_type, parameter_type, pointer_width_bits, tree) {
                candidate = None;
                break;
            }

            // unify constants across callsites
            match &candidate {
                None => candidate = Some(constant),
                Some(existing) if *existing == constant => {}
                Some(_) => {
                    candidate = None;
                    break;
                }
            }
        }

        constants[index] = candidate;
    }

    constants
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Constant call arguments are propagated into the callee.
    #[test]
    fn test_propagate_interprocedural_constants_inserts_constants() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}

function root(): int32 {
entry:
    v0: int32 = 40
    v1: int32 = 2
    v2: int32 = call callee(v0, v1)
    return v2
}
"#;

        let expected = r#"
function callee(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v3: int32 = 40
    v4: int32 = 2
    v2: int32 = int.add v3, v4
    return v2
}

function root(): int32 {
entry:
    v0: int32 = 40
    v1: int32 = 2
    v2: int32 = call callee(v0, v1)
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&PropagateInterproceduralConstants);
        test.assert_output(expected);
    }

    /// Differing constants across callsites do not propagate.
    #[test]
    fn test_propagate_interprocedural_constants_skips_mismatched_constants() {
        let input = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}

function root(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = call callee(v0)
    return v1
}

function other(): int32 {
entry:
    v0: int32 = 2
    v1: int32 = call callee(v0)
    return v1
}
"#;

        let expected = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}

function root(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = call callee(v0)
    return v1
}

function other(): int32 {
entry:
    v0: int32 = 2
    v1: int32 = call callee(v0)
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&PropagateInterproceduralConstants);
        test.assert_output(expected);
    }

    /// Indirect call signatures prevent propagation.
    #[test]
    fn test_propagate_interprocedural_constants_skips_indirect_signature() {
        let input = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}

function root(v0: fn(int32) => int32, v1: int32): int32 {
entry(v0: fn(int32) => int32, v1: int32):
    v2: int32 = call.indirect v0(v1): (int32) => int32
    v3: int32 = 4
    v4: int32 = call callee(v3)
    return v4
}
"#;

        let expected = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}

function root(v0: fn(int32) => int32, v1: int32): int32 {
entry(v0: fn(int32) => int32, v1: int32):
    v2: int32 = call.indirect v0(v1): (int32) => int32
    v3: int32 = 4
    v4: int32 = call callee(v3)
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&PropagateInterproceduralConstants);
        test.assert_output(expected);
    }

    /// Readonly global loads are not propagated as call constants.
    #[test]
    fn test_propagate_interprocedural_constants_skips_global_load() {
        let input = r#"
readonly global value: int32 = 7

function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function root(): int32 {
entry:
    v0: ref<int32, raw, readonly> = global.address value
    v1: int32 = load v0
    v2: int32 = call callee(v1)
    return v2
}
"#;

        let expected = r#"
readonly global value: int32 = 7

function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function root(): int32 {
entry:
    v0: ref<int32, raw, readonly> = global.address value
    v1: int32 = load v0
    v2: int32 = call callee(v1)
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&PropagateInterproceduralConstants);
        test.assert_output(expected);
    }

    /// Direct invokes contribute constants to the callee.
    #[test]
    fn test_propagate_interprocedural_constants_propagates_invoke() {
        let input = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function root(): int32 {
entry:
    v0: int32 = 4
    invoke callee(v0) => b1 | b2

b1(v1: int32):
    return v1

b2:
    unwind.resume
}
"#;

        let expected = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 4
    return v1
}

function root(): int32 {
entry:
    v0: int32 = 4
    invoke callee(v0) => b1 | b2

b1(v1: int32):
    return v1

b2:
    unwind.resume
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&PropagateInterproceduralConstants);
        test.assert_output(expected);
    }

    /// Tailcalls participate in constant propagation.
    #[test]
    fn test_propagate_interprocedural_constants_propagates_tailcall() {
        let input = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function root(): int32 {
entry:
    v0: int32 = 9
    tail.call callee(v0)
}
"#;

        let expected = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 9
    return v1
}

function root(): int32 {
entry:
    v0: int32 = 9
    tail.call callee(v0)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&PropagateInterproceduralConstants);
        test.assert_output(expected);
    }
}
