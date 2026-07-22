use std::collections::{HashMap, VecDeque};

use destack_mir as mir;

use super::owned::OwnedValues;
use super::state::DropState;

/// One planned ownership drop point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DropPoint {
    /// Instruction index where drop is inserted.
    pub(super) index: usize,
    /// Complete owned value to drop.
    pub(super) value: mir::Value,
}

/// Drop plan for one function.
pub(super) struct DropPlan<'a> {
    /// The function being planned.
    function: &'a mir::Function,
    /// The MIR tree.
    tree: &'a mir::Tree,
    /// Move-only values in this function.
    owned: OwnedValues,
    /// Receiver borrowed by a user-authored drop hook.
    hook_receiver: Option<mir::Value>,
    /// SSA value liveness.
    liveness: mir::FunctionLiveness,
    /// Available owned values at block entry.
    available_at_entry: HashMap<mir::LocalNodeId<mir::Block>, DropState>,
    /// Planned drops by block.
    drops: HashMap<mir::LocalNodeId<mir::Block>, Vec<DropPoint>>,
}

impl<'a> DropPlan<'a> {
    /// Build planned drops for one function.
    pub(super) fn build(
        function: &'a mir::Function,
        tree: &'a mir::Tree,
        hook_receiver: Option<mir::Value>,
    ) -> HashMap<mir::LocalNodeId<mir::Block>, Vec<DropPoint>> {
        let liveness = mir::FunctionLiveness::build(function, tree);

        let mut plan = Self {
            function,
            tree,
            owned: OwnedValues::new(function.value_types().len()),
            hook_receiver,
            liveness,
            available_at_entry: HashMap::new(),
            drops: HashMap::new(),
        };
        plan.owned = plan.collect_owned_values();
        plan.available_at_entry = plan.compute_available_entries();
        plan.plan_blocks();

        plan.drops
    }

    /// Collect move-only values that need explicit drop.
    fn collect_owned_values(&self) -> OwnedValues {
        let mut owned = OwnedValues::new(self.function.value_types().len());

        // seed parameters owned at function entry
        for parameter in &self.function.parameters {
            let value = parameter.value;
            if Some(value) == self.hook_receiver {
                continue;
            }

            if self.is_owned_value(value) {
                owned.insert(value);
            }
        }

        // track instruction destinations by value type
        for (index, ty) in self.function.value_types().iter().enumerate() {
            let value = mir::Value::new(index as u32);
            if Some(value) == self.hook_receiver {
                continue;
            }

            let Some(ty) = ty else {
                continue;
            };
            if self.tree.get(*ty).copy(self.tree).is_no() {
                owned.insert(value);
            }
        }

        owned
    }

    /// Compute available owned values at each block entry.
    fn compute_available_entries(&self) -> HashMap<mir::LocalNodeId<mir::Block>, DropState> {
        let Some(entry) = self.function.entry() else {
            return HashMap::new();
        };

        let graph = mir::ControlFlowGraph::build(self.function, self.tree);
        let mut entries = HashMap::new();
        let mut exits = HashMap::new();
        let mut worklist = VecDeque::new();

        entries.insert(entry, DropState::parameters(self.function, &self.owned));
        worklist.push_back(entry);

        // propagate availability until predecessor merges stop changing
        while let Some(block_id) = worklist.pop_front() {
            let entry_values = self.available_values_at_entry(block_id, &graph, &entries, &exits);
            let exit_values = self.transfer_available_values(block_id, entry_values.clone());
            let old_entry = entries.insert(block_id, entry_values);
            let old_exit = exits.insert(block_id, exit_values);

            // skip successors when the block state is stable
            if old_entry.as_ref() == entries.get(&block_id)
                && old_exit.as_ref() == exits.get(&block_id)
            {
                continue;
            }

            // revisit successors after changed exits
            let block = self.tree.get(block_id);
            let terminator = self.tree.get(block.terminator);
            for successor in terminator.successors(self.tree) {
                if !worklist.contains(&successor) {
                    worklist.push_back(successor);
                }
            }
        }

        entries
    }

    /// Return available owned values at one block entry.
    fn available_values_at_entry(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
        graph: &mir::ControlFlowGraph,
        entries: &HashMap<mir::LocalNodeId<mir::Block>, DropState>,
        exits: &HashMap<mir::LocalNodeId<mir::Block>, DropState>,
    ) -> DropState {
        if Some(block_id) == self.function.entry() {
            return entries
                .get(&block_id)
                .cloned()
                .expect("entry block must have initial drop state");
        }

        // merge only reached predecessors
        let mut predecessors = graph
            .predecessors(block_id)
            .iter()
            .copied()
            .filter(|predecessor| exits.contains_key(predecessor));
        let Some(first) = predecessors.next() else {
            return DropState::default();
        };

        // seed the intersection from the first predecessor
        let first_exit = exits
            .get(&first)
            .cloned()
            .expect("reached predecessor must have exit drop state");
        let mut merged = self.bind_edge_parameters(first_exit, first, block_id);

        // keep values carried by every reached predecessor
        for predecessor in predecessors {
            let exit = exits
                .get(&predecessor)
                .cloned()
                .expect("reached predecessor must have exit drop state");
            let next = self.bind_edge_parameters(exit, predecessor, block_id);
            merged.intersect_with(&next);
        }

        merged.retain_owned(&self.owned);

        merged
    }

    /// Transfer available owned values through one block.
    fn transfer_available_values(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
        mut available: DropState,
    ) -> DropState {
        let block = self.tree.get(block_id);

        // consume old values and define new owned values
        for &instruction_id in &block.instructions {
            let instruction = self.tree.get(instruction_id);
            let consumed = self.consumed_by_instruction(instruction);

            available.remove_consumed(&consumed);
            if let Some(value) = self.decomposed_value(instruction) {
                available.remove(value);
            }
            available.apply_instruction(instruction);
            available.insert_destination(instruction, &self.owned);
        }

        // move terminator values into the caller or successor edges
        let terminator = self.tree.get(block.terminator);
        let consumed = self.consumed_by_terminator(terminator);
        let carried = self.carried_by_terminator(terminator, &available);

        available.remove_consumed(&consumed);
        for value in self.drops_before_terminator(block_id, &available, &carried) {
            available.remove(value);
        }

        available
    }

    /// Bind available values to successor block parameters.
    fn bind_edge_parameters(
        &self,
        mut available: DropState,
        predecessor: mir::LocalNodeId<mir::Block>,
        successor: mir::LocalNodeId<mir::Block>,
    ) -> DropState {
        let predecessor = self.tree.get(predecessor);
        let terminator = self.tree.get(predecessor.terminator);
        let arguments = terminator.successor_arguments(self.tree, successor);
        let parameters = terminator.successor_parameters(self.tree, successor);

        // transfer ownership to matching successor parameters
        for (parameter, argument) in parameters.iter().zip(arguments) {
            available.bind(*argument, parameter.value, &self.owned);
        }

        available
    }

    /// Plan drops for every reachable block.
    fn plan_blocks(&mut self) {
        for &block_id in self.function.blocks() {
            let Some(mut available) = self.available_at_entry.get(&block_id).cloned() else {
                continue;
            };

            self.plan_block(block_id, &mut available);
        }
    }

    /// Plan drops inside one block.
    fn plan_block(&mut self, block_id: mir::LocalNodeId<mir::Block>, available: &mut DropState) {
        let block = self.tree.get(block_id);

        // drop last nonconsuming uses after the instruction
        for (index, &instruction_id) in block.instructions.iter().enumerate() {
            let instruction = self.tree.get(instruction_id);
            let consumed = self.consumed_by_instruction(instruction);

            available.remove_consumed(&consumed);
            if let Some(value) = self.decomposed_value(instruction) {
                available.remove(value);
            }
            available.apply_instruction(instruction);
            self.plan_after_instruction(block_id, index, instruction, available, &consumed);
            available.insert_destination(instruction, &self.owned);
            self.plan_dead_destination(block_id, index, instruction, available);
        }

        // drop remaining values before the terminator
        let terminator = self.tree.get(block.terminator);
        let consumed = self.consumed_by_terminator(terminator);
        let carried = self.carried_by_terminator(terminator, available);
        let end_index = block.instructions.len();

        available.remove_consumed(&consumed);
        for value in self.drops_before_terminator(block_id, available, &carried) {
            self.add_drop(block_id, end_index, value);
            available.remove(value);
        }
    }

    /// Plan drops after one instruction.
    fn plan_after_instruction(
        &mut self,
        block_id: mir::LocalNodeId<mir::Block>,
        index: usize,
        instruction: &mir::Instruction,
        available: &mut DropState,
        consumed: &OwnedValues,
    ) {
        let used = self.owned_instruction_uses(instruction);

        // skip values consumed by the instruction itself
        for value in used.values().rev() {
            if consumed.contains(value) || !available.contains(value) {
                continue;
            }
            if self
                .liveness
                .is_value_live_after_instruction(block_id, index, value, self.tree)
            {
                continue;
            }
            if self.has_live_borrow_after_instruction(block_id, index, value, available) {
                continue;
            }

            self.add_drop(block_id, index + 1, value);
            available.remove(value);
        }
    }

    /// Plan an unused owned destination immediately after its definition.
    fn plan_dead_destination(
        &mut self,
        block_id: mir::LocalNodeId<mir::Block>,
        index: usize,
        instruction: &mir::Instruction,
        available: &mut DropState,
    ) {
        let Some(destination) = instruction.destination() else {
            return;
        };
        if !available.contains(destination) {
            return;
        }
        if self
            .liveness
            .is_value_live_after_instruction(block_id, index, destination, self.tree)
        {
            return;
        }

        self.add_drop(block_id, index + 1, destination);
        available.remove(destination);
    }

    /// Return whether a borrow derived from one value is still live.
    fn has_live_borrow_after_instruction(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
        index: usize,
        value: mir::Value,
        available: &DropState,
    ) -> bool {
        available.values_derived_from(value).any(|borrow| {
            if !self.is_borrowed_reference(borrow) {
                return false;
            }

            self.liveness
                .is_value_live_after_instruction(block_id, index, borrow, self.tree)
        })
    }

    /// Return whether a borrow derived from one value is live after one terminator.
    fn has_live_borrow_after_terminator(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
        value: mir::Value,
        available: &DropState,
    ) -> bool {
        available.values_derived_from(value).any(|borrow| {
            self.is_borrowed_reference(borrow) && self.liveness.is_value_live_out(block_id, borrow)
        })
    }

    /// Return owned values used by one instruction.
    fn owned_instruction_uses(&self, instruction: &mir::Instruction) -> OwnedValues {
        let mut values = OwnedValues::new(self.function.value_types().len());

        for value in instruction.reads(self.tree) {
            values.insert_reference(value, &self.owned);
        }

        values
    }

    /// Return owned values consumed by one instruction.
    fn consumed_by_instruction(&self, instruction: &mir::Instruction) -> OwnedValues {
        let mut values = OwnedValues::new(self.function.value_types().len());

        for value in instruction.consumes(self.tree) {
            values.insert_reference(value, &self.owned);
        }

        values
    }

    /// Return aggregate ownership replaced by one projected destination.
    fn decomposed_value(&self, instruction: &mir::Instruction) -> Option<mir::Value> {
        match instruction {
            mir::Instruction::FieldGet {
                destination,
                aggregate,
                ..
            }
            | mir::Instruction::ElementGet {
                destination,
                aggregate,
                ..
            } if self.is_owned_value(*destination) => Some(*aggregate),
            _ => None,
        }
    }

    /// Return owned values consumed by one terminator.
    fn consumed_by_terminator(&self, terminator: &mir::Terminator) -> OwnedValues {
        let mut values = OwnedValues::new(self.function.value_types().len());

        for value in terminator.consumes(self.tree) {
            values.insert_reference(value, &self.owned);
        }

        values
    }

    /// Return owned values carried into successor block parameters.
    fn carried_by_terminator(
        &self,
        terminator: &mir::Terminator,
        available: &DropState,
    ) -> OwnedValues {
        let mut carried = OwnedValues::new(self.function.value_types().len());

        // keep edge argument ownership alive in successor parameters
        for successor in terminator.successors(self.tree) {
            let arguments = terminator.successor_arguments(self.tree, successor);
            let parameters = terminator.successor_parameters(self.tree, successor);

            // keep each carried value alive in successor parameters
            for (_, argument) in parameters.iter().zip(arguments) {
                if available.contains(*argument) {
                    carried.insert(*argument);
                }
            }
        }

        carried
    }

    /// Return values that should be dropped before one terminator.
    fn drops_before_terminator(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
        available: &DropState,
        carried: &OwnedValues,
    ) -> Vec<mir::Value> {
        available
            .owned
            .values()
            .rev()
            .filter(|value| !carried.contains(*value))
            .filter(|value| !self.is_value_used_by_terminator(block_id, *value))
            .filter(|value| !self.liveness.is_value_live_out(block_id, *value))
            .filter(|value| !self.has_live_borrow_after_terminator(block_id, *value, available))
            .collect()
    }

    /// Return whether one value is needed by the terminator itself.
    fn is_value_used_by_terminator(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
        value: mir::Value,
    ) -> bool {
        let block = self.tree.get(block_id);
        let terminator = self.tree.get(block.terminator);
        let consumed = self.consumed_by_terminator(terminator);
        if consumed.contains(value) {
            return false;
        }

        terminator
            .uses(self.tree)
            .iter()
            .copied()
            .any(|used| used == value)
    }

    /// Add one planned drop.
    fn add_drop(&mut self, block: mir::LocalNodeId<mir::Block>, index: usize, value: mir::Value) {
        self.drops
            .entry(block)
            .or_default()
            .push(DropPoint { index, value });
    }

    /// Return whether one value has move-only ownership.
    fn is_owned_value(&self, value: mir::Value) -> bool {
        let Some(ty) = self.function.value_type(value) else {
            return false;
        };

        self.tree.get(ty).copy(self.tree).is_no()
    }

    /// Return whether one value is a borrowed reference-like value.
    fn is_borrowed_reference(&self, value: mir::Value) -> bool {
        let Some(ty) = self.function.value_type(value) else {
            return false;
        };

        self.tree.get(ty).is_borrowed_reference()
    }
}
