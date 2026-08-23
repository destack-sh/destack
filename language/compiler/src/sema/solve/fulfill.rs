use destack_core::FxIndexMap;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    Cause, CauseKind, Check, CheckEvent, CheckFailure, CheckId, CheckOutcome, CheckState,
    CheckTable, ConversionCheck, EqualityCheck, FailedCheck, FlowNarrowing, NarrowingCheck,
    NodeCheck, ObligationEntry, ObligationPhase, Relation, RelationCheck, Resolve, SelectionCheck,
    Verdict, WorkState,
};
use crate::{CompilerError, CompilerResult};

/// The event one waiting check resumes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum Wake {
    /// One open inference variable closes.
    Variable(dir::TypeVariableId),
    /// One source node commits a type.
    Node(dir::GlobalNodeIdAny),
    /// Any resolve stage begins.
    Stage,
    /// The final resolve begins.
    Final,
}

/// One open speculative attempt: the checks queued before it, which the attempt cannot
/// step, and the wakes held back from them until the attempt commits.
#[derive(Debug, Default)]
struct Fence {
    /// The number of checks queued before the attempt.
    checks: usize,
    /// Wake events the attempt fired at fenced checks.
    wakes: Vec<Wake>,
}

/// The fulfillment queue driving pending checks to verdicts.
pub(in crate::sema) struct Fulfillment {
    /// Collected checks with their outcomes and scheduling state.
    pub(in crate::sema) checks: CheckTable,
    /// Failed checks kept until their cause trees are complete.
    pub(in crate::sema) failures: Vec<FailedCheck>,
    /// Checks queued to step.
    pub(in crate::sema) ready: Vec<CheckId>,
    /// Waiting checks keyed by the event each resumes on.
    pub(in crate::sema) waiting: FxIndexMap<Wake, SmallVec<[CheckId; 2]>>,
    /// Producing checks keyed by the variable each can still bound.
    pub(in crate::sema) producers: FxIndexMap<dir::TypeVariableId, SmallVec<[CheckId; 2]>>,
    /// The number of checks still queued or waiting.
    live: usize,
    /// Open speculative attempts, outermost first.
    fences: Vec<Fence>,
}

impl Fulfillment {
    /// Create an empty fulfillment queue.
    pub(in crate::sema) fn new() -> Self {
        Self {
            checks: CheckTable::new(),
            failures: Vec::new(),
            ready: Vec::new(),
            waiting: FxIndexMap::default(),
            producers: FxIndexMap::default(),
            live: 0,
            fences: Vec::new(),
        }
    }

    /// Return the number of checks the innermost open speculation fences off.
    fn fence(&self) -> usize {
        self.fences.last().map_or(0, |fence| fence.checks)
    }

    /// Open one speculative attempt, fencing off every check queued so far.
    pub(in crate::sema) fn open_speculation(&mut self) {
        self.fences.push(Fence {
            checks: self.checks.count(),
            wakes: Vec::new(),
        });
    }

    /// Commit one speculative attempt, firing the wakes it held back.
    pub(in crate::sema) fn commit_speculation(&mut self) {
        let fence = self.fences.pop().unwrap_or_default();
        for event in fence.wakes {
            self.wake(event);
        }
    }

    /// Abandon one speculative attempt, dropping the wakes it held back.
    pub(in crate::sema) fn abandon_speculation(&mut self) {
        self.fences.pop();
    }

    /// Queue the waiters of one event, holding fenced waiters back for the commit.
    fn queue_waiters(&mut self, event: Wake, waiters: SmallVec<[CheckId; 2]>) {
        let fence = self.fence();
        let mut held = SmallVec::<[CheckId; 2]>::new();
        for id in waiters {
            if id.index() < fence {
                held.push(id);
                continue;
            }
            if self.checks.entries[id.index()].state == WorkState::Waiting {
                self.checks.entries[id.index()].state = WorkState::Ready;
                self.ready.push(id);
            }
        }
        if let Some(fence) = self.fences.last_mut()
            && !held.is_empty()
        {
            self.waiting.insert(event, held);
            if !fence.wakes.contains(&event) {
                fence.wakes.push(event);
            }
        }
    }

    /// Queue one check's work, keeping repeated queuing single while live.
    pub(in crate::sema) fn queue_work(
        &mut self,
        check: CheckId,
        produces: SmallVec<[dir::TypeVariableId; 2]>,
    ) {
        // leave a live queue entry as it stands
        if self.checks.state(check) != WorkState::Done {
            return;
        }

        // index the variables the check can still bound
        for variable in &produces {
            let producers = self.producers.entry(*variable).or_default();
            if !producers.contains(&check) {
                producers.push(check);
            }
        }

        // queue the check to step
        let row = &mut self.checks.entries[check.index()];
        row.produces = produces;
        row.state = WorkState::Ready;
        self.ready.push(check);
        self.live += 1;
    }

    /// Wait one check on one wake event.
    pub(in crate::sema) fn wait(&mut self, id: CheckId, event: Wake) {
        self.checks.entries[id.index()].state = WorkState::Waiting;
        let waiters = self.waiting.entry(event).or_default();
        if !waiters.contains(&id) {
            waiters.push(id);
        }
    }

    /// Wait one check on the variables it watches, or on the next resolve stage.
    pub(in crate::sema) fn stall_work(&mut self, id: CheckId, watched: &[dir::TypeVariableId]) {
        // wait for the next resolve stage without any watched variable
        if watched.is_empty() {
            return self.wait(id, Wake::Stage);
        }

        // wait on every variable the check watches
        for variable in watched {
            self.wait(id, Wake::Variable(*variable));
        }
    }

    /// Wake the checks waiting on one event.
    pub(in crate::sema) fn wake(&mut self, event: Wake) {
        let Some(waiters) = self.waiting.swap_remove(&event) else {
            return;
        };

        self.queue_waiters(event, waiters);
    }

    /// Wake every waiting check for the final resolve.
    pub(in crate::sema) fn wake_all(&mut self) {
        for (event, waiters) in std::mem::take(&mut self.waiting) {
            self.queue_waiters(event, waiters);
        }
    }

    /// Complete one check's work.
    pub(in crate::sema) fn finish_work(&mut self, id: CheckId) {
        if self.checks.entries[id.index()].state != WorkState::Done {
            self.checks.entries[id.index()].state = WorkState::Done;
            self.live -= 1;

            // retire the completed check's producer entries
            for variable in std::mem::take(&mut self.checks.entries[id.index()].produces) {
                if let Some(producers) = self.producers.get_mut(&variable) {
                    producers.retain(|producer| *producer != id);
                }
            }
        }
    }

    /// Forward one completed variable's producers onto its successor variables.
    ///
    /// The source entries stay in place: a probe rollback can reopen the
    /// variable, and finished work drops out of every entry on its own.
    pub(in crate::sema) fn forward_producers(
        &mut self,
        variable: dir::TypeVariableId,
        successors: &[dir::TypeVariableId],
    ) {
        let Some(forwarded) = self.producers.get(&variable).cloned() else {
            return;
        };

        // forward each producing check onto every successor
        for successor in successors {
            let producers = self.producers.entry(*successor).or_default();
            for id in &forwarded {
                if !producers.contains(id) {
                    producers.push(*id);
                }
                let row = &mut self.checks.entries[id.index()];
                if !row.produces.contains(successor) {
                    row.produces.push(*successor);
                }
            }
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
        self.producers.retain(|_, producers| {
            producers.retain(|id| id.index() < count);

            !producers.is_empty()
        });
    }

    /// Return whether any collected check is still open.
    pub(in crate::sema) fn has_open_work(&self) -> bool {
        self.live != 0
    }
}

impl CheckState<'_> {
    /// Collect one check into fulfillment and queue it to step.
    pub(in crate::sema) fn queue_check(&mut self, check: Check) -> CompilerResult<CheckId> {
        let produces = self.produced_variables(&check)?;
        let id = self.fulfill.checks.allocate(check);
        self.fulfill.queue_work(id, produces);

        Ok(id)
    }

    /// Collect one check already stalled on the blockers it waits behind.
    pub(in crate::sema) fn queue_check_stalled(
        &mut self,
        check: Check,
        blockers: &[dir::TypeVariableId],
    ) -> CompilerResult<CheckId> {
        let produces = self.produced_variables(&check)?;
        let id = self.fulfill.checks.allocate(check);
        self.fulfill.queue_work(id, produces);
        self.fulfill.stall_work(id, blockers);

        Ok(id)
    }

    /// Return the open variables one check's completion can still bound.
    fn produced_variables(
        &self,
        check: &Check,
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 2]>> {
        let mut produces = SmallVec::new();
        match check {
            // land a blocked node's result on the expectation target
            Check::Node(node) => {
                produces = self.type_variables(node.expectation.target)?;
            }
            // solve a narrowing's own hole once its operation decides
            Check::Narrowing(narrowing) => {
                if let Some(hole) = self.open_root(narrowing.hole)? {
                    produces.push(hole);
                }
            }
            // overwrite the hole a reselection minted at its site
            Check::Selection(selection) => {
                if let Some(ty) = self.committed_node_type(selection.site.node)
                    && let Some(root) = self.root_variable(ty)?
                {
                    produces.push(root);
                }
            }
            Check::Relation(_) | Check::Conversion(_) | Check::Declared(_) | Check::Equality(_) => {
            }
        }

        Ok(produces)
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
        self.solve_relation(id, Resolve::Bounded)?;

        // queue the relation while an ambiguous verdict leaves it open
        if !self.fulfill.checks.is_complete(id) {
            self.fulfill.queue_work(id, SmallVec::new());
        }

        Ok(id)
    }

    /// Step the work ready at entry once, leaving fresh registrations queued.
    pub(in crate::sema) fn solve_where_possible(
        &mut self,
        resolve: Resolve,
    ) -> CompilerResult<bool> {
        // take the round's ready queue, leaving fresh work for the next
        let ready = std::mem::take(&mut self.fulfill.ready);
        let fence = self.fulfill.fence();
        let mut held = Vec::new();
        let mut round = false;

        // step each unfenced check that is still ready
        for id in ready {
            if id.index() < fence {
                held.push(id);
                continue;
            }
            if self.fulfill.checks.state(id) != WorkState::Ready {
                continue;
            }

            round |= self.step_check(id, resolve)?;
        }

        // keep the fenced checks queued ahead of the round's fresh work
        held.append(&mut self.fulfill.ready);
        self.fulfill.ready = held;

        Ok(round)
    }

    /// Step one pending check, dispatching by kind.
    ///
    /// This is the one driver every kind of check steps through, resolving a check's
    /// open blockers hard at the final resolve before re-evaluating.
    fn step_check(&mut self, id: CheckId, resolve: Resolve) -> CompilerResult<bool> {
        let check = self.fulfill.checks.get(id)?.clone();

        match check {
            Check::Relation(_) => self.step_relation(id, resolve),
            Check::Node(node) => self.step_node(id, node),
            Check::Conversion(conversion) => self.step_conversion(id, conversion, resolve),
            Check::Narrowing(narrowing) => self.step_narrowing(id, narrowing, resolve),
            Check::Selection(selection) => self.step_selection(id, selection, resolve),
            Check::Equality(equality) => self.step_equality(id, equality, resolve),
            Check::Declared(entry) => self.step_declared(id, entry, resolve),
        }
    }

    /// Resolve one check's blocking variables hard at the final resolve.
    fn resolve_blockers(
        &mut self,
        resolve: Resolve,
        blockers: &[dir::TypeVariableId],
    ) -> CompilerResult<()> {
        if resolve == Resolve::Final && !blockers.is_empty() {
            self.resolve_variables(blockers)?;
        }

        Ok(())
    }

    /// Step one pending relation check, returning whether it completed.
    fn step_relation(&mut self, id: CheckId, resolve: Resolve) -> CompilerResult<bool> {
        // solve the relation, finishing the work once it decides
        self.solve_relation(id, resolve)?;
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

    /// Step one blocked node by re-running its check, returning whether it completed.
    fn step_node(&mut self, id: CheckId, node: NodeCheck) -> CompilerResult<bool> {
        // re-run the whole check, which requeues while an inference barrier stays open
        self.fulfill.finish_work(id);
        let mut body = self.body();
        body.check_node(node.site, node.expectation)?;

        Ok(self.fulfill.checks.state(id) == WorkState::Done)
    }

    /// Step one pending conversion check, returning whether it completed.
    fn step_conversion(
        &mut self,
        id: CheckId,
        conversion: ConversionCheck,
        resolve: Resolve,
    ) -> CompilerResult<bool> {
        // read the pending conversion's expectation and site
        let mut expectation = conversion.expectation;
        let site = conversion.site;
        let mut source = self.body().literal_candidate(
            site.origin(),
            conversion.source,
            expectation.target,
            expectation.use_,
        )?;

        // close the operands hard at the final resolve, deciding the conversion
        let blockers = self.collect_open_variables([source.ty, expectation.target])?;
        self.resolve_blockers(resolve, &blockers)?;

        // roll an ambiguous conversion back and wait for its operands to close
        let mark = self.infer.mark(&mut self.fulfill);
        let mut verdict = self.body().constrain_conversion(
            site,
            expectation.cause,
            expectation.relation,
            source,
            expectation.target,
            expectation.use_,
        )?;
        let was_ambiguous = verdict == Verdict::Ambiguous;

        // fail an ambiguous conversion at the final resolve
        if was_ambiguous && resolve == Resolve::Final {
            verdict = Verdict::Fails;
        }

        // undo whatever this attempt decided
        if was_ambiguous {
            let poison = self.intern_type(dir::Type::Error)?;
            self.infer.rollback(mark, poison, &mut self.fulfill)?;

            if verdict == Verdict::Ambiguous {
                self.stall_operands(id, [source.ty, expectation.target])?;

                return Ok(false);
            }
        } else {
            // keep whatever a decided attempt bound
            self.infer.commit(mark, &mut self.fulfill);
        }

        // convert once both sides resolve
        source.ty = self.shallow_resolve(source.ty)?;
        expectation.target = self.shallow_resolve(expectation.target)?;
        if !self
            .collect_open_variables([source.ty, expectation.target])?
            .is_empty()
        {
            self.stall_operands(id, [source.ty, expectation.target])?;

            return Ok(false);
        }

        // convert the value at its site and commit the result
        self.fulfill.finish_work(id);
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

        Ok(true)
    }

    /// Step one pending narrowing check, returning whether it completed.
    fn step_narrowing(
        &mut self,
        id: CheckId,
        narrowing: NarrowingCheck,
        resolve: Resolve,
    ) -> CompilerResult<bool> {
        // wait for the consulted operation to decide, closing its hole hard at the final resolve
        if let Some(blocker) = self.operation_hole(narrowing.operation)? {
            self.resolve_blockers(resolve, &[blocker])?;
            if self.decision(narrowing.operation).is_none() {
                self.fulfill.stall_work(id, &[blocker]);

                return Ok(false);
            }
        }

        // recompute the narrowing now that the operation decided
        let narrowed =
            match self.flow_narrowed_type(narrowing.site, &narrowing.path, narrowing.source)? {
                FlowNarrowing::Narrowed(narrowed) => narrowed,
                FlowNarrowing::Unchanged => narrowing.source,
                FlowNarrowing::Pending { blocker, .. } => {
                    self.resolve_blockers(resolve, &[blocker])?;
                    if resolve != Resolve::Final {
                        self.fulfill.stall_work(id, &[blocker]);

                        return Ok(false);
                    }

                    narrowing.source
                }
            };

        // equate the narrowing's hole with the recomputed type
        self.fulfill.finish_work(id);
        let origin = narrowing.site.origin();
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        let hole = self.variable_type(narrowing.hole)?;
        self.constrain_type(origin, cause, Relation::Equal, hole, narrowed)?;

        Ok(true)
    }

    /// Step one pending selection check, returning whether it completed.
    fn step_selection(
        &mut self,
        id: CheckId,
        selection: SelectionCheck,
        resolve: Resolve,
    ) -> CompilerResult<bool> {
        // wait for a stalled selection's blocker to solve
        if let Some(stalled_on) = selection.stalled_on {
            let root = self.infer.alias_root(stalled_on)?;
            self.resolve_blockers(resolve, &[root])?;
            if self.infer.variable(root)?.state.is_open() {
                self.fulfill.stall_work(id, &[root]);

                return Ok(false);
            }
        }

        // reattempt the selection, which requeues itself while blocked
        self.fulfill.finish_work(id);
        let mut body = self.body();
        body.attempt_node(selection.site, selection.use_, None)?;

        Ok(self.fulfill.checks.state(id) == WorkState::Done)
    }

    /// Step one stalled switch equality selection, returning whether it completed.
    fn step_equality(
        &mut self,
        id: CheckId,
        equality: EqualityCheck,
        resolve: Resolve,
    ) -> CompilerResult<bool> {
        // wait for the stalled selection's blocker to solve
        let root = self.infer.alias_root(equality.stalled_on)?;
        self.resolve_blockers(resolve, &[root])?;
        if self.infer.variable(root)?.state.is_open() {
            self.fulfill.stall_work(id, &[root]);

            return Ok(false);
        }

        // reattempt the selection, which requeues itself while blocked
        self.fulfill.finish_work(id);
        let mut body = self.body();
        body.select_switch_equality(equality.value, equality.scrutinee, &equality.cases)?;

        Ok(self.fulfill.checks.state(id) == WorkState::Done)
    }

    /// Step one pending declared obligation, returning whether it completed.
    fn step_declared(
        &mut self,
        id: CheckId,
        entry: ObligationEntry,
        resolve: Resolve,
    ) -> CompilerResult<bool> {
        // complete obligations untouched outside checking
        if !self.is_checking() {
            self.fulfill.finish_work(id);

            return Ok(true);
        }

        // hold final-phase obligations for the final resolve
        let phase = entry.obligation.phase();
        if phase == ObligationPhase::Final && resolve != Resolve::Final {
            self.fulfill.wait(id, Wake::Final);

            return Ok(false);
        }

        // run through solved types once the owning scope checked the node
        if resolve != Resolve::Final {
            match self.committed_node_type(entry.obligation.source()) {
                // stall on the committed type's open root
                Some(ty) => {
                    let ty = self.shallow_resolve(ty)?;
                    if let Some(root) = self.root_variable(ty)? {
                        self.fulfill.stall_work(id, &[root]);

                        return Ok(false);
                    }
                }
                // wait until the source node commits a type
                None => {
                    self.fulfill.wait(id, Wake::Node(entry.obligation.source()));

                    return Ok(false);
                }
            }
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
        let open = self.collect_open_variables(operands)?;
        self.fulfill.stall_work(id, &open);

        Ok(())
    }

    /// Solve one collected relation check, keeping ambiguous failures pending.
    pub(in crate::sema) fn solve_relation(
        &mut self,
        id: CheckId,
        resolve: Resolve,
    ) -> CompilerResult<()> {
        if self.fulfill.checks.is_complete(id) {
            return Ok(());
        }

        let Check::Relation(relation) = self.fulfill.checks.get(id)?.clone() else {
            return Err(CompilerError::Internal {
                message: format!("relation check {id:?} names a non-relation check"),
            });
        };

        // close the operands at the final resolve, deciding the check hard
        let blockers = self.collect_open_variables([relation.source, relation.target])?;
        self.resolve_blockers(resolve, &blockers)?;

        // attempt the relation, keeping whatever it bound
        let mark = self.infer.mark(&mut self.fulfill);
        let verdict = self.constrain_type(
            relation.origin,
            relation.cause,
            relation.relation,
            relation.source,
            relation.target,
        )?;
        self.infer.commit(mark, &mut self.fulfill);

        // keep the attempt's bounds and leave the check pending while it is ambiguous
        if verdict == Verdict::Ambiguous && resolve != Resolve::Final {
            return Ok(());
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
        self.infer
            .set_check_result(&mut self.fulfill.checks, id, Some(outcome))?;

        // trace the finished check
        self.push_event(CheckEvent::Checked {
            check: id,
            is_finished: true,
        });

        Ok(())
    }

    /// Push one failed check, kept until its cause tree is complete.
    pub(in crate::sema) fn push_failure(&mut self, mut check: FailedCheck) -> CompilerResult<()> {
        // keep a failure over open operands provisional until they close
        check.is_provisional =
            (self.type_flags(check.source)? | self.type_flags(check.target)?).has_variable();
        self.fulfill.failures.push(check);

        Ok(())
    }
}
