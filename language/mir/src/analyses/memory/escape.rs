use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_core::{BitSet, FxIndexMap};
use tspp_serde::Reflect;

use crate as mir;
use crate::{
    Analysis, ArgumentEscape, CallTable, ControlTable, EffectTable, Function, Mutation, NodeTable,
    Point, ResolutionTable, Symbol, Tree, Value,
};

/// Allocation visibility and pointer escape results for one function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EscapeTable {
    /// Parameter and result relationships consumed by callers.
    pub effect: EscapeEffect,
    /// Allocation results sorted by program point.
    allocations: Vec<(Point, AllocationEscape)>,
    /// Values whose referents may be accessible outside this frame.
    exposed: BitSet,
}

/// Visibility and reuse restrictions for one allocation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AllocationEscape {
    /// Whether the allocation is reachable through the returned value.
    pub is_returned: bool,
    /// Whether the allocation is reachable through external storage.
    pub is_retained: bool,
    /// Whether the allocation can remain accessible across loop iterations.
    pub is_persistent: bool,
}

/// Pointer flow from a function's inputs to its callers and external storage.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EscapeEffect {
    /// Escape paths for each explicit parameter.
    pub parameters: Vec<ParameterEscape>,
    /// Escape paths for the hidden environment, when the function has one.
    pub environment: Option<ParameterEscape>,
    /// Minimum loads from the result to an external pointer independent of the arguments.
    pub external: Option<u32>,
}

/// Pointer retention and return paths for one function argument.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ParameterEscape {
    /// Minimum dereferences before a pointer flows into externally retained storage.
    pub retained: Option<u32>,
    /// Minimum dereferences before a pointer flows into the returned value.
    pub returned: Option<u32>,
}

/// Pointer locations, transfers, and calls extracted from one function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EscapeBody {
    /// Number of SSA value locations at the start of the location table.
    value_count: u32,
    /// Loop depth at each location's definition.
    depths: Vec<u32>,
    /// Minimum external pointer depth permitted by each location's ownership type.
    exposure_depths: Vec<i32>,
    /// Whether each location starts with an external pointer.
    external: Vec<bool>,
    /// Locations that can retain values across iterations.
    roots: Vec<u32>,

    /// Explicit parameter locations in signature order.
    parameters: Vec<u32>,
    /// Hidden environment location, when present.
    environment: Option<u32>,
    /// Destination for externally retained pointers.
    retained: u32,
    /// Destination for the function's returned value.
    result: u32,
    /// Allocation operations, reference values, and storage locations in program point order.
    allocations: Vec<Allocation>,

    /// Local pointer transfers.
    flows: Vec<EscapeFlow>,
    /// Calls whose argument paths depend on another function.
    calls: Vec<EscapeCall>,
}

/// One allocation operation and the storage its result addresses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct Allocation {
    /// The allocation instruction or terminator.
    point: mir::Point,
    /// The reference receiving the new storage address.
    value: mir::Value,
    /// The storage location in the pointer graph.
    location: u32,
}

/// Arguments and result locations for one call's pointer flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct EscapeCall {
    /// Known callees in symbol order.
    targets: Vec<mir::Symbol>,
    /// Whether additional callees remain possible.
    is_open: bool,
    /// Caller locations passed as explicit arguments.
    arguments: Vec<u32>,
    /// Caller location containing the hidden environment.
    environment: Option<u32>,
    /// Caller location receiving the result, absent for a discarded result.
    result: Option<u32>,
    /// Explicit argument escape declarations, when present.
    arguments_declared: Vec<mir::CallArgumentEffect>,
}

/// One pointer flow with its load count, or minus one for an address operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct EscapeFlow {
    /// The location supplying the value.
    source: u32,
    /// The location receiving the value.
    destination: u32,
    /// The number of loads applied to the source, or minus one to take its address.
    dereferences: i32,
}

/// Extract pointer transfers using canonical places and control flow.
struct EscapeBuilder<'a> {
    /// The MIR tree.
    tree: &'a mir::Tree,
    /// Callees at each callsite.
    resolution: &'a mir::ResolutionTable,
    /// Explicit call argument effects.
    effects: &'a mir::EffectTable,
    /// Canonical addresses in this function.
    places: mir::PlaceTable,
    /// Storage locations for function locals.
    locals: mir::NodeTable<mir::Local, u32>,
    /// Pointer transfers being collected.
    body: EscapeBody,
}

/// Reusable shortest path storage for one weighted pointer graph.
struct EscapeSolver<'a> {
    /// The extracted function locations and flows.
    body: &'a EscapeBody,
    /// First incoming flow offset for each location and the final flow count.
    offsets: Vec<usize>,
    /// Incoming flows grouped by destination.
    flows: Vec<EscapeFlow>,

    /// Whether external storage can retain the address of each location.
    retained: Vec<bool>,
    /// Whether the result can contain the address of each location.
    returned: Vec<bool>,
    /// Whether each location can remain accessible across iterations.
    persistent: Vec<bool>,
    /// Minimum load count for the current root's paths.
    distances: Vec<Option<i32>>,
    /// Locations visited by the current walk.
    visited: Vec<u32>,
    /// Locations waiting for a shorter path to propagate.
    pending: Vec<u32>,
    /// Locations already in the current path worklist.
    queued: BitSet,
}

impl EscapeBody {
    /// Analyse pointer flow through each recursive component in callee first order.
    pub(crate) fn analyse_module(
        resolution: &ResolutionTable,
        calls: &CallTable,
        declared: &EffectTable,
        tree: &Tree,
    ) -> NodeTable<Function, Arc<EscapeTable>> {
        let mut bodies = FxIndexMap::default();
        let mut effects = FxIndexMap::<Symbol, EscapeEffect>::default();
        let mut results = FxIndexMap::default();

        // extract definitions and declarations through the same pointer graph model
        let functions = tree
            .iter_nodes::<Function>()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        for id in functions {
            let symbol = tree.get(id).symbol;
            let graph = Arc::new(ControlTable::analyse(tree.get(id), tree));
            let body = EscapeBody::analyse(id, graph, resolution, declared, tree);
            effects.insert(symbol, body.initial_effect());
            bodies.insert(id, body);
        }

        // propagate parameter paths without rescanning MIR instructions
        for (component_index, component) in calls.components().enumerate() {
            let functions = component.collect::<Vec<_>>();
            loop {
                let mut changed = false;
                for &function in &functions {
                    let result = bodies[&function].analyse_calls(&effects);
                    let symbol = tree.get(function).symbol;
                    if effects[&symbol] != result.effect {
                        effects.insert(symbol, result.effect.clone());
                        changed = true;
                    }
                    if tree.get(function).is_defined() {
                        results.insert(function, result);
                    }
                }
                if !changed || !calls.is_recursive_component(component_index) {
                    break;
                }
            }
        }

        NodeTable::from_entries(
            results
                .into_iter()
                .map(|(function, result)| (function, Arc::new(result)))
                .collect(),
        )
    }
}

impl Analysis for EscapeTable {
    const INVALIDATED_BY: Mutation = ResolutionTable::INVALIDATED_BY.union(Mutation::EFFECT);
}

impl From<ArgumentEscape> for ParameterEscape {
    fn from(escape: ArgumentEscape) -> Self {
        match escape {
            ArgumentEscape::None => Self::default(),
            ArgumentEscape::Return => Self {
                retained: None,
                returned: Some(0),
            },
            ArgumentEscape::Escape => Self {
                retained: Some(0),
                returned: Some(0),
            },
        }
    }
}

impl EscapeTable {
    /// Return visibility and reuse restrictions for one allocation operation.
    pub fn allocation(&self, point: Point) -> Option<&AllocationEscape> {
        let index = self
            .allocations
            .binary_search_by_key(&point, |(point, _)| *point)
            .ok()?;

        Some(&self.allocations[index].1)
    }

    /// Return whether another frame or external storage may access this value's referent.
    pub fn is_exposed(&self, value: Value) -> bool {
        self.exposed.contains(value.id() as usize)
    }
}

impl EscapeBody {
    /// Extract pointer flow from a defined function's operations and exact control flow edges.
    pub fn analyse(
        function: mir::FunctionId,
        graph: Arc<mir::ControlTable>,
        resolution: &mir::ResolutionTable,
        effects: &mir::EffectTable,
        tree: &mir::Tree,
    ) -> Self {
        // describe externally defined functions through their declarations
        if !tree.get(function).is_defined() {
            return Self::declaration(function, tree);
        }

        // build the control and place tables required by storage flow
        let dominators = mir::DominatorTable::analyse(graph.clone());
        let loops = mir::LoopTable::analyse(tree.get(function), &graph, &dominators);
        let places = mir::PlaceTable::analyse(function, &graph, tree);
        let count = tree.get(function).value_capacity() as u32;
        let mut body = Self {
            exposure_depths: (0..count)
                .map(|value| {
                    i32::from(
                        {
                            let ty = tree.get(function).expect_value_type(mir::Value(value));
                            let ty = mir::Substitution::resolve(ty, tree);
                            tree.get(ty).reference_kind()
                        } == Some(mir::Reference::Unique),
                    )
                })
                .collect(),
            value_count: count,
            depths: vec![0; count as usize],
            external: vec![false; count as usize],
            roots: Vec::new(),
            parameters: tree
                .get(function)
                .parameters
                .iter()
                .map(|parameter| parameter.value.id())
                .collect(),
            environment: None,
            retained: count,
            result: count + 1,
            allocations: Vec::new(),
            flows: Vec::new(),
            calls: Vec::new(),
        };

        // create distinct destinations for returned and externally retained pointers
        body.location(0, false);
        body.location(0, false);
        body.exposure_depths[body.result as usize] = i32::from(
            tree.get(tree.get(function).return_type).reference_kind()
                == Some(mir::Reference::Unique),
        );
        body.roots.extend([body.retained, body.result]);
        if tree.get(function).environment.is_some() {
            body.environment = Some(body.location(0, false));
        }
        let locals = mir::NodeTable::from_entries(
            tree.get(function)
                .locals()
                .iter()
                .map(|&local| {
                    let location = body.location(0, false);
                    body.roots.push(location);
                    (local, location)
                })
                .collect(),
        );

        // compare allocation depths with locals and parameters that span iterations
        for block_id in graph.reachable_blocks() {
            let block = tree.get(block_id);
            let is_irreducible = loops.is_irreducible(block_id);
            let depth = loops.loop_depth(block_id) + u32::from(is_irreducible);

            // retain block arguments across each possible cycle entry
            let parameter_depth = if loops.is_loop_header(block_id) || is_irreducible {
                depth - 1
            } else {
                depth
            };
            for parameter in &block.parameters {
                body.depths[parameter.value.id() as usize] = parameter_depth;
                body.roots.push(parameter.value.id());
            }

            // assign storage to allocation instructions at this loop depth
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                if let Some(destination) = instruction.destination() {
                    body.depths[destination.id() as usize] = depth;
                    if matches!(
                        instruction,
                        mir::Instruction::NewZeroed { .. }
                            | mir::Instruction::NewUninit { .. }
                            | mir::Instruction::NewSliceZeroed { .. }
                            | mir::Instruction::NewSliceUninit { .. }
                            | mir::Instruction::ContextBind { .. }
                    ) {
                        body.allocate(mir::Point::Instruction(instruction_id), destination, depth);
                    }
                }
            }

            // assign storage to fallible allocation results on their success edges
            let terminator = tree.get(block.terminator);
            for (edge, target) in terminator.targets(tree, block_id) {
                if edge.successor == mir::Successor::NewSuccess {
                    let result = tree.get(target.block).parameters[0].value;
                    body.allocate(mir::Point::Terminator(block_id), result, depth);
                }
            }
        }

        // index allocation operations independently of traversal order
        body.allocations
            .sort_unstable_by_key(|allocation| allocation.point);

        // translate operations into pointer transfers once before solving callees
        let mut builder = EscapeBuilder {
            tree,
            resolution,
            effects,
            places,
            locals,
            body,
        };
        for block in graph.reachable_blocks() {
            builder.block(block);
        }
        builder
            .body
            .flows
            .sort_unstable_by_key(|flow| (flow.destination, flow.source, flow.dereferences));
        builder.body.flows.dedup();

        builder.body
    }

    /// Return the initial parameter effects before recursive propagation.
    pub fn initial_effect(&self) -> EscapeEffect {
        EscapeEffect {
            parameters: vec![ParameterEscape::default(); self.parameters.len()],
            environment: self.environment.map(|_| ParameterEscape::default()),
            external: None,
        }
    }

    /// Describe a declaration using conservative parameter retention and external results.
    fn declaration(function: mir::FunctionId, tree: &mir::Tree) -> Self {
        // allocate parameter, result, and external retention locations
        let count = tree.get(function).parameters.len() as u32;
        let environment = tree.get(function).environment.map(|_| count + 2);
        let total = count as usize + 2 + usize::from(environment.is_some());
        let mut body = Self {
            value_count: 0,
            depths: vec![0; total],
            exposure_depths: vec![0; total],
            external: vec![false; total],
            roots: vec![count, count + 1],
            parameters: (0..count).collect(),
            environment,
            retained: count,
            result: count + 1,
            allocations: Vec::new(),
            flows: Vec::new(),
            calls: Vec::new(),
        };

        // preserve ownership on returned references while allowing external pointers in contents
        body.external[body.result as usize] = true;
        body.exposure_depths[body.result as usize] = i32::from(
            tree.get(tree.get(function).return_type).reference_kind()
                == Some(mir::Reference::Unique),
        );
        for parameter in body.parameters.iter().copied().chain(body.environment) {
            body.flows.push(EscapeFlow {
                source: parameter,
                destination: body.retained,
                dereferences: 0,
            });
            body.flows.push(EscapeFlow {
                source: parameter,
                destination: body.result,
                dereferences: 0,
            });
        }

        body
    }

    /// Append a storage location with its execution depth and initial visibility.
    fn location(&mut self, depth: u32, is_external: bool) -> u32 {
        let location = self.depths.len() as u32;
        self.depths.push(depth);
        self.external.push(is_external);
        self.exposure_depths.push(0);

        location
    }

    /// Record an allocation and the reference to its newly created storage.
    fn allocate(&mut self, point: mir::Point, value: mir::Value, depth: u32) {
        let location = self.location(depth, false);
        self.allocations.push(Allocation {
            point,
            value,
            location,
        });
        self.flows.push(EscapeFlow {
            source: location,
            destination: value.id(),
            dereferences: -1,
        });
    }

    /// Solve pointer flows using the current effects of all named callees.
    pub fn analyse_calls(&self, effects: &FxIndexMap<Symbol, EscapeEffect>) -> EscapeTable {
        // copy local flows and initial pointer visibility
        let count = self.depths.len();
        let mut flows = self.flows.clone();
        let mut external = self
            .external
            .iter()
            .map(|&is_external| is_external.then_some(0))
            .collect::<Vec<_>>();

        // instantiate each callee's pointer paths at its argument and result locations
        for call in &self.calls {
            call.connect(self, effects, &mut flows, &mut external);
        }
        flows.sort_unstable_by_key(|flow| (flow.destination, flow.source, flow.dereferences));
        flows.dedup();
        let mut offsets = vec![0; count + 1];
        for flow in &flows {
            offsets[flow.destination as usize + 1] += 1;
        }
        for index in 0..count {
            offsets[index + 1] += offsets[index];
        }

        // initialize the two externally observed destinations
        let mut retained = vec![false; count];
        let mut returned = vec![false; count];
        retained[self.retained as usize] = true;
        returned[self.result as usize] = true;
        let mut solver = EscapeSolver {
            body: self,
            offsets,
            flows,
            retained,
            returned,
            persistent: vec![false; count],
            distances: vec![None; count],
            visited: Vec::new(),
            pending: Vec::new(),
            queued: BitSet::new(count),
        };
        solver.propagate();

        // record parameter paths to returned and externally retained storage
        let mut parameters = self.parameters.clone();
        parameters.extend(self.environment);
        let mut paths = solver.parameters(&parameters);
        let environment = self.environment.map(|_| {
            paths
                .pop()
                .unwrap_or_else(|| unreachable!("missing environment paths"))
        });

        // derive external result pointers before adding parameter aliases
        let (external, exposed) = solver.analyse_visibility(&external);
        let effect = EscapeEffect {
            parameters: paths,
            environment,
            external,
        };

        // retain allocation identity independently of the references that can select it
        let allocations = self
            .allocations
            .iter()
            .map(|allocation| {
                let index = allocation.location as usize;
                (
                    allocation.point,
                    AllocationEscape {
                        is_returned: solver.returned[index],
                        is_retained: solver.retained[index],
                        is_persistent: solver.persistent[index],
                    },
                )
            })
            .collect();

        EscapeTable {
            effect,
            allocations,
            exposed,
        }
    }
}

impl EscapeSolver<'_> {
    /// Propagate address visibility and persistence until no location changes.
    fn propagate(&mut self) {
        // schedule the storage roots that can retain pointers
        let count = self.body.depths.len();
        let mut pending = self.body.roots.clone();
        pending.sort_unstable();
        pending.dedup();
        let mut queued = vec![false; count];
        for &root in &pending {
            queued[root as usize] = true;
        }

        // revisit a root only when its visibility or persistence changes
        while let Some(root) = pending.pop() {
            queued[root as usize] = false;
            self.walk(root);
            for &location in &self.visited {
                let location = location as usize;
                let distance = self.distances[location]
                    .unwrap_or_else(|| unreachable!("visited location has no distance"));
                if distance >= 0 {
                    continue;
                }
                let retained = self.retained[root as usize];
                let returned = self.returned[root as usize];
                let persistent = self.persistent[root as usize]
                    || self.body.depths[root as usize] < self.body.depths[location];
                let changed = (retained && !self.retained[location])
                    || (returned && !self.returned[location])
                    || (persistent && !self.persistent[location]);
                self.retained[location] |= retained;
                self.returned[location] |= returned;
                self.persistent[location] |= persistent;

                // propagate newly visible addresses through the pointers their storage contains
                if changed && !queued[location] {
                    pending.push(location as u32);
                    queued[location] = true;
                }
            }
        }
    }

    /// Find minimum dereference counts while preventing negative address cycles.
    fn walk(&mut self, root: u32) {
        // clear only the locations touched by the previous walk
        for location in self.visited.drain(..) {
            self.distances[location as usize] = None;
        }

        // start the next walk at its root
        self.visited.push(root);
        self.distances[root as usize] = Some(0);
        self.pending.push(root);
        self.queued.insert(root as usize);

        // relax incoming flows, clamping after an address has reached a location
        while let Some(location) = self.pending.pop() {
            let location = location as usize;
            self.queued.remove(location);
            let distance = self.distances[location]
                .unwrap_or_else(|| unreachable!("queued escape location has no distance"));
            let distance = distance.max(0);
            for flow in &self.flows[self.offsets[location]..self.offsets[location + 1]] {
                let source = flow.source as usize;
                let candidate = distance + flow.dereferences;
                if self.distances[source].is_none_or(|current| candidate < current) {
                    if self.distances[source].is_none() {
                        self.visited.push(flow.source);
                    }
                    self.distances[source] = Some(candidate);
                    if self.queued.insert(source) {
                        self.pending.push(flow.source);
                    }
                }
            }
        }
    }

    /// Analyse external result pointers and values exposed through parameter aliases.
    fn analyse_visibility(&self, external: &[Option<i32>]) -> (Option<u32>, BitSet) {
        // index outgoing flows once for both visibility queries
        let count = self.body.depths.len();
        let mut outgoing = (0..self.flows.len()).collect::<Vec<_>>();
        outgoing.sort_unstable_by_key(|&index| self.flows[index].source);
        let mut offsets = vec![0; count + 1];
        for flow in &self.flows {
            offsets[flow.source as usize + 1] += 1;
        }
        for index in 0..count {
            offsets[index + 1] += offsets[index];
        }

        // seed external pointers and addresses of retained storage
        let mut depths = external
            .iter()
            .zip(&self.body.exposure_depths)
            .zip(&self.retained)
            .map(|((depth, minimum), &is_retained)| {
                if is_retained {
                    Some(-1)
                } else {
                    depth.map(|depth| depth.max(*minimum))
                }
            })
            .collect::<Vec<_>>();
        self.propagate_visibility(&mut depths, &offsets, &outgoing);
        let external = depths[self.body.result as usize].map(|depth| depth.max(0) as u32);

        // add parameter aliases to the established external paths
        let parameters = self
            .body
            .parameters
            .iter()
            .map(|&parameter| (parameter, self.body.exposure_depths[parameter as usize]))
            .chain(self.body.environment.map(|environment| (environment, 0)));
        for (parameter, depth) in parameters {
            let current = &mut depths[parameter as usize];
            *current = Some(current.map_or(depth, |current| current.min(depth)));
        }
        self.propagate_visibility(&mut depths, &offsets, &outgoing);

        // retain only the exposed SSA values
        let mut exposed = BitSet::new(self.body.value_count as usize);
        for (value, depth) in depths[..self.body.value_count as usize].iter().enumerate() {
            if depth.is_some_and(|depth| depth <= 0) {
                exposed.insert(value);
            }
        }

        (external, exposed)
    }

    /// Propagate minimum visibility depths through outgoing address and load operations.
    fn propagate_visibility(
        &self,
        depths: &mut [Option<i32>],
        offsets: &[usize],
        outgoing: &[usize],
    ) {
        // schedule each externally visible location
        let mut pending = (0..depths.len())
            .filter(|&index| depths[index].is_some())
            .collect::<Vec<_>>();
        let mut queued = BitSet::new(depths.len());
        for &index in &pending {
            queued.insert(index);
        }

        // propagate visibility through each address or load operation
        while let Some(source) = pending.pop() {
            queued.remove(source);
            let depth = depths[source]
                .unwrap_or_else(|| unreachable!("queued location has no visibility depth"));
            for &index in &outgoing[offsets[source]..offsets[source + 1]] {
                let flow = self.flows[index];
                let destination = flow.destination as usize;
                let candidate = (depth - flow.dereferences).max(-1);
                if depths[destination].is_none_or(|current| candidate < current) {
                    depths[destination] = Some(candidate);
                    if queued.insert(destination) {
                        pending.push(destination);
                    }
                }
            }
        }
    }

    /// Record every parameter's shortest paths to retained storage and the result.
    fn parameters(&mut self, parameters: &[u32]) -> Vec<ParameterEscape> {
        let mut results = vec![ParameterEscape::default(); parameters.len()];

        // walk each visible storage location once for all parameters
        for root in 0..self.body.depths.len() {
            if !self.retained[root] && !self.returned[root] {
                continue;
            }
            self.walk(root as u32);
            for (&parameter, result) in parameters.iter().zip(&mut results) {
                if let Some(distance) = self.distances[parameter as usize] {
                    let distance = distance.max(0) as u32;
                    if self.retained[root] {
                        result.retained = Some(
                            result
                                .retained
                                .map_or(distance, |current| current.min(distance)),
                        );
                    }
                    if self.returned[root] {
                        result.returned = Some(
                            result
                                .returned
                                .map_or(distance, |current| current.min(distance)),
                        );
                    }
                }
            }
        }

        results
    }
}

impl EscapeCall {
    /// Connect every possible callee's parameter paths to one caller's graph.
    fn connect(
        &self,
        body: &EscapeBody,
        effects: &FxIndexMap<Symbol, EscapeEffect>,
        flows: &mut Vec<EscapeFlow>,
        external: &mut [Option<i32>],
    ) {
        // preserve a conservative result for unresolved callees
        if self.is_open {
            self.connect_unknown(body, flows, external);
        }

        // instantiate effects from each named callee
        for target in &self.targets {
            let effect = effects
                .get(target)
                .unwrap_or_else(|| unreachable!("callee outside escape graph: {target:?}"));
            for (index, &argument) in self.arguments.iter().enumerate() {
                let parameter = self
                    .arguments_declared
                    .get(index)
                    .map(|argument| ParameterEscape::from(argument.escape));
                let parameter = parameter.as_ref().unwrap_or(&effect.parameters[index]);
                self.connect_parameter(argument, parameter, body, flows);
            }
            if let Some(environment) = &effect.environment {
                let argument = self
                    .environment
                    .unwrap_or_else(|| unreachable!("closure call has no environment"));
                self.connect_parameter(argument, environment, body, flows);
            }
            if let (Some(result), Some(depth)) = (self.result, effect.external) {
                let current = &mut external[result as usize];
                let depth = depth as i32;
                *current = Some(current.map_or(depth, |current| current.min(depth)));
            }
        }
    }

    /// Connect explicit argument declarations or conservative unknown call effects.
    fn connect_unknown(
        &self,
        body: &EscapeBody,
        flows: &mut Vec<EscapeFlow>,
        external: &mut [Option<i32>],
    ) {
        for (index, &argument) in self.arguments.iter().enumerate() {
            let escape = self
                .arguments_declared
                .get(index)
                .map(|argument| argument.escape)
                .unwrap_or(ArgumentEscape::Escape);
            let parameter = ParameterEscape::from(escape);
            self.connect_parameter(argument, &parameter, body, flows);
        }
        if let Some(environment) = self.environment {
            flows.push(EscapeFlow {
                source: environment,
                destination: body.retained,
                dereferences: 0,
            });
        }
        if let Some(result) = self.result {
            external[result as usize] = Some(0);
        }
    }

    /// Connect one parameter's load counts to the caller's retained and returned destinations.
    fn connect_parameter(
        &self,
        argument: u32,
        parameter: &ParameterEscape,
        body: &EscapeBody,
        flows: &mut Vec<EscapeFlow>,
    ) {
        if let Some(dereferences) = parameter.retained {
            flows.push(EscapeFlow {
                source: argument,
                destination: body.retained,
                dereferences: dereferences as i32,
            });
        }
        if let (Some(dereferences), Some(destination)) = (parameter.returned, self.result) {
            flows.push(EscapeFlow {
                source: argument,
                destination,
                dereferences: dereferences as i32,
            });
        }
    }
}

impl EscapeBuilder<'_> {
    /// Record one block's pointer transfers and incoming argument assignments.
    fn block(&mut self, block_id: mir::BlockId) {
        let block = self.tree.get(block_id);
        for &instruction_id in &block.instructions {
            self.instruction(instruction_id, self.tree.get(instruction_id));
        }

        // connect every explicit successor argument to the matching parameter
        let terminator = self.tree.get(block.terminator);
        for (edge, target) in terminator.targets(self.tree, block_id) {
            let parameters = terminator
                .target_parameters(self.tree, edge.successor, target)
                .unwrap_or_else(|| unreachable!("verified edge has invalid argument count"));
            for (parameter, &argument) in parameters.iter().zip(target.arguments(self.tree)) {
                self.flow(argument.id(), parameter.value.id(), 0);
            }
        }

        // route returned pointers and call results to their distinct destinations
        match terminator {
            mir::Terminator::Return { value } => {
                if let Some(value) = value {
                    self.flow(value.id(), self.body.result, 0);
                }
            }
            mir::Terminator::Panic { payload } => {
                if let Some(payload) = payload {
                    self.flow(payload.id(), self.body.retained, 0);
                }
            }
            mir::Terminator::Invoke { call, target, .. } => {
                let count = terminator.target_result_count(self.tree, mir::Successor::InvokeNormal);
                let result =
                    (count != 0).then(|| self.tree.get(target.block).parameters[0].value.id());
                self.call(mir::Point::Terminator(block_id), call, result);
            }
            mir::Terminator::TailCall { call } => self.call(
                mir::Point::Terminator(block_id),
                call,
                Some(self.body.result),
            ),
            mir::Terminator::Error => unreachable!("recovered terminator reached escape analysis"),
            mir::Terminator::Jump { .. }
            | mir::Terminator::Branch { .. }
            | mir::Terminator::Check { .. }
            | mir::Terminator::Switch { .. }
            | mir::Terminator::VariantSwitch { .. }
            | mir::Terminator::NewZeroedTry { .. }
            | mir::Terminator::NewUninitTry { .. }
            | mir::Terminator::NewSliceZeroedTry { .. }
            | mir::Terminator::NewSliceUninitTry { .. }
            | mir::Terminator::UnwindResume
            | mir::Terminator::Abort { .. }
            | mir::Terminator::Unreachable => {}
        }
    }

    /// Translate an instruction's pointer transfers and storage accesses.
    fn instruction(
        &mut self,
        id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
    ) {
        match instruction {
            mir::Instruction::Copy { destination, value } => {
                self.flow(value.id(), destination.id(), 0)
            }
            mir::Instruction::Address {
                destination, place, ..
            } => {
                self.read(place, *destination, -1);
            }
            mir::Instruction::ContextCurrent { destination }
            | mir::Instruction::DynamicFind { destination, .. } => {
                self.body.external[destination.id() as usize] = true;
            }
            mir::Instruction::FunctionEnvironmentCurrent { destination } => {
                let environment = self.body.environment.unwrap_or_else(|| {
                    unreachable!("environment instruction in a function without an environment")
                });
                self.flow(environment, destination.id(), 0);
            }
            mir::Instruction::Load {
                destination, place, ..
            }
            | mir::Instruction::AtomicLoad {
                destination, place, ..
            } => self.read(place, *destination, 0),
            mir::Instruction::Store { place, value }
            | mir::Instruction::AtomicStore { place, value, .. } => self.store(place, *value, 0),
            mir::Instruction::AtomicRmw {
                destination,
                place,
                value,
                ..
            } => {
                self.store(place, *value, 0);
                self.read(place, *destination, 0);
            }
            mir::Instruction::AtomicCompareExchange {
                destination,
                place,
                new_value,
                ..
            } => {
                self.store(place, *new_value, 0);
                self.read(place, *destination, 0);
            }
            mir::Instruction::Call { destination, call } => self.call(
                mir::Point::Instruction(id),
                call,
                destination.map(|value| value.id()),
            ),
            mir::Instruction::Drop { value } => self.flow(value.id(), self.body.retained, 0),
            mir::Instruction::ContextReplace {
                destination,
                context,
            } => {
                self.flow(context.id(), self.body.retained, 0);
                self.body.external[destination.id() as usize] = true;
            }
            mir::Instruction::ContextBind {
                destination,
                context,
                variable,
                value,
                ..
            } => {
                self.store(
                    &mir::Place::value(*destination).with_projection(mir::Projection::Deref),
                    *context,
                    0,
                );
                self.store(
                    &mir::Place::value(*destination).with_projection(mir::Projection::Deref),
                    *variable,
                    0,
                );
                self.store(
                    &mir::Place::value(*destination).with_projection(mir::Projection::Deref),
                    *value,
                    0,
                );
            }
            mir::Instruction::ContextGet {
                destination,
                context,
                ..
            } => self.flow(context.id(), destination.id(), 1),
            mir::Instruction::DynamicRead {
                destination,
                dynamic,
                ..
            } => self.flow(dynamic.id(), destination.id(), 1),
            mir::Instruction::FunctionBind {
                destination,
                environment,
                ..
            } => self.flow(environment.id(), destination.id(), 0),
            mir::Instruction::FunctionEnvironment {
                destination,
                function,
            } => self.flow(function.id(), destination.id(), 0),
            mir::Instruction::Cast {
                destination,
                argument,
                ..
            }
            | mir::Instruction::NewComplete {
                destination,
                value: argument,
                ..
            } => self.flow(argument.id(), destination.id(), 0),
            mir::Instruction::FieldGet {
                destination,
                aggregate,
                ..
            }
            | mir::Instruction::ElementGet {
                destination,
                aggregate,
                ..
            } => self.flow(aggregate.id(), destination.id(), 0),
            mir::Instruction::VariantPayload {
                destination,
                variant,
                ..
            } => self.flow(variant.id(), destination.id(), 0),
            mir::Instruction::DynamicBind {
                destination,
                payload,
                ..
            } => self.flow(payload.id(), destination.id(), 0),
            mir::Instruction::DynamicPayload {
                destination,
                dynamic,
                ..
            } => self.flow(dynamic.id(), destination.id(), 0),
            mir::Instruction::Select {
                destination,
                then_value,
                else_value,
                ..
            } => {
                self.flow(then_value.id(), destination.id(), 0);
                self.flow(else_value.id(), destination.id(), 0);
            }
            mir::Instruction::Aggregate {
                destination,
                values,
            } => {
                for value in self.tree.get_values(*values) {
                    self.flow(value.id(), destination.id(), 0);
                }
            }
            mir::Instruction::FieldSet {
                destination,
                aggregate,
                value,
                ..
            }
            | mir::Instruction::ElementSet {
                destination,
                aggregate,
                value,
                ..
            } => {
                self.flow(aggregate.id(), destination.id(), 0);
                self.flow(value.id(), destination.id(), 0);
            }
            mir::Instruction::VariantNew {
                destination,
                payload,
                ..
            } => {
                if let Some(payload) = payload {
                    self.flow(payload.id(), destination.id(), 0);
                }
            }
            mir::Instruction::Intrinsic {
                destination,
                intrinsic,
                arguments,
                ..
            } => self.intrinsic(*destination, *intrinsic, self.tree.get_values(*arguments)),
            mir::Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } if !operator.is_comparison() => {
                self.flow(left.id(), destination.id(), 0);
                self.flow(right.id(), destination.id(), 0);
            }
            mir::Instruction::Unary {
                destination,
                argument,
                ..
            } => self.flow(argument.id(), destination.id(), 0),
            mir::Instruction::VectorSplat { destination, value } => {
                self.flow(value.id(), destination.id(), 0)
            }
            mir::Instruction::VectorExtract {
                destination,
                vector,
                ..
            }
            | mir::Instruction::VectorReduce {
                destination,
                vector,
                ..
            }
            | mir::Instruction::VectorConvert {
                destination,
                vector,
                ..
            } => self.flow(vector.id(), destination.id(), 0),
            mir::Instruction::VectorInsert {
                destination,
                vector,
                value,
                ..
            } => {
                self.flow(vector.id(), destination.id(), 0);
                self.flow(value.id(), destination.id(), 0);
            }
            mir::Instruction::VectorShuffle {
                destination,
                left,
                right,
                ..
            } => {
                self.flow(left.id(), destination.id(), 0);
                self.flow(right.id(), destination.id(), 0);
            }
            mir::Instruction::VectorSelect {
                destination,
                then_value,
                else_value,
                ..
            } => {
                self.flow(then_value.id(), destination.id(), 0);
                self.flow(else_value.id(), destination.id(), 0);
            }
            mir::Instruction::Error => {
                unreachable!("recovered instruction reached escape analysis")
            }
            mir::Instruction::Const { .. }
            | mir::Instruction::Binary { .. }
            | mir::Instruction::FunctionAddr { .. }
            | mir::Instruction::VariantTag { .. }
            | mir::Instruction::VariantTagLoad { .. }
            | mir::Instruction::SliceLength { .. }
            | mir::Instruction::DynamicType { .. }
            | mir::Instruction::VectorCompare { .. }
            | mir::Instruction::NewZeroed { .. }
            | mir::Instruction::NewUninit { .. }
            | mir::Instruction::NewSliceZeroed { .. }
            | mir::Instruction::NewSliceUninit { .. }
            | mir::Instruction::Release { .. }
            | mir::Instruction::BarrierWrite { .. }
            | mir::Instruction::AtomicFence { .. }
            | mir::Instruction::Assume { .. }
            | mir::Instruction::ProfileIncrement { .. }
            | mir::Instruction::ProfileSample { .. }
            | mir::Instruction::Poll
            | mir::Instruction::Breakpoint => {}
        }
    }

    /// Record pointer transfers performed by a compiler intrinsic.
    fn intrinsic(
        &mut self,
        destination: Option<mir::Value>,
        intrinsic: mir::Intrinsic,
        arguments: &[mir::Value],
    ) {
        match intrinsic {
            mir::Intrinsic::Memcpy | mir::Intrinsic::Memmove => self.store(
                &mir::Place::value(arguments[0]).with_projection(mir::Projection::Deref),
                arguments[1],
                1,
            ),
            mir::Intrinsic::VolatileStore => self.store(
                &mir::Place::value(arguments[0]).with_projection(mir::Projection::Deref),
                arguments[1],
                0,
            ),
            mir::Intrinsic::VolatileLoad => {
                if let Some(destination) = destination {
                    self.flow(arguments[0].id(), destination.id(), 1);
                }
            }
            mir::Intrinsic::Transmute
            | mir::Intrinsic::SpaceCast
            | mir::Intrinsic::BlackBox
            | mir::Intrinsic::Expect => {
                if let Some(destination) = destination {
                    self.flow(arguments[0].id(), destination.id(), 0);
                }
            }
            _ => {
                // preserve pointer bits transformed by arithmetic and representation intrinsics
                if let Some(destination) = destination {
                    for argument in arguments {
                        self.flow(argument.id(), destination.id(), 0);
                    }
                }
            }
        }
    }

    /// Record a place read, subtracting one dereference when taking its address.
    fn read(&mut self, place: &mir::Place, destination: mir::Value, adjustment: i32) {
        let source = match place.origin {
            mir::PlaceOrigin::Value(value) => value.id(),
            mir::PlaceOrigin::Local(local) => *self.locals.get(local),
            mir::PlaceOrigin::Global(_) => {
                self.body.external[destination.id() as usize] = true;
                return;
            }
        };
        let dereferences = place
            .path
            .projections
            .iter()
            .filter(|projection| **projection == mir::Projection::Deref)
            .count() as i32;

        self.flow(source, destination.id(), dereferences + adjustment);
    }

    /// Store a pointer into a known local allocation or externally accessible storage.
    fn store(&mut self, place: &mir::Place, value: mir::Value, dereferences: i32) {
        // resolve the storage receiving the pointer
        let place = self.places.resolve_place(place);
        let destination = match place.origin {
            mir::PlaceOrigin::Local(local)
                if !place.path.projections.contains(&mir::Projection::Deref) =>
            {
                *self.locals.get(local)
            }
            mir::PlaceOrigin::Value(root)
                if place.path.first() == Some(&mir::Projection::Deref)
                    && !place.path.projections[1..].contains(&mir::Projection::Deref) =>
            {
                // select instruction allocations with an unambiguous result
                match self.body.allocations.iter().find(|allocation| {
                    matches!(allocation.point, mir::Point::Instruction(_))
                        && allocation.value == root
                }) {
                    Some(allocation) => allocation.location,
                    None => self.body.retained,
                }
            }
            mir::PlaceOrigin::Local(_)
            | mir::PlaceOrigin::Value(_)
            | mir::PlaceOrigin::Global(_) => self.body.retained,
        };

        self.flow(value.id(), destination, dereferences);
    }

    /// Record arguments, environment, and return storage for one callsite.
    fn call(&mut self, point: mir::Point, call: &mir::Call, result: Option<u32>) {
        // translate possible callees to persistent symbols
        let resolution = self.resolution.resolution(point);
        let mut targets = resolution
            .functions
            .iter()
            .map(|&function| self.tree.get(function).symbol)
            .collect::<Vec<_>>();
        targets.sort_unstable();
        targets.dedup();

        // record the hidden environment supplied by indirect calls
        let environment = match call.callee {
            mir::Callee::Indirect { value } => Some(value.id()),
            _ => None,
        };

        // apply explicit capture declarations to bodyless callees
        let is_bodyless = resolution
            .functions
            .iter()
            .all(|&function| !self.tree.get(function).is_defined());
        let arguments_declared = self
            .effects
            .call(point)
            .filter(|_| is_bodyless)
            .map(|call| call.arguments.clone())
            .unwrap_or_default();

        // retain the arguments and destinations used when solving callees
        self.body.calls.push(EscapeCall {
            targets,
            is_open: resolution.is_open,
            arguments: self
                .tree
                .get_values(call.arguments)
                .iter()
                .map(|value| value.id())
                .collect(),
            environment,
            result,
            arguments_declared,
        });
    }

    /// Append a pointer flow unless it copies a location into itself.
    fn flow(&mut self, source: u32, destination: u32, dereferences: i32) {
        if source != destination || dereferences < 0 {
            self.body.flows.push(EscapeFlow {
                source,
                destination,
                dereferences,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::analyses::tests::TestModule;
    use crate::{AllocationEscape, EscapeEffect, ParameterEscape, Point, Value};

    /// Keep allocations distinct when fallible operations share one success parameter.
    #[test]
    fn test_return_fallible_allocations() {
        let program = TestModule::new(
            r#"
function test(v0: boolean): uninit<ref<int32, unique, mutable>> {
entry(v0: boolean):
    branch v0 => left | right

left:
    new.uninit.try int32, local => join | failure

right:
    new.uninit.try int32, local => join | failure

join(v1: uninit<ref<int32, unique, mutable>>):
    return v1

failure:
    unreachable
}
"#,
        );
        let mut analyses = program.module_analyses();
        let function = program.entry_function_id();
        let escape = analyses.escape(function, &program.tree, &program.effects, &program.dispatch);
        let blocks = program.tree.get(function).blocks();
        let expected = AllocationEscape {
            is_returned: true,
            is_retained: false,
            is_persistent: false,
        };

        assert_eq!(
            escape.allocation(Point::Terminator(blocks[1])),
            Some(&expected)
        );
        assert_eq!(
            escape.allocation(Point::Terminator(blocks[2])),
            Some(&expected)
        );
    }

    /// Read the previous iteration's allocation after creating the next allocation.
    #[test]
    fn test_preserve_loop_allocations() {
        let program = TestModule::new(
            r#"
function test(v0: boolean): int32 {
    local l0: ref<int32, unique, mutable>

entry(v0: boolean):
    v1: ref<int32, unique, mutable> = new.zeroed int32, local
    v5: int32 = 7
    store (*v1), v5
    store l0, v1
    jump loop

loop:
    v2: ref<int32, unique, mutable> = load l0
    v3: ref<int32, unique, mutable> = new.zeroed int32, local
    v4: int32 = load (*v2)
    store (*v3), v5
    store l0, v3
    branch v0 => loop | exit

exit:
    return v4
}
"#,
        );
        let mut analyses = program.module_analyses();
        let function = program.entry_function_id();
        let escape = analyses.escape(function, &program.tree, &program.effects, &program.dispatch);
        let block = program.tree.get(function).blocks()[1];

        assert_eq!(
            escape.allocation(Point::Instruction(program.tree.get(block).instructions[1])),
            Some(&AllocationEscape {
                is_returned: false,
                is_retained: false,
                is_persistent: true,
            })
        );
    }

    /// Keep an allocation reusable when every reference expires within its loop iteration.
    #[test]
    fn test_reuse_loop_allocations() {
        let program = TestModule::new(
            r#"
function test(v0: boolean): void {
entry(v0: boolean):
    jump loop

loop:
    v1: ref<int32, unique, mutable> = new.zeroed int32, local
    branch v0 => loop | exit

exit:
    return
}
"#,
        );
        let mut analyses = program.module_analyses();
        let function = program.entry_function_id();
        let escape = analyses.escape(function, &program.tree, &program.effects, &program.dispatch);
        let block = program.tree.get(function).blocks()[1];

        assert_eq!(
            escape.allocation(Point::Instruction(program.tree.get(block).instructions[0])),
            Some(&AllocationEscape::default())
        );
    }

    /// Read a previous allocation after revisiting either entry of an irreducible cycle.
    #[test]
    fn test_preserve_irreducible_allocations() {
        let program = TestModule::new(
            r#"
function test(v0: boolean): int32 {
    local l0: ref<int32, unique, mutable>

entry(v0: boolean):
    v1: ref<int32, unique, mutable> = new.zeroed int32, local
    v7: int32 = 7
    store (*v1), v7
    store l0, v1
    branch v0 => left | right

left:
    v2: ref<int32, unique, mutable> = load l0
    v3: ref<int32, unique, mutable> = new.zeroed int32, local
    v4: int32 = load (*v2)
    store (*v3), v4
    store l0, v3
    jump right

right:
    branch v0 => left | exit

exit:
    v5: ref<int32, unique, mutable> = load l0
    v6: int32 = load (*v5)
    return v6
}
"#,
        );
        let mut analyses = program.module_analyses();
        let function = program.entry_function_id();
        let block = program.tree.get(function).blocks()[1];

        assert_eq!(
            analyses
                .escape(function, &program.tree, &program.effects, &program.dispatch)
                .allocation(Point::Instruction(program.tree.get(block).instructions[1])),
            Some(&AllocationEscape {
                is_returned: false,
                is_retained: false,
                is_persistent: true,
            })
        );
    }

    /// Reuse storage when an irreducible cycle consumes each pointer within its allocation block.
    #[test]
    fn test_reuse_irreducible_allocations() {
        let program = TestModule::new(
            r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 0
    branch v0 => left | right(v1)

left:
    v2: ref<int32, unique, mutable> = new.zeroed int32, local
    v3: int32 = load (*v2)
    jump right(v3)

right(v4: int32):
    branch v0 => left | exit

exit:
    return v4
}
"#,
        );
        let mut analyses = program.module_analyses();
        let function = program.entry_function_id();
        let block = program.tree.get(function).blocks()[1];

        assert_eq!(
            analyses
                .escape(function, &program.tree, &program.effects, &program.dispatch)
                .allocation(Point::Instruction(program.tree.get(block).instructions[0])),
            Some(&AllocationEscape::default()),
        );
    }

    /// Follow an allocation through a local address, store, and load before returning it.
    #[test]
    fn test_return_through_local_storage() {
        let program = TestModule::new(
            r#"
function test(): ref<int32, unique, mutable> {
    local l0: ref<int32, unique, mutable>

entry:
    v0: ref<int32, unique, mutable> = new.zeroed int32, local
    v1: ref<ref<int32, unique, mutable>, borrowed, 'frame, mutable> = address l0
    store (*v1), v0
    v2: ref<int32, unique, mutable> = load (*v1)
    return v2
}
"#,
        );
        let mut analyses = program.module_analyses();
        let function = program.entry_function_id();
        let escape = analyses.escape(function, &program.tree, &program.effects, &program.dispatch);
        let entry = program.tree.get(program.entry_block_id(function));

        assert_eq!(
            escape.allocation(Point::Instruction(entry.instructions[0])),
            Some(&AllocationEscape {
                is_returned: true,
                is_retained: false,
                is_persistent: false,
            })
        );
    }

    /// Record a returned pointer loaded from a parameter at one dereference.
    #[test]
    fn test_return_loaded_parameter() {
        let program = TestModule::new(
            r#"
function test(v0: ref<ref<int32, borrowed, 'static, readonly>, borrowed, 'static, readonly>): ref<int32, borrowed, 'static, readonly> {
entry(v0: ref<ref<int32, borrowed, 'static, readonly>, borrowed, 'static, readonly>):
    v1: ref<int32, borrowed, 'static, readonly> = load (*v0)
    return v1
}
"#,
        );
        let mut analyses = program.module_analyses();
        let escape = analyses.escape(
            program.entry_function_id(),
            &program.tree,
            &program.effects,
            &program.dispatch,
        );

        assert_eq!(
            escape.effect,
            EscapeEffect {
                parameters: vec![ParameterEscape {
                    retained: None,
                    returned: Some(1)
                }],
                environment: None,
                external: None,
            }
        );
        assert_eq!(
            [escape.is_exposed(Value(0)), escape.is_exposed(Value(1))],
            [true, true]
        );
    }

    /// Distinguish a unique parameter's private address from external pointers in its contents.
    #[test]
    fn test_distinguish_owned_parameter_contents() {
        let program = TestModule::new(
            r#"
function test(v0: ref<ref<int32, borrowed, 'static, readonly>, unique, readonly>): void {
entry(v0: ref<ref<int32, borrowed, 'static, readonly>, unique, readonly>):
    v1: ref<int32, borrowed, 'static, readonly> = load (*v0)
    return
}
"#,
        );
        let mut analyses = program.module_analyses();
        let escape = analyses.escape(
            program.entry_function_id(),
            &program.tree,
            &program.effects,
            &program.dispatch,
        );

        assert_eq!(
            [escape.is_exposed(Value(0)), escape.is_exposed(Value(1))],
            [false, true]
        );
        assert_eq!(escape.effect.parameters, vec![ParameterEscape::default()]);
    }

    /// Retain a pointer stored through an incoming reference.
    #[test]
    fn test_store_through_parameter() {
        let program = TestModule::new(
            r#"
function test(v0: ref<ref<int32, unique, mutable>, borrowed, 'static, mutable>): void {
entry(v0: ref<ref<int32, unique, mutable>, borrowed, 'static, mutable>):
    v1: ref<int32, unique, mutable> = new.zeroed int32, local
    store (*v0), v1
    return
}
"#,
        );
        let mut analyses = program.module_analyses();
        let function = program.entry_function_id();
        let escape = analyses.escape(function, &program.tree, &program.effects, &program.dispatch);
        let entry = program.tree.get(program.entry_block_id(function));

        assert_eq!(
            escape.allocation(Point::Instruction(entry.instructions[0])),
            Some(&AllocationEscape {
                is_returned: false,
                is_retained: true,
                is_persistent: false,
            })
        );
        assert!(escape.is_exposed(Value(1)));
    }

    /// Preserve a returned allocation address through integer vector operations.
    #[test]
    fn test_return_address_through_vector() {
        let program = TestModule::new(
            r#"
function test(): usize {
entry:
    v0: ref<int32, unique, mutable> = new.zeroed int32, local
    v1: usize = cast.pointerToInt v0 -> usize
    v2: vector<usize, 2> = vector.splat v1
    v3: usize = 0
    v4: usize = vector.extract v2, v3
    return v4
}
"#,
        );
        let mut analyses = program.module_analyses();
        let function = program.entry_function_id();
        let escape = analyses.escape(function, &program.tree, &program.effects, &program.dispatch);
        let entry = program.tree.get(program.entry_block_id(function));

        assert_eq!(
            escape.allocation(Point::Instruction(entry.instructions[0])),
            Some(&AllocationEscape {
                is_returned: true,
                is_retained: false,
                is_persistent: false,
            })
        );
    }

    /// Preserve returned allocation addresses through reversible bit operations.
    #[test]
    fn test_return_address_through_bit_operations() {
        let program = TestModule::new(
            r#"
function test(): usize {
entry:
    v0: ref<int32, unique, mutable> = new.zeroed int32, local
    v1: usize = cast.pointerToInt v0 -> usize
    v2: usize = intrinsic.math.bits.byteSwap(v1)
    v3: usize = intrinsic.math.bits.byteSwap(v2)
    return v3
}
"#,
        );
        let mut analyses = program.module_analyses();
        let function = program.entry_function_id();
        let block = program.entry_block_id(function);

        assert_eq!(
            analyses
                .escape(function, &program.tree, &program.effects, &program.dispatch)
                .allocation(Point::Instruction(program.tree.get(block).instructions[0])),
            Some(&AllocationEscape {
                is_returned: true,
                is_retained: false,
                is_persistent: false,
            })
        );
    }

    /// Return either allocation selected by the branch.
    #[test]
    fn test_return_merged_allocations() {
        let program = TestModule::new(
            r#"
function test(v0: boolean): ref<int32, unique, mutable> {
entry(v0: boolean):
    v1: ref<int32, unique, mutable> = new.zeroed int32, local
    v2: ref<int32, unique, mutable> = new.zeroed int32, local
    branch v0 => join(v1) | join(v2)

join(v3: ref<int32, unique, mutable>):
    return v3
}
"#,
        );
        let mut analyses = program.module_analyses();
        let function = program.entry_function_id();
        let escape = analyses.escape(function, &program.tree, &program.effects, &program.dispatch);
        let entry = program.tree.get(program.entry_block_id(function));
        let expected = AllocationEscape {
            is_returned: true,
            is_retained: false,
            is_persistent: false,
        };

        assert_eq!(
            escape.allocation(Point::Instruction(entry.instructions[0])),
            Some(&expected)
        );
        assert_eq!(
            escape.allocation(Point::Instruction(entry.instructions[1])),
            Some(&expected)
        );
        assert_eq!(escape.effect.parameters, vec![ParameterEscape::default()]);
    }

    /// Keep arguments private when a known callee reads nothing and retains nothing.
    #[test]
    fn test_preserve_uncaptured_call_arguments() {
        let program = TestModule::new(
            r#"
function sink(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(): void {
entry:
    v0: ref<int32, unique, mutable> = new.zeroed int32, local
    call sink(v0): (ref<int32, unique, mutable>) => void
    return
}
"#,
        );
        let mut analyses = program.module_analyses();
        let function = program.entry_function_id();
        let escape = analyses.escape(function, &program.tree, &program.effects, &program.dispatch);
        let entry = program.tree.get(program.entry_block_id(function));

        assert_eq!(
            escape.allocation(Point::Instruction(entry.instructions[0])),
            Some(&AllocationEscape::default())
        );
        assert!(!escape.is_exposed(Value(0)));
    }

    /// Distinguish returning an argument from retaining it outside the call.
    #[test]
    fn test_discard_returned_argument() {
        let program = TestModule::new(
            r#"
function identity(v0: ref<int32, unique, mutable>): ref<int32, unique, mutable> {
entry(v0: ref<int32, unique, mutable>):
    return v0
}

function test(): void {
entry:
    v0: ref<int32, unique, mutable> = new.zeroed int32, local
    v1: ref<int32, unique, mutable> = call identity(v0): (ref<int32, unique, mutable>) => ref<int32, unique, mutable>
    return
}
"#,
        );
        let mut analyses = program.module_analyses();
        let identity = analyses.escape(
            program.function_id_by_name("identity"),
            &program.tree,
            &program.effects,
            &program.dispatch,
        );
        let function = program.entry_function_id();
        let escape = analyses.escape(function, &program.tree, &program.effects, &program.dispatch);
        let entry = program.tree.get(program.entry_block_id(function));

        assert_eq!(
            identity.effect.parameters,
            vec![ParameterEscape {
                retained: None,
                returned: Some(0)
            }]
        );
        assert_eq!(
            escape.allocation(Point::Instruction(entry.instructions[0])),
            Some(&AllocationEscape::default())
        );
        assert!(!escape.is_exposed(Value(1)));
    }

    /// Propagate returned parameters through mutually recursive calls.
    #[test]
    fn test_return_through_recursive_calls() {
        let program = TestModule::new(
            r#"
function first(v0: ref<int32, unique, mutable>, v1: boolean): ref<int32, unique, mutable> {
entry(v0: ref<int32, unique, mutable>, v1: boolean):
    branch v1 => done | recurse

recurse:
    v2: ref<int32, unique, mutable> = call second(v0, v1): (ref<int32, unique, mutable>, boolean) => ref<int32, unique, mutable>
    return v2

done:
    return v0
}

function second(v0: ref<int32, unique, mutable>, v1: boolean): ref<int32, unique, mutable> {
entry(v0: ref<int32, unique, mutable>, v1: boolean):
    v2: ref<int32, unique, mutable> = call first(v0, v1): (ref<int32, unique, mutable>, boolean) => ref<int32, unique, mutable>
    return v2
}
"#,
        );
        let mut analyses = program.module_analyses();
        for name in ["first", "second"] {
            let escape = analyses.escape(
                program.function_id_by_name(name),
                &program.tree,
                &program.effects,
                &program.dispatch,
            );

            assert_eq!(
                escape.effect.parameters,
                vec![
                    ParameterEscape {
                        retained: None,
                        returned: Some(0)
                    },
                    ParameterEscape::default(),
                ]
            );
        }
    }

    /// Track the environment returned by a known closure target.
    #[test]
    fn test_return_closure_environment() {
        let program = TestModule::new(
            r#"
@environment(ref<int32, unique, mutable>)
function captured(): ref<int32, unique, mutable> {
entry:
    v0: ref<int32, unique, mutable> = function.environment.current
    return v0
}

function test(): ref<int32, unique, mutable> {
entry:
    v0: ref<int32, unique, mutable> = new.zeroed int32, local
    v1: function<() => ref<int32, unique, mutable>, once, unique, mutable> = function.bind captured, v0
    v2: ref<int32, unique, mutable> = call.indirect v1(): () => ref<int32, unique, mutable>
    return v2
}
"#,
        );
        let mut analyses = program.module_analyses();
        let function = program.entry_function_id();
        let block = program.entry_block_id(function);

        assert_eq!(
            analyses
                .escape(function, &program.tree, &program.effects, &program.dispatch)
                .allocation(Point::Instruction(program.tree.get(block).instructions[0])),
            Some(&AllocationEscape {
                is_returned: true,
                is_retained: false,
                is_persistent: false
            })
        );
        assert_eq!(
            analyses
                .escape(
                    program.function_id_by_name("captured"),
                    &program.tree,
                    &program.effects,
                    &program.dispatch
                )
                .effect
                .environment,
            Some(ParameterEscape {
                retained: None,
                returned: Some(0)
            })
        );
    }

    /// Retain arguments passed to an external call and expose its independent result.
    #[test]
    fn test_propagate_external_pointer_effects() {
        let program = TestModule::new(
            r#"
external function retain(ref<int32, unique, mutable>): void
external function fetch(): ref<int32, unique, mutable>
external function borrow(): ref<int32, borrowed, 'static, readonly>

function test(): ref<int32, unique, mutable> {
entry:
    v0: ref<int32, unique, mutable> = new.zeroed int32, local
    call retain(v0): (ref<int32, unique, mutable>) => void
    v1: ref<int32, unique, mutable> = call fetch(): () => ref<int32, unique, mutable>
    return v1
}

function receive(): ref<int32, borrowed, 'static, readonly> {
entry:
    v0: ref<int32, borrowed, 'static, readonly> = call borrow(): () => ref<int32, borrowed, 'static, readonly>
    return v0
}
"#,
        );
        let mut analyses = program.module_analyses();
        let function = program.entry_function_id();
        let escape = analyses.escape(function, &program.tree, &program.effects, &program.dispatch);
        let entry = program.tree.get(program.entry_block_id(function));

        assert_eq!(
            escape.allocations,
            vec![(
                Point::Instruction(entry.instructions[0]),
                AllocationEscape {
                    is_returned: false,
                    is_retained: true,
                    is_persistent: false,
                }
            )]
        );
        assert_eq!(
            escape.effect,
            EscapeEffect {
                parameters: vec![],
                environment: None,
                external: Some(1)
            }
        );
        assert_eq!(
            [escape.is_exposed(Value(0)), escape.is_exposed(Value(1))],
            [true, false]
        );

        // expose a borrowed result at its address while preserving unique result storage
        let borrowed = analyses.escape(
            program.function_id_by_name("receive"),
            &program.tree,
            &program.effects,
            &program.dispatch,
        );
        assert_eq!(
            borrowed.effect,
            EscapeEffect {
                parameters: vec![],
                environment: None,
                external: Some(0),
            }
        );
        assert!(borrowed.is_exposed(Value(0)));
        assert_eq!(borrowed.allocations, vec![]);
    }
}
