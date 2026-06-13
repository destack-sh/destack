use destack_dir as dir;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::Task;
use crate::{CompilerError, CompilerResult};

/// One decided node meaning.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Decision {
    /// Resolved lexical name.
    Name(dir::NameResolution),
    /// Resolved contextual receiver.
    Receiver(dir::ReceiverResolution),
    /// Resolved member access.
    Member(dir::MemberResolution),
    /// Resolved call, including builtin operator applications.
    Call(dir::CallResolution),
    /// Resolved paired read-write place accessors.
    ReadWrite(dir::ReadWriteResolution),
    /// Resolved construct expression.
    Construct(dir::ConstructResolution),
    /// Resolved pattern meaning.
    Pattern(dir::PatternResolution),
    /// Rejected node with reported diagnostics.
    Rejected,
}

/// One node decision slot.
#[derive(Debug)]
enum DecisionSlot {
    /// Tasks waiting for the decision.
    Pending(SmallVec<[Task; 2]>),
    /// The decided meaning.
    Decided(Box<Decision>),
}

/// Node decisions for one checked component.
#[derive(Debug)]
pub(in crate::check) struct DecisionTable {
    /// Decision slots keyed by source node.
    slots: IndexMap<dir::GlobalNodeIdAny, DecisionSlot>,
}

impl DecisionTable {
    /// Create an empty decision table.
    pub(in crate::check) fn new() -> Self {
        Self {
            slots: IndexMap::new(),
        }
    }

    /// Return one node decision when decided.
    pub(in crate::check) fn get(&self, node: dir::GlobalNodeIdAny) -> Option<&Decision> {
        match self.slots.get(&node) {
            Some(DecisionSlot::Decided(decision)) => Some(decision.as_ref()),
            Some(DecisionSlot::Pending(_)) | None => None,
        }
    }

    /// Record one node decision and return the woken waiters.
    /// Decisions are derived facts: re-deriving the same decision
    /// through another walk path collapses, conflicting ones error.
    pub(in crate::check) fn decide(
        &mut self,
        node: dir::GlobalNodeIdAny,
        decision: Decision,
    ) -> CompilerResult<SmallVec<[Task; 2]>> {
        // collapse identical re-derivations without waking anyone
        if let Some(DecisionSlot::Decided(previous)) = self.slots.get(&node)
            && previous.as_ref() == &decision
        {
            return Ok(SmallVec::new());
        }

        let previous = self
            .slots
            .insert(node, DecisionSlot::Decided(Box::new(decision)));
        match previous {
            // wake tasks parked on the pending slot
            Some(DecisionSlot::Pending(waiters)) => Ok(waiters),
            // first decision without waiters
            None => Ok(SmallVec::new()),
            // a node must decide exactly once
            Some(DecisionSlot::Decided(_)) => Err(CompilerError::Internal {
                message: format!("check node {node:?} was decided twice"),
            }),
        }
    }

    /// Forget one node decision during probe rollback, restoring the
    /// waiters that were parked on the slot before it decided.
    pub(in crate::check) fn undecide(
        &mut self,
        node: dir::GlobalNodeIdAny,
        waiters: SmallVec<[Task; 2]>,
    ) {
        if matches!(self.slots.get(&node), Some(DecisionSlot::Decided(_))) {
            self.slots.insert(node, DecisionSlot::Pending(waiters));
        }
    }

    /// Park one task until the node decides.
    pub(in crate::check) fn wait(&mut self, node: dir::GlobalNodeIdAny, task: Task) {
        let slot = self
            .slots
            .entry(node)
            .or_insert_with(|| DecisionSlot::Pending(SmallVec::new()));

        // park once on undecided slots
        if let DecisionSlot::Pending(waiters) = slot
            && !waiters.contains(&task)
        {
            waiters.push(task);
        }
    }

    /// Pop the youngest parked waiter during probe rollback.
    pub(in crate::check) fn pop_waiter(&mut self, node: dir::GlobalNodeIdAny) {
        if let Some(DecisionSlot::Pending(waiters)) = self.slots.get_mut(&node) {
            waiters.pop();
        }
    }

    /// Return the number of decided nodes.
    pub(in crate::check) fn count(&self) -> usize {
        self.slots
            .values()
            .filter(|slot| matches!(slot, DecisionSlot::Decided(_)))
            .count()
    }

    /// Iterate decided nodes with their decisions.
    pub(in crate::check) fn iter(
        &self,
    ) -> impl Iterator<Item = (dir::GlobalNodeIdAny, &Decision)> + '_ {
        self.slots.iter().filter_map(|(node, slot)| match slot {
            DecisionSlot::Decided(decision) => Some((*node, decision.as_ref())),
            DecisionSlot::Pending(_) => None,
        })
    }
}
