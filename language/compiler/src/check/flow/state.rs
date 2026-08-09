use std::mem::take;

use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Capture, ControlTarget, ControlTargetForm, FunctionFrame, Receiver, ReceiverBinding, TryTarget,
};

/// Flow state while walking one module.
#[derive(Debug)]
pub(in crate::check) struct FlowState {
    /// Function bodies currently being walked.
    pub(in crate::check::flow) functions: Vec<FunctionFrame>,
    /// Contextual receiver scopes currently visible outside function bodies.
    pub(in crate::check::flow) receivers: Vec<Option<Receiver>>,
    /// Generic template scopes enclosing the current flow point.
    pub(in crate::check::flow) template_scopes: Vec<dir::GlobalGenericTemplateId>,
    /// Control targets currently visible to `break` and `continue`.
    pub(in crate::check::flow) targets: Vec<ControlTarget>,
    /// Try targets currently visible to `?`.
    pub(in crate::check::flow) tries: Vec<TryTarget>,
    /// Places definitely assigned at the current flow point.
    pub(in crate::check::flow) assigned: FxIndexSet<AssignedPlace>,
    /// Places moved out at the current flow point, keyed to their move site.
    pub(in crate::check::flow) moved: FxIndexMap<AssignedPlace, MoveSite>,
    /// Flow narrowings keyed by static path.
    pub(in crate::check::flow) narrowings:
        FxIndexMap<dir::AccessPath, SmallVec<[FlowPredicate; 2]>>,
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
            moved: FxIndexMap::default(),
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
pub(in crate::check) struct FlowPointId {
    /// The point index in the flow point table.
    index: u32,
}

impl FlowPointId {
    /// The root flow point.
    const ROOT: Self = Self { index: 0 };

    /// Return the index in the module flow table.
    pub(in crate::check) fn index(self) -> usize {
        self.index as usize
    }
}

/// One durable flow point.
#[derive(Debug, Clone)]
pub(in crate::check) struct FlowPoint {
    /// The previous point in this flow path.
    pub(in crate::check) parent: Option<FlowPointId>,
    /// The change applied at this point.
    pub(in crate::check) change: FlowPointChange,
}

/// One durable flow point change.
#[derive(Debug, Clone)]
pub(in crate::check) enum FlowPointChange {
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
pub(in crate::check) struct FlowCheckpoint {
    /// The number of changes visible at the checkpoint.
    change_count: usize,
    /// The durable flow point visible at the checkpoint.
    point: FlowPointId,
}

/// Flow changes produced by one branch after a checkpoint.
#[derive(Debug, Clone, PartialEq, Default)]
pub(in crate::check) struct FlowBranch {
    /// Places assigned by this branch.
    assigned: FxIndexSet<AssignedPlace>,
    /// Places moved out by this branch, keyed to their move site.
    moved: FxIndexMap<AssignedPlace, MoveSite>,
    /// Narrowings touched by this branch.
    narrowings: FxIndexMap<dir::AccessPath, SmallVec<[FlowPredicate; 2]>>,
}

impl FlowBranch {
    /// Return whether this branch assigns one place.
    pub(in crate::check) fn assigns(&self, place: AssignedPlace) -> bool {
        self.assigned.contains(&place)
    }
}

/// One place assignment tracked by flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum AssignedPlace {
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

/// The source position that moved one place.
///
/// Marks are syntactic; obligations decide transfers from selected coercions and conformances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct MoveSite {
    /// The moving source expression.
    pub(in crate::check) node: dir::GlobalNodeIdAny,
    /// The enclosing call for argument and receiver positions.
    pub(in crate::check) call: Option<dir::GlobalNodeIdAny>,
    /// The receiving binding for initializer and assignment positions.
    pub(in crate::check) target: Option<dir::GlobalSymbolId>,
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
        /// The move displaced by the assignment, for rollback.
        was_moved: Option<MoveSite>,
    },
    /// One move-out change.
    Move {
        /// The moved place.
        place: AssignedPlace,
        /// The move already recorded for the place, for rollback.
        was_moved: Option<MoveSite>,
        /// Whether the place was assigned before the move.
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
}

/// One flow predicate recorded for a lexical access path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum FlowPredicate {
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
        /// Whether equal values reach this branch.
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
    /// Sync this walk's durable graph into one module flow table it owns.
    ///
    /// The body walk is the only writer of its module's flow store, so
    /// walk-local point ids stay durable identity ids.
    pub(in crate::check) fn sync_to(&self, points: &mut Vec<FlowPoint>) {
        while points.len() < self.points.len() {
            points.push(self.points[points.len()].clone());
        }
    }

    /// Restart the cursor context while keeping the durable point log.
    ///
    /// The next walk continues appending to the same log, so point ids
    /// stay durable identity ids without any rebasing.
    pub(in crate::check) fn reset_cursor(&mut self) {
        self.functions.clear();
        self.receivers.clear();
        self.template_scopes.clear();
        self.targets.clear();
        self.tries.clear();
        self.assigned.clear();
        self.moved.clear();
        self.narrowings.clear();
        self.unbound_jumps.clear();
        self.changes.clear();
        self.push_point(FlowPointChange::Start);
    }

    /// Return the current durable flow point.
    pub(in crate::check) fn point(&self) -> FlowPointId {
        self.current
    }

    /// Return the cursor's flow point graph.
    pub(in crate::check) fn points(&self) -> &[FlowPoint] {
        &self.points
    }

    /// Append one durable flow point.
    fn push_point(&mut self, change: FlowPointChange) {
        let point = FlowPointId {
            index: self.points.len() as u32,
        };
        self.points.push(FlowPoint {
            parent: Some(self.current),
            change,
        });
        self.current = point;
    }

    /// Mark one jump that bound no target.
    pub(in crate::check) fn mark_unbound_jump(&mut self, source: dir::LocalNodeIdAny) {
        self.unbound_jumps.insert(source);
    }

    /// Return whether one jump bound no target.
    pub(in crate::check) fn is_unbound_jump(&self, source: dir::LocalNodeIdAny) -> bool {
        self.unbound_jumps.contains(&source)
    }

    /// Enter one function body frame.
    pub(in crate::check) fn push_function(&mut self, function: FunctionFrame) {
        self.functions.push(function);
    }

    /// Leave the current function body.
    pub(in crate::check) fn pop_function(&mut self) -> (Capture, FlowBranch) {
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
    pub(in crate::check) fn current_function(&self) -> Option<&FunctionFrame> {
        self.functions.last()
    }

    /// Return the current function symbol.
    pub(in crate::check) fn current_function_symbol(&self) -> Option<dir::GlobalSymbolId> {
        self.functions.last().map(|function| function.symbol)
    }

    /// Enter one explicit contextual receiver scope.
    pub(in crate::check) fn push_receiver_scope(&mut self, receiver: Option<Receiver>) {
        self.receivers.push(receiver);
    }

    /// Enter one generic template scope.
    pub(in crate::check) fn push_template_scope(&mut self, template: dir::GlobalGenericTemplateId) {
        self.template_scopes.push(template);
    }

    /// Leave the current generic template scope.
    pub(in crate::check) fn pop_template_scope(&mut self) {
        if self.template_scopes.pop().is_none() {
            unreachable!("template scope stack underflow");
        }
    }

    /// Return the innermost generic template scope.
    pub(in crate::check) fn template_scope(&self) -> Option<dir::GlobalGenericTemplateId> {
        self.template_scopes.last().copied()
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

    /// Return the target index chosen by one break.
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
                    target.form.accepts_unlabeled_break().then_some(index)
                }
            })
    }

    /// Return the target index chosen by one continue.
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
                    (target.label == Some(label) && target.form.accepts_continue()).then_some(index)
                } else {
                    target.form.accepts_continue().then_some(index)
                }
            })
    }

    /// Return the control checkpoint chosen by one target index.
    pub(in crate::check) fn control_target_checkpoint(&self, index: usize) -> FlowCheckpoint {
        let target = &self.targets[index];

        target.checkpoint
    }

    /// Return the control form chosen by one target index.
    pub(in crate::check) fn control_target_form(&self, index: usize) -> ControlTargetForm {
        let target = &self.targets[index];

        target.form
    }

    /// Push one break branch onto a chosen control target.
    pub(in crate::check) fn push_break_branch(&mut self, index: usize, branch: FlowBranch) {
        self.targets[index].break_branches.push(branch);
    }

    /// Push one continue branch onto a chosen control target.
    pub(in crate::check) fn push_continue_branch(&mut self, index: usize, branch: FlowBranch) {
        self.targets[index].continue_branches.push(branch);
    }

    /// Take continue branches collected by the current control target.
    pub(in crate::check) fn take_continue_branches(&mut self) -> Vec<FlowBranch> {
        let Some(target) = self.targets.last_mut() else {
            unreachable!("continue branch collection requires an active control target");
        };

        take(&mut target.continue_branches)
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

    /// Mark one place as definitely assigned, reviving moved places.
    pub(in crate::check) fn mark_assigned(&mut self, place: AssignedPlace) {
        // record previous assignment state for rollback
        let was_assigned = self.assigned.contains(&place);
        let was_moved = self.moved.get(&place).copied();

        self.changes.push(FlowChange::Assign {
            place,
            was_assigned,
            was_moved,
        });
        self.assigned.insert(place);
        self.moved.shift_remove(&place);
    }

    /// Mark one place as moved out, clearing its assignment.
    pub(in crate::check) fn mark_moved(&mut self, place: AssignedPlace, site: MoveSite) {
        // record previous move and assignment state for rollback
        let was_moved = self.moved.get(&place).copied();
        let was_assigned = self.assigned.contains(&place);

        self.changes.push(FlowChange::Move {
            place,
            was_moved,
            was_assigned,
        });
        self.moved.insert(place, site);
        self.assigned.shift_remove(&place);
    }

    /// Return the recorded move site of one place at the current flow point.
    pub(in crate::check) fn moved_site(&self, place: AssignedPlace) -> Option<MoveSite> {
        self.moved.get(&place).copied()
    }

    /// Narrow one path at the current flow point.
    pub(in crate::check) fn apply_narrowing(
        &mut self,
        path: dir::AccessPath,
        predicate: FlowPredicate,
    ) {
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
    pub(in crate::check) fn fork(&self) -> FlowCheckpoint {
        FlowCheckpoint {
            change_count: self.changes.len(),
            point: self.current,
        }
    }

    /// Return the branch changes made after one checkpoint.
    pub(in crate::check) fn branch(&self, checkpoint: FlowCheckpoint) -> FlowBranch {
        let mut assigned = FxIndexSet::default();
        let mut narrowing_paths = FxIndexSet::default();

        // collect flow state touched since the checkpoint
        let mut moved = FxIndexMap::default();
        for change in &self.changes[checkpoint.change_count..] {
            match change {
                FlowChange::Assign { place, .. } | FlowChange::Move { place, .. } => {
                    if self.assigned.contains(place) {
                        assigned.insert(*place);
                    }
                    if let Some(site) = self.moved.get(place) {
                        moved.insert(*place, *site);
                    }
                }
                FlowChange::Narrowing { path, .. } => {
                    narrowing_paths.insert(path.as_ref().clone());
                }
                FlowChange::Clear { path, .. } => {
                    narrowing_paths.insert(path.as_ref().clone());
                }
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
            moved,
            narrowings,
        }
    }

    /// Restore the flow state to one checkpoint.
    pub(in crate::check) fn restore(&mut self, checkpoint: FlowCheckpoint) {
        let changes = self.changes.split_off(checkpoint.change_count);

        // roll back changes in reverse order
        for change in changes.into_iter().rev() {
            match change {
                FlowChange::Assign {
                    place,
                    was_assigned,
                    was_moved,
                } => {
                    // restore previous assignment and move state
                    if was_assigned {
                        self.assigned.insert(place);
                    } else {
                        self.assigned.shift_remove(&place);
                    }
                    if let Some(site) = was_moved {
                        self.moved.insert(place, site);
                    } else {
                        self.moved.shift_remove(&place);
                    }
                }
                FlowChange::Move {
                    place,
                    was_moved,
                    was_assigned,
                } => {
                    // restore previous move and assignment state
                    if let Some(site) = was_moved {
                        self.moved.insert(place, site);
                    } else {
                        self.moved.shift_remove(&place);
                    }
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
            }
        }
        self.current = checkpoint.point;
    }

    /// Restore one branch from its checkpoint.
    pub(in crate::check) fn restore_branch(
        &mut self,
        checkpoint: FlowCheckpoint,
        branch: &FlowBranch,
    ) {
        self.restore(checkpoint);

        // replay assigned places from the branch
        for place in &branch.assigned {
            self.mark_assigned(*place);
        }

        // replay moved places from the branch
        for (place, site) in &branch.moved {
            self.mark_moved(*place, *site);
        }

        // replay narrowings from the branch
        for (path, narrowings) in &branch.narrowings {
            self.set_narrowings(path.clone(), narrowings);
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

        // keep places assigned by both branches
        for place in left.assigned.intersection(&right.assigned) {
            self.mark_assigned(*place);
        }

        // keep every place moved by either branch moved
        for (place, site) in left.moved.iter().chain(&right.moved) {
            self.mark_moved(*place, *site);
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
        let previous = self.narrowings.shift_remove(&path).unwrap_or_default();
        if previous.is_empty() {
            return;
        }

        self.changes.push(FlowChange::Clear {
            path: Box::new(path.clone()),
            previous,
        });
        self.push_point(FlowPointChange::Clear {
            path: Box::new(path.clone()),
        });
    }

    /// Clear narrowings below one mutated path.
    pub(in crate::check) fn clear_narrowings_under(&mut self, path: &dir::AccessPath) {
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
