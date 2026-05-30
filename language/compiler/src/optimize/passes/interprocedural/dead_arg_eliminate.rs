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
        let unused = unused_parameter_indices(function_id, function, tree);
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
                mir::Terminator::Call { function, .. } => {
                    let Some(function) = function.function() else {
                        continue;
                    };

                    data.direct_calls
                        .entry(function)
                        .or_default()
                        .push(DirectCallSite::Terminator(block_id));
                }
                mir::Terminator::CallIndirect { call, .. }
                | mir::Terminator::CallVirtual { call, .. }
                | mir::Terminator::CallDynamic { call, .. } => {
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
                | mir::Terminator::TailCallDynamic { call, .. } => {
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
fn unused_parameter_indices(
    function_id: mir::LocalNodeId<mir::Function>,
    function: &mir::Function,
    tree: &mir::Tree,
) -> Vec<usize> {
    // collect uses without treating parameters as implicitly used
    let use_def = build_use_def_maps(function, tree);

    // collect parameters that are required by metadata
    let metadata = tree.metadata.functions.function(function_id);
    let required = required_parameter_indices(function, metadata);

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
    // prepare removal remapping data
    let remap = ParameterRemap::new(unused);

    // update function parameters and attributes
    let entry_id = {
        let function = tree.get_mut(function_id);
        function.parameters = remap.filter_by_index(&function.parameters);
        function.return_lifetime = remap
            .remap_return_lifetime(&function.return_lifetime)
            .expect("return lifetime parameter must be preserved");
        let Some(entry_id) = function.entry else {
            return;
        };

        entry_id
    };

    // update function metadata
    if let Some(metadata) = tree.metadata.functions.functions.get_mut(&function_id) {
        metadata.allocation_size = remap.remap_allocation_size(metadata.allocation_size);
    }

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
                let callsite = mir::CallSite::Instruction(instruction_id);
                let mut metadata = tree
                    .metadata
                    .functions
                    .call(callsite)
                    .cloned()
                    .unwrap_or_default();
                metadata.arguments = remap.filter_by_index(&metadata.arguments);
                metadata.allocation_size = remap.remap_allocation_size(metadata.allocation_size);

                let mut call = call;
                call.arguments = new_slice;
                call.signature = signature;

                let updated = mir::Instruction::Call {
                    destination,
                    function,
                    call,
                };
                *tree.get_mut(instruction_id) = updated;
                *tree.metadata.functions.call_mut(callsite) = metadata;
            }
            DirectCallSite::Terminator(block_id) => {
                let block = tree.get_mut(block_id);
                let terminator_id = block.terminator;
                let terminator = tree.get(terminator_id).clone();
                match &terminator {
                    mir::Terminator::Call {
                        function,
                        call,
                        target,
                        unwind,
                    } => {
                        // filter the argument list
                        let mut new_call = call.clone();
                        new_call.arguments = remap.filter_by_index(&call.arguments);

                        let new_terminator = mir::Terminator::Call {
                            function: *function,
                            call: new_call,
                            target: target.clone(),
                            unwind: unwind.clone(),
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

    /// Direct call terminators are trimmed for unused parameters.
    #[test]
    fn test_dead_arg_eliminate_updates_call_terminator() {
        let input = r#"
function callee(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    return v0
}
function root(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    call callee(v0, v1): (int32, int32) -> int32 -> b1
b1(v2: int32):
    return v2
b2(v3: ref<int32, managed, readonly>):
    panic v3
}"#;

        let expected = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function root(v0: int32): int32 {
b0(v0: int32):
    call callee(v0): (int32) -> int32 -> b1
b1(v1: int32):
    return v1
b2(v2: ref<int32, managed, readonly>):
    panic v2
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
function root(v0: (int32, int32) -> int32, v1: int32, v2: int32): int32  {
b0(v0: (int32, int32) -> int32, v1: int32, v2: int32):
    v3: int32 = call.indirect v0(v1, v2): (int32, int32) -> int32
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

        let callsite = mir::CallSite::Instruction(call_id);
        test.tree.metadata.functions.call_mut(callsite).arguments = vec![
            mir::CallArgumentEffect::default(),
            mir::CallArgumentEffect::default(),
        ];

        test.run_module_pass(&DeadArgEliminate);
        test.assert_output(expected);
        let callsite = mir::CallSite::Instruction(call_id);
        let metadata = test
            .tree
            .metadata
            .functions
            .call(callsite)
            .expect("missing call metadata");
        assert_eq!(metadata.arguments.len(), 1);
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
        test.tree
            .metadata
            .functions
            .function_mut(callee_id)
            .allocation_size = Some(mir::AllocationSize::new(2, Some(0)));

        test.run_module_pass(&DeadArgEliminate);
        test.assert_output(expected);

        let callee = test.tree.get(callee_id);
        let metadata = test
            .tree
            .metadata
            .functions
            .function(callee_id)
            .expect("missing function metadata");
        assert_eq!(callee.return_lifetime, mir::Lifetime::parameter_set([1]));
        assert_eq!(
            metadata.allocation_size,
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
        test.tree
            .metadata
            .functions
            .function_mut(callee_id)
            .allocation_size = Some(mir::AllocationSize::new(1, None));

        test.run_module_pass(&DeadArgEliminate);
        test.assert_output(input);
        let metadata = test
            .tree
            .metadata
            .functions
            .function(callee_id)
            .expect("missing function metadata");
        assert_eq!(
            metadata.allocation_size,
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

        let callsite = mir::CallSite::Instruction(call_id);
        test.tree
            .metadata
            .functions
            .call_mut(callsite)
            .allocation_size = Some(mir::AllocationSize::new(2, Some(0)));

        test.run_module_pass(&DeadArgEliminate);
        test.assert_output(expected);
        let metadata = test
            .tree
            .metadata
            .functions
            .call(callsite)
            .expect("missing call metadata");
        assert_eq!(metadata.allocation_size, None);
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
}
