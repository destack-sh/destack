use std::collections::HashMap;

use destack_core::{BitSet, DenseGraph};

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
    /// Component id by dense function id.
    function_scc: Vec<u32>,
    /// SCCs that are recursive.
    recursive_sccs: BitSet,
}

impl CallGraphScc {
    /// Build strongly connected components for one call graph.
    fn build(tree: &mir::Tree, callgraph: &CallGraph) -> Self {
        // size the dense graph from the function arena ids
        let function_count = tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id.get())
            .max()
            .map_or(0, |last| last + 1);

        // count direct call edges per function
        let mut edge_offsets = vec![0u32; function_count + 1];
        for (function_id, _) in tree.iter_nodes::<mir::Function>() {
            let source = function_id.get();
            let count = callgraph
                .outgoing(function_id)
                .iter()
                .filter(|edge| edge.is_direct())
                .count();
            edge_offsets[source + 1] = count as u32;
        }

        // prefix sum edge counts into CSR offsets
        for source in 0..function_count {
            edge_offsets[source + 1] += edge_offsets[source];
        }

        // copy direct call targets into dense edge storage
        let mut edge_targets = Vec::with_capacity(edge_offsets[function_count] as usize);
        for source in 0..function_count {
            let function_id = mir::LocalNodeId::<mir::Function>::new(source as u32);
            for edge in callgraph.outgoing(function_id) {
                if edge.is_direct() {
                    edge_targets.push(edge.callee.get() as u32);
                }
            }
        }

        // partition the direct call graph
        let graph = DenseGraph::new(&edge_offsets, &edge_targets);
        let partition = graph.strongly_connected_components();

        // count component sizes to identify multi-function cycles
        let component_count = partition.component_count() as usize;
        let mut component_sizes = vec![0usize; component_count];
        for component in partition.components() {
            component_sizes[*component as usize] += 1;
        }

        // map each function to its component
        let mut scc_map = CallGraphScc {
            function_scc: vec![0; function_count],
            recursive_sccs: BitSet::new(component_count),
        };
        for (function_id, _) in tree.iter_nodes::<mir::Function>() {
            let component = partition.component(function_id.get()) as usize;
            scc_map.function_scc[function_id.get()] = component as u32;
        }

        // mark recursive components from cycles or direct self-calls
        for (function_id, _) in tree.iter_nodes::<mir::Function>() {
            let component = partition.component(function_id.get()) as usize;
            let is_cycle = component_sizes[component] > 1;
            let is_self_call = callgraph
                .outgoing(function_id)
                .iter()
                .any(|edge| edge.is_direct() && edge.callee == function_id);

            if is_cycle || is_self_call {
                scc_map.recursive_sccs.insert(component);
            }
        }

        scc_map
    }

    /// Return the SCC id for a function.
    pub fn scc_id(&self, function_id: mir::LocalNodeId<mir::Function>) -> Option<usize> {
        self.function_scc
            .get(function_id.get())
            .map(|component| *component as usize)
    }

    /// Return true when the SCC is recursive.
    pub fn is_recursive_scc(&self, scc_id: usize) -> bool {
        scc_id < self.recursive_sccs.len() && self.recursive_sccs.contains(scc_id)
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
        Self::build(tree, &callgraph)
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

impl CallSite {
    /// Build a callsite from an instruction when it represents a call.
    fn from_instruction(
        caller: mir::LocalNodeId<mir::Function>,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
        tree: &mir::Tree,
    ) -> Option<Self> {
        let dispatch = instruction.call_dispatch_kind()?;
        let callee = Self::instruction_target(instruction_id, instruction, tree);
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
                callee: Some(*function),
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
                callee: Self::terminator_target(block_id, terminator, tree),
                is_precise: false,
            }),
            mir::Terminator::CallDynamic { slot, .. } => Some(Self {
                caller,
                callsite,
                dispatch: mir::CallDispatchKind::Dynamic { slot: *slot },
                callee: Self::terminator_target(block_id, terminator, tree),
                is_precise: false,
            }),
            mir::Terminator::TailCall { function, .. } => Some(Self {
                caller,
                callsite,
                dispatch: mir::CallDispatchKind::Direct,
                callee: Some(*function),
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
                callee: Self::terminator_target(block_id, terminator, tree),
                is_precise: false,
            }),
            mir::Terminator::TailCallDynamic { slot, .. } => Some(Self {
                caller,
                callsite,
                dispatch: mir::CallDispatchKind::Dynamic { slot: *slot },
                callee: Self::terminator_target(block_id, terminator, tree),
                is_precise: false,
            }),
            _ => None,
        }
    }

    /// Return the resolved function target for an instruction callsite.
    fn instruction_target(
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
        tree: &mir::Tree,
    ) -> Option<mir::LocalNodeId<mir::Function>> {
        instruction.call_direct_target().or_else(|| {
            tree.metadata
                .functions
                .call(mir::CallSite::Instruction(instruction_id))
                .and_then(|metadata| metadata.target)
        })
    }

    /// Return the resolved function target for a terminator callsite.
    fn terminator_target(
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &mir::Terminator,
        tree: &mir::Tree,
    ) -> Option<mir::LocalNodeId<mir::Function>> {
        terminator.call_direct_target().or_else(|| {
            tree.metadata
                .functions
                .call(mir::CallSite::Terminator(block_id))
                .and_then(|metadata| metadata.target)
        })
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
entry:
    v0: int32 = 7
    return v0
}

function test(): int32 {
entry:
    v0: int32 = call callee()
    return v0
}
"#,
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
entry:
    call beta()
    return
}

function beta(): void {
entry:
    call alpha()
    return
}

function gamma(): void {
entry:
    call gamma()
    return
}

function delta(): void {
entry:
    return
}
"#,
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
function test(v0: fn(int32) => int32, v1: int32): int32 {
entry(v0: fn(int32) => int32, v1: int32):
    v2: int32 = call.indirect v0(v1): (int32) => int32
    return v2
}
"#,
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
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    tail.call callee(v0)
}
"#,
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
function test(v0: fn(int32) => int32, v1: int32): int32 {
entry(v0: fn(int32) => int32, v1: int32):
    tail.call.indirect v0(v1): (int32) => int32
}
"#,
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
entry(v0: int32):
    return v0
}

function test(v0: fn(int32) => int32, v1: int32): int32 {
entry(v0: fn(int32) => int32, v1: int32):
    v2: int32 = call.indirect v0(v1): (int32) => int32
    return v2
}
"#,
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
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    call callee(v0) => b1

b1(v1: int32):
    return v1

b2(v2: ref<int32, managed, readonly>):
    panic v2
}
"#,
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
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call.virtual v0, int32, 1(v0): (int32) => int32
    return v1
}
"#,
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
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    call.virtual v0, int32, 1(v0): (int32) => int32 => b1

b1(v1: int32):
    return v1

b2(v2: ref<int32, managed, readonly>):
    panic v2
}
"#,
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
