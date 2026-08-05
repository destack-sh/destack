use std::collections::HashMap;

use destack_core::{BitSet, DenseGraph};

use crate as mir;

use super::{Analysis, AnalysisId, ModuleAnalysis, TreeAnalysisCache};

/// Directed edge in the call graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallEdge {
    /// The caller function id.
    pub caller: mir::LocalNodeId<mir::Function>,
    /// The callee function id.
    pub callee: mir::LocalNodeId<mir::Function>,
    /// The callsite that performs the call.
    pub callsite: mir::CallSite,
    /// The dispatch for this callsite.
    pub dispatch: mir::CallDispatch,
}

impl CallEdge {
    /// Return true when this edge is a direct call.
    pub fn is_direct(&self) -> bool {
        matches!(self.dispatch, mir::CallDispatch::Direct)
    }
}

/// Callsite whose target set is not closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpenCallSite {
    /// The caller function id.
    pub caller: mir::LocalNodeId<mir::Function>,
    /// The callsite that performs the call.
    pub callsite: mir::CallSite,
    /// The dispatch for this callsite.
    pub dispatch: mir::CallDispatch,
    /// The known target when one has been resolved.
    pub known_target: Option<mir::LocalNodeId<mir::Function>>,
}

/// Module scoped call graph.
#[derive(Debug)]
pub struct CallGraph {
    /// Outgoing edges by caller.
    outgoing: HashMap<mir::LocalNodeId<mir::Function>, Vec<CallEdge>>,
    /// Incoming edges by callee.
    incoming: HashMap<mir::LocalNodeId<mir::Function>, Vec<CallEdge>>,
    /// Open callsites by caller.
    open_callsites: HashMap<mir::LocalNodeId<mir::Function>, Vec<OpenCallSite>>,
    /// Component id by dense function id.
    function_component: Vec<u32>,
    /// Components that are recursive.
    recursive_components: BitSet,
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

    /// Get open callsites for a function.
    pub fn open_callsites(&self, caller: mir::LocalNodeId<mir::Function>) -> &[OpenCallSite] {
        const EMPTY: [OpenCallSite; 0] = [];
        self.open_callsites
            .get(&caller)
            .map(Vec::as_slice)
            .unwrap_or(&EMPTY)
    }

    /// Return the call graph component for a function.
    pub fn component(&self, function_id: mir::LocalNodeId<mir::Function>) -> Option<usize> {
        self.function_component
            .get(function_id.get())
            .map(|component| *component as usize)
    }

    /// Return true when a component is recursive.
    pub fn is_recursive_component(&self, component: usize) -> bool {
        component < self.recursive_components.len() && self.recursive_components.contains(component)
    }

    /// Return true when the function is part of a recursive component.
    pub fn is_recursive_function(&self, function_id: mir::LocalNodeId<mir::Function>) -> bool {
        let Some(component) = self.component(function_id) else {
            return false;
        };

        self.is_recursive_component(component)
    }

    /// Build a call graph for the given module.
    fn build(tree: &mir::Tree, effects: &mir::EffectTable) -> Self {
        let mut graph = Self {
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
            open_callsites: HashMap::new(),
            function_component: Vec::new(),
            recursive_components: BitSet::new(0),
        };

        // scan each function for callsites
        for (function_id, function) in tree.iter_nodes::<mir::Function>() {
            // skip functions without bodies
            if function.entry().is_none() {
                continue;
            }

            for block_id in function.blocks() {
                let block = tree.get(*block_id);
                let terminator = tree.get(block.terminator);

                for &instruction_id in &block.instructions {
                    let instruction = tree.get(instruction_id);

                    let Some(callsite) = ScannedCallSite::from_instruction(
                        function_id,
                        instruction_id,
                        instruction,
                        effects,
                    ) else {
                        continue;
                    };

                    graph.insert_call(callsite);
                }

                if let Some(callsite) =
                    ScannedCallSite::from_terminator(function_id, *block_id, terminator, effects)
                {
                    graph.insert_call(callsite);
                }
            }
        }

        graph.build_components(tree);
        graph
    }

    /// Insert one observed call into the graph.
    fn insert_call(&mut self, callsite: ScannedCallSite) {
        // record resolved edges when a target is known
        if let Some(callee) = callsite.known_target {
            let edge = CallEdge {
                caller: callsite.caller,
                callee,
                callsite: callsite.callsite,
                dispatch: callsite.dispatch,
            };

            self.outgoing.entry(callsite.caller).or_default().push(edge);
            self.incoming.entry(callee).or_default().push(edge);

            if callsite.is_closed {
                return;
            }
        }

        // record open callsites
        let open_callsite = OpenCallSite {
            caller: callsite.caller,
            callsite: callsite.callsite,
            dispatch: callsite.dispatch,
            known_target: callsite.known_target,
        };

        self.open_callsites
            .entry(callsite.caller)
            .or_default()
            .push(open_callsite);
    }

    /// Build direct-call recursion components.
    fn build_components(&mut self, tree: &mir::Tree) {
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
            let count = self
                .outgoing
                .get(&function_id)
                .map(Vec::as_slice)
                .unwrap_or(&[])
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
            for edge in self.outgoing(function_id) {
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
        self.function_component = vec![0; function_count];
        self.recursive_components = BitSet::new(component_count);
        for (function_id, _) in tree.iter_nodes::<mir::Function>() {
            let component = partition.component(function_id.get()) as usize;
            self.function_component[function_id.get()] = component as u32;
        }

        // mark recursive components from cycles or direct self-calls
        for (function_id, _) in tree.iter_nodes::<mir::Function>() {
            let component = partition.component(function_id.get()) as usize;
            let is_cycle = component_sizes[component] > 1;
            let is_self_call = self
                .outgoing(function_id)
                .iter()
                .any(|edge| edge.is_direct() && edge.callee == function_id);

            if is_cycle || is_self_call {
                self.recursive_components.insert(component);
            }
        }
    }
}

impl Analysis for CallGraph {
    const ID: AnalysisId = AnalysisId("callgraph");
}

impl ModuleAnalysis for CallGraph {
    /// Compute the module call graph.
    fn compute(tree: &mir::Tree, analyses: &TreeAnalysisCache) -> Self {
        Self::build(tree, analyses.effects())
    }
}

/// Callsite scanned during call graph construction.
struct ScannedCallSite {
    /// The caller function id.
    caller: mir::LocalNodeId<mir::Function>,
    /// The callsite identity.
    callsite: mir::CallSite,
    /// Dispatch for the callsite.
    dispatch: mir::CallDispatch,
    /// Resolved target when known.
    known_target: Option<mir::LocalNodeId<mir::Function>>,
    /// True when the dispatch target set is closed.
    is_closed: bool,
}

impl ScannedCallSite {
    /// Build a callsite from an instruction when it represents a call.
    fn from_instruction(
        caller: mir::LocalNodeId<mir::Function>,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
        effects: &mir::EffectTable,
    ) -> Option<Self> {
        let dispatch = instruction.call_dispatch()?;
        let known_target = Self::instruction_target(instruction_id, instruction, effects);
        let is_closed = matches!(dispatch, mir::CallDispatch::Direct);

        Some(Self {
            caller,
            callsite: mir::CallSite::Instruction(instruction_id),
            dispatch,
            known_target,
            is_closed,
        })
    }

    /// Build a callsite from a terminator when it represents a call.
    fn from_terminator(
        caller: mir::LocalNodeId<mir::Function>,
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &mir::Terminator,
        effects: &mir::EffectTable,
    ) -> Option<Self> {
        let callsite = mir::CallSite::Terminator(block_id);
        let dispatch = terminator.call_dispatch()?;
        let known_target = Self::terminator_target(block_id, terminator, effects);
        let is_closed = matches!(dispatch, mir::CallDispatch::Direct);

        Some(Self {
            caller,
            callsite,
            dispatch,
            known_target,
            is_closed,
        })
    }

    /// Return the resolved function target for an instruction callsite.
    fn instruction_target(
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
        effects: &mir::EffectTable,
    ) -> Option<mir::LocalNodeId<mir::Function>> {
        instruction.call_direct_target().or_else(|| {
            effects
                .call(mir::CallSite::Instruction(instruction_id))
                .and_then(|tables| tables.target)
        })
    }

    /// Return the resolved function target for a terminator callsite.
    fn terminator_target(
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &mir::Terminator,
        effects: &mir::EffectTable,
    ) -> Option<mir::LocalNodeId<mir::Function>> {
        terminator.call_direct_target().or_else(|| {
            effects
                .call(mir::CallSite::Terminator(block_id))
                .and_then(|tables| tables.target)
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
    v0: int32 = call callee(): () => int32
    return v0
}
"#,
        );

        let callee_id = test.function_id_by_name("callee");
        let test_id = test.function_id_by_name("test");

        let analyses = test.tree_analysis_cache();
        let callgraph = analyses.get::<CallGraph>(&test.tree);

        let outgoing = callgraph.outgoing(test_id);
        let incoming = callgraph.incoming(callee_id);

        assert_eq!(outgoing.len(), 1);
        assert_eq!(incoming.len(), 1);
        assert!(outgoing[0].is_direct());
        assert_eq!(outgoing[0].callee, callee_id);
        assert_eq!(incoming[0].caller, test_id);
        assert!(callgraph.open_callsites(test_id).is_empty());
    }

    /// Call graph components detect recursive functions.
    #[test]
    fn test_call_graph_component_recursion() {
        let test = TestProgram::new(
            r#"
function alpha(): void {
entry:
    call beta(): () => void
    return
}

function beta(): void {
entry:
    call alpha(): () => void
    return
}

function gamma(): void {
entry:
    call gamma(): () => void
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

        let analyses = test.tree_analysis_cache();
        let callgraph = analyses.get::<CallGraph>(&test.tree);

        assert!(callgraph.is_recursive_function(a_id));
        assert!(callgraph.is_recursive_function(b_id));
        assert!(callgraph.is_recursive_function(c_id));
        assert!(!callgraph.is_recursive_function(d_id));
    }

    /// Indirect calls without tables stay open.
    #[test]
    fn test_call_graph_indirect_open() {
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

        let analyses = test.tree_analysis_cache();
        let callgraph = analyses.get::<CallGraph>(&test.tree);

        assert!(callgraph.outgoing(test_id).is_empty());
        assert_eq!(callgraph.open_callsites(test_id).len(), 1);
        assert_eq!(
            callgraph.open_callsites(test_id)[0].dispatch,
            mir::CallDispatch::Indirect
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
    tail.call callee(v0): (int32) => int32
}
"#,
        );

        let callee_id = test.function_id_by_name("callee");
        let test_id = test.function_id_by_name("test");

        let analyses = test.tree_analysis_cache();
        let callgraph = analyses.get::<CallGraph>(&test.tree);

        let outgoing = callgraph.outgoing(test_id);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callee_id);
        assert!(matches!(outgoing[0].callsite, mir::CallSite::Terminator(_)));
    }

    /// Tailcall.indirect remains open without tables.
    #[test]
    fn test_call_graph_tailcall_indirect_open() {
        let test = TestProgram::new(
            r#"
function test(v0: fn(int32) => int32, v1: int32): int32 {
entry(v0: fn(int32) => int32, v1: int32):
    tail.call.indirect v0(v1): (int32) => int32
}
"#,
        );

        let test_id = test.function_id_by_name("test");

        let analyses = test.tree_analysis_cache();
        let callgraph = analyses.get::<CallGraph>(&test.tree);

        let open_callsite = callgraph.open_callsites(test_id);
        assert_eq!(open_callsite.len(), 1);
        assert_eq!(open_callsite[0].dispatch, mir::CallDispatch::Indirect);
        assert!(matches!(
            open_callsite[0].callsite,
            mir::CallSite::Terminator(_)
        ));
    }

    /// Call.indirect remains open without a known target.
    #[test]
    fn test_call_graph_call_indirect_open() {
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

        let analyses = test.tree_analysis_cache();
        let callgraph = analyses.get::<CallGraph>(&test.tree);

        let open_callsite = callgraph.open_callsites(test_id);
        assert_eq!(open_callsite.len(), 1);
        assert_eq!(open_callsite[0].dispatch, mir::CallDispatch::Indirect);
    }

    /// Direct invokes produce precise call edges.
    #[test]
    fn test_call_graph_invoke_direct() {
        let test = TestProgram::new(
            r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    invoke callee(v0): (int32) => int32 => b1 | b2

b1(v1: int32):
    return v1

b2:
    unwind.resume
}
"#,
        );

        let callee_id = test.function_id_by_name("callee");
        let test_id = test.function_id_by_name("test");

        let analyses = test.tree_analysis_cache();
        let callgraph = analyses.get::<CallGraph>(&test.tree);

        let outgoing = callgraph.outgoing(test_id);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callee_id);
        assert_eq!(outgoing[0].dispatch, mir::CallDispatch::Direct);
        assert!(matches!(outgoing[0].callsite, mir::CallSite::Terminator(_)));
        assert!(callgraph.open_callsites(test_id).is_empty());
    }

    /// Class dispatch keeps a call edge and records an open target.
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
    v1: int32 = call.virtual v0, int32, 0(v0): (int32) => int32
    return v1
}
"#,
        );

        let callee_id = test.function_id_by_name("callee");
        let test_id = test.function_id_by_name("test");

        let function = test.tree.get(test_id);
        let mut call_id = None;
        for &block_id in function.blocks() {
            let block = test.tree.get(block_id);
            for &instruction_id in &block.instructions {
                if matches!(
                    test.tree.get(instruction_id),
                    mir::Instruction::Call {
                        call: mir::Call {
                            callee: mir::Callee::Virtual { .. },
                            ..
                        },
                        ..
                    }
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

        test.effects
            .call_mut(mir::CallSite::Instruction(call_id))
            .target = Some(callee_id);

        let analyses = test.tree_analysis_cache();
        let callgraph = analyses.get::<CallGraph>(&test.tree);

        assert_eq!(callgraph.outgoing(test_id).len(), 1);
        assert_eq!(callgraph.outgoing(test_id)[0].callee, callee_id);
        assert_eq!(
            callgraph.outgoing(test_id)[0].dispatch,
            mir::CallDispatch::Virtual {
                slot: mir::DispatchSlot::new(0),
            }
        );
        assert_eq!(callgraph.open_callsites(test_id).len(), 1);
        assert_eq!(
            callgraph.open_callsites(test_id)[0].known_target,
            Some(callee_id)
        );
    }

    /// Virtual invokes keep the known target and open callsite.
    #[test]
    fn test_call_graph_invoke_virtual_is_partial() {
        let mut test = TestProgram::new(
            r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(v0: int32): int32 {
entry(v0: int32):
    invoke.virtual v0, int32, 0(v0): (int32) => int32 => b1 | b2

b1(v1: int32):
    return v1

b2:
    unwind.resume
}
"#,
        );

        let callee_id = test.function_id_by_name("callee");
        let test_id = test.function_id_by_name("test");
        let function = test.tree.get(test_id);
        let block_id = *function.blocks().first().expect("missing entry block");

        test.effects
            .call_mut(mir::CallSite::Terminator(block_id))
            .target = Some(callee_id);

        let analyses = test.tree_analysis_cache();
        let callgraph = analyses.get::<CallGraph>(&test.tree);

        let outgoing = callgraph.outgoing(test_id);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callee_id);
        assert_eq!(
            outgoing[0].dispatch,
            mir::CallDispatch::Virtual {
                slot: mir::DispatchSlot::new(0),
            }
        );
        assert!(matches!(outgoing[0].callsite, mir::CallSite::Terminator(_)));
        assert_eq!(callgraph.open_callsites(test_id).len(), 1);
        assert_eq!(
            callgraph.open_callsites(test_id)[0].known_target,
            Some(callee_id)
        );
    }
}
