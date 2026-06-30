use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Dependency, Origin, Relation, SolverSnapshot, TypeMark, TypeSubstitution,
};

/// Snapshot of check state before one probe.
#[derive(Debug)]
pub(in crate::check) struct ProbeSnapshot {
    /// The solver state before the probe.
    solver: SolverSnapshot,
    /// Layout segment marks before the probe.
    layouts: LayoutMark,
}

/// Layout mark for all loaded module layout segments.
#[derive(Debug, PartialEq, Eq)]
struct LayoutMark {
    /// Layout and type-layout counts for each loaded layout segment.
    modules: IndexMap<ModuleId, LayoutSegmentMark>,
}

/// Layout segment mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LayoutSegmentMark {
    /// The layout count before the probe.
    layouts: u32,
    /// The type-layout binding count before the probe.
    type_layouts: usize,
}

impl CheckState<'_> {
    /// Begin one probe transaction.
    pub(in crate::check) fn begin_probe(&mut self) -> ProbeSnapshot {
        let marks = self
            .modules
            .iter()
            .map(|(module, state)| (*module, state.types.type_count()))
            .collect();
        let solver = self.solver.snapshot(TypeMark::new(marks));
        let layouts = LayoutMark::new(&self.layouts);

        ProbeSnapshot { solver, layouts }
    }

    /// Roll back one probe transaction.
    pub(in crate::check) fn reject_probe(&mut self, snapshot: ProbeSnapshot) {
        let marks = self.solver.rollback(snapshot.solver);
        self.drop_probe_layouts(snapshot.layouts);
        self.drop_probe_types(marks);
    }

    /// Commit one probe transaction.
    pub(in crate::check) fn commit_probe(&mut self, snapshot: ProbeSnapshot) {
        self.solver.commit(snapshot.solver);
    }

    /// Return dependencies still live after a probe was rejected.
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

    /// Solve inference variables selected inside the active probe.
    pub(in crate::check) fn solve_probe_variables(
        &mut self,
        variables: impl IntoIterator<Item = dir::TypeVariableId>,
    ) -> CompilerResult<Answer<bool>> {
        let mut all_bounds_hold = true;
        let mut pending = SmallVec::<[Dependency; 2]>::new();
        let mut queue = variables.into_iter().collect::<Vec<_>>();

        while let Some(variable) = queue.pop() {
            match self.solve_variable(variable)? {
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

    /// Infer probe variables from an expected type.
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

    /// Return inference variables referenced by one substitution.
    pub(in crate::check) fn substitution_variables(
        &mut self,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 4]>> {
        let mut variables = SmallVec::new();
        for argument in substitution.arguments.iter().rev().copied() {
            if let Some(variable) = self.root_variable(argument)? {
                variables.push(variable);
            }
        }

        Ok(variables)
    }

    /// Drop probe-local types.
    fn drop_probe_types(&mut self, marks: TypeMark) {
        for (module, count) in marks.iter() {
            if let Some(state) = self.modules.get_mut(&module) {
                state.types.truncate_types(count);
            }
        }
    }

    /// Drop probe-local layouts.
    fn drop_probe_layouts(&mut self, marks: LayoutMark) {
        self.layouts
            .retain(|module, _| marks.modules.contains_key(module));

        for (module, mark) in marks.modules {
            let Some(segment) = self.layouts.get_mut(&module) else {
                continue;
            };
            segment.truncate_layouts(mark.layouts, mark.type_layouts);
        }
    }
}

impl LayoutMark {
    /// Mark every loaded layout segment.
    fn new(layouts: &IndexMap<ModuleId, dir::LayoutSegment>) -> Self {
        let modules = layouts
            .iter()
            .map(|(module, segment)| {
                (
                    *module,
                    LayoutSegmentMark {
                        layouts: segment.layout_count(),
                        type_layouts: segment.type_layout_count(),
                    },
                )
            })
            .collect();

        Self { modules }
    }
}
