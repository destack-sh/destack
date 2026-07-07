use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::{
    Answer, BoundMode, CheckState, Dependency, DumpContext, Relation, SolverSnapshot,
};
use crate::{CompilerError, CompilerResult};

/// Active speculative probe.
pub(in crate::check) struct Probe<'a, 'check> {
    /// The checked state under this probe.
    pub(in crate::check) state: &'a mut CheckState<'check>,
    /// The check state mark before this probe.
    mark: &'a ProbeMark,
}

/// Check state mark before one probe.
#[derive(Debug)]
struct ProbeMark {
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

impl<'a, 'check> Probe<'a, 'check> {
    /// Run one probe operation until it is ready or blocked outside this probe.
    pub(in crate::check) fn settle<T>(
        &mut self,
        mut attempt: impl FnMut(&mut Self) -> CompilerResult<Answer<T>>,
    ) -> CompilerResult<Answer<T>> {
        loop {
            let blockers = match attempt(self)? {
                Answer::Ready(value) => return Ok(Answer::Ready(value)),
                Answer::Pending(blockers) => blockers,
            };

            let local = self.local_variables(&blockers)?;
            if local.is_empty() {
                return Ok(Answer::Pending(blockers));
            }

            self.solve_variables(&local)?;
        }
    }

    /// Relate two types inside this probe.
    pub(in crate::check) fn constrain(
        &mut self,
        origin: crate::check::Origin,
        relation: Relation,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        self.settle(|probe| probe.state.constrain(origin, relation, left, right))
    }

    /// Return open variables allocated inside this probe.
    fn local_variables(
        &self,
        blockers: &[Dependency],
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 2]>> {
        let mut variables = SmallVec::<[dir::TypeVariableId; 2]>::new();

        for blocker in blockers {
            let Dependency::Variable(variable) = blocker else {
                continue;
            };
            let variable = self.state.solver.representative(*variable)?;
            if self.mark.solver.contains_variable(variable) {
                continue;
            }
            if self.state.solver.variable(variable)?.solution.is_none()
                && !variables.contains(&variable)
            {
                variables.push(variable);
            }
        }

        Ok(variables)
    }

    /// Solve probe-local variables once.
    fn solve_variables(&mut self, variables: &[dir::TypeVariableId]) -> CompilerResult<()> {
        let mut progressed = false;

        for variable in variables {
            let before = self.state.open_variable(*variable)?;
            match self.state.solve_variable(*variable, BoundMode::Weak)? {
                Answer::Ready(_) => {}
                Answer::Pending(_) => {}
            }
            let after = self.state.open_variable(*variable)?;
            progressed |= before != after;
        }

        if progressed {
            Ok(())
        } else {
            Err(self.state.local_probe_blocker_error(&variables))
        }
    }
}

impl CheckState<'_> {
    /// Run one speculative candidate.
    pub(in crate::check) fn probe_candidate<T>(
        &mut self,
        mut attempt: impl FnMut(&mut Probe<'_, '_>) -> CompilerResult<Answer<T>>,
        accept: impl Fn(&T) -> bool,
    ) -> CompilerResult<Answer<T>> {
        let mark = self.begin_probe();

        let answer = {
            let mut probe = Probe {
                state: self,
                mark: &mark,
            };

            probe.settle(|probe| attempt(probe))
        };
        let answer = match answer {
            Ok(answer) => answer,
            Err(error) => {
                self.reject_probe(mark);

                return Err(error);
            }
        };

        match answer {
            Answer::Ready(value) if accept(&value) => {
                self.commit_probe(mark);

                Ok(Answer::Ready(value))
            }
            Answer::Ready(value) => {
                self.reject_probe(mark);

                Ok(Answer::Ready(value))
            }
            Answer::Pending(blockers) => {
                self.require_outer_probe_blockers(&mark, &blockers)?;

                self.reject_probe(mark);

                Ok(Answer::Pending(blockers))
            }
        }
    }

    /// Return one internal error for candidate blockers that cannot settle.
    fn local_probe_blocker_error(&self, variables: &[dir::TypeVariableId]) -> CompilerError {
        // include bound origins so the failed candidate owner is visible
        let context = DumpContext::new(self);
        let variables = variables
            .iter()
            .map(|variable| match self.solver.variable(*variable) {
                Ok(state) => {
                    let source = context.origin_source_label(state.origin);
                    let lower = state
                        .lower
                        .iter()
                        .map(|bound| self.format_type(bound.ty))
                        .collect::<Vec<_>>();
                    let upper = state
                        .upper
                        .iter()
                        .map(|bound| self.format_type(bound.ty))
                        .collect::<Vec<_>>();
                    format!(
                        "{variable:?}=source({source}) lower({lower:?}) upper({upper:?}) state({state:?})"
                    )
                }
                Err(error) => format!("{variable:?}=<error {error:?}>"),
            })
            .collect::<Vec<_>>()
            .join(", ");

        CompilerError::Internal {
            message: format!("probe could not settle local variables: {variables}"),
        }
    }

    /// Require blockers that survive one rejected probe to come from outside it.
    fn require_outer_probe_blockers(
        &self,
        probe: &ProbeMark,
        blockers: &[Dependency],
    ) -> CompilerResult<()> {
        let variables = blockers
            .iter()
            .filter_map(|blocker| match blocker {
                Dependency::Variable(variable) if !probe.solver.contains_variable(*variable) => {
                    Some(*variable)
                }
                _ => None,
            })
            .collect::<SmallVec<[dir::TypeVariableId; 2]>>();
        if variables.is_empty() {
            Ok(())
        } else {
            Err(self.local_probe_blocker_error(&variables))
        }
    }

    /// Begin one probe.
    fn begin_probe(&mut self) -> ProbeMark {
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

        ProbeMark {
            solver,
            node_types: self.node_types.len(),
            decisions: self.decisions.count(),
            events: self.events.len(),
            modules,
        }
    }

    /// Roll back one rejected probe.
    fn reject_probe(&mut self, snapshot: ProbeMark) {
        let ProbeMark {
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
    fn commit_probe(&mut self, snapshot: ProbeMark) {
        self.solver.commit(snapshot.solver);
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
