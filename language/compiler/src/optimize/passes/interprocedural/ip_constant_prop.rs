use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::common::{
    SignatureKey, apply_constant_parameters, build_value_definition_map, constant_for_value,
    constant_matches_type, constant_type_of,
};
use crate::optimize::{AnalysisPreservation, ModulePass, PipelineContext};

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
    #[pass(id = "ip-constant-prop", requires(call_effects))]
    pub InterproceduralConstantPropagation,
    "Propagate constants across callsites"
}

impl ModulePass for InterproceduralConstantPropagation {
    /// Run interprocedural constant propagation for the module.
    fn run(&self, tree: &mut mir::Tree, ctx: &PipelineContext<'_>) -> AnalysisPreservation {
        let pointer_width_bits = ctx.options.type_context().pointer_width_bits;
        let changed = run_interprocedural_constant_prop(tree, pointer_width_bits);

        // report analysis preservation based on whether changes occurred
        if changed {
            ctx.strings.intern("ip-constant-prop");
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass display name.
    fn name(&self) -> &'static str {
        "InterproceduralConstantPropagation"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "ip-constant-prop"
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
fn run_interprocedural_constant_prop(tree: &mut mir::Tree, pointer_width_bits: u16) -> bool {
    // collect callsites up front
    let call_data = collect_call_data(tree);

    // cache value definitions by caller for fast constant lookups
    let definitions_by_function = build_definition_cache(tree);

    // track whether anything changed
    let mut changed = false;

    // scan candidate callees
    let function_ids: Vec<_> = tree
        .iter_nodes::<mir::Function>()
        .filter_map(|(id, function)| function.entry.is_some().then_some((id, function.linkage)))
        .collect();

    for (function_id, linkage) in function_ids {
        // skip imported and exported functions
        if linkage.is_exported() || linkage.is_import() {
            continue;
        }

        let function = tree.get(function_id);
        let Some(signature) = SignatureKey::from_function(tree, function) else {
            continue;
        };

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
        if apply_constant_parameters(function_id, &constants, tree) {
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
        if function.entry.is_none() {
            continue;
        }

        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);

                if let Some(dispatch) = instruction.call_dispatch_kind() {
                    if let mir::CallDispatchKind::Direct = dispatch
                        && let mir::Instruction::Call { function, call, .. } = instruction
                    {
                        let Some(function) = function.function() else {
                            continue;
                        };
                        let Some(arguments) = tree
                            .get_arguments(call.arguments)
                            .iter()
                            .map(|value| value.value())
                            .collect::<Option<Vec<_>>>()
                        else {
                            continue;
                        };

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
                        .and_then(|signature| signature.ty())
                        .and_then(|signature| SignatureKey::from_signature_type(tree, signature))
                    {
                        data.indirect_signatures.insert(signature);
                    }
                }
            }

            match terminator {
                mir::Terminator::Invoke { function, call, .. } => {
                    let Some(function) = function.function() else {
                        continue;
                    };
                    let Some(arguments) = call
                        .arguments
                        .iter()
                        .map(|value| value.value())
                        .collect::<Option<Vec<_>>>()
                    else {
                        continue;
                    };

                    data.direct_calls
                        .entry(function)
                        .or_default()
                        .push(DirectCallArgs {
                            caller: caller_id,
                            arguments,
                        });
                }
                mir::Terminator::InvokeIndirect { call, .. }
                | mir::Terminator::InvokeVirtual { call, .. }
                | mir::Terminator::InvokeInterface { call, .. } => {
                    let Some(signature) = call.signature.ty() else {
                        continue;
                    };
                    if let Some(signature) = SignatureKey::from_signature_type(tree, signature) {
                        data.indirect_signatures.insert(signature);
                    }
                }
                mir::Terminator::TailCall { function, call, .. } => {
                    let Some(function) = function.function() else {
                        continue;
                    };
                    let Some(arguments) = call
                        .arguments
                        .iter()
                        .map(|value| value.value())
                        .collect::<Option<Vec<_>>>()
                    else {
                        continue;
                    };

                    data.direct_calls
                        .entry(function)
                        .or_default()
                        .push(DirectCallArgs {
                            caller: caller_id,
                            arguments,
                        });
                }
                mir::Terminator::TailCallIndirect { call, .. }
                | mir::Terminator::TailCallVirtual { call, .. }
                | mir::Terminator::TailCallInterface { call, .. } => {
                    let Some(signature) = call.signature.ty() else {
                        continue;
                    };
                    if let Some(signature) = SignatureKey::from_signature_type(tree, signature) {
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
        if function.entry.is_none() {
            continue;
        }

        cache.insert(function_id, build_value_definition_map(function, tree));
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
            let Some(parameter_type) = param.ty.ty() else {
                candidate = None;
                break;
            };
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
    fn test_ip_constant_prop_inserts_constants() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}
function root(): int32 {
b0:
    v0: int32 = 40int32
    v1: int32 = 2int32
    v2: int32 = call callee(v0, v1): (int32, int32) -> int32
    return v2
}"#;

        let expected = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = 40int32
    v3: int32 = 2int32
    v4: int32 = int.add v2, v3
    return v4
}
function root(): int32 {
b0:
    v0: int32 = 40int32
    v1: int32 = 2int32
    v2: int32 = call callee(v0, v1): (int32, int32) -> int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&InterproceduralConstantPropagation);
        test.assert_output(expected);
    }

    /// Differing constants across callsites do not propagate.
    #[test]
    fn test_ip_constant_prop_skips_mismatched_constants() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}
function root(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = call callee(v0): (int32) -> int32
    return v1
}
function other(): int32 {
b0:
    v0: int32 = 2int32
    v1: int32 = call callee(v0): (int32) -> int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&InterproceduralConstantPropagation);
        test.assert_output(input);
    }

    /// Indirect call signatures prevent propagation.
    #[test]
    fn test_ip_constant_prop_skips_indirect_signature() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}
function root(v0: fn(int32) -> int32, v1: int32): int32  {
b0(v0: fn(int32) -> int32, v1: int32) -> v2: int32 = call.indirect v0(v1): (int32) -> int32
    v3: int32 = 4int32
    v4: int32 = call callee(v3): (int32) -> int32
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&InterproceduralConstantPropagation);
        test.assert_output(input);
    }

    /// Readonly global loads are not propagated as call constants.
    #[test]
    fn test_ip_constant_prop_skips_global_load() {
        let input = r#"
global value: int32, readonly = 7int32
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function root(): int32 {
b0:
    v0: ref<int32, raw, readonly> = global.address value
    v1: int32 = load v0
    v2: int32 = call callee(v1): (int32) -> int32
    return v2
}"#;

        let expected = r#"
global value: int32, readonly = 7int32
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function root(): int32 {
b0:
    v0: ref<int32, raw, readonly> = global.address value
    v1: int32 = load v0
    v2: int32 = call callee(v1): (int32) -> int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&InterproceduralConstantPropagation);
        test.assert_output(expected);
    }

    /// Exceptional direct call terminators contribute constants to the callee.
    #[test]
    fn test_ip_constant_prop_propagates_call_terminator() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function root(): int32 {
b0:
    v0: int32 = 4int32
    invoke callee(v0): (int32) -> int32 -> b1, catch b2
b1(v1: int32):
    return v1
b2(v2: ref<int32, managed, readonly>):
    throw v2
}"#;

        let expected = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 4int32
    return v1
}
function root(): int32 {
b0:
    v0: int32 = 4int32
    invoke callee(v0): (int32) -> int32 -> b1, catch b2
b1(v1: int32):
    return v1
b2(v2: ref<int32, managed, readonly>):
    throw v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&InterproceduralConstantPropagation);
        test.assert_output(expected);
    }

    /// Tailcalls participate in constant propagation.
    #[test]
    fn test_ip_constant_prop_propagates_tailcall() {
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
        test.run_module_pass(&InterproceduralConstantPropagation);
        test.assert_output(expected);
    }
}
