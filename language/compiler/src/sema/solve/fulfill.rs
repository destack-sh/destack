use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    BodyCheck, Check, CheckFailure, CheckId, CheckOutcome, CheckState, CheckTable, ConversionCheck,
    FailedCheck, ObligationEntry, Origin, PatternCheck, PlaceCheck, RelationCheck, Settle, Verdict,
    WorkState,
};
use crate::{CompilerError, CompilerResult};

/// The event one waiting check resumes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum Wake {
    /// One open inference variable closes.
    Variable(dir::TypeVariableId),
    /// The final resolve begins.
    Final,
}

/// The fulfillment queue stepping pending checks to verdicts.
pub(in crate::sema) struct Fulfillment {
    /// Collected checks with their outcomes and scheduling state.
    pub(in crate::sema) checks: CheckTable,
    /// Failed checks kept until their cause trees are complete.
    pub(in crate::sema) failures: Vec<FailedCheck>,
    /// Checks queued to step.
    pub(in crate::sema) ready: Vec<CheckId>,
    /// Waiting checks keyed by the event each resumes on.
    pub(in crate::sema) waiting: FxIndexMap<Wake, SmallVec<[CheckId; 2]>>,
    /// The check that binds each deferred term.
    pub(in crate::sema) binders: FxIndexMap<dir::TypeVariableId, CheckId>,
    /// The number of checks still queued or waiting.
    live: usize,
}

impl Fulfillment {
    /// Create an empty fulfillment queue.
    pub(in crate::sema) fn new() -> Self {
        Self {
            checks: CheckTable::new(),
            failures: Vec::new(),
            ready: Vec::new(),
            waiting: FxIndexMap::default(),
            binders: FxIndexMap::default(),
            live: 0,
        }
    }

    /// Queue the waiters of one event.
    fn queue_waiters(&mut self, waiters: SmallVec<[CheckId; 2]>) {
        // queue every waiter that still waits
        for id in waiters {
            if self.checks.entries[id.index()].state == WorkState::Waiting {
                self.checks.entries[id.index()].state = WorkState::Ready;
                self.ready.push(id);
            }
        }
    }

    /// Queue one check's work, queuing it once while it stays live.
    pub(in crate::sema) fn queue_work(&mut self, check: CheckId) {
        // leave a live queue entry as it stands
        if self.checks.state(check) != WorkState::Done {
            return;
        }

        // queue the check to step
        let row = &mut self.checks.entries[check.index()];
        row.state = WorkState::Ready;
        self.ready.push(check);
        self.live += 1;
    }

    /// Wait one check on one wake event.
    pub(in crate::sema) fn wait(&mut self, id: CheckId, event: Wake) {
        // record the check once against its wake event
        self.checks.entries[id.index()].state = WorkState::Waiting;
        let waiters = self.waiting.entry(event).or_default();
        if !waiters.contains(&id) {
            waiters.push(id);
        }
    }

    /// Wait one check on the variables it watches, or on the final resolve.
    pub(in crate::sema) fn stall_work(&mut self, id: CheckId, watched: &[dir::TypeVariableId]) {
        // wait for the final resolve when nothing is watched
        if watched.is_empty() {
            return self.wait(id, Wake::Final);
        }

        // wait on every variable the check watches
        for variable in watched {
            self.wait(id, Wake::Variable(*variable));
        }
    }

    /// Return the queued and waiting checks, to restore after a rolled back decision.
    pub(in crate::sema) fn scheduling(&self) -> Scheduling {
        Scheduling {
            ready: self.ready.clone(),
            waiting: self.waiting.clone(),
        }
    }

    /// Restore the queued and waiting checks of one rolled back decision.
    pub(in crate::sema) fn restore_scheduling(&mut self, scheduling: Scheduling) {
        self.ready = scheduling.ready;
        self.waiting = scheduling.waiting;
    }

    /// Wake the checks waiting on one event.
    pub(in crate::sema) fn wake(&mut self, event: Wake) {
        // take the waiters recorded on the event
        let Some(waiters) = self.waiting.swap_remove(&event) else {
            return;
        };

        self.queue_waiters(waiters);
    }

    /// Wake every waiting check past one mark for the final resolve.
    pub(in crate::sema) fn wake_all(&mut self, decided: usize) {
        // wake every waiter past the mark and keep the rest
        let mut kept = FxIndexMap::default();
        for (event, waiters) in std::mem::take(&mut self.waiting) {
            let (woken, held): (SmallVec<[CheckId; 2]>, SmallVec<[CheckId; 2]>) =
                waiters.into_iter().partition(|id| id.index() >= decided);
            self.queue_waiters(woken);
            if !held.is_empty() {
                kept.insert(event, held);
            }
        }

        self.waiting = kept;
    }

    /// Complete one check's work.
    pub(in crate::sema) fn finish_work(&mut self, id: CheckId) {
        // retire a live check once
        if self.checks.entries[id.index()].state != WorkState::Done {
            self.checks.entries[id.index()].state = WorkState::Done;
            self.live -= 1;
        }
    }

    /// Drop the checks collected past one mark, purging their scheduling.
    pub(in crate::sema) fn truncate(&mut self, count: usize) {
        // retire the scheduling every dropped check still holds
        for index in count..self.checks.count() {
            self.finish_work(CheckId::at(index));
        }
        self.checks.truncate(count);

        // purge the dropped ids from the queues
        self.ready.retain(|id| id.index() < count);
        self.waiting.retain(|_, waiters| {
            waiters.retain(|id| id.index() < count);

            !waiters.is_empty()
        });
    }
}

impl CheckState<'_> {
    /// Collect one check into fulfillment and queue it to step.
    pub(in crate::sema) fn queue_check(&mut self, check: Check) -> CompilerResult<CheckId> {
        let id = self.fulfill.checks.allocate(check);
        self.fulfill.queue_work(id);

        Ok(id)
    }

    /// Collect one check already stalled on the blockers it waits behind.
    pub(in crate::sema) fn queue_check_stalled(
        &mut self,
        check: Check,
        blockers: &[dir::TypeVariableId],
    ) -> CompilerResult<CheckId> {
        let id = self.queue_check(check)?;
        self.fulfill.stall_work(id, blockers);

        Ok(id)
    }

    /// Collect one relation and queue it without attempting it yet.
    pub(in crate::sema) fn queue_relation(
        &mut self,
        relation: RelationCheck,
    ) -> CompilerResult<CheckId> {
        self.queue_check(Check::Relation(relation))
    }

    /// Collect one relation and solve it in place, queuing it while ambiguous.
    pub(in crate::sema) fn push_relation(
        &mut self,
        relation: RelationCheck,
    ) -> CompilerResult<CheckId> {
        let id = self.fulfill.checks.allocate_relation(relation);
        self.solve_relation(id, Settle::Possible)?;

        // queue the relation while an ambiguous verdict leaves it open
        if !self.fulfill.checks.is_complete(id) {
            self.fulfill.queue_work(id);
        }

        Ok(id)
    }

    /// Step the work ready at entry once, leaving fresh registrations queued.
    pub(in crate::sema) fn solve_where_possible(&mut self, stage: Settle) -> CompilerResult<bool> {
        // take the round's ready queue, leaving fresh work for the next
        let ready = std::mem::take(&mut self.fulfill.ready);
        let decided = self.infer.decided_checks();
        let mut held = Vec::new();
        let mut round = false;

        // step each ready check of the open decision
        for id in ready {
            if id.index() < decided {
                held.push(id);
                continue;
            }
            if self.fulfill.checks.state(id) != WorkState::Ready {
                continue;
            }

            round |= self.step_check(id, stage)?;
        }

        // keep the checks outside the decision queued ahead of the round's fresh work
        held.append(&mut self.fulfill.ready);
        self.fulfill.ready = held;

        // report whether ready work remains inside the open decision
        let decided = self.infer.decided_checks();
        let has_ready =
            self.fulfill.ready.iter().any(|id| {
                id.index() >= decided && self.fulfill.checks.state(*id) == WorkState::Ready
            });

        Ok(round || has_ready)
    }

    /// Step one pending check, dispatching by kind.
    fn step_check(&mut self, id: CheckId, stage: Settle) -> CompilerResult<bool> {
        // read the check to step
        let check = self.fulfill.checks.get(id)?.clone();

        // dispatch by the check's kind
        match check {
            Check::Relation(_) => self.step_relation(id, stage),
            Check::Conversion(conversion) => self.step_conversion(id, conversion, stage),
            Check::Declared(entry) => self.step_declared(id, entry),
            Check::Body(body) => self.step_body(id, body, stage),
            Check::Pattern(pattern) => self.step_pattern(id, pattern, stage),
            Check::Place(place) => self.step_place(id, place, stage),
        }
    }

    /// Step one pending destructuring pattern, returning whether it completed.
    fn step_pattern(
        &mut self,
        id: CheckId,
        pattern: PatternCheck,
        stage: Settle,
    ) -> CompilerResult<bool> {
        // wait for the input to close, forcing it at the final settle
        let Some(root) = self.root_variable(pattern.target)? else {
            self.fulfill.finish_work(id);
            self.check_destructuring_pattern(pattern.site, pattern.target)?;

            return Ok(true);
        };

        // settle the input variable, stalling while it stays open
        self.settle_variables(&[root], stage)?;
        if self.infer.variable(root)?.state.is_open() && !stage.settles() {
            self.fulfill.stall_work(id, &[root]);

            return Ok(false);
        }

        // check the pattern once its input closes
        self.fulfill.finish_work(id);
        self.check_destructuring_pattern(pattern.site, pattern.target)?;

        Ok(true)
    }

    /// Step one pending binding place, returning whether it completed.
    fn step_place(
        &mut self,
        id: CheckId,
        place: PlaceCheck,
        stage: Settle,
    ) -> CompilerResult<bool> {
        // settle the slot before the final resolve
        if let Some(root) = self.root_variable(place.ty)? {
            self.settle_variables(&[root], stage)?;
            if self.infer.variable(root)?.state.is_open() {
                if !stage.settles() {
                    self.fulfill.stall_work(id, &[root]);

                    return Ok(false);
                }

                // leave an unsolved slot's terms to their defaults
                self.fulfill.finish_work(id);
                self.fulfill.binders.retain(|_, binder| *binder != id);

                return Ok(true);
            }
        }

        // select the place once the slot closes and bind the deferred terms to it
        self.fulfill.finish_work(id);
        self.fulfill.binders.retain(|_, binder| *binder != id);
        self.select_deferred_binding_place(place.site, place.ty)?;

        Ok(true)
    }

    /// Close one check's blocking variables at the final resolve.
    fn resolve_blockers(
        &mut self,
        stage: Settle,
        origin: Origin,
        blockers: &[dir::TypeVariableId],
    ) -> CompilerResult<()> {
        // leave the blockers open outside a settling stage
        if !stage.settles() || blockers.is_empty() {
            return Ok(());
        }

        // settle every blocker before reading it back
        self.settle_variables(blockers, Settle::All)?;
        if self.infer.is_deciding() {
            return Ok(());
        }

        // report each blocker that stays open and give it an error solution
        for blocker in blockers {
            let root = self.infer.alias_root(*blocker)?;
            if self.infer.variable(root)?.state.is_open() {
                self.report_cannot_infer_type(origin, Some(root), &mut FxIndexSet::default())?;
                let error = self.intern_type(dir::Type::Error)?;
                self.commit_error_solution(root, error)?;
            }
        }

        Ok(())
    }

    /// Step one pending relation check, returning whether it completed.
    fn step_relation(&mut self, id: CheckId, stage: Settle) -> CompilerResult<bool> {
        // solve the relation, finishing the work once it decides
        self.solve_relation(id, stage)?;
        if self.fulfill.checks.is_complete(id) {
            self.fulfill.finish_work(id);

            return Ok(true);
        }

        // stall on the open operands while the verdict stays ambiguous
        let Check::Relation(relation) = self.fulfill.checks.get(id)? else {
            return Err(CompilerError::Internal {
                message: format!("relation work item {id:?} names a non-relation check"),
            });
        };
        self.stall_operands(id, [relation.source, relation.target])?;

        Ok(false)
    }

    /// Stall one check on the open variables of its operands, returning whether any stay open.
    fn stall_open_operands(
        &mut self,
        id: CheckId,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // stall the check while either operand keeps an open variable
        if self.collect_open_variables([source, target])?.is_empty() {
            return Ok(false);
        }
        self.stall_operands(id, [source, target])?;

        Ok(true)
    }

    /// Step one pending conversion check, returning whether it completed.
    fn step_conversion(
        &mut self,
        id: CheckId,
        conversion: ConversionCheck,
        stage: Settle,
    ) -> CompilerResult<bool> {
        // read the pending conversion's expectation and site
        let mut expectation = conversion.expectation;
        let site = conversion.site;
        let mut source = conversion.source;

        // close the operands at the final resolve
        let blockers = self.collect_open_variables([source.ty, expectation.target])?;
        self.resolve_blockers(stage, site.origin(), &blockers)?;

        // wait for open operands to close before coercing
        if self.stall_open_operands(id, source.ty, expectation.target)? {
            return Ok(false);
        }

        // decide the closed conversion, keeping what it bound
        let verdict = self.constrain_conversion(
            site,
            expectation.cause,
            expectation.relation,
            source,
            expectation.target,
            expectation.use_,
        )?;

        // wait for the next resolve stage while the conversion stays undecided
        if verdict == Verdict::Ambiguous && !stage.settles() {
            self.stall_operands(id, [source.ty, expectation.target])?;

            return Ok(false);
        }

        // convert once both sides resolve
        source.ty = self.shallow_resolve(source.ty)?;
        expectation.target = self.shallow_resolve(expectation.target)?;
        if self.stall_open_operands(id, source.ty, expectation.target)? {
            return Ok(false);
        }

        // convert the value at its site and commit the result
        self.fulfill.finish_work(id);
        let written = conversion.source;
        let converted = self.convert_value(
            site,
            expectation.cause,
            expectation.relation,
            written,
            expectation.target,
            expectation.use_,
            expectation.mode,
        )?;
        self.commit_value_conversion(site, written.ty, expectation, converted)?;

        Ok(true)
    }

    /// Step one pending function value body, checking it once its parameter slots settle.
    fn step_body(&mut self, id: CheckId, body: BodyCheck, stage: Settle) -> CompilerResult<bool> {
        // fix the parameters the body reads from their candidates so far
        let parameters = self.function_value_parameters(body.node)?;
        self.fix_variables(&parameters)?;
        self.settle_variables(&parameters, stage)?;
        let mut open = SmallVec::<[dir::TypeVariableId; 2]>::new();
        for parameter in &parameters {
            if self.infer.variable(*parameter)?.state.is_open() {
                open.push(*parameter);
            }
        }
        if !open.is_empty() && !stage.settles() {
            self.fulfill.stall_work(id, &open);

            return Ok(false);
        }

        // check the body once its parameters close
        self.fulfill.finish_work(id);
        self.check_function_body(body.node)?;

        Ok(true)
    }

    /// Step one pending declared obligation, returning whether it completed.
    fn step_declared(&mut self, id: CheckId, entry: ObligationEntry) -> CompilerResult<bool> {
        // complete obligations directly outside checking
        if !self.is_checking() {
            self.fulfill.finish_work(id);

            return Ok(true);
        }

        // stall the obligation on the variables its check reports open
        if let Some(stalls) = self.run_obligation(id, &entry)? {
            self.fulfill.stall_work(id, &stalls);

            return Ok(false);
        }

        self.fulfill.finish_work(id);

        Ok(true)
    }

    /// Wait one check on its operands' open variables, or on the next resolve stage.
    fn stall_operands(
        &mut self,
        id: CheckId,
        operands: [dir::GlobalTypeId; 2],
    ) -> CompilerResult<()> {
        // wait on the open variables both operands hold
        let open = self.collect_open_variables(operands)?;
        self.fulfill.stall_work(id, &open);

        Ok(())
    }

    /// Solve one collected relation check, keeping ambiguous failures pending.
    pub(in crate::sema) fn solve_relation(
        &mut self,
        id: CheckId,
        stage: Settle,
    ) -> CompilerResult<()> {
        // leave a completed check as it stands
        if self.fulfill.checks.is_complete(id) {
            return Ok(());
        }

        // require a relation check
        let Check::Relation(relation) = self.fulfill.checks.get(id)?.clone() else {
            return Err(CompilerError::Internal {
                message: format!("relation check {id:?} names a non-relation check"),
            });
        };

        // attempt the relation, keeping whatever it bound
        let mut verdict = self.constrain_type(
            relation.origin,
            relation.cause,
            relation.relation,
            relation.source,
            relation.target,
        )?;

        // keep the attempt's bounds and leave the check pending while it is ambiguous
        if verdict == Verdict::Ambiguous && !stage.settles() {
            return Ok(());
        }

        // close the operands an undecided relation waits on, then decide it hard
        if verdict == Verdict::Ambiguous {
            let blockers = self.collect_open_variables([relation.source, relation.target])?;
            self.resolve_blockers(stage, relation.origin, &blockers)?;
            verdict = self.constrain_type(
                relation.origin,
                relation.cause,
                relation.relation,
                relation.source,
                relation.target,
            )?;
        }

        // fail an undecided relation at the final resolve
        let outcome = match verdict {
            Verdict::Ambiguous => CheckOutcome::Fails(CheckFailure::Undecided),
            verdict => self.complete_constraint_check(
                relation.origin,
                relation.relation,
                relation.source,
                relation.target,
                verdict,
            )?,
        };
        self.fulfill.checks.set_result(id, Some(outcome))?;

        // trace the finished check
        self.record_check_event(id, true)?;

        Ok(())
    }

    /// Push one failed check, kept until its cause tree is complete.
    pub(in crate::sema) fn push_failure(&mut self, check: FailedCheck) -> CompilerResult<()> {
        self.fulfill.failures.push(check);

        Ok(())
    }
}

/// The queued and waiting checks at one point, restored when a decision rolls back.
pub(in crate::sema) struct Scheduling {
    /// The checks ready to step.
    ready: Vec<CheckId>,
    /// The checks waiting on each event.
    waiting: FxIndexMap<Wake, SmallVec<[CheckId; 2]>>,
}
