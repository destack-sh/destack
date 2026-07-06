use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Dependency, Origin, Relation, SolveMode, SolverSnapshot};

/// Check state mark before one probe.
#[derive(Debug)]
pub(in crate::check) struct Probe {
    /// The solver state before the probe.
    solver: SolverSnapshot,
    /// The node type count before the probe.
    node_types: usize,
    /// The decision count before the probe.
    decisions: usize,
    /// The event count before the probe.
    events: usize,
    /// Module marks before the probe.
    modules: IndexMap<ModuleId, ModuleProbeMark>,
}

/// Per-module check state mark before one probe.
#[derive(Debug)]
struct ModuleProbeMark {
    /// Working type segment mark before the probe.
    types: dir::TypeMark,
    /// Diagnostic count before the probe.
    diagnostics: usize,
    /// Warning count before the probe.
    warnings: usize,
}

impl CheckState<'_> {
    /// Run one speculative check and keep it only when its ready value is accepted.
    pub(in crate::check) fn probe_accept<T>(
        &mut self,
        attempt: impl FnOnce(&mut Self) -> CompilerResult<Answer<T>>,
        accept: impl FnOnce(&T) -> bool,
    ) -> CompilerResult<Answer<T>> {
        let probe = self.begin_probe();
        let result = attempt(self);

        match result {
            Ok(Answer::Ready(value)) if accept(&value) => {
                self.commit_probe(probe);

                Ok(Answer::Ready(value))
            }
            Ok(Answer::Ready(value)) => {
                self.reject_probe(probe);

                Ok(Answer::Ready(value))
            }
            Ok(Answer::Pending(blockers)) => {
                self.reject_probe(probe);

                Ok(Answer::Pending(self.live_blockers(blockers)))
            }
            Err(error) => {
                self.reject_probe(probe);

                Err(error)
            }
        }
    }

    /// Run one speculative check and keep accepted ready state or pending state.
    pub(in crate::check) fn probe_accept_or_pending<T>(
        &mut self,
        attempt: impl FnOnce(&mut Self) -> CompilerResult<Answer<T>>,
        accept: impl FnOnce(&T) -> bool,
    ) -> CompilerResult<Answer<T>> {
        let probe = self.begin_probe();
        let result = attempt(self);

        match result {
            Ok(Answer::Ready(value)) if accept(&value) => {
                self.commit_probe(probe);

                Ok(Answer::Ready(value))
            }
            Ok(Answer::Ready(value)) => {
                self.reject_probe(probe);

                Ok(Answer::Ready(value))
            }
            Ok(Answer::Pending(blockers)) => {
                self.commit_probe(probe);

                Ok(Answer::Pending(blockers))
            }
            Err(error) => {
                self.reject_probe(probe);

                Err(error)
            }
        }
    }

    /// Begin one probe.
    fn begin_probe(&mut self) -> Probe {
        let modules = self
            .modules
            .iter()
            .map(|(module, state)| {
                (
                    *module,
                    ModuleProbeMark {
                        types: state.types_tail.mark(),
                        diagnostics: state.diagnostics.len(),
                        warnings: state.warnings.len(),
                    },
                )
            })
            .collect();
        let solver = self.solver.snapshot();

        Probe {
            solver,
            node_types: self.node_types.len(),
            decisions: self.decisions.count(),
            events: self.events.len(),
            modules,
        }
    }

    /// Roll back one rejected probe.
    fn reject_probe(&mut self, snapshot: Probe) {
        let Probe {
            solver,
            node_types,
            decisions,
            events,
            modules,
        } = snapshot;

        self.solver.rollback(solver);
        self.drop_probe_state(node_types, decisions, events, modules);
    }

    /// Commit one accepted probe.
    fn commit_probe(&mut self, snapshot: Probe) {
        self.solver.commit(snapshot.solver);
    }

    /// Return dependencies that survived a rejected probe.
    pub(in crate::check) fn live_blockers(
        &self,
        blockers: SmallVec<[Dependency; 2]>,
    ) -> SmallVec<[Dependency; 2]> {
        let mut live = SmallVec::<[Dependency; 2]>::new();
        for blocker in blockers {
            match blocker {
                Dependency::Variable(variable) => {
                    let is_live = self
                        .solver
                        .solution(variable)
                        .is_ok_and(|solution| solution.is_none());
                    if is_live && !live.contains(&blocker) {
                        live.push(blocker);
                    }
                }
                Dependency::NodeType(node) => {
                    if self.node_type_maybe(node).is_none() && !live.contains(&blocker) {
                        live.push(blocker);
                    }
                }
                Dependency::SymbolType(symbol) => {
                    if self.symbol_type_maybe(symbol).is_none() && !live.contains(&blocker) {
                        live.push(blocker);
                    }
                }
                Dependency::Decision(node) => {
                    if self.decision(node).is_none() && !live.contains(&blocker) {
                        live.push(blocker);
                    }
                }
            }
        }

        live
    }

    /// Solve inference variables opened inside the active probe.
    pub(in crate::check) fn solve_probe_variables(
        &mut self,
        variables: impl IntoIterator<Item = dir::TypeVariableId>,
    ) -> CompilerResult<Answer<bool>> {
        let mut all_bounds_hold = true;
        let mut pending = SmallVec::<[Dependency; 2]>::new();
        let mut queue = variables.into_iter().collect::<Vec<_>>();
        let mut attempted = indexmap::IndexSet::new();

        while let Some(variable) = queue.pop() {
            // attempt each variable once: revisits are inference cycles
            if !attempted.insert(variable) {
                let blocker = Dependency::Variable(variable);
                if !pending.contains(&blocker) {
                    pending.push(blocker);
                }

                continue;
            }

            match self.solve_variable(variable, SolveMode::Weak)? {
                Answer::Ready(holds) => all_bounds_hold &= holds,
                Answer::Pending(blockers) => {
                    let mut blocked_variables = SmallVec::<[dir::TypeVariableId; 2]>::new();
                    for blocker in blockers {
                        match blocker {
                            Dependency::Variable(blocker) => {
                                let blocker = self.solver.representative(blocker)?;
                                let solution = self.solver.solution(blocker)?;
                                if solution.is_none() && !blocked_variables.contains(&blocker) {
                                    blocked_variables.push(blocker);
                                }
                            }
                            Dependency::NodeType(_)
                            | Dependency::SymbolType(_)
                            | Dependency::Decision(_) => {
                                if !pending.contains(&blocker) {
                                    pending.push(blocker);
                                }
                            }
                        }
                    }
                    if pending.is_empty() {
                        queue.extend(blocked_variables);
                    }
                }
            }
        }

        let pending = self.live_blockers(pending);

        Ok(Answer::ready_unless_blocked(all_bounds_hold, pending))
    }

    /// Try to infer variables from an expected type.
    pub(in crate::check) fn infer_from_expected(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        expected: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        match self.probe_accept(
            |state| state.constrain(origin, relation, source, expected),
            |holds| *holds,
        )? {
            Answer::Ready(true) => Ok(Answer::Ready(())),
            Answer::Ready(false) => Ok(Answer::Ready(())),
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Drop state allocated inside a rejected probe.
    fn drop_probe_state(
        &mut self,
        node_types: usize,
        decisions: usize,
        events: usize,
        modules: IndexMap<ModuleId, ModuleProbeMark>,
    ) {
        while self.node_types.len() > node_types {
            self.node_types.pop();
        }
        self.decisions.truncate_to(decisions);
        self.events.truncate(events);

        for (module, mark) in modules {
            if let Some(state) = self.modules.get_mut(&module) {
                state.types_tail.truncate_to(mark.types);
                state.diagnostics.truncate(mark.diagnostics);
                state.warnings.truncate(mark.warnings);
            }
        }
    }
}
