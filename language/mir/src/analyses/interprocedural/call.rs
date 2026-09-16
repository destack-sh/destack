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
    /// Recursive components in callee first order.
    components: CallComponentTable,
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

        self.components.component(function) as usize
    }

    /// Return true when a component is recursive.
    pub fn is_recursive_component(&self, component: usize) -> bool {
        self.components.recursive.contains(component)
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

        // collect outgoing edges and unresolved calls in one table
        let mut result = Self {
            functions,
            outgoing: Vec::new(),
            incoming: Vec::new(),
            open_callsites: Vec::new(),
            outgoing_offsets: Vec::new(),
            incoming_offsets: Vec::new(),
            open_offsets: Vec::new(),
            components: CallComponentTable::default(),
        };
        for (caller, function) in tree.iter_nodes::<mir::Function>() {
            for &block_id in function.blocks() {
                let block = tree.get(block_id);
                for &instruction in &block.instructions {
                    if let Some(dispatch) = tree.get(instruction).call_dispatch() {
                        let point = mir::Point::Instruction(instruction);
                        result.record_call(caller, point, dispatch, resolution.resolution(point));
                    }
                }

                // include invokes and tail calls at block exits
                if let Some(dispatch) = tree.get(block.terminator).call_dispatch() {
                    let point = mir::Point::Terminator(block_id);
                    result.record_call(caller, point, dispatch, resolution.resolution(point));
                }
            }
        }

        // group outgoing and incoming edges by their indexed functions
        result
            .outgoing
            .sort_unstable_by_key(|edge| (edge.caller, edge.callsite, edge.callee));
        result.incoming = result.outgoing.clone();
        result
            .incoming
            .sort_unstable_by_key(|edge| (edge.callee, edge.callsite, edge.caller));
        result
            .open_callsites
            .sort_unstable_by_key(|callsite| (callsite.caller, callsite.callsite));
        result.outgoing_offsets =
            Self::offsets(&result.functions, &result.outgoing, |edge| edge.caller);
        result.incoming_offsets =
            Self::offsets(&result.functions, &result.incoming, |edge| edge.callee);
        result.open_offsets =
            Self::offsets(&result.functions, &result.open_callsites, |callsite| {
                callsite.caller
            });

        // find recursive components using the outgoing edge ranges
        let targets = result
            .outgoing
            .iter()
            .map(|edge| result.function_index(edge.callee) as u32)
            .collect::<Vec<_>>();
        result.components = CallComponentTable::analyse(&result.outgoing_offsets, &targets);

        result
    }

    /// Record known callees and unresolved alternatives at one callsite.
    fn record_call(
        &mut self,
        caller: mir::FunctionId,
        callsite: mir::Point,
        dispatch: mir::CallDispatch,
        resolution: &mir::Resolution,
    ) {
        // add an edge for each known callee
        self.outgoing
            .extend(resolution.functions.iter().map(|&callee| CallEdge {
                caller,
                callee,
                callsite,
                dispatch,
            }));

        // retain calls that can select an additional target
        if resolution.is_open {
            self.open_callsites.push(OpenCallSite {
                caller,
                callsite,
                dispatch,
            });
        }
    }

    /// Iterate call components in callee first order.
    pub fn components(&self) -> impl Iterator<Item = impl Iterator<Item = mir::FunctionId> + '_> {
        self.components.components().map(|members| {
            members
                .iter()
                .map(|&function| self.functions[function as usize])
        })
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
pub struct CallComponentTable {
    /// The component id of each symbol, by dense id.
    component: Vec<u32>,
    /// Whether each component contains a cycle, by component id.
    recursive: BitSet,

    /// First member offset for each component and the final member count.
    offsets: Vec<u32>,
    /// Dense node indices grouped by component.
    nodes: Vec<u32>,
}

impl CallComponentTable {
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

        // group nodes by Tarjan's callee first component order
        let mut offsets = vec![0; count + 1];
        for component in 0..count {
            offsets[component + 1] = offsets[component] + sizes[component];
        }
        let mut cursors = offsets[..count].to_vec();
        let mut nodes = vec![0; graph.node_count()];
        for node in 0..graph.node_count() {
            let component = partition.component(node) as usize;
            nodes[cursors[component] as usize] = node as u32;
            cursors[component] += 1;
        }

        Self {
            component: partition.components().to_vec(),
            recursive,
            offsets,
            nodes,
        }
    }

    /// Return the dense function indices in one recursive component.
    pub fn members(&self, component: usize) -> &[u32] {
        let start = self.offsets[component] as usize;
        let end = self.offsets[component + 1] as usize;

        &self.nodes[start..end]
    }

    /// Iterate components in callee first order and their nodes in dense index order.
    pub fn components(&self) -> impl ExactSizeIterator<Item = &[u32]> {
        self.offsets
            .windows(2)
            .map(|range| &self.nodes[range[0] as usize..range[1] as usize])
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
        self.component.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::analyses::tests::TestModule;
    use crate::{CallDispatch, CallEdge, OpenCallSite, Point};

    /// Direct calls create edges in the call graph.
    #[test]
    fn test_record_direct_calls() {
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

        let entry = test.tree.get(test.tree.get(test_id).block(0));
        let edge = CallEdge {
            caller: test_id,
            callee: callee_id,
            callsite: Point::Instruction(entry.instructions[0]),
            dispatch: CallDispatch::Direct,
        };

        assert_eq!(calls.outgoing(test_id), &[edge]);
        assert_eq!(calls.incoming(callee_id), &[edge]);
        assert!(calls.open_callsites(test_id).is_empty());

        // visit the callee before its caller when neither function recurses
        let components = calls
            .components()
            .map(|members| members.collect::<Vec<_>>())
            .collect::<Vec<_>>();
        assert_eq!(components, vec![vec![callee_id], vec![test_id]]);
        assert!(!calls.components.is_empty());
        assert!(!calls.is_recursive_function(callee_id));
        assert!(!calls.is_recursive_function(test_id));
    }

    /// Call graph components detect recursive functions.
    #[test]
    fn test_identify_recursive_components() {
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

        let alpha = test.function_id_by_name("alpha");
        let beta = test.function_id_by_name("beta");
        let gamma = test.function_id_by_name("gamma");
        let delta = test.function_id_by_name("delta");

        let mut analyses = test.module_analyses();
        let calls = analyses.call(&test.tree, &test.dispatch);

        let components = calls
            .components()
            .map(|members| members.collect::<Vec<_>>())
            .collect::<Vec<_>>();
        assert_eq!(
            components,
            vec![vec![alpha, beta], vec![gamma], vec![delta]]
        );
        assert_eq!(
            [alpha, beta, gamma, delta].map(|function| calls.is_recursive_function(function)),
            [true, true, true, false]
        );
        assert_eq!(
            [alpha, beta, gamma, delta].map(|function| calls
                .outgoing(function)
                .iter()
                .map(|edge| edge.callee)
                .collect::<Vec<_>>()),
            [vec![beta], vec![alpha], vec![gamma], vec![]]
        );
    }

    /// Indirect calls without tables stay open.
    #[test]
    fn test_record_open_indirect_calls() {
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

        let entry = test.tree.get(test.tree.get(test_id).block(0));

        assert!(calls.outgoing(test_id).is_empty());
        assert_eq!(
            calls.open_callsites(test_id),
            &[OpenCallSite {
                caller: test_id,
                callsite: Point::Instruction(entry.instructions[0]),
                dispatch: CallDispatch::Indirect,
            }]
        );
    }

    /// Tail calls are tracked as call edges.
    #[test]
    fn test_record_direct_tail_calls() {
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

        let entry = test.tree.get(test_id).block(0);

        assert_eq!(
            calls.outgoing(test_id),
            &[CallEdge {
                caller: test_id,
                callee: callee_id,
                callsite: Point::Terminator(entry),
                dispatch: CallDispatch::Direct,
            }]
        );
    }

    /// Preserve unresolved targets for indirect tail calls.
    #[test]
    fn test_record_open_indirect_tail_calls() {
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

        let entry = test.tree.get(test_id).block(0);

        assert_eq!(
            calls.open_callsites(test_id),
            &[OpenCallSite {
                caller: test_id,
                callsite: Point::Terminator(entry),
                dispatch: CallDispatch::Indirect,
            }]
        );
    }

    /// Preserve unresolved targets beside unreferenced functions.
    #[test]
    fn test_preserve_open_calls_beside_unreferenced_functions() {
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

        let entry = test.tree.get(test.tree.get(test_id).block(0));

        assert_eq!(
            calls.open_callsites(test_id),
            &[OpenCallSite {
                caller: test_id,
                callsite: Point::Instruction(entry.instructions[0]),
                dispatch: CallDispatch::Indirect,
            }]
        );
        let callee = test.function_id_by_name("callee");
        assert_eq!(calls.outgoing(test_id), &[]);
        assert_eq!(calls.incoming(callee), &[]);
        assert_eq!(calls.outgoing(callee), &[]);
    }

    /// Direct invokes produce precise call edges.
    #[test]
    fn test_record_direct_invokes() {
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

        let entry = test.tree.get(test_id).block(0);

        assert_eq!(
            calls.outgoing(test_id),
            &[CallEdge {
                caller: test_id,
                callee: callee_id,
                callsite: Point::Terminator(entry),
                dispatch: CallDispatch::Direct,
            }]
        );
        assert!(calls.open_callsites(test_id).is_empty());
    }
}
