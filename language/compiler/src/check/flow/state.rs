use destack_dir as dir;
use indexmap::{IndexMap, IndexSet};

use crate::check::{
    Capture, ControlTarget, FlowPath, FunctionFrame, ReceiverCapture, TryTarget, VariableId,
};

/// Flow state while walking one module.
#[derive(Debug, Default)]
pub(in crate::check) struct FlowState {
    /// Function bodies currently being walked.
    pub(in crate::check) functions: Vec<FunctionFrame>,
    /// Control targets currently visible to `break` and `continue`.
    pub(in crate::check) targets: Vec<ControlTarget>,
    /// Try targets currently visible to `?`.
    pub(in crate::check) tries: Vec<TryTarget>,
    /// Local symbols definitely assigned at the current walk point.
    pub(in crate::check) assigned_symbols: IndexSet<dir::GlobalSymbolId>,
    /// Narrowed type variables keyed by flow path.
    pub(in crate::check) narrowings: IndexMap<FlowPath, VariableId>,
    /// Flow mutations made since walking started.
    changes: Vec<FlowChange>,
}

/// A checkpoint in the flow mutation log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct FlowCheckpoint {
    /// The number of mutations visible at the checkpoint.
    change_count: usize,
}

/// Flow changes produced by one branch after a checkpoint.
#[derive(Debug, Clone)]
pub(in crate::check) struct FlowBranch {
    /// Symbols assigned by this branch.
    assigned_symbols: IndexSet<dir::GlobalSymbolId>,
    /// Narrowings touched by this branch.
    narrowings: IndexMap<FlowPath, Option<VariableId>>,
}

/// One reversible flow mutation.
#[derive(Debug, Clone)]
enum FlowChange {
    /// One definite assignment change.
    Assigned {
        /// The assigned symbol.
        symbol: dir::GlobalSymbolId,
        /// Whether the symbol was already assigned.
        was_assigned: bool,
    },
    /// One narrowing change.
    Narrowing {
        /// The narrowed path.
        path: FlowPath,
        /// The previous narrowing at the same path.
        previous: Option<VariableId>,
    },
}

impl FlowState {
    /// Enter one function body while walking.
    pub(in crate::check) fn push_function(&mut self, function: FunctionFrame) {
        self.functions.push(function);
    }

    /// Leave the current function body.
    pub(in crate::check) fn pop_function(&mut self) -> Option<Capture> {
        let function = self.functions.pop()?;
        let symbols = function.captured_symbols.iter().copied().collect();
        let capture = Capture {
            symbol: function.symbol,
            symbols,
            receiver: function.captured_receiver,
            directive: None,
        };

        self.restore(function.checkpoint);

        Some(capture)
    }

    /// Return the current function body.
    pub(in crate::check) fn current_function(&self) -> Option<&FunctionFrame> {
        self.functions.last()
    }

    /// Enter one break or continue target.
    pub(in crate::check) fn push_target(&mut self, target: ControlTarget) {
        self.targets.push(target);
    }

    /// Leave the current break or continue target.
    pub(in crate::check) fn pop_target(&mut self) -> Option<ControlTarget> {
        self.targets.pop()
    }

    /// Enter one try failure target.
    pub(in crate::check) fn push_try(&mut self, target: TryTarget) {
        self.tries.push(target);
    }

    /// Leave the current try failure target.
    pub(in crate::check) fn pop_try(&mut self) -> Option<TryTarget> {
        self.tries.pop()
    }

    /// Return the current try failure target.
    pub(in crate::check) fn current_try_mut(&mut self) -> Option<&mut TryTarget> {
        self.tries.last_mut()
    }

    /// Return the target index selected by one break.
    pub(in crate::check) fn find_break_target_index(
        &self,
        label: Option<dir::StringId>,
    ) -> Option<usize> {
        self.targets
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, target)| {
                if let Some(label) = label {
                    (target.label == Some(label)).then_some(index)
                } else {
                    (target.label.is_none() && target.allows_continue).then_some(index)
                }
            })
    }

    /// Return the target index selected by one continue.
    pub(in crate::check) fn find_continue_target_index(
        &self,
        label: Option<dir::StringId>,
    ) -> Option<usize> {
        self.targets
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, target)| {
                if let Some(label) = label {
                    (target.label == Some(label) && target.allows_continue).then_some(index)
                } else {
                    (target.label.is_none() && target.allows_continue).then_some(index)
                }
            })
    }

    /// Record one symbol captured by the current function body.
    pub(in crate::check) fn capture_symbol(&mut self, symbol: dir::GlobalSymbolId) {
        if let Some(function) = self.functions.last_mut() {
            function.captured_symbols.insert(symbol);
        }
    }

    /// Record one receiver captured by the current function body.
    pub(in crate::check) fn capture_receiver(&mut self, receiver: ReceiverCapture) {
        if let Some(function) = self.functions.last_mut() {
            function.captured_receiver = Some(receiver);
        }
    }

    /// Return the lexical receiver visible to the current function.
    pub(in crate::check) fn lexical_receiver(&self) -> Option<(usize, ReceiverCapture)> {
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

    /// Mark one local symbol as definitely assigned.
    pub(in crate::check) fn mark_assigned(&mut self, symbol: dir::GlobalSymbolId) {
        let was_assigned = self.assigned_symbols.contains(&symbol);

        self.changes.push(FlowChange::Assigned {
            symbol,
            was_assigned,
        });
        self.assigned_symbols.insert(symbol);
    }

    /// Narrow one path at the current walk point.
    pub(in crate::check) fn narrow(&mut self, path: FlowPath, ty: VariableId) {
        let previous = self.narrowings.get(&path).copied();

        self.changes.push(FlowChange::Narrowing {
            path: path.clone(),
            previous,
        });
        self.narrowings.insert(path, ty);
    }

    /// Return a checkpoint for later branch rollback.
    pub(in crate::check) fn checkpoint(&self) -> FlowCheckpoint {
        FlowCheckpoint {
            change_count: self.changes.len(),
        }
    }

    /// Return the branch changes made after one checkpoint.
    pub(in crate::check) fn branch(&self, checkpoint: FlowCheckpoint) -> FlowBranch {
        let mut assigned_symbols = IndexSet::new();
        let mut narrowing_paths = IndexSet::new();

        // collect facts touched since the checkpoint
        for change in &self.changes[checkpoint.change_count..] {
            match change {
                FlowChange::Assigned { symbol, .. } => {
                    if self.assigned_symbols.contains(symbol) {
                        assigned_symbols.insert(*symbol);
                    }
                }
                FlowChange::Narrowing { path, .. } => {
                    narrowing_paths.insert(path.clone());
                }
            }
        }

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
        while self.changes.len() > checkpoint.change_count {
            let Some(change) = self.changes.pop() else {
                break;
            };

            // undo the latest mutation
            match change {
                FlowChange::Assigned {
                    symbol,
                    was_assigned,
                } => {
                    if was_assigned {
                        self.assigned_symbols.insert(symbol);
                    } else {
                        self.assigned_symbols.shift_remove(&symbol);
                    }
                }
                FlowChange::Narrowing { path, previous } => {
                    if let Some(previous) = previous {
                        self.narrowings.insert(path, previous);
                    } else {
                        self.narrowings.shift_remove(&path);
                    }
                }
            }
        }
    }

    /// Apply one branch after restoring its checkpoint.
    pub(in crate::check) fn apply_branch(
        &mut self,
        checkpoint: FlowCheckpoint,
        branch: &FlowBranch,
    ) {
        self.restore(checkpoint);

        // replay assigned facts
        for symbol in &branch.assigned_symbols {
            self.mark_assigned(*symbol);
        }

        // replay narrowed facts
        for (path, narrowing) in &branch.narrowings {
            if let Some(narrowing) = narrowing {
                self.narrow(path.clone(), *narrowing);
            } else {
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

        let mut paths = IndexSet::new();
        paths.extend(left.narrowings.keys().cloned());
        paths.extend(right.narrowings.keys().cloned());

        // keep narrowings with equal final values in both branches
        for path in paths {
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
            if left == right {
                if let Some(narrowing) = left {
                    self.narrow(path, narrowing);
                } else {
                    self.clear_narrowing(path);
                }
            } else {
                self.clear_narrowing(path);
            }
        }
    }

    /// Clear one current narrowing.
    fn clear_narrowing(&mut self, path: FlowPath) {
        let previous = self.narrowings.get(&path).copied();

        self.changes.push(FlowChange::Narrowing {
            path: path.clone(),
            previous,
        });
        self.narrowings.shift_remove(&path);
    }

    /// Clear narrowings below one written path.
    pub(in crate::check) fn clear_narrowings_under(&mut self, path: &FlowPath) {
        let paths: Vec<_> = self
            .narrowings
            .keys()
            .filter(|narrowing| narrowing.starts_with(path))
            .cloned()
            .collect();

        for path in paths {
            self.clear_narrowing(path);
        }
    }
}
