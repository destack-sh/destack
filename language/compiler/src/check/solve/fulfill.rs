use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    CauseId, CheckEvent, CheckFailure, CheckState, Constraint, ConstraintId, ConstraintResult,
    Expectation, FailedCheck, FlowSite, ObligationId, ObligationPhase, PlaceUse, Relation, Value,
    ValueUse, Verdict,
};

/// One unit of pending inference work.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum PendingWork {
    /// Check one collected constraint.
    Constraint(ConstraintId),
    /// Run one deferred check.
    Check(DeferredCheck),
    /// Check one collected obligation.
    Obligation(ObligationId),
}

/// One check deferred until its operands close.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum DeferredCheck {
    /// Infer one module-level expression.
    Infer {
        /// The expression site.
        site: FlowSite,
        /// The value use inferred at the site.
        use_: PlaceUse,
        /// The open variable the inference stalled on.
        stalled_on: Option<dir::TypeVariableId>,
    },
    /// Check one node against its walked expectation.
    Expect {
        /// The checked site.
        site: FlowSite,
        /// The expectation the walk recorded.
        expectation: Expectation,
    },
    /// Convert one checked value once its inference closes.
    Convert {
        /// The converted site.
        site: FlowSite,
        /// The checked source value.
        source: Value,
        /// The conversion expectation.
        expectation: Expectation,
    },
}

impl CheckState<'_> {
    /// Return whether the outermost close is judging every remainder.
    pub(in crate::check) fn is_final_round(&self) -> bool {
        self.infer.forcing && self.infer.scope_depth == 1
    }

    /// Register one check with fulfillment, keeping repeats single.
    pub(in crate::check) fn register_check(&mut self, check: DeferredCheck) {
        let work = PendingWork::Check(check);
        if !self.infer.pending.contains(&work) {
            self.infer.pending.push(work);
        }
    }

    /// Collect one candidate constraint and register it with fulfillment.
    pub(in crate::check) fn register_constraint(&mut self, constraint: Constraint) -> ConstraintId {
        let id = self.infer.allocate_constraint(constraint);
        self.infer.pending.push(PendingWork::Constraint(id));

        id
    }

    /// Collect one constraint and solve it in place.
    pub(in crate::check) fn push_constraint(
        &mut self,
        constraint: Constraint,
    ) -> CompilerResult<ConstraintId> {
        let id = self.infer.allocate_constraint(constraint);
        self.solve_constraint(id)?;

        Ok(id)
    }

    /// Run one solving round over the pending work.
    pub(in crate::check) fn solve_where_possible(&mut self) -> CompilerResult<bool> {
        // run deferred selections before the conversions and constraints reading their holes
        let mut pending = std::mem::take(&mut self.infer.pending);
        if self.infer.forcing {
            pending.sort_by_key(|work| {
                !matches!(work, PendingWork::Check(DeferredCheck::Infer { .. }))
            });
        }

        // step every item once, re-queueing whatever still stalls
        let mut round = false;
        for work in pending {
            round |= match work {
                PendingWork::Constraint(id) => self.step_constraint(id)?,
                PendingWork::Check(check) => self.step_check(check)?,
                PendingWork::Obligation(id) => self.step_obligation(id)?,
            };
        }

        Ok(round)
    }

    /// Step one pending constraint, returning whether it completed.
    fn step_constraint(&mut self, id: ConstraintId) -> CompilerResult<bool> {
        self.solve_constraint(id)?;

        Ok(self.infer.constraints.is_complete(id))
    }

    /// Step one deferred check, returning whether it ran.
    fn step_check(&mut self, check: DeferredCheck) -> CompilerResult<bool> {
        // wait for the blocker to solve, or for the final round to ask
        if self.defer_stalled_check(check)? {
            return Ok(false);
        }

        // hold an ambiguous conversion pending until the outermost close
        if self.defer_ambiguous_conversion(check)? {
            return Ok(false);
        }

        // run the check, which re-registers itself while its operands stay open
        self.run_pending_check(check)?;

        Ok(!self.infer.pending.contains(&PendingWork::Check(check)))
    }

    /// Re-queue one stalled selection while its blocker stays open.
    fn defer_stalled_check(&mut self, check: DeferredCheck) -> CompilerResult<bool> {
        let DeferredCheck::Infer {
            stalled_on: Some(stalled_on),
            ..
        } = check
        else {
            return Ok(false);
        };
        if self.infer.forcing {
            return Ok(false);
        }

        // wait for the blocker to solve
        let root = self.infer.alias_root(stalled_on)?;
        if !self.infer.variable(root)?.state.is_open() {
            return Ok(false);
        }
        self.infer.pending.push(PendingWork::Check(check));

        Ok(true)
    }

    /// Relate one deferred conversion, re-queueing it while it stays ambiguous.
    fn defer_ambiguous_conversion(&mut self, check: DeferredCheck) -> CompilerResult<bool> {
        let DeferredCheck::Convert {
            site,
            source,
            expectation,
        } = check
        else {
            return Ok(false);
        };
        if self.is_final_round() {
            return Ok(false);
        }

        // roll an ambiguous relation back and wait for the outermost close
        let work = PendingWork::Check(check);
        let mark = self.infer.mark();
        let verdict = self.constrain_relation(
            site.origin(),
            expectation.cause,
            expectation.relation,
            source.ty,
            expectation.target,
        )?;
        if verdict == Verdict::Ambiguous {
            let poison = self.intern_type(dir::Type::Error)?;
            self.infer.rollback(mark, poison)?;
            self.infer.pending.push(work);

            return Ok(true);
        }
        self.infer.commit(mark);

        // convert over solved operands only
        let operands = [source.ty, expectation.target];
        if !self.open_type_variables(operands)?.is_empty() && !self.is_final_round() {
            self.infer.pending.push(work);

            return Ok(true);
        }

        Ok(false)
    }

    /// Step one pending obligation, returning whether it judged.
    fn step_obligation(&mut self, id: ObligationId) -> CompilerResult<bool> {
        // complete obligations untouched outside checking
        if !self.is_checking() {
            return Ok(true);
        }

        // hold judge-phase obligations for the final round
        let phase = self.infer.obligations.get(id)?.obligation.phase();
        if phase == ObligationPhase::Judge && !self.is_final_round() {
            self.infer.pending.push(PendingWork::Obligation(id));

            return Ok(false);
        }

        // judge through solved types once the owning scope checked the node
        let source = self.infer.obligations.get(id)?.obligation.source();
        let is_solved = match self.committed_node_type(source) {
            Some(ty) => {
                let ty = self.shallow_resolve(ty)?;

                self.root_variable(ty)?.is_none()
            }
            None => false,
        };
        if !is_solved && !self.is_final_round() {
            self.infer.pending.push(PendingWork::Obligation(id));

            return Ok(false);
        }
        self.run_obligation(id)?;

        Ok(true)
    }

    /// Run one deferred check and resolve the components it opens.
    fn run_pending_check(&mut self, check: DeferredCheck) -> CompilerResult<()> {
        // mark the variables opened from here on
        let first_variable = self.infer.variable_count();
        match check {
            DeferredCheck::Infer { site, use_, .. } => {
                let mut body = self.body();
                body.attempt_node(site, use_, None)?;
            }
            DeferredCheck::Expect { site, expectation } => {
                let mut body = self.body();
                body.check_node(site, expectation)?;
            }
            DeferredCheck::Convert {
                site,
                source,
                expectation,
            } => {
                let mut body = self.body();
                let conversion = body.convert_value(
                    site,
                    expectation.cause,
                    expectation.relation,
                    source,
                    expectation.target,
                    expectation.use_,
                    expectation.mode,
                )?;
                body.commit_value_conversion(site, source.ty, expectation, conversion)?;
            }
        }

        // settle the roots the check opened, widening included
        let mut roots = Vec::new();
        for index in first_variable..self.infer.variable_count() {
            let variable = dir::TypeVariableId(index as u32);
            let root = self.infer.alias_root(variable)?;
            if self.infer.variable(root)?.state.is_open() && !roots.contains(&root) {
                roots.push(root);
            }
        }
        if !roots.is_empty() {
            self.resolve_variables(&roots)?;
        }

        Ok(())
    }

    /// Solve one collected constraint, keeping ambiguous failures pending.
    pub(in crate::check) fn solve_constraint(&mut self, id: ConstraintId) -> CompilerResult<()> {
        if self.infer.constraints.is_complete(id) {
            return Ok(());
        }

        // attempt the relation reversibly, so an ambiguous verdict rolls back
        let constraint = *self.infer.constraints.get(id)?;
        let mark = self.infer.mark();
        let verdict = self.constrain_relation(
            constraint.origin,
            constraint.cause,
            constraint.relation,
            constraint.source,
            constraint.target,
        )?;
        if verdict == Verdict::Ambiguous && !self.is_final_round() {
            let poison = self.intern_type(dir::Type::Error)?;
            self.infer.rollback(mark, poison)?;
            let work = PendingWork::Constraint(id);
            if !self.infer.pending.contains(&work) {
                self.infer.pending.push(work);
            }

            return Ok(());
        }
        self.infer.commit(mark);

        // record the decided verdict against the constraint
        let outcome = self.complete_constraint_check(
            constraint.origin,
            constraint.relation,
            constraint.source,
            constraint.target,
            verdict == Verdict::Holds,
        )?;
        let result = ConstraintResult {
            source: constraint.source,
            target: constraint.target,
            outcome,
        };
        self.infer.set_constraint_result(id, result)?;
        self.record_event(CheckEvent::RelationChecked {
            constraint: id,
            is_finished: true,
        });

        Ok(())
    }

    /// Collect the open variables one pending work item waits on.
    pub(in crate::check) fn pending_blockers(
        &self,
        work: &PendingWork,
    ) -> CompilerResult<smallvec::SmallVec<[dir::TypeVariableId; 2]>> {
        // read the types this work item waits on
        let types: smallvec::SmallVec<[dir::GlobalTypeId; 2]> = match work {
            PendingWork::Constraint(id) => {
                let constraint = *self.infer.constraints.get(*id)?;

                smallvec::smallvec![constraint.source, constraint.target]
            }
            PendingWork::Check(DeferredCheck::Infer {
                site, stalled_on, ..
            }) => match stalled_on {
                Some(variable) => {
                    let root = self.infer.alias_root(*variable)?;

                    match self.infer.variable(root)?.state.is_open() {
                        true => return Ok(smallvec::smallvec![root]),
                        false => smallvec::SmallVec::new(),
                    }
                }
                None => self.committed_node_type(site.node).into_iter().collect(),
            },
            PendingWork::Check(DeferredCheck::Expect { site, expectation }) => self
                .committed_node_type(site.node)
                .into_iter()
                .chain([expectation.target])
                .collect(),
            PendingWork::Check(DeferredCheck::Convert {
                source,
                expectation,
                ..
            }) => smallvec::smallvec![source.ty, expectation.target],
            // wait on the checked node, not on inference
            PendingWork::Obligation(_) => smallvec::SmallVec::new(),
        };

        // collect the open variables behind those types
        let mut blockers = smallvec::SmallVec::new();
        for ty in types {
            for variable in self.type_variables(ty)? {
                if !blockers.contains(&variable) {
                    blockers.push(variable);
                }
            }
        }

        Ok(blockers)
    }

    /// Retain one failed check until its cause tree is complete.
    pub(in crate::check) fn record_failure(
        &mut self,
        cause: CauseId,
        relation: Relation,
        use_: Option<ValueUse>,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        failure: CheckFailure,
    ) -> CompilerResult<()> {
        // keep a failure over open operands provisional until the final round
        let is_provisional = !self.is_final_round()
            && (self.type_flags(source)? | self.type_flags(target)?).has_variable();
        self.infer.failures.push(FailedCheck {
            cause,
            relation,
            use_,
            source,
            target,
            failure,
            is_provisional,
        });

        Ok(())
    }
}
