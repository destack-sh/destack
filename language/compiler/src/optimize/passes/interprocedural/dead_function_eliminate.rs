use std::collections::{HashMap, HashSet, VecDeque};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::CallGraph;
use crate::optimize::common::SignatureKey;
use crate::optimize::{AnalysisPreservation, ModuleAnalyses, ModulePass, PipelineContext};

declare_pass! {
    /// Remove functions that are not reachable from exported roots.
    ///
    /// This pass keeps exported functions and anything reachable via direct calls, plus any signatures reachable from unresolved indirect calls.
    ///
    /// ```mir
    /// export function @root() -> void {
    /// block0:
    ///     call @live() -> fn() -> void
    ///     return
    /// }
    /// function @live() -> void {
    /// block0:
    ///     return
    /// }
    /// function @dead() -> void {
    /// block0:
    ///     return
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// export function @root() -> void {
    /// block0:
    ///     call @live() -> fn() -> void
    ///     return
    /// }
    /// function @live() -> void {
    /// block0:
    ///     return
    /// }
    /// extern function @dead() -> void
    /// ```
    #[pass(id = "dead-function-eliminate")]
    pub DeadFunctionEliminate,
    "Eliminate dead functions"
}

impl ModulePass for DeadFunctionEliminate {
    /// Run dead function elimination for the module.
    fn run(&self, tree: &mut mir::NodeTree, ctx: &PipelineContext<'_>) -> AnalysisPreservation {
        let changed = run_dead_function_eliminate(tree);

        // report analysis preservation based on whether changes occurred
        if changed {
            ctx.strings.intern("dead-function-eliminate");
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass display name.
    fn name(&self) -> &'static str {
        "DeadFunctionEliminate"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "dead-function-eliminate"
    }
}

/// Constraint describing a potential indirect call target set.
#[derive(Debug, Clone)]
enum CallConstraint {
    /// Indirect call with no usable signature information.
    Unknown,
    /// Indirect call with a known signature.
    Signature(SignatureKey),
}

/// Run dead function elimination over the module.
pub(crate) fn run_dead_function_eliminate(tree: &mut mir::NodeTree) -> bool {
    // build the module call graph
    let analyses = ModuleAnalyses::new(tree);
    let callgraph = analyses.get::<CallGraph>();

    // collect functions that have bodies
    let defined_functions: Vec<_> = tree
        .iter_nodes::<mir::Function>()
        .filter_map(|(id, function)| function.entry.is_some().then_some(id))
        .collect();

    // return early when there are no definitions
    if defined_functions.is_empty() {
        return false;
    }

    // build a signature index for indirect resolution
    let signature_index = build_signature_index(tree, &defined_functions);

    // seed roots with exported functions
    let mut roots: Vec<_> = defined_functions
        .iter()
        .copied()
        .filter(|id| tree.get(*id).linkage.is_exported())
        .collect();

    // keep all functions when no explicit roots exist
    if roots.is_empty() {
        roots = defined_functions.clone();
    }

    // walk reachable functions using a worklist
    let mut reachable = HashSet::new();
    let mut worklist: VecDeque<_> = roots.into();
    let mut keep_all = false;
    while let Some(function_id) = worklist.pop_front() {
        // skip functions that are already marked reachable
        if !reachable.insert(function_id) {
            continue;
        }

        // enqueue direct callees
        for edge in callgraph.outgoing(function_id) {
            if edge.dispatch == mir::CallDispatchKind::Direct {
                worklist.push_back(edge.callee);
            }
        }

        // handle unresolved callsites conservatively
        let constraints = unknown_call_constraints(tree, function_id);
        for constraint in constraints {
            match constraint {
                CallConstraint::Unknown => {
                    keep_all = true;
                    break;
                }
                CallConstraint::Signature(signature) => {
                    if let Some(candidates) = signature_index.get(&signature) {
                        for &candidate in candidates {
                            worklist.push_back(candidate);
                        }
                    }
                }
            }
        }

        // stop once we determine all functions are reachable
        if keep_all {
            break;
        }
    }

    // avoid stripping when unresolved calls exist
    if keep_all {
        return false;
    }

    // strip bodies from unreachable local functions
    let mut changed = false;
    for function_id in defined_functions {
        if reachable.contains(&function_id) {
            continue;
        }
        let function = tree.get_mut(function_id);
        if function.entry.is_none() {
            continue;
        }
        if function.linkage.is_exported() {
            continue;
        }
        strip_function_body(function_id, tree);
        changed = true;
    }

    changed
}

/// Build a map from signature keys to candidate function ids.
fn build_signature_index(
    tree: &mir::NodeTree,
    functions: &[mir::LocalNodeId<mir::Function>],
) -> HashMap<SignatureKey, Vec<mir::LocalNodeId<mir::Function>>> {
    // insert each function under its signature key
    let mut index: HashMap<SignatureKey, Vec<mir::LocalNodeId<mir::Function>>> = HashMap::new();

    // populate the signature index
    for function_id in functions {
        let function = tree.get(*function_id);
        let signature = SignatureKey::from_function(tree, function);
        index.entry(signature).or_default().push(*function_id);
    }

    index
}

/// Collect unresolved call constraints for a function.
fn unknown_call_constraints(
    tree: &mir::NodeTree,
    function_id: mir::LocalNodeId<mir::Function>,
) -> Vec<CallConstraint> {
    // scan the function blocks for indirect calls
    let function = tree.get(function_id);
    let mut constraints = Vec::new();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if let Some(constraint) = call_constraint_from_instruction(tree, instruction) {
                constraints.push(constraint);
            }
        }

        if let Some(constraint) = call_constraint_from_terminator(tree, &block.terminator) {
            constraints.push(constraint);
        }
    }

    constraints
}

/// Resolve a call constraint from a call instruction.
fn call_constraint_from_instruction(
    tree: &mir::NodeTree,
    instruction: &mir::Instruction,
) -> Option<CallConstraint> {
    let dispatch = instruction.call_dispatch_kind()?;
    if dispatch == mir::CallDispatchKind::Direct {
        return None;
    }

    let signature = instruction.call_signature()?;
    let declared_target = instruction.call_declared_target();
    call_constraint_from_signature(tree, signature, declared_target)
}

/// Resolve a call constraint from a tail call terminator.
fn call_constraint_from_terminator(
    tree: &mir::NodeTree,
    terminator: &mir::Terminator,
) -> Option<CallConstraint> {
    // classify tail call terminators
    match terminator {
        mir::Terminator::TailCall { .. } => None,
        mir::Terminator::TailCallIndirect { signature, .. } => {
            call_constraint_from_signature(tree, *signature, None)
        }
        mir::Terminator::TailCallVirtual {
            signature,
            declared_target,
            ..
        } => call_constraint_from_signature(tree, *signature, *declared_target),
        mir::Terminator::TailCallInterface {
            signature,
            declared_target,
            ..
        } => call_constraint_from_signature(tree, *signature, *declared_target),
        _ => None,
    }
}

/// Resolve call constraints from a signature type.
fn call_constraint_from_signature(
    tree: &mir::NodeTree,
    signature: mir::LocalNodeId<mir::Type>,
    declared_target: Option<mir::LocalNodeId<mir::Function>>,
) -> Option<CallConstraint> {
    if let Some(signature) = SignatureKey::from_signature_type(tree, signature) {
        return Some(CallConstraint::Signature(signature));
    }

    if let Some(target) = declared_target {
        let signature = SignatureKey::from_function(tree, tree.get(target));
        return Some(CallConstraint::Signature(signature));
    }

    Some(CallConstraint::Unknown)
}

/// Strip the body of a function, leaving an import declaration.
fn strip_function_body(function_id: mir::LocalNodeId<mir::Function>, tree: &mut mir::NodeTree) {
    // collect blocks and instructions before stripping the body
    let block_ids = tree.get(function_id).blocks.clone();
    let mut instruction_ids = Vec::new();

    for block_id in &block_ids {
        let block = tree.get(*block_id);
        instruction_ids.extend(block.instructions.iter().copied());
    }

    // read the function scope before clearing debug metadata
    let function_scope = tree.debug_info.function_scopes.get(&function_id).copied();

    // convert the definition into an import declaration
    let function = tree.get_mut(function_id);
    function.linkage = mir::Linkage::Import;
    function.locals.clear();
    function.blocks.clear();
    function.entry = None;

    // remove per block debug scopes
    for block_id in block_ids {
        tree.debug_info.block_scopes.remove(&block_id);
    }

    // remove instruction metadata tied to stripped blocks
    for instruction_id in instruction_ids {
        tree.memory_table
            .memory_accesses_by_instruction_id
            .remove(&instruction_id);
        tree.debug_info
            .instruction_locations
            .remove(&instruction_id);
    }

    // clear debug variable locations tied to the stripped function
    if let Some(function_scope) = function_scope {
        // rewrite debug locations for variables in the function scope
        for (index, variable) in tree.debug_info.variables.iter().enumerate() {
            // skip variables outside the function scope
            if !scope_in_function(variable.scope, function_scope, &tree.debug_info) {
                continue;
            }

            // update variable locations to undefined
            let var_id = mir::DebugVariableId::new(index as u32);
            if tree.debug_info.variable_locations.contains_key(&var_id) {
                tree.debug_info
                    .variable_locations
                    .insert(var_id, mir::DebugValueLocation::Undefined);
            }
        }
    }

    // remove the function scope entry
    tree.debug_info.function_scopes.remove(&function_id);
}

/// Return true when a debug scope belongs to a function scope.
fn scope_in_function(
    scope: mir::DebugScopeId,
    function_scope: mir::DebugScopeId,
    debug_info: &mir::DebugInfoTable,
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
    use destack_source::{FileId, Span};

    /// Unreferenced local functions are removed.
    #[test]
    fn test_dead_function_eliminate_local() {
        let input = r#"export function @root() -> void {
block0:
    call @live() -> fn() -> void
    return
}
function @live() -> void {
block0:
    return
}
function @dead() -> void {
block0:
    return
}"#;

        let expected = r#"export function @root() -> void {
block0:
    call @live() -> fn() -> void
    return
}
function @live() -> void {
block0:
    return
}
extern function @dead() -> void"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&DeadFunctionEliminate);
        test.assert_output(expected);
    }

    /// Unknown indirect calls keep all functions.
    #[test]
    fn test_dead_function_eliminate_unknown_indirect() {
        let input = r#"export function @root(v0: fn() -> void) -> void {
block0(v0: fn() -> void):
    call.indirect v0() -> fn() -> void
    return
}
function @live() -> void {
block0:
    return
}
function @dead() -> void {
block0:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&DeadFunctionEliminate);
        test.assert_output(input);
    }

    /// Modules without exports keep all definitions.
    #[test]
    fn test_dead_function_eliminate_no_exports() {
        let input = r#"function @alpha() -> void {
block0:
    return
}
function @beta() -> void {
block0:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&DeadFunctionEliminate);
        test.assert_output(input);
    }

    /// Signature constrained indirect calls keep matching functions only.
    #[test]
    fn test_dead_function_eliminate_signature_indirect() {
        let input = r#"export function @root(v0: fn(i32) -> i32, v1: i32) -> void {
block0(v0: fn(i32) -> i32, v1: i32):
    v2 = call.indirect v0(v1) -> fn(i32) -> i32
    return
}
function @keep(v0: i32) -> i32 {
block0(v0: i32):
    return v0
}
function @drop(v0: i64) -> i64 {
block0(v0: i64):
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&DeadFunctionEliminate);
        let expected = r#"export function @root(v0: fn(i32) -> i32, v1: i32) -> void {
block0(v0: fn(i32) -> i32, v1: i32):
    v2 = call.indirect v0(v1) -> fn(i32) -> i32
    return
}
function @keep(v0: i32) -> i32 {
block0(v0: i32):
    return v0
}
extern function @drop(i64) -> i64"#;

        test.assert_output(expected);
    }

    /// Tailcall indirect sites keep all functions.
    #[test]
    fn test_dead_function_eliminate_tailcall_indirect() {
        let input = r#"export function @root(v0: fn() -> void) -> void {
block0(v0: fn() -> void):
    tailcall.indirect v0() -> fn() -> void
}
function @live() -> void {
block0:
    return
}
function @dead() -> void {
block0:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&DeadFunctionEliminate);
        test.assert_output(input);
    }

    /// Debug metadata for stripped functions is cleared.
    #[test]
    fn test_dead_function_eliminate_clears_debug_metadata() {
        let input = r#"export function @root() -> void {
block0:
    call @live() -> fn() -> void
    return
}
function @live() -> void {
block0:
    return
}
function @dead(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    return v0
}"#;

        let expected = r#"export function @root() -> void {
block0:
    call @live() -> fn() -> void
    return
}
function @live() -> void {
block0:
    return
}
extern function @dead(i32) -> i32"#;

        let mut test = TestProgram::new(input);
        let dead_id = test.function_id_by_name("dead");
        let dead_block = test.entry_block_id(dead_id);
        let dead_instruction = test
            .instructions_in_block(dead_block)
            .first()
            .copied()
            .expect("missing instruction");
        let file_id = FileId::new(0);
        let span = Span::empty(file_id);
        let function_scope =
            test.tree
                .debug_info
                .create_scope(mir::DebugScopeKind::Function, None, span, None);
        test.tree
            .debug_info
            .function_scopes
            .insert(dead_id, function_scope);
        let variable_id = test.tree.debug_info.create_variable(
            test.tree.get(dead_id).name,
            test.tree.get(dead_id).parameters[0].ty,
            function_scope,
            true,
            false,
        );
        test.tree.debug_info.variable_locations.insert(
            variable_id,
            mir::DebugValueLocation::Value(test.tree.get(dead_id).parameters[0].value),
        );
        test.tree.debug_info.instruction_locations.insert(
            dead_instruction,
            mir::DebugLocation {
                span,
                scope: function_scope,
                inlined_at: None,
            },
        );

        test.run_module_pass(&DeadFunctionEliminate);
        test.assert_output(expected);

        assert!(!test.tree.debug_info.function_scopes.contains_key(&dead_id));
        assert_eq!(
            test.tree.debug_info.variable_locations.get(&variable_id),
            Some(&mir::DebugValueLocation::Undefined)
        );
        assert!(
            !test
                .tree
                .debug_info
                .instruction_locations
                .contains_key(&dead_instruction)
        );
    }
}
