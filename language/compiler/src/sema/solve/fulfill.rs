use destack_core::FxIndexMap;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    Cause, CauseKind, Check, CheckEvent, CheckFailure, CheckId, CheckOutcome, CheckState,
    CheckTable, ConversionCheck, FailedCheck, FlowNarrowing, NarrowingCheck, NodeCheck,
    ObligationEntry, ObligationPhase, Relation, RelationCheck, SelectionCheck, Settle, Verdict,
    WorkState,
};
use crate::{CompilerError, CompilerResult};

/// The fulfillment queue driving pending checks to verdicts.
pub(in crate::sema) struct Fulfillment {
    /// Collected checks with their outcomes and scheduling state.
    pub(in crate::sema) checks: CheckTable,
    /// Failed checks retained until their cause trees are complete.
    pub(in crate::sema) failures: Vec<FailedCheck>,
    /// Checks queued to step.
    pub(in crate::sema) ready: Vec<CheckId>,
    /// Checks parked until the next settle stage.
    pub(in crate::sema) parked: Vec<CheckId>,
    /// Stalled checks keyed by the open variable each watches.
    pub(in crate::sema) watchers: FxIndexMap<dir::TypeVariableId, SmallVec<[CheckId; 2]>>,
    /// Producing checks keyed by the variable each can still bound.
    pub(in crate::sema) producers: FxIndexMap<dir::TypeVariableId, SmallVec<[CheckId; 2]>>,
    /// The number of checks still queued, stalled, or parked.
    live: usize,
}

impl Fulfillment {
    /// Create an empty fulfillment queue.
    pub(in crate::sema) fn new() -> Self {
        Self {
            checks: CheckTable::new(),
            failures: Vec::new(),
            ready: Vec::new(),
            parked: Vec::new(),
            watchers: FxIndexMap::default(),
            producers: FxIndexMap::default(),
            live: 0,
        }
    }

    /// Register one check's work, keeping repeated registrations single while live.
    pub(in crate::sema) fn register_work(
        &mut self,
        check: CheckId,
        produces: SmallVec<[dir::TypeVariableId; 2]>,
    ) {
        // leave a live registration as it stands
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

    /// Stall one check on the variables it watches, parking it when it watches none.
    pub(in crate::sema) fn stall_work(&mut self, id: CheckId, watched: &[dir::TypeVariableId]) {
        // park work without any watched variable
        if watched.is_empty() {
            return self.park_work(id);
        }

        // mark the check stalled
        self.checks.entries[id.index()].state = WorkState::Stalled;

        // watch every variable the check waits on
        for variable in watched {
            let watchers = self.watchers.entry(*variable).or_default();
            if !watchers.contains(&id) {
                watchers.push(id);
            }
        }
    }

    /// Park one check until the next settle stage.
    pub(in crate::sema) fn park_work(&mut self, id: CheckId) {
        self.checks.entries[id.index()].state = WorkState::Parked;
        self.parked.push(id);
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

    /// Wake the checks watching one variable.
    pub(in crate::sema) fn wake_variable(&mut self, variable: dir::TypeVariableId) {
        let Some(watchers) = self.watchers.swap_remove(&variable) else {
            return;
        };

        // queue each watcher that is still stalled
        for id in watchers {
            if self.checks.entries[id.index()].state == WorkState::Stalled {
                self.checks.entries[id.index()].state = WorkState::Ready;
                self.ready.push(id);
            }
        }
    }

    /// Re-queue every stalled check for one settle stage.
    pub(in crate::sema) fn requeue_stalled(&mut self) -> bool {
        // ready every check still waiting on a watched variable
        let mut requeued = false;
        for (_, watchers) in std::mem::take(&mut self.watchers) {
            for id in watchers {
                if self.checks.entries[id.index()].state == WorkState::Stalled {
                    self.checks.entries[id.index()].state = WorkState::Ready;
                    self.ready.push(id);
                    requeued = true;
                }
            }
        }

        requeued
    }

    /// Re-queue every parked check for one settle stage.
    pub(in crate::sema) fn requeue_parked(&mut self) -> bool {
        // ready every check still parked
        let mut requeued = false;
        for id in std::mem::take(&mut self.parked) {
            if self.checks.entries[id.index()].state == WorkState::Parked {
                self.checks.entries[id.index()].state = WorkState::Ready;
                self.ready.push(id);
                requeued = true;
            }
        }

        requeued
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
        self.parked.retain(|id| id.index() < count);
        self.watchers.retain(|_, watchers| {
            watchers.retain(|id| id.index() < count);

            !watchers.is_empty()
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
    /// Register one check with fulfillment, queuing it to step.
    pub(in crate::sema) fn register_check(&mut self, check: Check) -> CheckId {
        let produces = self.check_produces(&check).unwrap_or_default();
        let id = self.fulfill.checks.allocate(check);
        self.fulfill.register_work(id, produces);

        id
    }

    /// Register one check already stalled on the blockers it waits behind.
    ///
    /// A check still behind an open inference barrier watches its blockers like any
    /// other stalled work.
    /// A check bouncing back onto the ready queue every round stays live with a target
    /// naming the blocker, which the fulfillment queue reads as a producer still able
    /// to grow it.
    pub(in crate::sema) fn register_check_stalled(
        &mut self,
        check: Check,
        blockers: &[dir::TypeVariableId],
    ) -> CheckId {
        let produces = self.check_produces(&check).unwrap_or_default();
        let id = self.fulfill.checks.allocate(check);
        self.fulfill.register_work(id, produces);
        self.fulfill.stall_work(id, blockers);

        id
    }

    /// Return the open variables one check's completion can still bound.
    ///
    /// A blocked node lands its result on the expectation target, a
    /// narrowing solves its own hole, and a reselection overwrites the hole
    /// it minted at its site; every other check contributed its bounds when
    /// it was collected.
    fn check_produces(&self, check: &Check) -> CompilerResult<SmallVec<[dir::TypeVariableId; 2]>> {
        let mut produces = SmallVec::new();
        match check {
            // land a blocked node's result on the expectation target
            Check::Node(node) => {
                produces = self.type_variables(node.expectation.target)?;
            }
            // solve a narrowing's own hole once its operation decides
            Check::Narrowing(narrowing) => {
                if let Some(hole) = self.open_variable(narrowing.hole)? {
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
            Check::Relation(_) | Check::Conversion(_) | Check::Declared(_) => {}
        }

        Ok(produces)
    }

    /// Collect one relation and queue it without attempting it yet.
    pub(in crate::sema) fn register_relation(&mut self, relation: RelationCheck) -> CheckId {
        self.register_check(Check::Relation(relation))
    }

    /// Collect one relation and solve it in place, queuing it while ambiguous.
    pub(in crate::sema) fn push_relation(
        &mut self,
        relation: RelationCheck,
    ) -> CompilerResult<CheckId> {
        let id = self.fulfill.checks.allocate_relation(relation);
        self.solve_relation(id, Settle::Bounded)?;

        // queue the relation while an ambiguous verdict leaves it open
        if !self.fulfill.checks.is_complete(id) {
            self.fulfill.register_work(id, SmallVec::new());
        }

        Ok(id)
    }

    /// Step the work ready at entry once, leaving fresh registrations queued.
    pub(in crate::sema) fn solve_where_possible(&mut self, settle: Settle) -> CompilerResult<bool> {
        // take the round's ready queue, leaving fresh registrations for the next
        let ready = std::mem::take(&mut self.fulfill.ready);
        let mut round = false;

        // step each check that is still ready
        for id in ready {
            if self.fulfill.checks.state(id) != WorkState::Ready {
                continue;
            }

            round |= self.step_check(id, settle)?;
        }

        Ok(round)
    }

    /// Step one pending check, dispatching by kind.
    ///
    /// This is the one driver every kind of check steps through, resolving a check's
    /// open blockers hard at the final settle before re-evaluating.
    fn step_check(&mut self, id: CheckId, settle: Settle) -> CompilerResult<bool> {
        let check = self.fulfill.checks.get(id)?.clone();

        match check {
            Check::Relation(_) => self.step_relation(id, settle),
            Check::Node(node) => self.step_node(id, node),
            Check::Conversion(conversion) => self.step_conversion(id, conversion, settle),
            Check::Narrowing(narrowing) => self.step_narrowing(id, narrowing, settle),
            Check::Selection(selection) => self.step_selection(id, selection, settle),
            Check::Declared(entry) => self.step_declared(id, entry, settle),
        }
    }

    /// Resolve one check's blocking variables hard at the final settle.
    fn settle_blockers(
        &mut self,
        settle: Settle,
        blockers: &[dir::TypeVariableId],
    ) -> CompilerResult<()> {
        if settle == Settle::Final && !blockers.is_empty() {
            self.resolve_variables(blockers)?;
        }

        Ok(())
    }

    /// Step one pending relation check, returning whether it completed.
    fn step_relation(&mut self, id: CheckId, settle: Settle) -> CompilerResult<bool> {
        // solve the relation, finishing the work once it decides
        self.solve_relation(id, settle)?;
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
        self.stall_or_park(id, [relation.source, relation.target])?;

        Ok(false)
    }

    /// Step one blocked node by re-running its check, returning whether it completed.
    fn step_node(&mut self, id: CheckId, node: NodeCheck) -> CompilerResult<bool> {
        // re-run the whole check, which re-registers while an inference barrier stays open
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
        settle: Settle,
    ) -> CompilerResult<bool> {
        let mut source = conversion.source;

        // read the pending conversion's expectation and site
        let mut expectation = conversion.expectation;
        let site = conversion.site;

        // close the operands hard at the final settle, deciding the conversion
        let blockers = self.open_type_variables([source.ty, expectation.target])?;
        self.settle_blockers(settle, &blockers)?;

        // roll an ambiguous relation back and wait for its operands to close
        let mark = self.infer.mark(&self.fulfill);
        let mut verdict = self.constrain_type(
            site.origin(),
            expectation.cause,
            expectation.relation,
            source.ty,
            expectation.target,
        )?;
        let was_ambiguous = verdict == Verdict::Ambiguous;

        // fail an ambiguous conversion at the final settle
        if was_ambiguous && settle == Settle::Final {
            verdict = Verdict::Fails;
        }

        // undo whatever this attempt decided
        if was_ambiguous {
            let poison = self.intern_type(dir::Type::Error)?;
            self.infer.rollback(mark, poison, &mut self.fulfill)?;

            if verdict == Verdict::Ambiguous {
                self.stall_or_park(id, [source.ty, expectation.target])?;

                return Ok(false);
            }
        } else {
            // keep whatever a decided attempt bound
            self.infer.commit(mark);
        }

        // convert once both sides settle
        source.ty = self.shallow_resolve(source.ty)?;
        expectation.target = self.shallow_resolve(expectation.target)?;
        if !self
            .open_type_variables([source.ty, expectation.target])?
            .is_empty()
        {
            self.stall_or_park(id, [source.ty, expectation.target])?;

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
        settle: Settle,
    ) -> CompilerResult<bool> {
        // wait for the consulted operation to decide, closing its hole hard at the final settle
        if let Some(blocker) = self.open_operation_hole(narrowing.operation)? {
            self.settle_blockers(settle, &[blocker])?;
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
                    self.settle_blockers(settle, &[blocker])?;
                    if settle != Settle::Final {
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
        settle: Settle,
    ) -> CompilerResult<bool> {
        // wait for a stalled selection's blocker to solve
        if let Some(stalled_on) = selection.stalled_on {
            let root = self.infer.alias_root(stalled_on)?;
            self.settle_blockers(settle, &[root])?;
            if self.infer.variable(root)?.state.is_open() {
                self.fulfill.stall_work(id, &[root]);

                return Ok(false);
            }
        }

        // reattempt the selection, which re-registers itself while blocked
        self.fulfill.finish_work(id);
        let mut body = self.body();
        body.attempt_node(selection.site, selection.use_, None)?;

        Ok(self.fulfill.checks.state(id) == WorkState::Done)
    }

    /// Step one pending declared obligation, returning whether it completed.
    fn step_declared(
        &mut self,
        id: CheckId,
        entry: ObligationEntry,
        settle: Settle,
    ) -> CompilerResult<bool> {
        // complete obligations untouched outside checking
        if !self.is_checking() {
            self.fulfill.finish_work(id);

            return Ok(true);
        }

        // hold settle-phase obligations for the final settle
        let phase = entry.obligation.phase();
        if phase == ObligationPhase::Judge && settle != Settle::Final {
            self.fulfill.park_work(id);

            return Ok(false);
        }

        // run through solved types once the owning scope checked the node
        if settle != Settle::Final {
            match self.committed_node_type(entry.obligation.source()) {
                // stall on the committed type's open root
                Some(ty) => {
                    let ty = self.shallow_resolve(ty)?;
                    if let Some(root) = self.root_variable(ty)? {
                        self.fulfill.stall_work(id, &[root]);

                        return Ok(false);
                    }
                }
                // park sources without a committed type
                None => {
                    self.fulfill.park_work(id);

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

    /// Stall one check on its open operand variables, or park it.
    fn stall_or_park(
        &mut self,
        id: CheckId,
        operands: [dir::GlobalTypeId; 2],
    ) -> CompilerResult<()> {
        let open = self.open_type_variables(operands)?;
        self.fulfill.stall_work(id, &open);

        Ok(())
    }

    /// Solve one collected relation check, keeping ambiguous failures pending.
    pub(in crate::sema) fn solve_relation(
        &mut self,
        id: CheckId,
        settle: Settle,
    ) -> CompilerResult<()> {
        if self.fulfill.checks.is_complete(id) {
            return Ok(());
        }

        let Check::Relation(relation) = self.fulfill.checks.get(id)?.clone() else {
            return Err(CompilerError::Internal {
                message: format!("relation check {id:?} names a non-relation check"),
            });
        };

        // close the operands at the final settle, deciding the check hard
        let blockers = self.open_type_variables([relation.source, relation.target])?;
        self.settle_blockers(settle, &blockers)?;

        // attempt the relation, keeping whatever it bound
        let mark = self.infer.mark(&self.fulfill);
        let verdict = self.constrain_type(
            relation.origin,
            relation.cause,
            relation.relation,
            relation.source,
            relation.target,
        )?;
        self.infer.commit(mark);

        // keep the attempt's bounds and leave the check pending while it is ambiguous
        if verdict == Verdict::Ambiguous && settle != Settle::Final {
            return Ok(());
        }

        // fail an undecided relation at the final settle
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
        self.record_event(CheckEvent::Checked {
            check: id,
            is_finished: true,
        });

        Ok(())
    }

    /// Retain one failed check until its cause tree is complete.
    pub(in crate::sema) fn record_failure(&mut self, mut check: FailedCheck) -> CompilerResult<()> {
        // keep a failure over open operands provisional until they close
        check.is_provisional =
            (self.type_flags(check.source)? | self.type_flags(check.target)?).has_variable();
        self.fulfill.failures.push(check);

        Ok(())
    }
}
