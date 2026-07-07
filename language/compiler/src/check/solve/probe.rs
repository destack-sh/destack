use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::{Answer, CheckState, Dependency, DumpContext, SolverSnapshot};
use crate::{CompilerError, CompilerResult};

/// One outcome from a speculative check.
enum ProbeOutcome<T> {
    /// Keep the speculative state and return the value.
    Commit(T),
    /// Roll back the speculative state and return the value.
    Reject(T),
    /// Roll back the speculative state and return outer dependencies.
    Pending(SmallVec<[Dependency; 2]>),
}

/// Check state mark before one probe.
#[derive(Debug)]
struct Probe {
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
    fn probe<T>(
        &mut self,
        attempt: impl FnOnce(&mut Self) -> CompilerResult<ProbeOutcome<T>>,
    ) -> CompilerResult<Answer<T>> {
        let probe = self.begin_probe();
        let result = attempt(self);

        match result {
            Ok(ProbeOutcome::Commit(value)) => {
                self.commit_probe(probe);

                Ok(Answer::Ready(value))
            }
            Ok(ProbeOutcome::Reject(value)) => {
                self.reject_probe(probe);

                Ok(Answer::Ready(value))
            }
            Ok(ProbeOutcome::Pending(blockers)) => {
                self.require_outer_probe_blockers(&probe, &blockers)?;
                self.reject_probe(probe);

                Ok(Answer::Pending(blockers))
            }
            Err(error) => {
                self.reject_probe(probe);

                Err(error)
            }
        }
    }

    /// Run one speculative candidate.
    pub(in crate::check) fn probe_candidate<T>(
        &mut self,
        attempt: impl FnOnce(&mut Self) -> CompilerResult<Answer<T>>,
        accept: impl FnOnce(&T) -> bool,
    ) -> CompilerResult<Answer<T>> {
        self.probe(|state| {
            let answer = attempt(state)?;
            let outcome = match answer {
                Answer::Ready(value) if accept(&value) => ProbeOutcome::Commit(value),
                Answer::Ready(value) => ProbeOutcome::Reject(value),
                Answer::Pending(blockers) => ProbeOutcome::Pending(blockers),
            };

            Ok(outcome)
        })
    }

    /// Require blockers that survive one rejected probe to come from outside it.
    fn require_outer_probe_blockers(
        &self,
        probe: &Probe,
        blockers: &[Dependency],
    ) -> CompilerResult<()> {
        let local_variables = blockers
            .iter()
            .filter_map(|blocker| match blocker {
                Dependency::Variable(variable) if !probe.solver.contains_variable(*variable) => {
                    Some(*variable)
                }
                _ => None,
            })
            .collect::<SmallVec<[dir::TypeVariableId; 2]>>();
        if local_variables.is_empty() {
            return Ok(());
        }

        // include bound origins so the failed candidate owner is visible
        let context = DumpContext::new(self);
        let variables = local_variables
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

        Err(CompilerError::Internal {
            message: format!(
                "probe returned blockers on local variables {local_variables:?}: {variables}"
            ),
        })
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
