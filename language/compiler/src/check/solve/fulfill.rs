use destack_dir as dir;

use crate::CompilerResult;
use destack_core::FxIndexMap;
use smallvec::SmallVec;

use crate::check::{
    CauseId, CheckEvent, CheckFailure, CheckState, Constraint, ConstraintId, ConstraintResult,
    ConstraintTable, Expectation, FailedCheck, FlowSite, ObligationEntry, ObligationId,
    ObligationPhase, ObligationTable, PlaceUse, Relation, Settle, Value, ValueUse, Verdict,
};

/// One unit of pending inference work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum PendingWork {
    /// Check one collected constraint.
    Constraint(ConstraintId),
    /// Run one deferred check.
    Check(DeferredCheck),
    /// Check one collected obligation.
    Obligation(ObligationId),
}

/// One check deferred until its operands close.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// One registered unit of fulfillment work.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct WorkEntry {
    /// The registered work.
    pub(in crate::check) kind: PendingWork,
    /// The stepping state.
    pub(in crate::check) state: WorkState,
}

/// The stepping state of one registered work item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum WorkState {
    /// Queued to step.
    Ready,
    /// Awaiting one of its watched variables.
    Stalled,
    /// Parked until the next settle stage.
    Parked,
    /// Completed or cancelled.
    Done,
}

/// The fulfillment queue driving pending work to verdicts.
pub(in crate::check) struct Fulfillment {
    /// Collected constraints and their completed results.
    pub(in crate::check) constraints: ConstraintTable,
    /// Collected obligations awaiting their settle points.
    pub(in crate::check) obligations: ObligationTable,
    /// Failed checks retained until their cause trees are complete.
    pub(in crate::check) failures: Vec<FailedCheck>,
    /// Registered work in registration order.
    pub(in crate::check) work: Vec<WorkEntry>,
    /// Work queued to step.
    pub(in crate::check) ready: Vec<usize>,
    /// Work parked until the next settle stage.
    pub(in crate::check) parked: Vec<usize>,
    /// Stalled work keyed by the open variable it watches.
    pub(in crate::check) watchers: FxIndexMap<dir::TypeVariableId, SmallVec<[usize; 2]>>,
    /// Active work ids keyed by their registered shape.
    pub(in crate::check) active: FxIndexMap<PendingWork, usize>,
    /// The number of registered items still open.
    pub(in crate::check) open_work: usize,
}

impl Fulfillment {
    /// Create an empty fulfillment queue.
    pub(in crate::check) fn new() -> Self {
        Self {
            constraints: ConstraintTable::new(),
            obligations: ObligationTable::new(),
            failures: Vec::new(),
            work: Vec::new(),
            ready: Vec::new(),
            parked: Vec::new(),
            watchers: FxIndexMap::default(),
            active: FxIndexMap::default(),
            open_work: 0,
        }
    }

    /// Allocate one constraint.
    pub(in crate::check) fn allocate_constraint(&mut self, constraint: Constraint) -> ConstraintId {
        // collect one constraint per check, however often walks repeat it
        if let Some(id) = self.constraints.lookup(&constraint) {
            return id;
        }

        let id = ConstraintId::at(self.constraints.count());
        self.constraints.insert(id, constraint);

        id
    }

    /// Allocate one obligation.
    pub(in crate::check) fn allocate_obligation(&mut self, entry: ObligationEntry) -> ObligationId {
        let id = ObligationId::at(self.obligations.count());
        self.obligations.insert(id, entry);

        id
    }

    /// Register one work item, queuing it to step.
    pub(in crate::check) fn register_work(&mut self, kind: PendingWork) -> usize {
        // keep repeated registrations single while they stay live
        if let Some(id) = self.active.get(&kind) {
            return *id;
        }

        let id = self.work.len();
        self.work.push(WorkEntry {
            kind,
            state: WorkState::Ready,
        });
        self.ready.push(id);
        self.active.insert(kind, id);
        self.open_work += 1;

        id
    }

    /// Stall one work item on its watched variables.
    pub(in crate::check) fn stall_work(&mut self, id: usize, watched: &[dir::TypeVariableId]) {
        self.work[id].state = WorkState::Stalled;

        // watch every variable the item waits on
        for variable in watched {
            let watchers = self.watchers.entry(*variable).or_default();
            if !watchers.contains(&id) {
                watchers.push(id);
            }
        }
    }

    /// Park one work item until the next settle stage.
    pub(in crate::check) fn park_work(&mut self, id: usize) {
        self.work[id].state = WorkState::Parked;
        self.parked.push(id);
    }

    /// Complete one work item.
    pub(in crate::check) fn finish_work(&mut self, id: usize) {
        if self.work[id].state != WorkState::Done {
            self.work[id].state = WorkState::Done;
            self.active.swap_remove(&self.work[id].kind);
            self.open_work -= 1;
        }
    }

    /// Wake the work watching one variable.
    pub(in crate::check) fn wake_variable(&mut self, variable: dir::TypeVariableId) {
        let Some(watchers) = self.watchers.swap_remove(&variable) else {
            return;
        };

        // queue each watcher that is still stalled
        for id in watchers {
            if self.work[id].state == WorkState::Stalled {
                self.work[id].state = WorkState::Ready;
                self.ready.push(id);
            }
        }
    }

    /// Re-queue every stalled item for one decisive settle stage.
    pub(in crate::check) fn requeue_stalled(&mut self) -> bool {
        let mut requeued = false;
        for (_, watchers) in std::mem::take(&mut self.watchers) {
            for id in watchers {
                if self.work[id].state == WorkState::Stalled {
                    self.work[id].state = WorkState::Ready;
                    self.ready.push(id);
                    requeued = true;
                }
            }
        }

        requeued
    }

    /// Re-queue every parked item for one settle stage.
    pub(in crate::check) fn requeue_parked(&mut self) -> bool {
        let mut requeued = false;
        for id in std::mem::take(&mut self.parked) {
            if self.work[id].state == WorkState::Parked {
                self.work[id].state = WorkState::Ready;
                self.ready.push(id);
                requeued = true;
            }
        }

        requeued
    }

    /// Return the number of collected constraints.
    pub(in crate::check) fn constraint_count(&self) -> usize {
        self.constraints.count()
    }

    /// Return the number of collected obligations.
    pub(in crate::check) fn obligation_count(&self) -> usize {
        self.obligations.count()
    }

    /// Cancel the work registered past one mark.
    pub(in crate::check) fn cancel_work_from(&mut self, mark: usize) {
        for id in mark..self.work.len() {
            self.finish_work(id);
        }
    }
}

impl CheckState<'_> {
    /// Register one check with fulfillment, keeping repeats single.
    pub(in crate::check) fn register_check(&mut self, check: DeferredCheck) {
        self.fulfill.register_work(PendingWork::Check(check));
    }

    /// Collect one candidate constraint and register it with fulfillment.
    pub(in crate::check) fn register_constraint(&mut self, constraint: Constraint) -> ConstraintId {
        let id = self.fulfill.allocate_constraint(constraint);
        self.fulfill.register_work(PendingWork::Constraint(id));

        id
    }

    /// Collect one constraint and solve it in place.
    pub(in crate::check) fn push_constraint(
        &mut self,
        constraint: Constraint,
    ) -> CompilerResult<ConstraintId> {
        let id = self.fulfill.allocate_constraint(constraint);
        self.solve_constraint(id, Settle::Bounded)?;

        // queue the constraint while an ambiguous verdict leaves it open
        if !self.fulfill.constraints.is_complete(id) {
            self.fulfill.register_work(PendingWork::Constraint(id));
        }

        Ok(id)
    }

    /// Step the work ready at entry once, leaving fresh registrations queued.
    pub(in crate::check) fn solve_where_possible(
        &mut self,
        settle: Settle,
    ) -> CompilerResult<bool> {
        // take the round's queue, so items registered while stepping wait
        let ready = std::mem::take(&mut self.fulfill.ready);
        let mut round = false;

        for id in ready {
            if self.fulfill.work[id].state != WorkState::Ready {
                continue;
            }

            let kind = self.fulfill.work[id].kind;
            round |= match kind {
                PendingWork::Constraint(constraint) => {
                    self.step_constraint(id, constraint, settle)?
                }
                PendingWork::Check(check) => self.step_check(id, check, settle)?,
                PendingWork::Obligation(obligation) => {
                    self.step_obligation(id, obligation, settle)?
                }
            };
        }

        Ok(round)
    }

    /// Step one pending constraint, returning whether it completed.
    fn step_constraint(
        &mut self,
        work: usize,
        id: ConstraintId,
        settle: Settle,
    ) -> CompilerResult<bool> {
        // solve the constraint at this settle stage
        self.solve_constraint(id, settle)?;
        if self.fulfill.constraints.is_complete(id) {
            self.fulfill.finish_work(work);

            return Ok(true);
        }

        // stall on the open operands while the verdict stays ambiguous
        let constraint = *self.fulfill.constraints.get(id)?;
        self.stall_or_park(work, [constraint.source, constraint.target])?;

        Ok(false)
    }

    /// Step one deferred check, returning whether it ran.
    fn step_check(
        &mut self,
        work: usize,
        check: DeferredCheck,
        settle: Settle,
    ) -> CompilerResult<bool> {
        // wait for a stalled selection's blocker to solve
        if let DeferredCheck::Infer {
            stalled_on: Some(stalled_on),
            ..
        } = check
        {
            let root = self.infer.alias_root(stalled_on)?;
            if self.infer.variable(root)?.state.is_open() {
                self.fulfill.stall_work(work, &[root]);

                return Ok(false);
            }
        }

        // hold an ambiguous conversion while its operands stay open
        if let DeferredCheck::Convert {
            site,
            source,
            expectation,
        } = check
        {
            // roll an ambiguous relation back and wait for its operands to close
            let mark = self.infer.mark(&self.fulfill);
            let verdict = self.constrain_relation(
                site.origin(),
                expectation.cause,
                expectation.relation,
                source.ty,
                expectation.target,
            )?;

            // stall failed targets with open variables and open union targets
            let target = self.resolve_head(expectation.target)?;
            let is_choice = matches!(
                self.ty(target)?,
                dir::Type::Union(_) | dir::Type::Intersection(_)
            );
            let stalled = verdict == Verdict::Ambiguous
                || ((verdict == Verdict::Fails || is_choice)
                    && settle != Settle::Final
                    && !self.open_type_variables([expectation.target])?.is_empty());

            // undo the attempt and wait when the relation stayed undecided
            if stalled {
                let poison = self.intern_type(dir::Type::Error)?;
                self.infer.rollback(mark, poison, &mut self.fulfill)?;
                self.stall_or_park(work, [source.ty, expectation.target])?;

                return Ok(false);
            }

            self.infer.commit(mark);

            // convert once the source settles: coercion binds open targets
            if !self.open_type_variables([source.ty])?.is_empty() {
                self.stall_or_park(work, [source.ty, expectation.target])?;

                return Ok(false);
            }
        }

        // run the check, which re-registers itself while its operands stay open
        self.fulfill.finish_work(work);
        self.run_pending_check(check)?;

        Ok(!self.fulfill.active.contains_key(&PendingWork::Check(check)))
    }

    /// Step one pending obligation, returning whether it ran.
    fn step_obligation(
        &mut self,
        work: usize,
        id: ObligationId,
        settle: Settle,
    ) -> CompilerResult<bool> {
        // complete obligations untouched outside checking
        if !self.is_checking() {
            self.fulfill.finish_work(work);

            return Ok(true);
        }

        // hold settle-phase obligations for the final settle
        let phase = self.fulfill.obligations.get(id)?.obligation.phase();
        if phase == ObligationPhase::Judge && settle != Settle::Final {
            self.fulfill.park_work(work);

            return Ok(false);
        }

        // run through solved types once the owning scope checked the node
        let source = self.fulfill.obligations.get(id)?.obligation.source();
        let blocker = match self.committed_node_type(source) {
            Some(ty) => {
                let ty = self.resolve_head(ty)?;

                self.root_variable(ty)?.map(Some)
            }
            None => Some(None),
        };
        if let Some(root) = blocker
            && settle != Settle::Final
        {
            // stall on the open root, or park sources without a committed type
            match root {
                Some(root) => self.fulfill.stall_work(work, &[root]),
                None => self.fulfill.park_work(work),
            }

            return Ok(false);
        }

        // stall the obligation on the variables its check reports open
        if let Some(stalls) = self.run_obligation(id)? {
            match stalls.is_empty() {
                true => self.fulfill.park_work(work),
                false => self.fulfill.stall_work(work, &stalls),
            }

            return Ok(false);
        }

        self.fulfill.finish_work(work);

        Ok(true)
    }

    /// Stall one work item on its open operand variables, or park it.
    fn stall_or_park(
        &mut self,
        work: usize,
        operands: [dir::GlobalTypeId; 2],
    ) -> CompilerResult<()> {
        let open = self.open_type_variables(operands)?;
        match open.is_empty() {
            true => self.fulfill.park_work(work),
            false => self.fulfill.stall_work(work, &open),
        }

        Ok(())
    }

    /// Run one deferred check and resolve the components it opens.
    fn run_pending_check(&mut self, check: DeferredCheck) -> CompilerResult<()> {
        // mark the variables opened from here on
        let first_variable = self.infer.variable_count();

        // run the check in its own body walker
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
    pub(in crate::check) fn solve_constraint(
        &mut self,
        id: ConstraintId,
        settle: Settle,
    ) -> CompilerResult<()> {
        if self.fulfill.constraints.is_complete(id) {
            return Ok(());
        }

        // attempt the relation reversibly, so an ambiguous verdict rolls back
        let constraint = *self.fulfill.constraints.get(id)?;
        let mark = self.infer.mark(&self.fulfill);
        let mut verdict = self.constrain_relation(
            constraint.origin,
            constraint.cause,
            constraint.relation,
            constraint.source,
            constraint.target,
        )?;

        // report ambiguous predicates over closed heads at the final settle
        if verdict == Verdict::Ambiguous
            && settle == Settle::Final
            && self
                .root_variable(self.resolve_head(constraint.source)?)?
                .is_none()
            && self
                .root_variable(self.resolve_head(constraint.target)?)?
                .is_none()
        {
            verdict = Verdict::Fails;
        }

        // undo the attempt and leave the constraint pending while it is ambiguous
        if verdict == Verdict::Ambiguous {
            let poison = self.intern_type(dir::Type::Error)?;
            self.infer.rollback(mark, poison, &mut self.fulfill)?;

            return Ok(());
        }

        self.infer.commit(mark);

        // record the decided verdict against the constraint
        let outcome = self.complete_constraint_check(
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
        self.infer
            .set_constraint_result(&mut self.fulfill.constraints, id, result)?;

        // trace the finished constraint
        self.record_event(CheckEvent::RelationChecked {
            constraint: id,
            is_finished: true,
        });

        Ok(())
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
        // keep a failure over open operands provisional until they close
        let is_provisional = (self.type_flags(source)? | self.type_flags(target)?).has_variable();
        self.fulfill.failures.push(FailedCheck {
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
