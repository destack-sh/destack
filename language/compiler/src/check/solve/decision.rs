use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::{CheckEvent, CheckState, Task};
use crate::{CompilerError, CompilerResult};

/// One decided node meaning.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Decision {
    /// Resolved lexical name.
    Name(dir::NameResolution),
    /// Resolved explicit generic application.
    Instantiation(dir::InstantiationResolution),
    /// Resolved contextual receiver.
    Receiver(dir::ReceiverResolution),
    /// Resolved member access.
    Member(dir::MemberResolution),
    /// Resolved call, including builtin operator applications.
    Call(dir::CallResolution),
    /// Resolved paired read-write place accessors.
    ReadWrite(dir::ReadWriteResolution),
    /// Resolved runtime predicate expression.
    Guard(dir::GuardResolution),
    /// Resolved construct expression.
    Construct(dir::ConstructResolution),
    /// Resolved pattern meaning.
    Pattern(dir::PatternResolution),
    /// Resolved assignment pattern meaning.
    AssignPattern(dir::AssignPatternResolution),
    /// Rejected node with reported diagnostics.
    Rejected,
}

impl Decision {
    /// Return selected generic argument bindings when the decision applies a generic target.
    pub(in crate::check) fn generic_arguments(&self) -> Option<&[dir::GenericArgumentBinding]> {
        match self {
            Self::Call(resolution) => resolution.target.direct_generic_arguments(),
            Self::Construct(resolution) => Some(resolution.target.generic_arguments()),
            _ => None,
        }
    }
}

impl CheckState<'_> {
    /// Record one node decision and wake its waiters.
    pub(in crate::check) fn record_decision(
        &mut self,
        node: dir::GlobalNodeIdAny,
        decision: Decision,
    ) -> CompilerResult<()> {
        if matches!(decision, Decision::Rejected) {
            let error = self.push_type(node.module_id, dir::Type::Error, node.local_id)?;
            self.bind_node_type(node, error)?;
        }

        let waiters = self.solver.decide(node, decision)?;
        self.record_event(CheckEvent::NodeDecided { node });

        // wake tasks parked on the decision
        for waiter in waiters {
            self.queue_task(waiter);
        }

        Ok(())
    }
}

/// One node decision slot.
#[derive(Debug, Clone)]
pub(in crate::check) enum DecisionSlot {
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

    /// Return one raw decision slot.
    pub(in crate::check) fn slot(&self, node: dir::GlobalNodeIdAny) -> Option<&DecisionSlot> {
        self.slots.get(&node)
    }

    /// Record one node decision and return the woken waiters.
    /// Decisions are derived facts: re-deriving the same decision
    /// through another walk path collapses, conflicting ones error.
    pub(in crate::check) fn decide(
        &mut self,
        node: dir::GlobalNodeIdAny,
        decision: Decision,
    ) -> CompilerResult<SmallVec<[Task; 2]>> {
        match self.slots.get(&node) {
            // collapse identical re-derivations without waking anyone
            Some(DecisionSlot::Decided(previous)) if previous.as_ref() == &decision => {
                Ok(SmallVec::new())
            }
            // a node must decide exactly once
            Some(DecisionSlot::Decided(_)) => Err(CompilerError::Internal {
                message: format!("check node {node:?} was decided twice"),
            }),
            // wake tasks parked on the pending slot
            Some(DecisionSlot::Pending(_)) => {
                let Some(DecisionSlot::Pending(waiters)) = self
                    .slots
                    .insert(node, DecisionSlot::Decided(Box::new(decision)))
                else {
                    unreachable!("pending decision slot was just matched");
                };

                Ok(waiters)
            }
            // first decision without waiters
            None => {
                self.slots
                    .insert(node, DecisionSlot::Decided(Box::new(decision)));

                Ok(SmallVec::new())
            }
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

    /// Return the number of decided nodes.
    pub(in crate::check) fn count(&self) -> usize {
        self.slots
            .values()
            .filter(|slot| matches!(slot, DecisionSlot::Decided(_)))
            .count()
    }

    /// Take decided nodes owned by one module.
    pub(in crate::check) fn take_module(
        &mut self,
        module: ModuleId,
    ) -> Vec<(dir::GlobalNodeIdAny, Decision)> {
        let mut decisions = Vec::new();
        let nodes = self
            .slots
            .iter()
            .filter_map(|(node, slot)| {
                (node.module_id == module && matches!(slot, DecisionSlot::Decided(_)))
                    .then_some(*node)
            })
            .collect::<Vec<_>>();

        for node in nodes {
            let Some(DecisionSlot::Decided(decision)) = self.slots.swap_remove(&node) else {
                continue;
            };
            decisions.push((node, *decision));
        }

        decisions
    }

    /// Insert one raw decision slot.
    pub(in crate::check) fn insert_slot(&mut self, node: dir::GlobalNodeIdAny, slot: DecisionSlot) {
        self.slots.insert(node, slot);
    }

    /// Remove one slot.
    pub(in crate::check) fn remove(&mut self, node: dir::GlobalNodeIdAny) -> Option<DecisionSlot> {
        self.slots.swap_remove(&node)
    }
}
