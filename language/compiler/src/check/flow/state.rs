use destack_dir as dir;
use indexmap::{IndexMap, IndexSet};

use crate::check::{
    Capture, Condition, ControlTarget, FlowPath, FunctionFrame, Receiver, ReceiverBinding,
    TryTarget,
};

/// Flow state while walking one module.
#[derive(Debug)]
pub(in crate::check) struct FlowState {
    /// Function bodies currently being walked.
    pub(in crate::check::flow) functions: Vec<FunctionFrame>,
    /// Static guards currently guarding walked work.
    pub(in crate::check::flow) guards: Vec<Condition>,
    /// Contextual receiver scopes currently visible outside function bodies.
    pub(in crate::check::flow) receivers: Vec<Option<Receiver>>,
    /// Control targets currently visible to `break` and `continue`.
    pub(in crate::check::flow) targets: Vec<ControlTarget>,
    /// Try targets currently visible to `?`.
    pub(in crate::check::flow) tries: Vec<TryTarget>,
    /// Local symbols definitely assigned at the current walk point.
    pub(in crate::check::flow) assigned: IndexSet<dir::GlobalSymbolId>,
    /// Narrowed type operands keyed by flow path.
    pub(in crate::check::flow) narrowings: IndexMap<FlowPath, dir::GlobalTypeId>,
    /// Jumps that bound no target and recovered as completing statements.
    unbound_jumps: IndexSet<dir::LocalNodeIdAny>,

    /// Flow mutations made since walking started.
    mutations: Vec<FlowMutation>,
}

impl Default for FlowState {
    /// Create empty flow state.
    fn default() -> Self {
        Self {
            functions: Vec::new(),
            guards: Vec::new(),
            receivers: Vec::new(),
            targets: Vec::new(),
            tries: Vec::new(),
            assigned: IndexSet::new(),
            narrowings: IndexMap::new(),
            unbound_jumps: IndexSet::new(),
            mutations: Vec::new(),
        }
    }
}

/// A checkpoint in the flow mutation log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct FlowCheckpoint {
    /// The number of mutations visible at the checkpoint.
    mutation_count: usize,
}

/// Flow changes produced by one branch after a checkpoint.
#[derive(Debug, Clone)]
pub(in crate::check) struct FlowBranch {
    /// Symbols assigned by this branch.
    assigned_symbols: IndexSet<dir::GlobalSymbolId>,
    /// Narrowings touched by this branch.
    narrowings: IndexMap<FlowPath, Option<dir::GlobalTypeId>>,
}

/// One reversible flow mutation.
#[derive(Debug, Clone)]
enum FlowMutation {
    /// One definite assignment change.
    Assign {
        /// The assigned symbol.
        symbol: dir::GlobalSymbolId,
        /// Whether the symbol was already assigned.
        was_assigned: bool,
    },
    /// One narrowing change.
    Narrow {
        /// The narrowed path.
        path: Box<FlowPath>,
        /// The previous narrowing at the same path.
        previous: Option<dir::GlobalTypeId>,
    },
}

impl FlowState {
    /// Record one jump that bound no target.
    pub(in crate::check) fn record_unbound_jump(&mut self, source: dir::LocalNodeIdAny) {
        self.unbound_jumps.insert(source);
    }

    /// Return whether one jump bound no target.
    pub(in crate::check) fn is_unbound_jump(&self, source: dir::LocalNodeIdAny) -> bool {
        self.unbound_jumps.contains(&source)
    }

    /// Push one static guard while walking.
    pub(in crate::check) fn push_static_guard(&mut self, condition: Condition) {
        // combine nested guards eagerly
        let condition = self.active_static_guard().and(condition);

        self.guards.push(condition);
    }

    /// Pop the current static guard.
    pub(in crate::check) fn pop_static_guard(&mut self) {
        // require balanced guard pushes
        if self.guards.pop().is_none() {
            unreachable!("static guard stack underflow");
        }
    }

    /// Return the active static guard.
    pub(in crate::check) fn active_static_guard(&self) -> Condition {
        self.guards.last().cloned().unwrap_or(Condition::Always)
    }

    /// Enter one function body while walking.
    pub(in crate::check) fn push_function(&mut self, function: FunctionFrame) {
        self.functions.push(function);
    }

    /// Leave the current function body.
    pub(in crate::check) fn pop_function(&mut self) -> Capture {
        // require an active function frame
        let Some(function) = self.functions.pop() else {
            unreachable!("function stack underflow");
        };

        // require balanced control targets
        if self.targets.len() != function.target_start {
            unreachable!("control target stack leaked out of function");
        }

        // require balanced try targets
        if self.tries.len() != function.try_start {
            unreachable!("try target stack leaked out of function");
        }

        // collect frame capture output
        let symbols = function.captured_symbols.iter().copied().collect();
        let capture = Capture {
            symbol: function.symbol,
            symbols,
            receiver: function.captured_receiver,
            directive: None,
        };

        // restore outer definite assignment and narrowing state
        self.restore(function.checkpoint);

        capture
    }

    /// Return the current function body.
    pub(in crate::check) fn current_function(&self) -> Option<&FunctionFrame> {
        self.functions.last()
    }

    /// Enter one contextual receiver.
    pub(in crate::check) fn push_receiver(&mut self, receiver: Receiver) {
        self.receivers.push(Some(receiver));
    }

    /// Enter one explicit contextual receiver scope.
    pub(in crate::check) fn push_receiver_scope(&mut self, receiver: Option<Receiver>) {
        self.receivers.push(receiver);
    }

    /// Leave the current contextual receiver.
    pub(in crate::check) fn pop_receiver(&mut self) {
        if self.receivers.pop().is_none() {
            unreachable!("receiver stack underflow");
        }
    }

    /// Return the current contextual receiver.
    pub(in crate::check) fn current_receiver(&self) -> Option<Receiver> {
        self.receivers.last().copied().flatten()
    }

    /// Enter one break or continue target.
    pub(in crate::check) fn push_target(&mut self, target: ControlTarget) {
        self.targets.push(target);
    }

    /// Leave the current break or continue target.
    pub(in crate::check) fn pop_target(&mut self) -> ControlTarget {
        let Some(target) = self.targets.pop() else {
            unreachable!("control target stack underflow");
        };

        target
    }

    /// Enter one try failure target.
    pub(in crate::check) fn push_try(&mut self, target: TryTarget) {
        self.tries.push(target);
    }

    /// Leave the current try failure target.
    pub(in crate::check) fn pop_try(&mut self) -> TryTarget {
        // require an active try target
        let Some(target) = self.tries.pop() else {
            unreachable!("try target stack underflow");
        };

        target
    }

    /// Return the current try failure target.
    pub(in crate::check) fn current_try_mut(&mut self) -> Option<&mut TryTarget> {
        // hide try targets from outer functions
        let start = self.current_try_start();
        if self.tries.len() == start {
            return None;
        }

        self.tries.last_mut()
    }

    /// Return the target index selected by one break.
    pub(in crate::check) fn break_target_index(
        &self,
        label: Option<dir::StringId>,
    ) -> Option<usize> {
        let start = self.current_target_start();
        self.targets
            .iter()
            .enumerate()
            .skip(start)
            .rev()
            .find_map(|(index, target)| {
                if let Some(label) = label {
                    (target.label == Some(label)).then_some(index)
                } else {
                    Some(index)
                }
            })
    }

    /// Return the target index selected by one continue.
    pub(in crate::check) fn continue_target_index(
        &self,
        label: Option<dir::StringId>,
    ) -> Option<usize> {
        let start = self.current_target_start();
        self.targets
            .iter()
            .enumerate()
            .skip(start)
            .rev()
            .find_map(|(index, target)| {
                if let Some(label) = label {
                    (target.label == Some(label) && target.allows_continue).then_some(index)
                } else {
                    target.allows_continue.then_some(index)
                }
            })
    }

    /// Return the control checkpoint and result selected by one target index.
    pub(in crate::check) fn control_target_result(
        &self,
        index: usize,
    ) -> (FlowCheckpoint, dir::GlobalTypeId) {
        let target = &self.targets[index];

        (target.checkpoint, target.result)
    }

    /// Push one break branch onto a selected control target.
    pub(in crate::check) fn push_break_branch(
        &mut self,
        index: usize,
        value: dir::GlobalTypeId,
        branch: FlowBranch,
    ) {
        let target = &mut self.targets[index];

        target.break_values.push(value);
        target.break_branches.push(branch);
    }

    /// Push one continue branch onto a selected control target.
    pub(in crate::check) fn push_continue_branch(&mut self, index: usize, branch: FlowBranch) {
        self.targets[index].continue_branches.push(branch);
    }

    /// Take continue branches collected by the current control target.
    pub(in crate::check) fn take_continue_branches(&mut self) -> Vec<FlowBranch> {
        let Some(target) = self.targets.last_mut() else {
            unreachable!("continue branch collection requires an active control target");
        };

        std::mem::take(&mut target.continue_branches)
    }

    /// Capture one symbol in the current function body.
    pub(in crate::check) fn capture_symbol(&mut self, symbol: dir::GlobalSymbolId) {
        let Some(function) = self.functions.last_mut() else {
            unreachable!("symbol capture requires an active function");
        };

        function.captured_symbols.insert(symbol);
    }

    /// Capture one receiver in the current function body.
    pub(in crate::check) fn capture_receiver(&mut self, receiver: ReceiverBinding) {
        let Some(function) = self.functions.last_mut() else {
            unreachable!("receiver capture requires an active function");
        };

        function.captured_receiver = Some(receiver);
    }

    /// Return the lexical receiver visible to the current function.
    pub(in crate::check) fn lexical_receiver(&self) -> Option<(usize, ReceiverBinding)> {
        self.functions
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, function)| function.receiver.map(|receiver| (index, receiver)))
    }

    /// Return whether one function frame is the innermost active function.
    pub(in crate::check) fn is_current_function(&self, index: usize) -> bool {
        index + 1 == self.functions.len()
    }

    /// Return the first control target visible to the current function.
    fn current_target_start(&self) -> usize {
        match self.functions.last() {
            // restrict control targets to the innermost function
            Some(function) => function.target_start,
            // allow every control target at module scope
            None => 0,
        }
    }

    /// Return the first try target visible to the current function.
    fn current_try_start(&self) -> usize {
        match self.functions.last() {
            // restrict try targets to the innermost function
            Some(function) => function.try_start,
            // allow every try target at module scope
            None => 0,
        }
    }

    /// Mark one local symbol as definitely assigned.
    pub(in crate::check) fn mark_assigned(&mut self, symbol: dir::GlobalSymbolId) {
        // record previous assignment state for rollback
        let was_assigned = self.assigned.contains(&symbol);

        self.mutations.push(FlowMutation::Assign {
            symbol,
            was_assigned,
        });
        self.assigned.insert(symbol);
    }

    /// Return the current narrowing for one flow path.
    pub(in crate::check) fn narrowing(&self, path: &FlowPath) -> Option<dir::GlobalTypeId> {
        self.narrowings.get(path).copied()
    }

    /// Narrow one path at the current walk point.
    pub(in crate::check) fn narrow(&mut self, path: FlowPath, ty: dir::GlobalTypeId) {
        // record previous narrowing for rollback
        let previous = self.narrowings.get(&path).copied();

        self.mutations.push(FlowMutation::Narrow {
            path: Box::new(path.clone()),
            previous,
        });
        self.narrowings.insert(path, ty);
    }

    /// Return a checkpoint for later branch rollback.
    pub(in crate::check) fn fork(&self) -> FlowCheckpoint {
        FlowCheckpoint {
            mutation_count: self.mutations.len(),
        }
    }

    /// Return the branch changes made after one checkpoint.
    pub(in crate::check) fn branch(&self, checkpoint: FlowCheckpoint) -> FlowBranch {
        let mut assigned_symbols = IndexSet::new();
        let mut narrowing_paths = IndexSet::new();

        // collect flow state touched since the checkpoint
        for change in &self.mutations[checkpoint.mutation_count..] {
            match change {
                FlowMutation::Assign { symbol, .. } => {
                    if self.assigned.contains(symbol) {
                        assigned_symbols.insert(*symbol);
                    }
                }
                FlowMutation::Narrow { path, .. } => {
                    narrowing_paths.insert(path.as_ref().clone());
                }
            }
        }

        // snapshot final narrowing values for touched paths
        let narrowings = narrowing_paths
            .into_iter()
            .map(|path| {
                let value = self.narrowings.get(&path).copied();

                (path, value)
            })
            .collect();

        FlowBranch {
            assigned_symbols,
            narrowings,
        }
    }

    /// Restore the flow state to one checkpoint.
    pub(in crate::check) fn restore(&mut self, checkpoint: FlowCheckpoint) {
        // roll back mutations in reverse order
        while self.mutations.len() > checkpoint.mutation_count {
            let Some(change) = self.mutations.pop() else {
                break;
            };

            // undo the latest mutation
            match change {
                FlowMutation::Assign {
                    symbol,
                    was_assigned,
                } => {
                    // restore previous assignment state
                    if was_assigned {
                        self.assigned.insert(symbol);
                    } else {
                        self.assigned.shift_remove(&symbol);
                    }
                }
                FlowMutation::Narrow { path, previous } => {
                    // restore previous narrowing state
                    if let Some(previous) = previous {
                        self.narrowings.insert(*path, previous);
                    } else {
                        self.narrowings.shift_remove(path.as_ref());
                    }
                }
            }
        }
    }

    /// Restore one branch from its checkpoint.
    pub(in crate::check) fn restore_branch(
        &mut self,
        checkpoint: FlowCheckpoint,
        branch: &FlowBranch,
    ) {
        self.restore(checkpoint);

        // replay assigned symbols from the branch
        for symbol in &branch.assigned_symbols {
            self.mark_assigned(*symbol);
        }

        // replay narrowings from the branch
        for (path, narrowing) in &branch.narrowings {
            // restore present narrowing
            if let Some(narrowing) = narrowing {
                self.narrow(path.clone(), *narrowing);
            }
            // restore cleared narrowing
            else {
                self.clear_narrowing(path.clone());
            }
        }
    }

    /// Merge two completed branches after one checkpoint.
    pub(in crate::check) fn merge_branches(
        &mut self,
        checkpoint: FlowCheckpoint,
        left: &FlowBranch,
        right: &FlowBranch,
    ) {
        self.restore(checkpoint);

        // keep symbols assigned by both branches
        for symbol in left.assigned_symbols.intersection(&right.assigned_symbols) {
            self.mark_assigned(*symbol);
        }

        // collect all touched narrowing paths
        let mut paths = IndexSet::new();
        paths.extend(left.narrowings.keys().cloned());
        paths.extend(right.narrowings.keys().cloned());

        // keep narrowings with equal final values in both branches
        for path in paths {
            // read final value from each branch
            let left = left
                .narrowings
                .get(&path)
                .copied()
                .unwrap_or_else(|| self.narrowings.get(&path).copied());
            let right = right
                .narrowings
                .get(&path)
                .copied()
                .unwrap_or_else(|| self.narrowings.get(&path).copied());

            // preserve equal branch results
            if left == right {
                if let Some(narrowing) = left {
                    self.narrow(path, narrowing);
                }
                // preserve equally cleared result
                else {
                    self.clear_narrowing(path);
                }
            }
            // clear diverging branch results
            else {
                self.clear_narrowing(path);
            }
        }
    }

    /// Clear one current narrowing.
    fn clear_narrowing(&mut self, path: FlowPath) {
        // record previous narrowing for rollback
        let previous = self.narrowings.get(&path).copied();

        self.mutations.push(FlowMutation::Narrow {
            path: Box::new(path.clone()),
            previous,
        });
        self.narrowings.shift_remove(&path);
    }

    /// Clear narrowings below one mutated path.
    pub(in crate::check) fn clear_narrowings_under(&mut self, path: &FlowPath) {
        // collect invalidated paths before mutating the map
        let paths: Vec<_> = self
            .narrowings
            .keys()
            .filter(|narrowing| narrowing.starts_with(path))
            .cloned()
            .collect();

        // clear each affected narrowing
        for path in paths {
            self.clear_narrowing(path);
        }
    }
}
