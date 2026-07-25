use std::collections::{HashMap, HashSet};

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{MirOptimized, ModulePass, PipelineContext};
use destack_mir::{Mutation, ParameterRemap, SignatureKey, build_use_def_maps};

declare_pass! {
    /// Remove unused parameters from local functions and their callsites.
    ///
    /// This pass removes parameters that are not used by a function body, updates
    /// direct callsites, and trims any callsite tables. Indirect calls are
    /// treated conservatively and block changes for matching signatures.
    ///
    /// ```mir
    /// function before(v0: int32, v1: int32): int32 {
    /// b0(v0: int32, v1: int32):
    ///     return v0
    /// }
    /// function root(v0: int32, v1: int32): int32 {
    /// b0(v0: int32, v1: int32):
    ///     v2 = call before(v0, v1)
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: int32): int32 {
    /// b0(v0: int32):
    ///     return v0
    /// }
    /// function root(v0: int32, v1: int32): int32 {
    /// b0(v0: int32, v1: int32):
    ///     v2 = call after(v0)
    ///     return v2
    /// }
    /// ```
    #[pass(id = "eliminate-dead-arguments")]
    pub EliminateDeadArguments,
    "Eliminate unused function arguments"
}

impl ModulePass for EliminateDeadArguments {
    /// Run dead argument elimination for the module.
    fn run(
        &self,
        optimized: &mut MirOptimized,
        ctx: &PipelineContext<'_>,
        _analyses: &mir::TreeAnalysisCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;
        let layouts = &mut optimized.layouts;
        let effects = &mut optimized.effects;

        let changed = run_eliminate_dead_arguments(tree, layouts, effects);

        // report what this pass changed
        if changed {
            ctx.strings.intern("eliminate-dead-arguments");
            Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }

    /// Return the pass display name.
    fn name(&self) -> &'static str {
        "EliminateDeadArguments"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "eliminate-dead-arguments"
    }
}

/// Direct callsite reference for updating arguments.
#[derive(Debug, Clone, Copy)]
enum DirectCallSite {
    /// Direct call instruction.
    Instruction(mir::LocalNodeId<mir::Instruction>),
    /// Direct invoke or tail call.
    Terminator(mir::LocalNodeId<mir::Block>),
}

/// Collected callsite data for dead argument elimination.
#[derive(Debug, Default)]
struct CallData {
    /// Direct callsites keyed by callee function id.
    direct_calls: HashMap<mir::LocalNodeId<mir::Function>, Vec<DirectCallSite>>,
    /// Signatures that may be targeted by indirect calls.
    indirect_signatures: HashSet<SignatureKey>,
}

/// Run dead argument elimination over the module.
fn run_eliminate_dead_arguments(
    tree: &mut mir::Tree,
    layouts: &mut mir::LayoutTable,
    effects: &mut mir::EffectTable,
) -> bool {
    // collect callsite information up front
    let call_data = collect_call_data(tree);

    // track whether anything changed
    let mut changed = false;

    // scan each defined function for unused parameters
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

        // skip functions that might be called indirectly
        if call_data.indirect_signatures.contains(&signature) {
            continue;
        }

        // collect unused parameter indices
        let unused = unused_parameter_indices(function, tree);
        if unused.is_empty() {
            continue;
        }

        // update the function signature and entry block parameters
        apply_parameter_removals(function_id, &unused, tree);

        // update direct callsites that target this function
        if let Some(calls) = call_data.direct_calls.get(&function_id) {
            update_call_sites(function_id, calls, &unused, tree, layouts, effects);
        }

        changed = true;
    }

    changed
}

/// Collect direct callsites and indirect signatures for the module.
fn collect_call_data(tree: &mir::Tree) -> CallData {
    // prepare the callsite data container
    let mut data = CallData::default();

    // scan each function body for calls
    for (_function_id, function) in tree.iter_nodes::<mir::Function>() {
        if function.entry().is_none() {
            continue;
        }

        for &block_id in function.blocks() {
            let block = tree.get(block_id);

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);

                if let Some(dispatch) = instruction.call_dispatch() {
                    if let mir::CallDispatch::Direct = dispatch
                        && let Some(function) = instruction.call_direct_target()
                    {
                        data.direct_calls
                            .entry(function)
                            .or_default()
                            .push(DirectCallSite::Instruction(instruction_id));
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

            // record invokes and tail calls
            let terminator = tree.get(block.terminator);
            match terminator {
                mir::Terminator::Invoke { call, .. } | mir::Terminator::TailCall { call } => {
                    if let Some(function) = call.callee.function() {
                        data.direct_calls
                            .entry(function)
                            .or_default()
                            .push(DirectCallSite::Terminator(block_id));
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

/// Collect unused parameter indices for a function body.
fn unused_parameter_indices(function: &mir::Function, tree: &mir::Tree) -> Vec<usize> {
    // collect uses without treating parameters as implicitly used
    let use_def = build_use_def_maps(function, tree);

    // collect parameters required by signature obligations
    let required = ParameterRemap::required_indices(function, tree);

    // collect parameters that have no uses
    let mut unused = Vec::new();
    for (index, param) in function.parameters.iter().enumerate() {
        if required.contains(&index) {
            continue;
        }

        let value = param.value;

        if !use_def.use_blocks.contains_key(&value) {
            unused.push(index);
        }
    }

    unused
}

/// Apply parameter removals to a function signature and entry block.
fn apply_parameter_removals(
    function_id: mir::LocalNodeId<mir::Function>,
    unused: &[usize],
    tree: &mut mir::Tree,
) {
    // prepare removal remapping data
    let remap = ParameterRemap::new(unused);

    // update function parameters and attributes
    let entry_id = {
        let function = tree.get_mut(function_id);
        function.parameters = remap.filter_by_index(&function.parameters);

        function
            .entry()
            .unwrap_or_else(|| panic!("missing entry for rewritten function: {function_id:?}"))
    };

    // update entry block parameters to match the new signature
    let entry = tree.get_mut(entry_id);
    entry.parameters = remap.filter_by_index(&entry.parameters);
}

/// Update direct callsites to drop removed arguments.
fn update_call_sites(
    function_id: mir::LocalNodeId<mir::Function>,
    call_sites: &[DirectCallSite],
    unused: &[usize],
    tree: &mut mir::Tree,
    layouts: &mut mir::LayoutTable,
    effects: &mut mir::EffectTable,
) {
    // prepare removal remapping data
    let remap = ParameterRemap::new(unused);

    // insert the rewritten function signature once
    let function = tree.get(function_id);
    let signature = tree.intern_type(function.signature());

    // update callsite arguments
    for site in call_sites {
        match *site {
            DirectCallSite::Instruction(instruction_id) => {
                let (destination, slice, call) = match tree.get(instruction_id) {
                    mir::Instruction::Call { destination, call }
                        if call.callee.function() == Some(function_id) =>
                    {
                        (*destination, call.arguments, call.clone())
                    }
                    _ => panic!("stale direct callsite instruction: {instruction_id:?}"),
                };

                // filter the argument list
                let arguments = remap.filter_by_index(tree.get_values(slice));

                // preserve the no storage signature layout
                layouts.copy_type_entries(call.signature, signature);

                // update the call instruction with the new argument slice
                let new_slice = tree.add_values(&arguments);
                let callsite = mir::CallSite::Instruction(instruction_id);
                let mut call = call;
                call.arguments = new_slice;
                call.signature = signature;

                let updated = mir::Instruction::Call { destination, call };
                *tree.get_mut(instruction_id) = updated;

                // preserve tables when the callsite carries it
                if let Some(tables) = effects.calls.get_mut(&callsite) {
                    tables.arguments = remap.filter_by_index(&tables.arguments);
                }
            }
            DirectCallSite::Terminator(block_id) => {
                let block = tree.get_mut(block_id);
                let terminator_id = block.terminator;
                let terminator = tree.get(terminator_id).clone();
                match &terminator {
                    mir::Terminator::Invoke {
                        call,
                        target,
                        unwind,
                    } if call.callee.function() == Some(function_id) => {
                        // filter the argument list
                        let mut new_call = call.clone();
                        let arguments = remap.filter_by_index(tree.get_values(call.arguments));
                        new_call.arguments = tree.add_values(&arguments);
                        layouts.copy_type_entries(call.signature, signature);
                        new_call.signature = signature;

                        let new_terminator = mir::Terminator::Invoke {
                            call: new_call,
                            target: target.clone(),
                            unwind: unwind.clone(),
                        };
                        tree.set(terminator_id, new_terminator);
                    }
                    mir::Terminator::TailCall { call }
                        if call.callee.function() == Some(function_id) =>
                    {
                        // filter the argument list
                        let mut new_call = call.clone();
                        let arguments = remap.filter_by_index(tree.get_values(call.arguments));
                        new_call.arguments = tree.add_values(&arguments);
                        layouts.copy_type_entries(call.signature, signature);
                        new_call.signature = signature;

                        let new_terminator = mir::Terminator::TailCall { call: new_call };
                        tree.set(terminator_id, new_terminator);
                    }
                    _ => panic!("stale direct callsite terminator: {block_id:?}"),
                }

                // preserve tables when the terminator carries it
                let callsite = mir::CallSite::Terminator(block_id);
                if let Some(tables) = effects.calls.get_mut(&callsite) {
                    tables.arguments = remap.filter_by_index(&tables.arguments);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Unused parameters are removed from direct callsites.
    #[test]
    fn test_eliminate_dead_arguments_removes_unused_param() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    return v0
}

function root(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = call callee(v0, v1): (int32, int32) => int32
    return v2
}
"#;

        let expected = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function root(v0: int32): int32 {
entry(v0: int32):
    v2: int32 = call callee(v0): (int32) => int32
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateDeadArguments);
        test.assert_output(expected);
    }

    /// Tailcall arguments are trimmed for unused parameters.
    #[test]
    fn test_eliminate_dead_arguments_updates_tailcall() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    return v0
}

function root(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    tail.call callee(v0, v1): (int32, int32) => int32
}
"#;

        let expected = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function root(v0: int32): int32 {
entry(v0: int32):
    tail.call callee(v0): (int32) => int32
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateDeadArguments);
        test.assert_output(expected);
    }

    /// Direct invokes are trimmed for unused parameters.
    #[test]
    fn test_eliminate_dead_arguments_updates_invoke() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    return v0
}

function root(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    invoke callee(v0, v1): (int32, int32) => int32 => b1 | b2

b1(v2: int32):
    return v2

b2(v3: ref<int32, managed, readonly>):
    panic v3
}
"#;

        let expected = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function root(v0: int32): int32 {
entry(v0: int32):
    invoke callee(v0): (int32) => int32 => b1 | b2

b1(v2: int32):
    return v2

b2(v3: ref<int32, managed, readonly>):
    panic v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateDeadArguments);
        test.assert_output(expected);
    }

    /// Exported functions are not rewritten.
    #[test]
    fn test_eliminate_dead_arguments_skips_exports() {
        let input = r#"
export function callee(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    return v0
}

function root(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = call callee(v0, v1): (int32, int32) => int32
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateDeadArguments);
        test.assert_output(input);
    }

    /// Indirect signatures block argument removal.
    #[test]
    fn test_eliminate_dead_arguments_skips_indirect_signature() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    return v0
}

function root(v0: fn(int32, int32) => int32, v1: int32, v2: int32): int32 {
entry(v0: fn(int32, int32) => int32, v1: int32, v2: int32):
    v3: int32 = call.indirect v0(v1, v2): (int32, int32) => int32
    v4: int32 = call callee(v1, v2): (int32, int32) => int32
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&EliminateDeadArguments);
        test.assert_output(input);
    }

    /// Call tables argument lists are trimmed alongside arguments.
    #[test]
    fn test_eliminate_dead_arguments_updates_call_entries() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    return v0
}

function root(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = call callee(v0, v1): (int32, int32) => int32
    return v2
}
"#;

        let expected = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function root(v0: int32): int32 {
entry(v0: int32):
    v2: int32 = call callee(v0): (int32) => int32
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        let root_id = test.function_id_by_name("root");
        let call_id = test
            .entry_instructions(root_id)
            .into_iter()
            .find(|instruction_id| {
                matches!(
                    test.optimized.tree.get(*instruction_id),
                    mir::Instruction::Call { .. }
                )
            })
            .expect("missing call instruction");

        let callsite = mir::CallSite::Instruction(call_id);
        test.optimized.effects.call_mut(callsite).arguments = vec![
            mir::CallArgumentEffect::default(),
            mir::CallArgumentEffect::default(),
        ];

        test.run_module_pass(&EliminateDeadArguments);
        test.assert_output(expected);
        let callsite = mir::CallSite::Instruction(call_id);
        let tables = test
            .optimized
            .effects
            .call(callsite)
            .expect("missing call metadata");
        assert_eq!(tables.arguments.len(), 1);
    }
}
