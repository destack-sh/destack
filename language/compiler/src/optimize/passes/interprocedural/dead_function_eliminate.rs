use std::collections::{HashMap, HashSet, VecDeque};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::CallGraph;
use crate::optimize::common::TypeKey;
use crate::optimize::{AnalysisPreservation, ModuleAnalyses, ModulePass, PipelineContext};

declare_pass! {
    /// Remove functions that are not reachable from exported roots.
    ///
    /// This pass keeps exported functions and anything reachable via direct calls, plus any signatures reachable from unresolved indirect calls.
    ///
    /// ```mir
    /// export function @root() -> void {
    /// block0:
    ///     call @live()
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
    ///     call @live()
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

/// Function signature key for indirect call matching.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SignatureKey {
    /// Parameter type keys for the signature.
    parameters: Vec<TypeKey>,
    /// Return type key for the signature.
    result: TypeKey,
}

impl SignatureKey {
    /// Build a signature key from a function definition.
    fn from_function(tree: &mir::NodeTree, function: &mir::Function) -> Self {
        // collect parameter type keys
        let parameters = function
            .parameters
            .iter()
            .map(|param| TypeKey::from_type(param.ty, tree))
            .collect();
        let result = TypeKey::from_type(function.return_type, tree);
        Self { parameters, result }
    }

    /// Build a signature key from a function pointer type.
    fn from_signature_type(
        tree: &mir::NodeTree,
        signature: mir::LocalNodeId<mir::Type>,
    ) -> Option<Self> {
        // resolve the function pointer type signature
        let mir::Type::FunctionPointer { parameters, result } = tree.get(signature) else {
            return None;
        };
        let parameters = parameters
            .iter()
            .map(|param| TypeKey::from_type(*param, tree))
            .collect();
        let result = TypeKey::from_type(*result, tree);
        Some(Self { parameters, result })
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
fn run_dead_function_eliminate(tree: &mut mir::NodeTree) -> bool {
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
        strip_function_body(function);
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
            if let Some(constraint) =
                call_constraint_from_instruction(tree, instruction_id, instruction)
            {
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
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    instruction: &mir::Instruction,
) -> Option<CallConstraint> {
    // read callsite metadata for dispatch decisions
    let metadata = tree.call_table.call_metadata(instruction_id);

    // classify call instruction variants
    match instruction {
        mir::Instruction::Call { function, .. } => {
            let dispatch = metadata
                .map(|meta| meta.dispatch)
                .unwrap_or(mir::CallDispatchKind::Direct);
            if dispatch == mir::CallDispatchKind::Direct {
                return None;
            }
            call_constraint_from_metadata(tree, metadata, Some(*function))
        }
        mir::Instruction::CallIndirect { .. } => {
            let dispatch = metadata
                .map(|meta| meta.dispatch)
                .unwrap_or(mir::CallDispatchKind::Indirect);
            if dispatch == mir::CallDispatchKind::Direct {
                return None;
            }
            call_constraint_from_metadata(tree, metadata, None)
        }
        _ => None,
    }
}

/// Resolve a call constraint from a tail call terminator.
fn call_constraint_from_terminator(
    _tree: &mir::NodeTree,
    terminator: &mir::Terminator,
) -> Option<CallConstraint> {
    // classify tail call terminators
    match terminator {
        mir::Terminator::TailCall { .. } => None,
        mir::Terminator::TailCallIndirect { .. } => Some(CallConstraint::Unknown),
        _ => None,
    }
}

/// Resolve call constraints from callsite metadata when possible.
fn call_constraint_from_metadata(
    tree: &mir::NodeTree,
    metadata: Option<&mir::CallMetadata>,
    fallback_target: Option<mir::LocalNodeId<mir::Function>>,
) -> Option<CallConstraint> {
    // prefer the metadata signature when present
    if let Some(metadata) = metadata {
        if let Some(signature) = SignatureKey::from_signature_type(tree, metadata.signature) {
            return Some(CallConstraint::Signature(signature));
        }
        if let Some(target) = metadata.declared_target {
            let signature = SignatureKey::from_function(tree, tree.get(target));
            return Some(CallConstraint::Signature(signature));
        }
    }

    // fall back to the direct target signature when available
    if let Some(target) = fallback_target {
        let signature = SignatureKey::from_function(tree, tree.get(target));
        return Some(CallConstraint::Signature(signature));
    }

    Some(CallConstraint::Unknown)
}

/// Strip the body of a function, leaving an import declaration.
fn strip_function_body(function: &mut mir::Function) {
    // convert the definition into an import declaration
    function.linkage = mir::Linkage::Import;
    function.locals.clear();
    function.blocks.clear();
    function.entry = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Unreferenced local functions are removed.
    #[test]
    fn test_dead_function_eliminate_local() {
        let input = r#"export function @root() -> void {
block0:
    call @live()
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
    call @live()
    return
}
function @live() -> void {
block0:
    return
}
extern function @dead() -> void"#;

        let mut program = TestProgram::new(input);
        program.run_module_pass(&DeadFunctionEliminate);
        program.assert_output(expected);
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

        let mut program = TestProgram::new(input);
        program.run_module_pass(&DeadFunctionEliminate);
        program.assert_output(input);
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

        let mut program = TestProgram::new(input);
        program.run_module_pass(&DeadFunctionEliminate);
        program.assert_output(input);
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

        let mut program = TestProgram::new(input);
        let root_id = program.function_id_by_name("root");
        let keep_id = program.function_id_by_name("keep");
        let signature = program.call_signature_for_callee(keep_id);

        let root_block = program.entry_block_id(root_id);
        let call_instruction_id = program
            .instructions_in_block(root_block)
            .into_iter()
            .find(|instruction_id| {
                matches!(
                    program.tree.get(*instruction_id),
                    mir::Instruction::CallIndirect { .. }
                )
            })
            .expect("missing call.indirect");
        program
            .tree
            .call_table
            .insert_call_metadata(call_instruction_id, mir::CallMetadata::indirect(signature));

        program.run_module_pass(&DeadFunctionEliminate);
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

        program.assert_output(expected);
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

        let mut program = TestProgram::new(input);
        program.run_module_pass(&DeadFunctionEliminate);
        program.assert_output(input);
    }
}
