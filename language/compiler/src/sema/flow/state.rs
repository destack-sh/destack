use std::mem::take;

use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    Capture, ControlTarget, ControlTargetForm, FunctionFrame, Receiver, ReceiverBinding, TryTarget,
};

/// Flow state while walking one module.
#[derive(Debug)]
pub(in crate::sema) struct FlowState {
    /// Function bodies currently being walked.
    pub(in crate::sema::flow) functions: Vec<FunctionFrame>,
    /// Contextual receiver scopes currently visible outside function bodies.
    pub(in crate::sema::flow) receivers: Vec<Option<Receiver>>,
    /// Generic template scopes enclosing the current flow point.
    pub(in crate::sema::flow) template_scopes: Vec<dir::GlobalGenericTemplateId>,
    /// Control targets currently visible to `break` and `continue`.
    pub(in crate::sema::flow) targets: Vec<ControlTarget>,
    /// Try targets currently visible to `?`.
    pub(in crate::sema::flow) tries: Vec<TryTarget>,
    /// Places definitely assigned at the current flow point.
    pub(in crate::sema::flow) assigned: FxIndexSet<AssignedPlace>,
    /// Flow narrowings keyed by static path.
    pub(in crate::sema::flow) narrowings: FxIndexMap<dir::AccessPath, SmallVec<[FlowPredicate; 2]>>,
    /// Jumps that bound no target and complete as statements.
    unbound_jumps: FxIndexSet<dir::LocalNodeIdAny>,

    /// Flow changes made since walking started.
    changes: Vec<FlowChange>,
    /// Durable flow points built by this walk.
    points: Vec<FlowPoint>,
    /// The current durable flow point.
    current: FlowPointId,
}

impl Default for FlowState {
    /// Create empty flow state.
    fn default() -> Self {
        Self {
            functions: Vec::new(),
            receivers: Vec::new(),
            template_scopes: Vec::new(),
            targets: Vec::new(),
            tries: Vec::new(),
            assigned: FxIndexSet::default(),
            narrowings: FxIndexMap::default(),
            unbound_jumps: FxIndexSet::default(),
            changes: Vec::new(),
            points: vec![FlowPoint {
                parent: None,
                change: FlowPointChange::Start,
            }],
            current: FlowPointId::ROOT,
        }
    }
}

/// The id of one durable flow point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) struct FlowPointId {
    /// The point index in the flow point table.
    index: u32,
}

impl FlowPointId {
    /// The root flow point.
    const ROOT: Self = Self { index: 0 };

    /// Return the index in the module flow table.
    pub(in crate::sema) fn index(self) -> usize {
        self.index as usize
    }
}

/// One durable flow point.
#[derive(Debug, Clone)]
pub(in crate::sema) struct FlowPoint {
    /// The previous point in this flow path.
    pub(in crate::sema) parent: Option<FlowPointId>,
    /// The change applied at this point.
    pub(in crate::sema) change: FlowPointChange,
}

/// One durable flow point change.
#[derive(Debug, Clone)]
pub(in crate::sema) enum FlowPointChange {
    /// Initial flow state.
    Start,
    /// One narrowing applied to a path.
    Narrowing {
        /// The tested path.
        path: Box<dir::AccessPath>,
        /// The applied predicate.
        predicate: FlowPredicate,
    },
    /// One cleared narrowing.
    Clear {
        /// The cleared path.
        path: Box<dir::AccessPath>,
    },
}

/// A checkpoint in the flow change log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) struct FlowCheckpoint {
    /// The number of changes visible at the checkpoint.
    change_count: usize,
    /// The durable flow point visible at the checkpoint.
    point: FlowPointId,
}

/// The flow captured at one point, read by a body checked later.
/// NOTE #Performance: a deferred body copies the flow points built so far.
#[derive(Debug, Clone)]
pub(in crate::sema) struct FlowSnapshot {
    /// Places definitely assigned at the point.
    assigned: FxIndexSet<AssignedPlace>,
    /// Flow narrowings visible at the point.
    narrowings: FxIndexMap<dir::AccessPath, SmallVec<[FlowPredicate; 2]>>,
    /// Durable flow points built up to the point.
    points: Vec<FlowPoint>,
    /// The durable flow point.
    current: FlowPointId,
}

/// Flow changes produced by one branch after a checkpoint.
#[derive(Debug, Clone, PartialEq, Default)]
pub(in crate::sema) struct FlowBranch {
    /// Places assigned by this branch.
    assigned: FxIndexSet<AssignedPlace>,
    /// Narrowings touched by this branch.
    narrowings: FxIndexMap<dir::AccessPath, SmallVec<[FlowPredicate; 2]>>,
    /// Whether the branch never completes, ending in a return, a jump, or a `never` value.
    diverges: bool,
}

impl FlowBranch {
    /// Return whether this branch assigns one place, a diverging branch assigning every place.
    pub(in crate::sema) fn is_assigned(&self, place: AssignedPlace) -> bool {
        self.diverges || self.assigned.contains(&place)
    }
}

/// One place assignment tracked by flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum AssignedPlace {
    /// The base constructor call of a derived class constructor.
    Delegated,
    /// A local or member declaration symbol.
    Symbol(dir::GlobalSymbolId),
    /// A member identified by receiver type and key.
    Member {
        /// The receiver type.
        receiver: dir::GlobalTypeId,
        /// The member key.
        key: dir::StaticKey,
    },
}

/// One reversible flow change.
#[derive(Debug, Clone)]
enum FlowChange {
    /// One definite assignment change.
    Assign {
        /// The assigned place.
        place: AssignedPlace,
        /// Whether the place was already assigned.
        was_assigned: bool,
    },
    /// One appended narrowing.
    Narrowing {
        /// The tested path.
        path: Box<dir::AccessPath>,
    },
    /// One cleared narrowing list.
    Clear {
        /// The tested path.
        path: Box<dir::AccessPath>,
        /// The narrowings visible before the clear.
        previous: SmallVec<[FlowPredicate; 2]>,
    },
    /// The path ends here.
    Diverge,
}

/// One flow predicate recorded for a lexical access path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum FlowPredicate {
    /// The values accepted or rejected by one pattern.
    Pattern {
        /// The pattern node.
        pattern: dir::GlobalNodeId<dir::Pattern>,
        /// Whether matching values are kept.
        is_positive: bool,
    },
    /// The branch selected by one equality operation.
    Equality {
        /// The selected binary operator or switch case.
        operation: dir::GlobalNodeIdAny,
        /// Whether equal values flow into this branch.
        is_equal: bool,
    },
    /// The type predicate selected by one guard expression.
    Guard {
        /// The guard expression node.
        guard: dir::GlobalNodeId<dir::Expression>,
        /// Whether this is the positive branch.
        is_positive: bool,
    },
}

impl FlowState {
    /// Sync this walk's durable graph into its module flow table.
    pub(in crate::sema) fn sync_to(&self, points: &mut Vec<FlowPoint>) {
        while points.len() < self.points.len() {
            points.push(self.points[points.len()].clone());
        }
    }

    /// Restart the cursor context while keeping the durable point log.
    pub(in crate::sema) fn reset_cursor(&mut self) {
        self.functions.clear();
        self.receivers.clear();
        self.template_scopes.clear();
        self.targets.clear();
        self.tries.clear();
        self.assigned.clear();
        self.narrowings.clear();
        self.unbound_jumps.clear();
        self.changes.clear();
        self.push_point(FlowPointChange::Start);
    }

    /// Return the current durable flow point.
    pub(in crate::sema) fn point(&self) -> FlowPointId {
        self.current
    }

    /// Return the cursor's flow point graph.
    pub(in crate::sema) fn points(&self) -> &[FlowPoint] {
        &self.points
    }

    /// Append one durable flow point.
    fn push_point(&mut self, change: FlowPointChange) {
        // link the new point to the current one
        let point = FlowPointId {
            index: self.points.len() as u32,
        };
        self.points.push(FlowPoint {
            parent: Some(self.current),
            change,
        });
        self.current = point;
    }

    /// Add one jump that bound no target.
    pub(in crate::sema) fn insert_unbound_jump(&mut self, source: dir::LocalNodeIdAny) {
        self.unbound_jumps.insert(source);
    }

    /// Return whether one jump bound no target.
    pub(in crate::sema) fn is_unbound_jump(&self, source: dir::LocalNodeIdAny) -> bool {
        self.unbound_jumps.contains(&source)
    }

    /// Enter one function body frame.
    pub(in crate::sema) fn push_function(&mut self, function: FunctionFrame) {
        self.functions.push(function);
    }

    /// Leave the current function body.
    pub(in crate::sema) fn pop_function(&mut self) -> (Capture, FlowBranch) {
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
            annotation: None,
        };
        let branch = self.branch(function.checkpoint);

        // restore outer definite assignment and narrowing state
        self.restore(function.checkpoint);

        (capture, branch)
    }

    /// Return the current function body.
    pub(in crate::sema) fn current_function(&self) -> Option<&FunctionFrame> {
        self.functions.last()
    }

    /// Return the class the nearest enclosing constructor frame initializes.
    pub(in crate::sema) fn enclosing_initializes(&self) -> Option<dir::GlobalSymbolId> {
        self.functions
            .iter()
            .rev()
            .find_map(|function| function.initializes)
    }

    /// Return the current function receiver.
    pub(in crate::sema) fn current_function_receiver(&self) -> Option<&ReceiverBinding> {
        self.functions
            .last()
            .and_then(|function| function.receiver.as_ref())
    }

    /// Return the current function symbol.
    pub(in crate::sema) fn current_function_symbol(&self) -> Option<dir::GlobalSymbolId> {
        self.functions.last().map(|function| function.symbol)
    }

    /// Enter one explicit contextual receiver scope.
    pub(in crate::sema) fn push_receiver_scope(&mut self, receiver: Option<Receiver>) {
        self.receivers.push(receiver);
    }

    /// Enter one generic template scope.
    pub(in crate::sema) fn push_template_scope(&mut self, template: dir::GlobalGenericTemplateId) {
        self.template_scopes.push(template);
    }

    /// Leave the current generic template scope.
    pub(in crate::sema) fn pop_template_scope(&mut self) {
        if self.template_scopes.pop().is_none() {
            unreachable!("template scope stack underflow");
        }
    }

    /// Return the innermost generic template scope.
    pub(in crate::sema) fn template_scope(&self) -> Option<dir::GlobalGenericTemplateId> {
        self.template_scopes.last().copied()
    }

    /// Leave the current contextual receiver.
    pub(in crate::sema) fn pop_receiver(&mut self) {
        if self.receivers.pop().is_none() {
            unreachable!("receiver stack underflow");
        }
    }

    /// Return the current contextual receiver.
    pub(in crate::sema) fn current_receiver(&self) -> Option<Receiver> {
        self.receivers.last().copied().flatten()
    }

    /// Enter one break or continue target.
    pub(in crate::sema) fn push_target(&mut self, target: ControlTarget) {
        self.targets.push(target);
    }

    /// Leave the current break or continue target.
    pub(in crate::sema) fn pop_target(&mut self) -> ControlTarget {
        // require an active control target
        let Some(target) = self.targets.pop() else {
            unreachable!("control target stack underflow");
        };

        target
    }

    /// Enter one try failure target.
    pub(in crate::sema) fn push_try(&mut self, target: TryTarget) {
        self.tries.push(target);
    }

    /// Leave the current try failure target.
    pub(in crate::sema) fn pop_try(&mut self) -> TryTarget {
        // require an active try target
        let Some(target) = self.tries.pop() else {
            unreachable!("try target stack underflow");
        };

        target
    }

    /// Return the current try failure target.
    pub(in crate::sema) fn current_try_mut(&mut self) -> Option<&mut TryTarget> {
        // hide try targets from outer functions
        let start = self.current_try_start();
        if self.tries.len() == start {
            return None;
        }

        self.tries.last_mut()
    }

    /// Return the target index chosen by one break.
    pub(in crate::sema) fn break_target_index(
        &self,
        label: Option<dir::StringId>,
    ) -> Option<usize> {
        // search the visible targets from the innermost outward
        let start = self.current_target_start();
        self.targets
            .iter()
            .enumerate()
            .skip(start)
            .rev()
            .find_map(|(index, target)| {
                if let Some(label) = label {
                    target
                        .label
                        .is_some_and(|target| target.name == label)
                        .then_some(index)
                } else {
                    target.form.is_unlabeled_break_target().then_some(index)
                }
            })
    }

    /// Return the target index chosen by one continue.
    pub(in crate::sema) fn continue_target_index(
        &self,
        label: Option<dir::StringId>,
    ) -> Option<usize> {
        // search the visible targets from the innermost outward
        let start = self.current_target_start();
        self.targets
            .iter()
            .enumerate()
            .skip(start)
            .rev()
            .find_map(|(index, target)| {
                if let Some(label) = label {
                    (target.label.is_some_and(|target| target.name == label)
                        && target.form.is_continue_target())
                    .then_some(index)
                } else {
                    target.form.is_continue_target().then_some(index)
                }
            })
    }

    /// Return the expression that introduced one control target.
    pub(in crate::sema) fn control_target_source(
        &self,
        index: usize,
    ) -> dir::GlobalNodeId<dir::Expression> {
        self.targets[index].source
    }

    /// Return the control checkpoint chosen by one target index.
    pub(in crate::sema) fn control_target_checkpoint(&self, index: usize) -> FlowCheckpoint {
        let target = &self.targets[index];

        target.checkpoint
    }

    /// Return the control form chosen by one target index.
    pub(in crate::sema) fn control_target_form(&self, index: usize) -> ControlTargetForm {
        let target = &self.targets[index];

        target.form
    }

    /// Push one break branch onto a chosen control target.
    pub(in crate::sema) fn push_break_branch(&mut self, index: usize, branch: FlowBranch) {
        self.targets[index].break_branches.push(branch);
    }

    /// Push one continue branch onto a chosen control target.
    pub(in crate::sema) fn push_continue_branch(&mut self, index: usize, branch: FlowBranch) {
        self.targets[index].continue_branches.push(branch);
    }

    /// Take continue branches collected by the current control target.
    pub(in crate::sema) fn take_continue_branches(&mut self) -> Vec<FlowBranch> {
        // require an active control target
        let Some(target) = self.targets.last_mut() else {
            unreachable!("continue branch collection requires an active control target");
        };

        take(&mut target.continue_branches)
    }

    /// Capture one symbol in the current function body.
    pub(in crate::sema) fn capture_symbol(&mut self, symbol: dir::GlobalSymbolId) {
        // require an active function frame
        let Some(function) = self.functions.last_mut() else {
            unreachable!("symbol capture requires an active function");
        };

        function.captured_symbols.insert(symbol);
    }

    /// Capture one receiver in the current function body.
    pub(in crate::sema) fn capture_receiver(&mut self, receiver: ReceiverBinding) {
        // require an active function frame
        let Some(function) = self.functions.last_mut() else {
            unreachable!("receiver capture requires an active function");
        };

        function.captured_receiver = Some(receiver);
    }

    /// Return the lexical receiver visible to the current function and whether it binds it.
    pub(in crate::sema) fn lexical_receiver(&self) -> Option<(bool, ReceiverBinding)> {
        self.functions
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, function)| {
                function
                    .receiver
                    .map(|receiver| (self.is_current_function(index), receiver))
                    .or(function
                        .enclosing_receiver
                        .map(|receiver| (false, receiver)))
            })
    }

    /// Return whether one function frame is the innermost active function.
    pub(in crate::sema) fn is_current_function(&self, index: usize) -> bool {
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

    /// Return whether one place is definitely assigned at the current point.
    pub(in crate::sema) fn is_assigned(&self, place: AssignedPlace) -> bool {
        self.assigned.contains(&place)
    }

    /// Add one place to the definitely assigned set.
    pub(in crate::sema) fn insert_assigned(&mut self, place: AssignedPlace) {
        // keep previous assignment state for rollback
        let was_assigned = self.assigned.contains(&place);

        self.changes.push(FlowChange::Assign {
            place,
            was_assigned,
        });
        self.assigned.insert(place);
    }

    /// Narrow one path at the current flow point.
    pub(in crate::sema) fn apply_narrowing(
        &mut self,
        path: dir::AccessPath,
        predicate: FlowPredicate,
    ) {
        // record the narrowing in the change log and the durable graph
        self.changes.push(FlowChange::Narrowing {
            path: Box::new(path.clone()),
        });
        self.push_point(FlowPointChange::Narrowing {
            path: Box::new(path.clone()),
            predicate,
        });
        self.narrowings.entry(path).or_default().push(predicate);
    }

    /// Return a checkpoint for later branch rollback.
    pub(in crate::sema) fn fork(&self) -> FlowCheckpoint {
        FlowCheckpoint {
            change_count: self.changes.len(),
            point: self.current,
        }
    }

    /// Return the branch changes made after one checkpoint.
    pub(in crate::sema) fn branch(&self, checkpoint: FlowCheckpoint) -> FlowBranch {
        // collect flow state touched since the checkpoint
        let mut assigned = FxIndexSet::default();
        let mut narrowing_paths = FxIndexSet::default();
        let mut diverges = false;
        for change in &self.changes[checkpoint.change_count..] {
            match change {
                FlowChange::Assign { place, .. } => {
                    if self.assigned.contains(place) {
                        assigned.insert(*place);
                    }
                }
                FlowChange::Narrowing { path, .. } => {
                    narrowing_paths.insert(path.as_ref().clone());
                }
                FlowChange::Clear { path, .. } => {
                    narrowing_paths.insert(path.as_ref().clone());
                }
                FlowChange::Diverge => diverges = true,
            }
        }

        // snapshot final narrowing values for touched paths
        let narrowings = narrowing_paths
            .into_iter()
            .map(|path| {
                let value = self.narrowings.get(&path).cloned().unwrap_or_default();

                (path, value)
            })
            .collect();

        FlowBranch {
            assigned,
            narrowings,
            diverges,
        }
    }

    /// End the current path at a return, a jump, or a `never` value.
    pub(in crate::sema) fn insert_diverge(&mut self) {
        self.changes.push(FlowChange::Diverge);
    }

    /// Capture the flow at the current point.
    pub(in crate::sema) fn snapshot(&self) -> FlowSnapshot {
        FlowSnapshot {
            assigned: self.assigned.clone(),
            narrowings: self.narrowings.clone(),
            points: self.points.clone(),
            current: self.current,
        }
    }

    /// Return the flow state one snapshot captured, outside every frame.
    pub(in crate::sema) fn from_snapshot(snapshot: FlowSnapshot) -> Self {
        Self {
            assigned: snapshot.assigned,
            narrowings: snapshot.narrowings,
            points: snapshot.points,
            current: snapshot.current,
            ..Self::default()
        }
    }

    /// Restore the flow state to one checkpoint.
    pub(in crate::sema) fn restore(&mut self, checkpoint: FlowCheckpoint) {
        // take the changes made after the checkpoint
        let changes = self.changes.split_off(checkpoint.change_count);

        // roll back changes in reverse order
        for change in changes.into_iter().rev() {
            match change {
                FlowChange::Assign {
                    place,
                    was_assigned,
                } => {
                    // restore previous assignment state
                    if was_assigned {
                        self.assigned.insert(place);
                    } else {
                        self.assigned.shift_remove(&place);
                    }
                }
                FlowChange::Narrowing { path } => {
                    // remove the appended narrowing
                    let remove = self
                        .narrowings
                        .get_mut(path.as_ref())
                        .is_some_and(|narrowings| {
                            narrowings.pop();

                            narrowings.is_empty()
                        });
                    if remove {
                        self.narrowings.shift_remove(path.as_ref());
                    }
                }
                FlowChange::Clear { path, previous } => {
                    // restore the cleared narrowing list
                    self.narrowings.insert(*path, previous);
                }
                FlowChange::Diverge => {}
            }
        }

        // return the cursor to the checkpoint's point
        self.current = checkpoint.point;
    }

    /// Restore one branch from its checkpoint.
    pub(in crate::sema) fn restore_branch(
        &mut self,
        checkpoint: FlowCheckpoint,
        branch: &FlowBranch,
    ) {
        self.restore(checkpoint);

        // apply assigned places from the branch
        for place in &branch.assigned {
            self.insert_assigned(*place);
        }

        // apply narrowings from the branch
        for (path, narrowings) in &branch.narrowings {
            self.set_narrowings(path.clone(), narrowings);
        }
    }

    /// Merge two completed branches after one checkpoint.
    pub(in crate::sema) fn merge_branches(
        &mut self,
        checkpoint: FlowCheckpoint,
        left: &FlowBranch,
        right: &FlowBranch,
    ) {
        self.restore(checkpoint);

        // continue the other branch alone past a diverging one
        match (left.diverges, right.diverges) {
            (true, true) => {
                self.insert_diverge();

                return;
            }
            (true, false) => return self.restore_branch(checkpoint, right),
            (false, true) => return self.restore_branch(checkpoint, left),
            (false, false) => {}
        }

        // keep places assigned by both branches
        for place in left.assigned.intersection(&right.assigned) {
            self.insert_assigned(*place);
        }

        // collect all touched narrowing paths
        let mut paths = FxIndexSet::default();
        paths.extend(left.narrowings.keys().cloned());
        paths.extend(right.narrowings.keys().cloned());

        // keep narrowings with equal final values in both branches
        for path in paths {
            // read final value from each branch
            let inherited = self.narrowings.get(&path).cloned().unwrap_or_default();
            let left = left
                .narrowings
                .get(&path)
                .cloned()
                .unwrap_or_else(|| inherited.clone());
            let right = right.narrowings.get(&path).cloned().unwrap_or(inherited);

            // preserve equal branch results
            if left == right {
                self.set_narrowings(path, &left);
            }
            // clear diverging branch results
            else {
                self.set_narrowings(path, &[]);
            }
        }
    }

    /// Replace the current narrowings of one path.
    fn set_narrowings(&mut self, path: dir::AccessPath, target: &[FlowPredicate]) {
        // leave an unchanged conjunction alone
        let current = self
            .narrowings
            .get(&path)
            .map_or(&[][..], SmallVec::as_slice);
        if current == target {
            return;
        }

        // append a target extending the current conjunction
        if target.starts_with(current) {
            let current_len = current.len();
            for predicate in target[current_len..].iter().copied() {
                self.apply_narrowing(path.clone(), predicate);
            }

            return;
        }

        // replace a changed conjunction in full
        self.clear_narrowings(path.clone());
        for predicate in target.iter().copied() {
            self.apply_narrowing(path.clone(), predicate);
        }
    }

    /// Clear one current narrowing list.
    fn clear_narrowings(&mut self, path: dir::AccessPath) {
        // stop where the path has no narrowing
        let previous = self.narrowings.shift_remove(&path).unwrap_or_default();
        if previous.is_empty() {
            return;
        }

        // record the clear in the change log and the durable graph
        self.changes.push(FlowChange::Clear {
            path: Box::new(path.clone()),
            previous,
        });
        self.push_point(FlowPointChange::Clear {
            path: Box::new(path.clone()),
        });
    }

    /// Clear narrowings below one mutated path.
    pub(in crate::sema) fn clear_narrowings_under(&mut self, path: &dir::AccessPath) {
        // collect invalidated paths before mutating the map
        let paths: Vec<_> = self
            .narrowings
            .keys()
            .filter(|narrowed| narrowed.starts_with(path))
            .cloned()
            .collect();

        // clear each invalidated narrowing
        for path in paths {
            self.clear_narrowings(path);
        }
    }
}
