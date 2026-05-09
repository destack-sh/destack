use std::collections::{HashMap, HashSet};

use crate::declare_mir_pass;
use destack_mir as mir;

use crate::common::mir::{
    ParameterRemap, SignatureKey, build_signature_type, build_use_def_maps,
    required_parameter_indices,
};
use crate::optimize::{AnalysisPreservation, ModulePass, PipelineContext};

declare_mir_pass! {
    /// Remove unused parameters from local functions and their callsites.
    ///
    /// This pass removes parameters that are not used by a function body, updates
    /// direct callsites, and trims any callsite metadata. Indirect calls are
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
    #[pass(id = "dead-arg-eliminate", requires(call_effects))]
    pub DeadArgEliminate,
    "Eliminate unused function arguments"
}

impl ModulePass for DeadArgEliminate {
    /// Run dead argument elimination for the module.
    fn run(&self, tree: &mut mir::Tree, ctx: &PipelineContext<'_>) -> AnalysisPreservation {
        let changed = run_dead_arg_eliminate(tree);

        // report analysis preservation based on whether changes occurred
        if changed {
            ctx.strings.intern("dead-arg-eliminate");
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass display name.
    fn name(&self) -> &'static str {
        "DeadArgEliminate"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "dead-arg-eliminate"
    }
}

/// Direct callsite reference for updating arguments.
#[derive(Debug, Clone, Copy)]
enum DirectCallSite {
    /// Direct call instruction.
    Instruction(mir::LocalNodeId<mir::Instruction>),
    /// Direct call terminator.
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
fn run_dead_arg_eliminate(tree: &mut mir::Tree) -> bool {
    // collect callsite information up front
    let call_data = collect_call_data(tree);

    // track whether anything changed
    let mut changed = false;

    // scan each defined function for unused parameters
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
            update_call_sites(function_id, calls, &unused, tree);
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
        if function.entry.is_none() {
            continue;
        }

        for &block_id in &function.blocks {
            let block = tree.get(block_id);

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);

                if let Some(dispatch) = instruction.call_dispatch_kind() {
                    if let mir::CallDispatchKind::Direct = dispatch
                        && let mir::Instruction::Call { function, .. } = instruction
                        && let Some(function) = function.function()
                    {
                        data.direct_calls
                            .entry(function)
                            .or_default()
                            .push(DirectCallSite::Instruction(instruction_id));
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

            // record call terminators
            let terminator = tree.get(block.terminator);
            match terminator {
                mir::Terminator::Invoke { function, .. } => {
                    let Some(function) = function.function() else {
                        continue;
                    };

                    data.direct_calls
                        .entry(function)
                        .or_default()
                        .push(DirectCallSite::Terminator(block_id));
                }
                mir::Terminator::InvokeIndirect { call, .. }
                | mir::Terminator::InvokeVirtual { call, .. }
                | mir::Terminator::InvokeInterface { call, .. } => {
                    if let Some(signature) = SignatureKey::from_signature_type(tree, call.signature)
                    {
                        data.indirect_signatures.insert(signature);
                    }
                }
                mir::Terminator::TailCall { function, .. } => {
                    let Some(function) = function.function() else {
                        continue;
                    };

                    data.direct_calls
                        .entry(function)
                        .or_default()
                        .push(DirectCallSite::Terminator(block_id));
                }
                mir::Terminator::TailCallIndirect { call, .. }
                | mir::Terminator::TailCallVirtual { call, .. }
                | mir::Terminator::TailCallInterface { call, .. } => {
                    if let Some(signature) = SignatureKey::from_signature_type(tree, call.signature)
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

    // collect parameters that are required by metadata
    let required = required_parameter_indices(function);

    // collect parameters that have no uses
    let mut unused = Vec::new();
    for (index, param) in function.parameters.iter().enumerate() {
        if required.contains(&index) {
            continue;
        }

        let Some(value) = param.value.value() else {
            continue;
        };

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
    // collect removed parameter values for debug updates
    let removed_values: HashSet<mir::Value> = {
        let function = tree.get(function_id);
        unused
            .iter()
            .filter_map(|index| function.parameters.get(*index))
            .filter_map(|param| param.value.value())
            .collect()
    };

    // prepare removal remapping data
    let remap = ParameterRemap::new(unused);

    // update function parameters and attributes
    let entry_id = {
        let function = tree.get_mut(function_id);
        function.parameters = remap.filter_by_index(&function.parameters);
        function.parameter_attributes = remap.filter_by_index(&function.parameter_attributes);
        function.return_lifetime = remap
            .remap_return_lifetime(&function.return_lifetime)
            .expect("return lifetime parameter must be preserved");
        function.allocation_size = remap.remap_allocation_size(function.allocation_size);
        let Some(entry_id) = function.entry else {
            return;
        };

        entry_id
    };

    // update entry block parameters to match the new signature
    let entry = tree.get_mut(entry_id);
    entry.parameters = remap.filter_by_index(&entry.parameters);

    // update debug info for removed parameters
    update_debug_for_removed_parameters(function_id, &removed_values, tree);
}

/// Update direct callsites to drop removed arguments.
fn update_call_sites(
    function_id: mir::LocalNodeId<mir::Function>,
    call_sites: &[DirectCallSite],
    unused: &[usize],
    tree: &mut mir::Tree,
) {
    // prepare removal remapping data
    let remap = ParameterRemap::new(unused);

    // prepare a signature type for updated call signatures
    let mut signature_type: Option<mir::LocalNodeId<mir::Type>> = None;

    // update callsite arguments
    for site in call_sites {
        match *site {
            DirectCallSite::Instruction(instruction_id) => {
                let (destination, function, slice, call) = match tree.get(instruction_id) {
                    mir::Instruction::Call {
                        destination,
                        function,
                        call,
                    } => (*destination, *function, call.arguments, call.clone()),
                    _ => continue,
                };

                // filter the argument list
                let arguments = remap.filter_by_index(tree.get_arguments(slice));

                // refresh the signature when arguments are removed
                let signature = if unused.is_empty() {
                    call.signature
                } else {
                    (*signature_type.get_or_insert_with(|| build_signature_type(function_id, tree)))
                        .into()
                };

                // update the call instruction with the new argument slice
                let new_slice = tree.add_arguments(&arguments);
                let mut call = call;
                call.arguments = new_slice;
                call.signature = signature;
                call.argument_attributes = remap.filter_by_index(&call.argument_attributes);
                call.allocation_size = remap.remap_allocation_size(call.allocation_size);

                let updated = mir::Instruction::Call {
                    destination,
                    function,
                    call,
                };
                *tree.get_mut(instruction_id) = updated;
            }
            DirectCallSite::Terminator(block_id) => {
                let block = tree.get_mut(block_id);
                let terminator_id = block.terminator;
                let terminator = tree.get(terminator_id).clone();
                match &terminator {
                    mir::Terminator::Invoke {
                        function,
                        call,
                        normal_target,
                        unwind_target,
                    } => {
                        // filter the argument list
                        let mut new_call = call.clone();
                        new_call.arguments = remap.filter_by_index(&call.arguments);

                        let new_terminator = mir::Terminator::Invoke {
                            function: *function,
                            call: new_call,
                            normal_target: normal_target.clone(),
                            unwind_target: unwind_target.clone(),
                        };
                        tree.replace(terminator_id, new_terminator);
                    }
                    mir::Terminator::TailCall { function, call } => {
                        // filter the argument list
                        let mut new_call = call.clone();
                        new_call.arguments = remap.filter_by_index(&call.arguments);

                        let new_terminator = mir::Terminator::TailCall {
                            function: *function,
                            call: new_call,
                        };
                        tree.replace(terminator_id, new_terminator);
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Update debug info for removed parameter values.
fn update_debug_for_removed_parameters(
    function_id: mir::LocalNodeId<mir::Function>,
    removed_values: &HashSet<mir::Value>,
    tree: &mut mir::Tree,
) {
    // read the function scope for parameter variables
    let Some(function_scope) = tree
        .metadata
        .debug
        .function_scopes
        .get(&function_id)
        .copied()
    else {
        return;
    };

    // collect debug variables that reference removed values
    let mut to_update = Vec::new();
    for (index, binding) in tree.metadata.debug.bindings.iter().enumerate() {
        // skip non parameter bindings
        if binding.kind != mir::DebugBindingKind::Parameter {
            continue;
        }

        // skip bindings outside the function scope
        if !scope_in_function(binding.scope, function_scope, &tree.metadata.debug) {
            continue;
        }

        // read the current binding location ranges
        let binding_id = mir::DebugBindingId::new(index as u32);
        let Some(ranges) = tree.metadata.debug.binding_ranges.get(&binding_id) else {
            continue;
        };

        // collect removed parameter locations
        let is_removed = ranges.iter().any(|range| {
            matches!(&range.location, mir::DebugValueLocation::Value(value) if removed_values.contains(value))
        });

        if is_removed {
            to_update.push(binding_id);
        }
    }

    // rewrite removed parameter locations to undefined
    for binding_id in to_update {
        if let Some(ranges) = tree.metadata.debug.binding_ranges.get_mut(&binding_id) {
            for range in ranges {
                range.location = mir::DebugValueLocation::Undefined;
            }
        }
    }
}

/// Return true when a debug scope belongs to a function scope.
fn scope_in_function(
    scope: mir::DebugScopeId,
    function_scope: mir::DebugScopeId,
    debug_info: &mir::DebugMetadata,
) -> bool {
    // walk the scope chain to find the function scope
    let mut current = Some(scope);

    while let Some(scope_id) = current {
        // stop once the function scope is found
        if scope_id == function_scope {
            return true;
        }

        // step to the parent scope
        current = debug_info.scope(scope_id).parent;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Unused parameters are removed from direct callsites.
    #[test]
    fn test_dead_arg_eliminate_removes_unused_param() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    return v0
}
function root(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = call callee(v0, v1): (int32, int32) -> int32
    return v2
}"#;

        let expected = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function root(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = call callee(v0): (int32) -> int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&DeadArgEliminate);
        test.assert_output(expected);
    }

    /// Tailcall arguments are trimmed for unused parameters.
    #[test]
    fn test_dead_arg_eliminate_updates_tailcall() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    return v0
}
function root(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    tailCall callee(v0, v1): (int32, int32) -> int32
}"#;

        let expected = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function root(v0: int32): int32 {
b0(v0: int32):
    tailCall callee(v0): (int32) -> int32
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&DeadArgEliminate);
        test.assert_output(expected);
    }

    /// Exceptional direct call terminators are trimmed for unused parameters.
    #[test]
    fn test_dead_arg_eliminate_updates_call_terminator() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    return v0
}
function root(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    invoke callee(v0, v1): (int32, int32) -> int32 -> b1, catch b2
b1(v2: int32):
    return v2
b2(v3: ref<int32, managed, readonly>):
    throw v3
}"#;

        let expected = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function root(v0: int32): int32 {
b0(v0: int32):
    invoke callee(v0): (int32) -> int32 -> b1, catch b2
b1(v1: int32):
    return v1
b2(v2: ref<int32, managed, readonly>):
    throw v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&DeadArgEliminate);
        test.assert_output(expected);
    }

    /// Exported functions are not rewritten.
    #[test]
    fn test_dead_arg_eliminate_skips_exports() {
        let input = r#"
export function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    return v0
}
function root(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = call callee(v0, v1): (int32, int32) -> int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&DeadArgEliminate);
        test.assert_output(input);
    }

    /// Indirect signatures block argument removal.
    #[test]
    fn test_dead_arg_eliminate_skips_indirect_signature() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    return v0
}
function root(v0: fn(int32, int32) -> int32, v1: int32, v2: int32): int32  {
b0(v0: fn(int32, int32) -> int32, v1: int32, v2: int32) -> v3: int32 = call.indirect v0(v1, v2): (int32, int32) -> int32
    v4: int32 = call callee(v1, v2): (int32, int32) -> int32
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&DeadArgEliminate);
        test.assert_output(input);
    }

    /// Call metadata argument lists are trimmed alongside arguments.
    #[test]
    fn test_dead_arg_eliminate_updates_call_metadata() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    return v0
}
function root(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = call callee(v0, v1): (int32, int32) -> int32
    return v2
}"#;

        let expected = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function root(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = call callee(v0): (int32) -> int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        let root_id = test.function_id_by_name("root");
        let call_id = test
            .entry_instructions(root_id)
            .into_iter()
            .find(|instruction_id| {
                matches!(
                    test.tree.get(*instruction_id),
                    mir::Instruction::Call { .. }
                )
            })
            .expect("missing call instruction");

        let instruction = test.tree.get_mut(call_id);
        let mir::Instruction::Call { call, .. } = instruction else {
            panic!("expected call instruction");
        };
        call.argument_attributes = vec![
            mir::ArgumentAttribute::default(),
            mir::ArgumentAttribute::default(),
        ];

        test.run_module_pass(&DeadArgEliminate);
        test.assert_output(expected);
        let instruction = test.tree.get(call_id);
        assert_eq!(
            instruction
                .call_argument_attributes()
                .expect("missing call argument attributes")
                .len(),
            1
        );
    }

    /// Metadata parameter indices are remapped after removal.
    #[test]
    fn test_dead_arg_eliminate_remaps_metadata_indices() {
        let input = r#"
function callee(v0: int32, v1: int32, v2: int32): int32 {
b0(v0: int32, v1: int32, v2: int32):
    return v0
}
function root(v0: int32, v1: int32, v2: int32): int32 {
b0(v0: int32, v1: int32, v2: int32):
    v3: int32 = call callee(v0, v1, v2): (int32, int32, int32) -> int32
    return v3
}"#;

        let expected = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    return v0
}
function root(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = call callee(v0, v1): (int32, int32) -> int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        let callee_id = test.function_id_by_name("callee");
        let callee = test.tree.get_mut(callee_id);
        callee.return_lifetime = mir::Lifetime::parameter_set([2]);
        callee.allocation_size = Some(mir::AllocationSize::new(2, Some(0)));

        test.run_module_pass(&DeadArgEliminate);
        test.assert_output(expected);

        let callee = test.tree.get(callee_id);
        assert_eq!(callee.return_lifetime, mir::Lifetime::parameter_set([1]));
        assert_eq!(
            callee.allocation_size,
            Some(mir::AllocationSize::new(1, Some(0)))
        );
    }

    /// Allocation metadata prevents removing its parameters.
    #[test]
    fn test_dead_arg_eliminate_preserves_alloc_size_param() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    return v0
}"#;

        let mut test = TestProgram::new(input);
        let callee_id = test.function_id_by_name("callee");
        let callee = test.tree.get_mut(callee_id);
        callee.allocation_size = Some(mir::AllocationSize::new(1, None));

        test.run_module_pass(&DeadArgEliminate);
        test.assert_output(input);
        assert_eq!(
            test.tree.get(callee_id).allocation_size,
            Some(mir::AllocationSize::new(1, None))
        );
    }

    /// Allocation metadata is remapped at callsites.
    #[test]
    fn test_dead_arg_eliminate_remaps_call_allocation_size() {
        let input = r#"
function callee(v0: int32, v1: int32, v2: int32): int32 {
b0(v0: int32, v1: int32, v2: int32):
    return v0
}
function root(v0: int32, v1: int32, v2: int32): int32 {
b0(v0: int32, v1: int32, v2: int32):
    v3: int32 = call callee(v0, v1, v2): (int32, int32, int32) -> int32
    return v3
}"#;

        let expected = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function root(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = call callee(v0): (int32) -> int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        let root_id = test.function_id_by_name("root");
        let call_id = test
            .entry_instructions(root_id)
            .into_iter()
            .find(|instruction_id| {
                matches!(
                    test.tree.get(*instruction_id),
                    mir::Instruction::Call { .. }
                )
            })
            .expect("missing call instruction");

        let instruction = test.tree.get_mut(call_id);
        let mir::Instruction::Call { call, .. } = instruction else {
            panic!("expected call instruction");
        };
        call.allocation_size = Some(mir::AllocationSize::new(2, Some(0)));

        test.run_module_pass(&DeadArgEliminate);
        test.assert_output(expected);
        let instruction = test.tree.get(call_id);
        assert_eq!(instruction.call_allocation_size(), None);
    }

    /// Return-region metadata preserves parameters.
    #[test]
    fn test_dead_arg_eliminate_preserves_return_lifetime_param() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    return v0
}"#;

        let mut test = TestProgram::new(input);
        let callee_id = test.function_id_by_name("callee");
        let callee = test.tree.get_mut(callee_id);
        callee.return_lifetime = mir::Lifetime::parameter_set([1]);

        test.run_module_pass(&DeadArgEliminate);
        test.assert_output(input);
        assert_eq!(
            test.tree.get(callee_id).return_lifetime,
            mir::Lifetime::parameter_set([1])
        );
    }

    /// Debug parameter locations are cleared for removed parameters.
    #[test]
    fn test_dead_arg_eliminate_clears_debug_parameter_location() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    return v0
}
function root(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = call callee(v0, v1): (int32, int32) -> int32
    return v2
}"#;

        let expected = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function root(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = call callee(v0): (int32) -> int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        let callee_id = test.function_id_by_name("callee");
        let callee = test.tree.get(callee_id);
        let param_value = callee.parameters[1]
            .value
            .value()
            .expect("parameter value should be concrete");
        let param_type = callee.parameters[1]
            .ty
            .ty()
            .expect("parameter type should be concrete");
        let callee_name = callee.name;
        let scope_id = test.tree.metadata.debug.create_scope(None, None, None);
        test.tree
            .metadata
            .debug
            .function_scopes
            .insert(callee_id, scope_id);
        let binding_id = test.tree.metadata.debug.create_binding(
            callee_name,
            param_type,
            scope_id,
            None,
            mir::DebugBindingKind::Parameter,
        );
        test.tree.metadata.debug.binding_ranges.insert(
            binding_id,
            vec![mir::DebugBindingRange {
                location: mir::DebugValueLocation::Value(param_value),
                start: None,
                end: None,
            }],
        );

        test.run_module_pass(&DeadArgEliminate);
        test.assert_output(expected);
        let location = test
            .tree
            .metadata
            .debug
            .binding_ranges
            .get(&binding_id)
            .expect("missing debug binding location");

        assert_eq!(
            location,
            &vec![mir::DebugBindingRange {
                location: mir::DebugValueLocation::Undefined,
                start: None,
                end: None,
            }]
        );
    }
}
