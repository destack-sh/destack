use std::collections::{HashMap, HashSet};

use crate as mir;

use super::{Analysis, AnalysisId, ModuleAnalyses, ModuleAnalysis};

/// Directed edge in the call graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallEdge {
    /// The caller function id.
    pub caller: mir::LocalNodeId<mir::Function>,
    /// The callee function id.
    pub callee: mir::LocalNodeId<mir::Function>,
    /// The callsite that performs the call.
    pub callsite: CallSiteRef,
    /// The dispatch kind for this callsite.
    pub dispatch: mir::CallDispatchKind,
}

impl CallEdge {
    /// Return true when this edge is a direct call.
    pub fn is_direct(&self) -> bool {
        matches!(self.dispatch, mir::CallDispatchKind::Direct)
    }
}

/// Callsite that does not have a resolved target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnknownCallSite {
    /// The caller function id.
    pub caller: mir::LocalNodeId<mir::Function>,
    /// The callsite that performs the call.
    pub callsite: CallSiteRef,
    /// The dispatch kind for this callsite.
    pub dispatch: mir::CallDispatchKind,
    /// The declared callee when known.
    pub callee: Option<mir::LocalNodeId<mir::Function>>,
}

/// Callsite reference for module call graphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallSiteRef {
    /// Callsite is an instruction.
    Instruction(mir::LocalNodeId<mir::Instruction>),
    /// Callsite is a terminator in the block.
    Terminator(mir::LocalNodeId<mir::Block>),
}

/// Module scoped call graph.
#[derive(Debug)]
pub struct CallGraph {
    /// Outgoing edges by caller.
    outgoing: HashMap<mir::LocalNodeId<mir::Function>, Vec<CallEdge>>,
    /// Incoming edges by callee.
    incoming: HashMap<mir::LocalNodeId<mir::Function>, Vec<CallEdge>>,
    /// Unresolved callsites by caller.
    unknown: HashMap<mir::LocalNodeId<mir::Function>, Vec<UnknownCallSite>>,
}

impl CallGraph {
    /// Get outgoing call edges for a function.
    pub fn outgoing(&self, caller: mir::LocalNodeId<mir::Function>) -> &[CallEdge] {
        const EMPTY: [CallEdge; 0] = [];
        self.outgoing
            .get(&caller)
            .map(Vec::as_slice)
            .unwrap_or(&EMPTY)
    }

    /// Get incoming call edges for a function.
    pub fn incoming(&self, callee: mir::LocalNodeId<mir::Function>) -> &[CallEdge] {
        const EMPTY: [CallEdge; 0] = [];
        self.incoming
            .get(&callee)
            .map(Vec::as_slice)
            .unwrap_or(&EMPTY)
    }

    /// Get unresolved callsites for a function.
    pub fn unknown_calls(&self, caller: mir::LocalNodeId<mir::Function>) -> &[UnknownCallSite] {
        const EMPTY: [UnknownCallSite; 0] = [];
        self.unknown
            .get(&caller)
            .map(Vec::as_slice)
            .unwrap_or(&EMPTY)
    }

    /// Build a call graph for the given module.
    fn build(tree: &mir::Tree) -> Self {
        let mut graph = Self {
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
            unknown: HashMap::new(),
        };

        // scan each function for call instructions
        for (function_id, function) in tree.iter_nodes::<mir::Function>() {
            // skip functions without bodies
            if function.entry.is_none() {
                continue;
            }

            for block_id in &function.blocks {
                let block = tree.get(*block_id);
                let terminator = tree.get(block.terminator);

                for &instruction_id in &block.instructions {
                    let instruction = tree.get(instruction_id);

                    let Some(callsite) =
                        CallSite::from_instruction(function_id, instruction_id, instruction, tree)
                    else {
                        continue;
                    };

                    graph.insert_callsite(callsite);
                }

                if let Some(callsite) =
                    CallSite::from_terminator(function_id, *block_id, terminator, tree)
                {
                    graph.insert_callsite(callsite);
                }
            }
        }

        graph
    }

    /// Insert a callsite into the graph.
    fn insert_callsite(&mut self, callsite: CallSite) {
        // record resolved edges when a target is known
        if let Some(callee) = callsite.callee {
            let edge = CallEdge {
                caller: callsite.caller,
                callee,
                callsite: callsite.callsite,
                dispatch: callsite.dispatch,
            };

            self.outgoing.entry(callsite.caller).or_default().push(edge);
            self.incoming.entry(callee).or_default().push(edge);

            if callsite.is_precise {
                return;
            }
        }

        // record unresolved or partially resolved callsites
        let unknown = UnknownCallSite {
            caller: callsite.caller,
            callsite: callsite.callsite,
            dispatch: callsite.dispatch,
            callee: callsite.callee,
        };

        self.unknown
            .entry(callsite.caller)
            .or_default()
            .push(unknown);
    }
}

impl Analysis for CallGraph {
    const ID: AnalysisId = AnalysisId("callgraph");
}

impl ModuleAnalysis for CallGraph {
    /// Compute the module call graph.
    fn compute(tree: &mir::Tree, _analyses: &ModuleAnalyses) -> Self {
        Self::build(tree)
    }
}

/// Strongly connected components for a module call graph.
#[derive(Debug, Default)]
pub struct CallGraphScc {
    /// Maps each function to its SCC id.
    function_scc: HashMap<mir::LocalNodeId<mir::Function>, usize>,
    /// SCCs that are recursive.
    recursive_sccs: HashSet<usize>,
}

impl CallGraphScc {
    /// Return the SCC id for a function.
    pub fn scc_id(&self, function_id: mir::LocalNodeId<mir::Function>) -> Option<usize> {
        self.function_scc.get(&function_id).copied()
    }

    /// Return true when the SCC is recursive.
    pub fn is_recursive_scc(&self, scc_id: usize) -> bool {
        self.recursive_sccs.contains(&scc_id)
    }

    /// Return true when the function is part of a recursive SCC.
    pub fn is_recursive_function(&self, function_id: mir::LocalNodeId<mir::Function>) -> bool {
        let Some(scc_id) = self.scc_id(function_id) else {
            return false;
        };

        self.is_recursive_scc(scc_id)
    }
}

impl Analysis for CallGraphScc {
    const ID: AnalysisId = AnalysisId("callgraph-scc");
}

impl ModuleAnalysis for CallGraphScc {
    /// Compute the SCCs for the module call graph.
    fn compute(tree: &mir::Tree, analyses: &ModuleAnalyses) -> Self {
        let callgraph = analyses.get::<CallGraph>(tree);
        compute_callgraph_scc(tree, &callgraph)
    }
}

/// Compute SCCs for the module call graph.
fn compute_callgraph_scc(tree: &mir::Tree, callgraph: &CallGraph) -> CallGraphScc {
    // prepare tarjan state
    let mut index = 0usize;
    let mut next_scc_id = 0usize;
    let mut stack = Vec::new();
    let mut on_stack = HashSet::new();
    let mut indices = HashMap::new();
    let mut lowlinks = HashMap::new();
    let mut scc_map = CallGraphScc::default();

    // collect function ids for traversal
    let function_ids: Vec<_> = tree
        .iter_nodes::<mir::Function>()
        .map(|(id, _)| id)
        .collect();

    // run tarjan across all functions
    for function_id in function_ids {
        if !indices.contains_key(&function_id) {
            tarjan_visit(
                function_id,
                callgraph,
                &mut index,
                &mut next_scc_id,
                &mut stack,
                &mut on_stack,
                &mut indices,
                &mut lowlinks,
                &mut scc_map,
            );
        }
    }

    scc_map
}

/// Tarjan recursion for SCC discovery.
#[allow(clippy::too_many_arguments)]
fn tarjan_visit(
    function_id: mir::LocalNodeId<mir::Function>,
    callgraph: &CallGraph,
    index: &mut usize,
    next_scc_id: &mut usize,
    stack: &mut Vec<mir::LocalNodeId<mir::Function>>,
    on_stack: &mut HashSet<mir::LocalNodeId<mir::Function>>,
    indices: &mut HashMap<mir::LocalNodeId<mir::Function>, usize>,
    lowlinks: &mut HashMap<mir::LocalNodeId<mir::Function>, usize>,
    scc_map: &mut CallGraphScc,
) {
    // initialize tarjan state for this node
    indices.insert(function_id, *index);
    lowlinks.insert(function_id, *index);
    *index += 1;
    stack.push(function_id);
    on_stack.insert(function_id);

    // walk direct call edges to discover SCCs
    for edge in callgraph.outgoing(function_id) {
        // skip non direct edges for recursion detection
        if edge.dispatch != mir::CallDispatchKind::Direct {
            continue;
        }

        // visit the callee for scc discovery
        let callee = edge.callee;
        if !indices.contains_key(&callee) {
            tarjan_visit(
                callee,
                callgraph,
                index,
                next_scc_id,
                stack,
                on_stack,
                indices,
                lowlinks,
                scc_map,
            );

            // update the lowlink with the child lowlink
            let lowlink = lowlinks[&function_id].min(lowlinks[&callee]);
            lowlinks.insert(function_id, lowlink);
        } else if on_stack.contains(&callee) {
            let lowlink = lowlinks[&function_id].min(indices[&callee]);
            lowlinks.insert(function_id, lowlink);
        }
    }

    // finalize SCC if this node is a root
    if lowlinks[&function_id] == indices[&function_id] {
        // assign a new scc id
        let scc_id = *next_scc_id;
        *next_scc_id += 1;
        let mut scc_members = Vec::new();

        // pop the scc nodes from the stack
        while let Some(node) = stack.pop() {
            on_stack.remove(&node);
            scc_map.function_scc.insert(node, scc_id);
            scc_members.push(node);
            if node == function_id {
                break;
            }
        }

        // mark the scc as recursive when it has a cycle
        if scc_members.len() > 1 {
            scc_map.recursive_sccs.insert(scc_id);
        } else {
            let node = scc_members[0];

            // detect a self edge to mark recursion
            let self_edge = callgraph
                .outgoing(node)
                .iter()
                .any(|edge| edge.callee == node && edge.dispatch == mir::CallDispatchKind::Direct);
            if self_edge {
                scc_map.recursive_sccs.insert(scc_id);
            }
        }
    }
}

/// Resolved callsite data for call graph construction.
struct CallSite {
    /// The caller function id.
    caller: mir::LocalNodeId<mir::Function>,
    /// The callsite reference.
    callsite: CallSiteRef,
    /// Dispatch kind for the callsite.
    dispatch: mir::CallDispatchKind,
    /// Resolved callee when known.
    callee: Option<mir::LocalNodeId<mir::Function>>,
    /// True when the dispatch is fully resolved.
    is_precise: bool,
}

/// Return the resolved function target for an instruction callsite.
pub fn instruction_resolved_target(
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    instruction: &mir::Instruction,
    tree: &mir::Tree,
) -> Option<mir::LocalNodeId<mir::Function>> {
    instruction
        .call_direct_target()
        .and_then(|target| target.function())
        .or_else(|| {
            tree.metadata
                .functions
                .call(mir::CallSite::Instruction(instruction_id))
                .and_then(|metadata| metadata.target)
        })
}

/// Return the resolved function target for a terminator callsite.
pub fn terminator_resolved_target(
    block_id: mir::LocalNodeId<mir::Block>,
    terminator: &mir::Terminator,
    tree: &mir::Tree,
) -> Option<mir::LocalNodeId<mir::Function>> {
    terminator
        .call_direct_target()
        .and_then(|target| target.function())
        .or_else(|| {
            tree.metadata
                .functions
                .call(mir::CallSite::Terminator(block_id))
                .and_then(|metadata| metadata.target)
        })
}

impl CallSite {
    /// Build a callsite from an instruction when it represents a call.
    fn from_instruction(
        caller: mir::LocalNodeId<mir::Function>,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
        tree: &mir::Tree,
    ) -> Option<Self> {
        let dispatch = instruction.call_dispatch_kind()?;
        let callee = instruction_resolved_target(instruction_id, instruction, tree);
        let is_precise = matches!(dispatch, mir::CallDispatchKind::Direct);

        Some(Self {
            caller,
            callsite: CallSiteRef::Instruction(instruction_id),
            dispatch,
            callee,
            is_precise,
        })
    }

    /// Build a callsite from a terminator when it represents a call.
    fn from_terminator(
        caller: mir::LocalNodeId<mir::Function>,
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &mir::Terminator,
        tree: &mir::Tree,
    ) -> Option<Self> {
        let callsite = CallSiteRef::Terminator(block_id);

        match terminator {
            mir::Terminator::Call { function, .. } => Some(Self {
                caller,
                callsite,
                dispatch: mir::CallDispatchKind::Direct,
                callee: function.function(),
                is_precise: true,
            }),
            mir::Terminator::CallIndirect { .. } => Some(Self {
                caller,
                callsite,
                dispatch: mir::CallDispatchKind::Indirect,
                callee: None,
                is_precise: false,
            }),
            mir::Terminator::CallVirtual { slot, .. } => Some(Self {
                caller,
                callsite,
                dispatch: mir::CallDispatchKind::Virtual { slot: *slot },
                callee: terminator_resolved_target(block_id, terminator, tree),
                is_precise: false,
            }),
            mir::Terminator::CallDynamic { slot, .. } => Some(Self {
                caller,
                callsite,
                dispatch: mir::CallDispatchKind::Dynamic { slot: *slot },
                callee: terminator_resolved_target(block_id, terminator, tree),
                is_precise: false,
            }),
            mir::Terminator::TailCall { function, .. } => Some(Self {
                caller,
                callsite,
                dispatch: mir::CallDispatchKind::Direct,
                callee: function.function(),
                is_precise: true,
            }),
            mir::Terminator::TailCallIndirect { .. } => Some(Self {
                caller,
                callsite,
                dispatch: mir::CallDispatchKind::Indirect,
                callee: None,
                is_precise: false,
            }),
            mir::Terminator::TailCallVirtual { slot, .. } => Some(Self {
                caller,
                callsite,
                dispatch: mir::CallDispatchKind::Virtual { slot: *slot },
                callee: terminator_resolved_target(block_id, terminator, tree),
                is_precise: false,
            }),
            mir::Terminator::TailCallDynamic { slot, .. } => Some(Self {
                caller,
                callsite,
                dispatch: mir::CallDispatchKind::Dynamic { slot: *slot },
                callee: terminator_resolved_target(block_id, terminator, tree),
                is_precise: false,
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestProgram;

    /// Direct calls create edges in the call graph.
    #[test]
    fn test_call_graph_direct_call() {
        let test = TestProgram::new(
            r#"
function callee(): int32 {
b0:
    v0: int32 = 7int32
    return v0
}
function test(): int32 {
b0:
    v0: int32 = call callee(): () -> int32
    return v0
}"#,
        );

        let callee_id = test.function_id_by_name("callee");
        let test_id = test.function_id_by_name("test");

        let analyses = ModuleAnalyses::new();
        let callgraph = analyses.get::<CallGraph>(&test.tree);

        let outgoing = callgraph.outgoing(test_id);
        let incoming = callgraph.incoming(callee_id);

        assert_eq!(outgoing.len(), 1);
        assert_eq!(incoming.len(), 1);
        assert!(outgoing[0].is_direct());
        assert_eq!(outgoing[0].callee, callee_id);
        assert_eq!(incoming[0].caller, test_id);
        assert!(callgraph.unknown_calls(test_id).is_empty());
    }

    /// Call graph SCCs detect recursive functions.
    #[test]
    fn test_call_graph_scc_recursion() {
        let test = TestProgram::new(
            r#"
function alpha(): void {
b0:
    call beta(): () -> void
    return
}
function beta(): void {
b0:
    call alpha(): () -> void
    return
}
function gamma(): void {
b0:
    call gamma(): () -> void
    return
}
function delta(): void {
b0:
    return
}"#,
        );

        let a_id = test.function_id_by_name("alpha");
        let b_id = test.function_id_by_name("beta");
        let c_id = test.function_id_by_name("gamma");
        let d_id = test.function_id_by_name("delta");

        let analyses = ModuleAnalyses::new();
        let scc = analyses.get::<CallGraphScc>(&test.tree);

        assert!(scc.is_recursive_function(a_id));
        assert!(scc.is_recursive_function(b_id));
        assert!(scc.is_recursive_function(c_id));
        assert!(!scc.is_recursive_function(d_id));
    }

    /// Indirect calls without metadata remain unresolved.
    #[test]
    fn test_call_graph_indirect_unknown() {
        let test = TestProgram::new(
            r#"
function test(v0: (int32) -> int32, v1: int32): int32  {
b0(v0: (int32) -> int32, v1: int32):
    v2: int32 = call.indirect v0(v1): (int32) -> int32
    return v2
}"#,
        );

        let test_id = test.function_id_by_name("test");

        let analyses = ModuleAnalyses::new();
        let callgraph = analyses.get::<CallGraph>(&test.tree);

        assert!(callgraph.outgoing(test_id).is_empty());
        assert_eq!(callgraph.unknown_calls(test_id).len(), 1);
        assert_eq!(
            callgraph.unknown_calls(test_id)[0].dispatch,
            mir::CallDispatchKind::Indirect
        );
    }

    /// Tail calls are tracked as call edges.
    #[test]
    fn test_call_graph_tailcall_direct() {
        let test = TestProgram::new(
            r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function test(v0: int32): int32 {
b0(v0: int32):
    tailCall callee(v0): (int32) -> int32
}"#,
        );

        let callee_id = test.function_id_by_name("callee");
        let test_id = test.function_id_by_name("test");

        let analyses = ModuleAnalyses::new();
        let callgraph = analyses.get::<CallGraph>(&test.tree);

        let outgoing = callgraph.outgoing(test_id);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callee_id);
        assert!(matches!(outgoing[0].callsite, CallSiteRef::Terminator(_)));
    }

    /// Tailcall.indirect remains unresolved without metadata.
    #[test]
    fn test_call_graph_tailcall_indirect_unknown() {
        let test = TestProgram::new(
            r#"
function test(v0: (int32) -> int32, v1: int32): int32 {
b0(v0: (int32) -> int32, v1: int32):
    tailCall.indirect v0(v1): (int32) -> int32
}"#,
        );

        let test_id = test.function_id_by_name("test");

        let analyses = ModuleAnalyses::new();
        let callgraph = analyses.get::<CallGraph>(&test.tree);

        let unknown = callgraph.unknown_calls(test_id);
        assert_eq!(unknown.len(), 1);
        assert_eq!(unknown[0].dispatch, mir::CallDispatchKind::Indirect);
        assert!(matches!(unknown[0].callsite, CallSiteRef::Terminator(_)));
    }

    /// Call.indirect remains unresolved without a declared target.
    #[test]
    fn test_call_graph_call_indirect_unknown() {
        let test = TestProgram::new(
            r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}

function test(v0: (int32) -> int32, v1: int32): int32  {
b0(v0: (int32) -> int32, v1: int32):
    v2: int32 = call.indirect v0(v1): (int32) -> int32
    return v2
}"#,
        );

        let test_id = test.function_id_by_name("test");

        let analyses = ModuleAnalyses::new();
        let callgraph = analyses.get::<CallGraph>(&test.tree);

        let unknown = callgraph.unknown_calls(test_id);
        assert_eq!(unknown.len(), 1);
        assert_eq!(unknown[0].dispatch, mir::CallDispatchKind::Indirect);
    }

    /// Direct call terminators produce precise call edges.
    #[test]
    fn test_call_graph_call_terminator_direct() {
        let test = TestProgram::new(
            r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function test(v0: int32): int32 {
b0(v0: int32):
    call callee(v0): (int32) -> int32 -> b1
b1(v1: int32):
    return v1
b2(v2: ref<int32, managed, readonly>):
    panic v2
}"#,
        );

        let callee_id = test.function_id_by_name("callee");
        let test_id = test.function_id_by_name("test");

        let analyses = ModuleAnalyses::new();
        let callgraph = analyses.get::<CallGraph>(&test.tree);

        let outgoing = callgraph.outgoing(test_id);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callee_id);
        assert_eq!(outgoing[0].dispatch, mir::CallDispatchKind::Direct);
        assert!(matches!(outgoing[0].callsite, CallSiteRef::Terminator(_)));
        assert!(callgraph.unknown_calls(test_id).is_empty());
    }

    /// Class dispatch keeps a call edge and records an unknown target.
    #[test]
    fn test_call_graph_virtual_dispatch_is_partial() {
        let mut test = TestProgram::new(
            r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = call.virtual v0, int32, 1(v0): (int32) -> int32
    return v1
}"#,
        );

        let callee_id = test.function_id_by_name("callee");
        let test_id = test.function_id_by_name("test");

        let function = test.tree.get(test_id);
        let mut call_id = None;
        for &block_id in &function.blocks {
            let block = test.tree.get(block_id);
            for &instruction_id in &block.instructions {
                if matches!(
                    test.tree.get(instruction_id),
                    mir::Instruction::CallVirtual { .. }
                ) {
                    call_id = Some(instruction_id);
                    break;
                }
            }
            if call_id.is_some() {
                break;
            }
        }
        let call_id = call_id.expect("missing virtual call instruction");

        test.tree
            .metadata
            .functions
            .call_mut(mir::CallSite::Instruction(call_id))
            .target = Some(callee_id);

        let analyses = ModuleAnalyses::new();
        let callgraph = analyses.get::<CallGraph>(&test.tree);

        assert_eq!(callgraph.outgoing(test_id).len(), 1);
        assert_eq!(callgraph.outgoing(test_id)[0].callee, callee_id);
        assert_eq!(
            callgraph.outgoing(test_id)[0].dispatch,
            mir::CallDispatchKind::Virtual {
                slot: mir::DispatchSlot::new(1),
            }
        );
        assert_eq!(callgraph.unknown_calls(test_id).len(), 1);
        assert_eq!(callgraph.unknown_calls(test_id)[0].callee, Some(callee_id));
    }

    /// Class call terminators keep the declared target and unknown edge.
    #[test]
    fn test_call_graph_call_virtual_terminator_is_partial() {
        let mut test = TestProgram::new(
            r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function test(v0: int32): int32 {
b0(v0: int32):
    call.virtual v0, int32, 1(v0): (int32) -> int32 -> b1
b1(v1: int32):
    return v1
b2(v2: ref<int32, managed, readonly>):
    panic v2
}"#,
        );

        let callee_id = test.function_id_by_name("callee");
        let test_id = test.function_id_by_name("test");
        let function = test.tree.get(test_id);
        let block_id = *function.blocks.first().expect("missing entry block");

        test.tree
            .metadata
            .functions
            .call_mut(mir::CallSite::Terminator(block_id))
            .target = Some(callee_id);

        let analyses = ModuleAnalyses::new();
        let callgraph = analyses.get::<CallGraph>(&test.tree);

        let outgoing = callgraph.outgoing(test_id);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callee_id);
        assert_eq!(
            outgoing[0].dispatch,
            mir::CallDispatchKind::Virtual {
                slot: mir::DispatchSlot::new(1),
            }
        );
        assert!(matches!(outgoing[0].callsite, CallSiteRef::Terminator(_)));
        assert_eq!(callgraph.unknown_calls(test_id).len(), 1);
        assert_eq!(callgraph.unknown_calls(test_id)[0].callee, Some(callee_id));
    }
}
