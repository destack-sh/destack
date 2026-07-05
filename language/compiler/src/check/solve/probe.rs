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
    /// The decision count before the probe, asserted stable on rejection.
    decisions: usize,
    /// Working type segment marks for each loaded module before the probe.
    types: IndexMap<ModuleId, dir::TypeMark>,
}

impl CheckState<'_> {
    /// Begin one probe.
    pub(in crate::check) fn begin_probe(&mut self) -> Probe {
        let types = self
            .modules
            .iter()
            .map(|(module, state)| (*module, state.types_tail.mark()))
            .collect();
        let solver = self.solver.snapshot();

        Probe {
            solver,
            decisions: self.decisions.count(),
            types,
        }
    }

    /// Roll back one rejected probe.
    pub(in crate::check) fn reject_probe(&mut self, snapshot: Probe) {
        debug_assert_eq!(
            snapshot.decisions,
            self.decisions.count(),
            "a rejected probe may not commit node decisions"
        );
        self.solver.rollback(snapshot.solver);
        self.drop_probe_types(snapshot.types);
    }

    /// Commit one accepted probe.
    pub(in crate::check) fn commit_probe(&mut self, snapshot: Probe) {
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
                    if self.committed_node_type_maybe(node).is_none() && !live.contains(&blocker) {
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
        variables: impl IntoIterator<Item = dir::TypeVariableId>,
    ) -> CompilerResult<Answer<()>> {
        let variables = variables.into_iter().collect::<SmallVec<[_; 4]>>();
        if variables.is_empty() {
            return Ok(Answer::Ready(()));
        }

        let probe = self.begin_probe();
        match self.constrain(origin, relation, source, expected)? {
            Answer::Ready(true) => {}
            Answer::Ready(false) => {
                self.reject_probe(probe);

                return Ok(Answer::Ready(()));
            }
            Answer::Pending(blockers) => {
                self.reject_probe(probe);

                return Ok(Answer::Pending(self.live_blockers(blockers)));
            }
        }

        match self.solve_probe_variables(variables)? {
            Answer::Ready(true) => {
                self.commit_probe(probe);

                Ok(Answer::Ready(()))
            }
            Answer::Ready(false) => {
                self.reject_probe(probe);

                Ok(Answer::Ready(()))
            }
            Answer::Pending(blockers) => {
                self.reject_probe(probe);

                Ok(Answer::Pending(self.live_blockers(blockers)))
            }
        }
    }

    /// Drop types allocated inside a rejected probe.
    fn drop_probe_types(&mut self, marks: IndexMap<ModuleId, dir::TypeMark>) {
        for (module, mark) in marks {
            if let Some(state) = self.modules.get_mut(&module) {
                state.types_tail.truncate_to(mark);
            }
        }
    }
}
