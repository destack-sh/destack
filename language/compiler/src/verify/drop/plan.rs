use std::collections::{HashMap, VecDeque};

use crate::common::mir::terminator_arguments_for_successor;
use crate::verify::value::{instruction_consumes, instruction_uses, terminator_consumes};
use destack_mir as mir;

use super::owned::OwnedValues;
use super::state::DropState;

/// One explicit lifetime end marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DropMarker {
    /// Instruction index where the marker is inserted.
    pub(super) index: usize,
    /// Place whose lifetime ends.
    pub(super) place: mir::Place,
    /// Storage release required before the marker.
    pub(super) release: DropRelease,
}

/// Storage release required before one drop marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DropRelease {
    /// No storage release is needed.
    None,
    /// Release unique heap storage.
    Free(mir::Value),
}

/// Drop plan for one function.
pub(super) struct DropPlan<'a> {
    /// The function being planned.
    function: &'a mir::Function,
    /// The MIR tree.
    tree: &'a mir::Tree,
    /// Move-only values in this function.
    owned: OwnedValues,
    /// SSA value liveness.
    liveness: mir::FunctionLiveness,
    /// Available owned values at block entry.
    available_at_entry: HashMap<mir::LocalNodeId<mir::Block>, DropState>,
    /// Planned drops by block.
    drops: HashMap<mir::LocalNodeId<mir::Block>, Vec<DropMarker>>,
}

impl<'a> DropPlan<'a> {
    /// Build planned drops for one function.
    pub(super) fn build(
        function: &'a mir::Function,
        tree: &'a mir::Tree,
    ) -> HashMap<mir::LocalNodeId<mir::Block>, Vec<DropMarker>> {
        let liveness = mir::FunctionLiveness::build(function, tree);

        let mut plan = Self {
            function,
            tree,
            owned: OwnedValues::default(),
            liveness,
            available_at_entry: HashMap::new(),
            drops: HashMap::new(),
        };
        plan.owned = plan.collect_owned_values();
        plan.available_at_entry = plan.compute_available_entries();
        plan.plan_blocks();

        plan.drops
    }

    /// Collect move-only values that need an explicit lifetime end.
    fn collect_owned_values(&self) -> OwnedValues {
        let mut owned = OwnedValues::default();

        // seed parameters owned at function entry
        for parameter in &self.function.parameters {
            let Some(value) = parameter.value.value() else {
                continue;
            };
            if self.is_owned_value(value) {
                owned.insert(value);
            }
        }

        // track instruction destinations by value type
        for (index, ty) in self.function.value_types.iter().enumerate() {
            let Some(ty) = ty else {
                continue;
            };
            if self.tree.get(*ty).copy().is_no() {
                owned.insert(mir::Value::new(index as u32));
            }
        }

        owned
    }

    /// Compute available owned values at each block entry.
    fn compute_available_entries(&self) -> HashMap<mir::LocalNodeId<mir::Block>, DropState> {
        let Some(entry) = self.function.entry else {
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
            for successor in terminator.successors() {
                let Some(successor) = successor.block() else {
                    continue;
                };
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
        if Some(block_id) == self.function.entry {
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
            let moved_places = self.places_moved_by_instruction(instruction);

            available.remove_consumed(&consumed);
            for place in moved_places {
                available.move_place(place);
            }
            available.insert_destination(instruction, &self.owned);
        }

        // move terminator values into the caller or successor edges
        let terminator = self.tree.get(block.terminator);
        let consumed = self.consumed_by_terminator(terminator);
        available.remove_consumed(&consumed);

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
        let arguments = terminator_arguments_for_successor(terminator, successor);
        let successor_block = self.tree.get(successor);

        // transfer ownership to matching successor parameters
        for (parameter, argument) in successor_block.parameters.iter().zip(arguments) {
            let (Some(parameter), Some(argument)) = (parameter.value.value(), argument.value())
            else {
                continue;
            };
            if !available.contains(argument) || !self.owned.contains(parameter) {
                continue;
            }

            available.move_place(mir::Place::value(argument.into()));
            available.owned.insert(parameter);
        }

        available
    }

    /// Plan drops for every reachable block.
    fn plan_blocks(&mut self) {
        for &block_id in &self.function.blocks {
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
            let moved_places = self.places_moved_by_instruction(instruction);

            available.remove_consumed(&consumed);
            for place in moved_places {
                available.move_place(place);
            }
            self.plan_after_instruction(block_id, index, instruction, available, &consumed);
            available.insert_destination(instruction, &self.owned);
        }

        // drop remaining values before the terminator
        let terminator = self.tree.get(block.terminator);
        let consumed = self.consumed_by_terminator(terminator);
        let carried = self.carried_by_terminator(terminator, available);
        let end_index = block.instructions.len();

        available.remove_consumed(&consumed);
        for value in self.drops_before_terminator(block_id, available, &carried) {
            self.add_value_drop(block_id, end_index, value, &available.moved);
            available.move_place(mir::Place::value(value.into()));
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
        for value in used.values() {
            if consumed.contains(value) || !available.contains(value) {
                continue;
            }
            if self
                .liveness
                .is_value_live_after_instruction(block_id, index, value, self.tree)
            {
                continue;
            }

            self.add_value_drop(block_id, index + 1, value, &available.moved);
            available.move_place(mir::Place::value(value.into()));
        }
    }

    /// Return owned values used by one instruction.
    fn owned_instruction_uses(&self, instruction: &mir::Instruction) -> OwnedValues {
        let mut values = OwnedValues::default();

        for value in instruction_uses(instruction, self.tree) {
            values.insert_reference(value, &self.owned);
        }

        values
    }

    /// Return owned values consumed by one instruction.
    fn consumed_by_instruction(&self, instruction: &mir::Instruction) -> OwnedValues {
        let mut values = OwnedValues::default();

        for value in instruction_consumes(instruction, self.tree) {
            values.insert_reference(value, &self.owned);
        }

        values
    }

    /// Return places moved by one instruction.
    fn places_moved_by_instruction(&self, instruction: &mir::Instruction) -> Vec<mir::Place> {
        // move known fields when extracting move-only values
        if let mir::Instruction::FieldGet {
            destination,
            aggregate,
            index,
            ..
        } = instruction
            && self.is_owned_reference(*destination)
        {
            return vec![self.place_moved_by_projection(
                *aggregate,
                mir::PlaceProjection::Field { index: *index },
            )];
        }

        // move known elements when extracting move-only values
        if let mir::Instruction::ElementGet {
            destination,
            array,
            index,
            ..
        } = instruction
            && self.is_owned_reference(*destination)
        {
            return vec![self.place_moved_by_projection(
                *array,
                mir::PlaceProjection::Element { index: *index },
            )];
        }

        Vec::new()
    }

    /// Return owned values consumed by one terminator.
    fn consumed_by_terminator(&self, terminator: &mir::Terminator) -> OwnedValues {
        let mut values = OwnedValues::default();

        for value in terminator_consumes(terminator) {
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
        let mut carried = OwnedValues::default();

        // keep edge argument ownership alive in successor parameters
        for successor in terminator.successors() {
            let Some(successor) = successor.block() else {
                continue;
            };
            let arguments = terminator_arguments_for_successor(terminator, successor);
            let successor_block = self.tree.get(successor);

            // keep each carried value alive in successor parameters
            for (parameter, argument) in successor_block.parameters.iter().zip(arguments) {
                let (Some(_), Some(argument)) = (parameter.value.value(), argument.value()) else {
                    continue;
                };
                if available.contains(argument) {
                    carried.insert(argument);
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
            .filter(|value| !carried.contains(*value))
            .filter(|value| !self.liveness.is_value_live_out(block_id, *value))
            .collect()
    }

    /// Add planned drops for one value.
    fn add_value_drop(
        &mut self,
        block: mir::LocalNodeId<mir::Block>,
        index: usize,
        value: mir::Value,
        moved: &[mir::Place],
    ) {
        let place = self.place_for_value(value.into());
        let ty = self
            .type_for_value(value)
            .expect("owned value must have a type");

        for place in self.drop_places_for(place, ty, moved) {
            self.add_drop(block, index, place);
        }
    }

    /// Add one planned drop.
    fn add_drop(&mut self, block: mir::LocalNodeId<mir::Block>, index: usize, place: mir::Place) {
        let release = self.release_for_place(&place);

        self.drops.entry(block).or_default().push(DropMarker {
            index,
            place,
            release,
        });
    }

    /// Return whether one value has move-only ownership.
    fn is_owned_value(&self, value: mir::Value) -> bool {
        let Some(ty) = self.type_for_value(value) else {
            return false;
        };

        self.tree.get(ty).copy().is_no()
    }

    /// Return whether one value reference has move-only ownership.
    fn is_owned_reference(&self, value: mir::ValueReference) -> bool {
        let Some(value) = value.value() else {
            return false;
        };

        self.is_owned_value(value)
    }

    /// Return the moved place for one projection.
    fn place_moved_by_projection(
        &self,
        value: mir::ValueReference,
        projection: mir::PlaceProjection,
    ) -> mir::Place {
        let place = self.place_for_value(value);
        if self.is_union_value(value) {
            return place;
        }

        place.with_projection(projection)
    }

    /// Return whether one value has a union type.
    fn is_union_value(&self, value: mir::ValueReference) -> bool {
        let Some(ty) = value.value().and_then(|value| self.type_for_value(value)) else {
            return false;
        };

        matches!(self.tree.get(ty), mir::Type::Variant { .. })
    }

    /// Return the storage release required for one place.
    fn release_for_place(&self, place: &mir::Place) -> DropRelease {
        let mir::PlaceOrigin::Value(value) = place.origin else {
            return DropRelease::None;
        };
        if !place.projections.is_empty() {
            return DropRelease::None;
        }

        let Some(value) = value.value() else {
            return DropRelease::None;
        };
        let ty = self
            .type_for_value(value)
            .expect("owned value must have a type");

        if self.tree.get(ty).is_unique_reference() {
            DropRelease::Free(value)
        } else {
            DropRelease::None
        }
    }

    /// Return the best known place for one value.
    fn place_for_value(&self, value: mir::ValueReference) -> mir::Place {
        let Some(value) = value.value() else {
            return mir::Place::value(value);
        };

        self.function
            .value_place(value)
            .cloned()
            .unwrap_or_else(|| mir::Place::value(value.into()))
    }

    /// Return minimal initialized places that need drops.
    fn drop_places_for(
        &self,
        place: mir::Place,
        ty: mir::LocalNodeId<mir::Type>,
        moved: &[mir::Place],
    ) -> Vec<mir::Place> {
        if moved.iter().any(|moved| moved.contains(&place)) {
            return Vec::new();
        }
        if !moved.iter().any(|moved| place.contains(moved)) {
            return vec![place];
        }

        self.child_drop_places(place, ty, moved)
    }

    /// Return child drop places after a partial move.
    fn child_drop_places(
        &self,
        place: mir::Place,
        ty: mir::LocalNodeId<mir::Type>,
        moved: &[mir::Place],
    ) -> Vec<mir::Place> {
        match self.tree.get(ty) {
            mir::Type::Struct { fields, .. } => fields
                .iter()
                .enumerate()
                .flat_map(|(index, field)| {
                    let field = self.tree.get(*field);
                    self.drop_child_place(
                        &place,
                        mir::PlaceProjection::Field {
                            index: index as u32,
                        },
                        field.ty.clone(),
                        moved,
                    )
                })
                .collect(),
            mir::Type::Tuple { elements, .. } => elements
                .iter()
                .enumerate()
                .flat_map(|(index, element)| {
                    self.drop_child_place(
                        &place,
                        mir::PlaceProjection::Field {
                            index: index as u32,
                        },
                        element.clone(),
                        moved,
                    )
                })
                .collect(),
            mir::Type::Array {
                element, length, ..
            } => (0..*length)
                .flat_map(|index| {
                    self.drop_child_place(
                        &place,
                        mir::PlaceProjection::Element {
                            index: index as u32,
                        },
                        element.clone(),
                        moved,
                    )
                })
                .collect(),
            mir::Type::Newtype { inner, .. } => self.drop_child_place(
                &place,
                mir::PlaceProjection::Field { index: 0 },
                inner.clone(),
                moved,
            ),
            mir::Type::Variant { .. } => {
                panic!("partial union drops require active variant metadata")
            }
            ty => {
                panic!("partial drop is not defined for type {ty:?}")
            }
        }
    }

    /// Return drop places under one child projection.
    fn drop_child_place(
        &self,
        parent: &mir::Place,
        projection: mir::PlaceProjection,
        ty: mir::TypeReference,
        moved: &[mir::Place],
    ) -> Vec<mir::Place> {
        let Some(ty) = ty.ty() else {
            panic!("partial drop requires a concrete child type")
        };
        if self.tree.get(ty).copy().is_yes() {
            return Vec::new();
        }

        let place = parent.clone().with_projection(projection);

        self.drop_places_for(place, ty, moved)
    }

    /// Return the known type for one value.
    fn type_for_value(&self, value: mir::Value) -> Option<mir::LocalNodeId<mir::Type>> {
        if let Some(ty) = self.function.value_type(value) {
            return Some(ty);
        }

        self.function
            .parameters
            .iter()
            .find(|parameter| parameter.value.value() == Some(value))
            .and_then(|parameter| parameter.ty.ty())
    }
}
