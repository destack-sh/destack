use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_core::{BitSet, DenseGraph};

use crate as mir;

use crate::{Analysis, ResolutionTable};

/// Module scoped call graph.
#[derive(Debug)]
pub struct CallTable {
    /// Functions sorted by id.
    functions: Vec<mir::FunctionId>,
    /// First outgoing edge offset for each function and the final edge count.
    outgoing_offsets: Vec<u32>,
    /// Outgoing edges grouped by caller.
    outgoing: Vec<CallEdge>,
    /// First incoming edge offset for each function and the final edge count.
    incoming_offsets: Vec<u32>,
    /// Incoming edges grouped by callee.
    incoming: Vec<CallEdge>,
    /// First open callsite offset for each function and the final callsite count.
    open_offsets: Vec<u32>,
    /// Open callsites grouped by caller.
    open_callsites: Vec<OpenCallSite>,
    /// Component id by dense function id.
    function_component: Vec<u32>,
    /// Components that are recursive.
    recursive_components: BitSet,
}

/// Directed edge in the call graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallEdge {
    /// The caller function id.
    pub caller: mir::LocalNodeId<mir::Function>,
    /// The callee function id.
    pub callee: mir::LocalNodeId<mir::Function>,
    /// The callsite that performs the call.
    pub callsite: mir::Point,
    /// The dispatch for this callsite.
    pub dispatch: mir::CallDispatch,
}

/// Callsite whose target remains open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpenCallSite {
    /// The caller function id.
    pub caller: mir::FunctionId,
    /// The callsite that performs the call.
    pub callsite: mir::Point,
    /// The dispatch for this callsite.
    pub dispatch: mir::CallDispatch,
}

impl CallTable {
    /// Return outgoing call edges for a function.
    pub fn outgoing(&self, caller: mir::LocalNodeId<mir::Function>) -> &[CallEdge] {
        let function = self.function_index(caller);

        Self::entries(function, &self.outgoing_offsets, &self.outgoing)
    }

    /// Return incoming call edges for a function.
    pub fn incoming(&self, callee: mir::LocalNodeId<mir::Function>) -> &[CallEdge] {
        let function = self.function_index(callee);

        Self::entries(function, &self.incoming_offsets, &self.incoming)
    }

    /// Return open callsites for a function.
    pub fn open_callsites(&self, caller: mir::LocalNodeId<mir::Function>) -> &[OpenCallSite] {
        let function = self.function_index(caller);

        Self::entries(function, &self.open_offsets, &self.open_callsites)
    }

    /// Return the call graph component for a function.
    pub fn component(&self, function_id: mir::LocalNodeId<mir::Function>) -> usize {
        let function = self.function_index(function_id);

        self.function_component
            .get(function)
            .map(|component| *component as usize)
            .unwrap_or_else(|| unreachable!("function outside call table: {function_id:?}"))
    }

    /// Return true when a component is recursive.
    pub fn is_recursive_component(&self, component: usize) -> bool {
        if component >= self.recursive_components.len() {
            unreachable!("component outside call table: {component}");
        }

        self.recursive_components.contains(component)
    }

    /// Return true when the function is part of a recursive component.
    pub fn is_recursive_function(&self, function_id: mir::LocalNodeId<mir::Function>) -> bool {
        let component = self.component(function_id);

        self.is_recursive_component(component)
    }

    /// Build a call graph for the given module.
    pub fn analyse(resolution: &ResolutionTable, tree: &mir::Tree) -> Self {
        let mut functions = tree
            .iter_nodes::<mir::Function>()
            .map(|(function, _)| function)
            .collect::<Vec<_>>();
        functions.sort_unstable();

        let mut outgoing = Vec::new();
        let mut incoming = Vec::new();
        let mut open_callsites = Vec::new();

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
                        resolution,
                    ) else {
                        continue;
                    };

                    Self::record_call(callsite, &mut outgoing, &mut incoming, &mut open_callsites);
                }

                if let Some(callsite) =
                    ScannedCallSite::from_terminator(function_id, *block_id, terminator, resolution)
                {
                    Self::record_call(callsite, &mut outgoing, &mut incoming, &mut open_callsites);
                }
            }
        }

        // group every relation by its indexed function
        outgoing.sort_unstable_by_key(|edge| (edge.caller.get(), edge.callsite, edge.callee.get()));
        incoming.sort_unstable_by_key(|edge| (edge.callee.get(), edge.callsite, edge.caller.get()));
        open_callsites.sort_unstable_by_key(|callsite| (callsite.caller.get(), callsite.callsite));

        let outgoing_offsets = Self::offsets(&functions, &outgoing, |edge| edge.caller);
        let incoming_offsets = Self::offsets(&functions, &incoming, |edge| edge.callee);
        let open_offsets = Self::offsets(&functions, &open_callsites, |callsite| callsite.caller);

        let mut graph = Self {
            functions,
            outgoing_offsets,
            outgoing,
            incoming_offsets,
            incoming,
            open_offsets,
            open_callsites,
            function_component: Vec::new(),
            recursive_components: BitSet::new(0),
        };
        graph.build_components();

        graph
    }

    /// Record one observed call.
    fn record_call(
        callsite: ScannedCallSite,
        outgoing: &mut Vec<CallEdge>,
        incoming: &mut Vec<CallEdge>,
        open_callsites: &mut Vec<OpenCallSite>,
    ) {
        // record resolved edges when a target is known
        if let Some(callee) = callsite.known_target {
            let edge = CallEdge {
                caller: callsite.caller,
                callee,
                callsite: callsite.callsite,
                dispatch: callsite.dispatch,
            };

            outgoing.push(edge);
            incoming.push(edge);

            return;
        }

        // record open callsites
        let open_callsite = OpenCallSite {
            caller: callsite.caller,
            callsite: callsite.callsite,
            dispatch: callsite.dispatch,
        };

        open_callsites.push(open_callsite);
    }

    /// Build recursion components from closed call edges.
    fn build_components(&mut self) {
        let function_count = self.functions.len();

        // count closed call edges per function
        let mut edge_offsets = vec![0u32; function_count + 1];
        for (source, &function_id) in self.functions.iter().enumerate() {
            let count = self.outgoing(function_id).len();
            edge_offsets[source + 1] = count as u32;
        }

        // prefix sum edge counts into CSR offsets
        for source in 0..function_count {
            edge_offsets[source + 1] += edge_offsets[source];
        }

        // copy closed call targets into dense edge storage
        let mut edge_targets = Vec::with_capacity(edge_offsets[function_count] as usize);
        for &function_id in &self.functions {
            for edge in self.outgoing(function_id) {
                let target = Self::index(&self.functions, edge.callee);
                edge_targets.push(target as u32);
            }
        }

        // partition the closed call graph
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
        for function in 0..function_count {
            let component = partition.component(function) as usize;
            self.function_component[function] = component as u32;
        }

        // mark recursive components from cycles or direct self-calls
        for (function, &function_id) in self.functions.iter().enumerate() {
            let component = partition.component(function) as usize;
            let is_cycle = component_sizes[component] > 1;
            let is_self_call = self
                .outgoing(function_id)
                .iter()
                .any(|edge| edge.callee == function_id);

            if is_cycle || is_self_call {
                self.recursive_components.insert(component);
            }
        }
    }

    /// Build dense offsets for entries sorted by function id.
    fn offsets<T>(
        functions: &[mir::FunctionId],
        entries: &[T],
        function: impl Fn(&T) -> mir::FunctionId,
    ) -> Vec<u32> {
        let function_count = functions.len();
        let mut offsets = vec![0u32; function_count + 1];

        // count entries for each function
        for entry in entries {
            let index = Self::index(functions, function(entry));
            offsets[index + 1] += 1;
        }

        // convert counts into entry ranges
        for function in 0..function_count {
            offsets[function + 1] += offsets[function];
        }

        offsets
    }

    /// Return one function's compact index.
    fn function_index(&self, function: mir::FunctionId) -> usize {
        Self::index(&self.functions, function)
    }

    /// Return one function's index in a sorted function set.
    fn index(functions: &[mir::FunctionId], function: mir::FunctionId) -> usize {
        functions
            .binary_search(&function)
            .unwrap_or_else(|_| unreachable!("function outside call table: {function:?}"))
    }

    /// Return one function's entries from flattened storage.
    fn entries<'a, T>(function: usize, offsets: &[u32], entries: &'a [T]) -> &'a [T] {
        let range = offsets
            .get(function..=function + 1)
            .unwrap_or_else(|| unreachable!("function outside call table: {function}"));
        let start = range[0] as usize;
        let end = range[1] as usize;

        &entries[start..end]
    }
}

impl Analysis for CallTable {
    const INVALIDATED_BY: mir::Mutation = ResolutionTable::INVALIDATED_BY;
}

/// Strongly connected components of the whole-program call graph, by dense symbol id.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CallComponentGraph {
    /// The component id of each symbol, by dense id.
    component: Vec<u32>,
    /// Whether each component contains a cycle, by component id.
    recursive: BitSet,
}

impl CallComponentGraph {
    /// Analyse dense call edges to find recursive components.
    pub fn analyse(offsets: &[u32], targets: &[u32]) -> Self {
        // partition the call graph into strongly connected components
        let graph = DenseGraph::new(offsets, targets);
        let partition = graph.strongly_connected_components();
        let count = partition.component_count() as usize;

        // count nodes in each component
        let mut sizes = vec![0u32; count];
        for &component in partition.components() {
            sizes[component as usize] += 1;
        }

        // mark multi-node cycles and direct self-calls
        let mut recursive = BitSet::new(count);
        for node in 0..graph.node_count() {
            let component = partition.component(node);
            let start = offsets[node] as usize;
            let end = offsets[node + 1] as usize;
            let calls_self = targets[start..end].contains(&(node as u32));
            if sizes[component as usize] > 1 || calls_self {
                recursive.insert(component as usize);
            }
        }

        Self {
            component: partition.components().to_vec(),
            recursive,
        }
    }

    /// Return the component id of a symbol by dense id.
    pub fn component(&self, symbol: usize) -> u32 {
        self.component[symbol]
    }

    /// Return whether a symbol's component contains a cycle, by dense id.
    pub fn is_recursive(&self, symbol: usize) -> bool {
        self.recursive.contains(self.component[symbol] as usize)
    }

    /// Return the number of components.
    pub fn len(&self) -> usize {
        self.recursive.len()
    }

    /// Return whether there are no components.
    pub fn is_empty(&self) -> bool {
        self.recursive.is_empty()
    }
}

/// Callsite scanned during call graph construction.
struct ScannedCallSite {
    /// The caller function id.
    caller: mir::LocalNodeId<mir::Function>,
    /// The callsite identity.
    callsite: mir::Point,
    /// Dispatch for the callsite.
    dispatch: mir::CallDispatch,
    /// Resolved target when known.
    known_target: Option<mir::LocalNodeId<mir::Function>>,
}

impl ScannedCallSite {
    /// Build a callsite from an instruction when it represents a call.
    fn from_instruction(
        caller: mir::LocalNodeId<mir::Function>,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
        resolution: &ResolutionTable,
    ) -> Option<Self> {
        let dispatch = instruction.call_dispatch()?;
        let callsite = mir::Point::Instruction(instruction_id);
        let known_target = instruction
            .call_direct_target()
            .or_else(|| resolution.target(callsite));

        Some(Self {
            caller,
            callsite,
            dispatch,
            known_target,
        })
    }

    /// Build a callsite from a terminator when it represents a call.
    fn from_terminator(
        caller: mir::LocalNodeId<mir::Function>,
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &mir::Terminator,
        resolution: &ResolutionTable,
    ) -> Option<Self> {
        let callsite = mir::Point::Terminator(block_id);
        let dispatch = terminator.call_dispatch()?;
        let known_target = terminator
            .call_direct_target()
            .or_else(|| resolution.target(callsite));

        Some(Self {
            caller,
            callsite,
            dispatch,
            known_target,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestModule;

    /// Direct calls create edges in the call graph.
    #[test]
    fn test_call_graph_direct_call() {
        let test = TestModule::new(
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

        let mut analyses = test.module_analyses();
        let calls = analyses.call(&test.tree, &test.dispatch);

        let outgoing = calls.outgoing(test_id);
        let incoming = calls.incoming(callee_id);

        assert_eq!(outgoing.len(), 1);
        assert_eq!(incoming.len(), 1);
        assert_eq!(outgoing[0].dispatch, mir::CallDispatch::Direct);
        assert_eq!(outgoing[0].callee, callee_id);
        assert_eq!(incoming[0].caller, test_id);
        assert!(calls.open_callsites(test_id).is_empty());
    }

    /// Call graph components detect recursive functions.
    #[test]
    fn test_call_graph_component_recursion() {
        let test = TestModule::new(
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

        let mut analyses = test.module_analyses();
        let calls = analyses.call(&test.tree, &test.dispatch);

        assert!(calls.is_recursive_function(a_id));
        assert!(calls.is_recursive_function(b_id));
        assert!(calls.is_recursive_function(c_id));
        assert!(!calls.is_recursive_function(d_id));
    }

    /// Indirect calls without tables stay open.
    #[test]
    fn test_call_graph_indirect_open() {
        let test = TestModule::new(
            r#"
function test(v0: fn(int32) => int32, v1: int32): int32 {
entry(v0: fn(int32) => int32, v1: int32):
    v2: int32 = call.indirect v0(v1): (int32) => int32
    return v2
}
"#,
        );

        let test_id = test.function_id_by_name("test");

        let mut analyses = test.module_analyses();
        let calls = analyses.call(&test.tree, &test.dispatch);

        assert!(calls.outgoing(test_id).is_empty());
        assert_eq!(calls.open_callsites(test_id).len(), 1);
        assert_eq!(
            calls.open_callsites(test_id)[0].dispatch,
            mir::CallDispatch::Indirect
        );
    }

    /// Tail calls are tracked as call edges.
    #[test]
    fn test_call_graph_tailcall_direct() {
        let test = TestModule::new(
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

        let mut analyses = test.module_analyses();
        let calls = analyses.call(&test.tree, &test.dispatch);

        let outgoing = calls.outgoing(test_id);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callee_id);
        assert!(matches!(outgoing[0].callsite, mir::Point::Terminator(_)));
    }

    /// Tailcall.indirect remains open without tables.
    #[test]
    fn test_call_graph_tailcall_indirect_open() {
        let test = TestModule::new(
            r#"
function test(v0: fn(int32) => int32, v1: int32): int32 {
entry(v0: fn(int32) => int32, v1: int32):
    tail.call.indirect v0(v1): (int32) => int32
}
"#,
        );

        let test_id = test.function_id_by_name("test");

        let mut analyses = test.module_analyses();
        let calls = analyses.call(&test.tree, &test.dispatch);

        let open_callsite = calls.open_callsites(test_id);
        assert_eq!(open_callsite.len(), 1);
        assert_eq!(open_callsite[0].dispatch, mir::CallDispatch::Indirect);
        assert!(matches!(
            open_callsite[0].callsite,
            mir::Point::Terminator(_)
        ));
    }

    /// Call.indirect remains open without a known target.
    #[test]
    fn test_call_graph_call_indirect_open() {
        let test = TestModule::new(
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

        let mut analyses = test.module_analyses();
        let calls = analyses.call(&test.tree, &test.dispatch);

        let open_callsite = calls.open_callsites(test_id);
        assert_eq!(open_callsite.len(), 1);
        assert_eq!(open_callsite[0].dispatch, mir::CallDispatch::Indirect);
    }

    /// Direct invokes produce precise call edges.
    #[test]
    fn test_call_graph_invoke_direct() {
        let test = TestModule::new(
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

        let mut analyses = test.module_analyses();
        let calls = analyses.call(&test.tree, &test.dispatch);

        let outgoing = calls.outgoing(test_id);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callee_id);
        assert_eq!(outgoing[0].dispatch, mir::CallDispatch::Direct);
        assert!(matches!(outgoing[0].callsite, mir::Point::Terminator(_)));
        assert!(calls.open_callsites(test_id).is_empty());
    }
}
