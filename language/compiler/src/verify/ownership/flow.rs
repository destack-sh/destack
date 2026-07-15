use destack_mir as mir;

use super::borrow::BorrowMap;
use super::loan::LoanSet;
use super::r#move::MoveSet;

/// Ownership state at one control-flow point.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct FlowState {
    /// Moved places.
    pub(super) moves: MoveSet,
    /// Active loans.
    pub(super) loans: LoanSet,
    /// Borrow sources.
    pub(super) borrows: BorrowMap,
    /// Known places keyed by SSA value.
    places: Vec<Option<mir::Place>>,
    /// Borrow references already reported as invalid.
    pub(super) invalid_borrows: Vec<mir::Value>,
}

impl FlowState {
    /// Create an empty flow state.
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Merge predecessor flow states.
    pub(super) fn merge_predecessors(predecessors: &[FlowState]) -> Self {
        if predecessors.is_empty() {
            return Self::new();
        }

        let mut borrows = BorrowMap::default();
        let mut loans = LoanSet::default();
        let places = Self::merge_places(predecessors);
        let mut invalid_borrows = Vec::new();

        // merge borrow sources conservatively across incoming edges
        for predecessor in predecessors {
            for binding in &predecessor.borrows.bindings {
                borrows.merge_sources_at(binding.value, &binding.path, &binding.sources);
            }
        }

        // keep active loans carried by any incoming edge
        for predecessor in predecessors {
            for loan in &predecessor.loans.loans {
                if !loans.loans.contains(loan) {
                    loans.loans.push(loan.clone());
                }
            }
        }

        // keep invalid references carried by any incoming edge
        for predecessor in predecessors {
            for value in &predecessor.invalid_borrows {
                if !invalid_borrows.contains(value) {
                    invalid_borrows.push(*value);
                }
            }
        }

        Self {
            moves: MoveSet::merge_predecessors(predecessors),
            loans,
            borrows,
            places,
            invalid_borrows,
        }
    }

    /// Bind successor parameters to predecessor arguments.
    pub(super) fn bind(&mut self, argument: mir::Value, parameter: mir::Value) {
        self.moves.bind(argument, parameter);
        self.loans.bind(argument, parameter);
        self.borrows.bind(argument, parameter);
        self.bind_place(argument, parameter);

        for value in &mut self.invalid_borrows {
            if *value == argument {
                *value = parameter;
            }
        }
    }

    /// Return whether one borrow reference is invalid.
    pub(super) fn is_invalid_borrow(&self, value: mir::Value) -> bool {
        self.invalid_borrows.contains(&value)
    }

    /// Mark one borrow reference invalid.
    pub(super) fn invalidate_borrow(&mut self, value: mir::Value) {
        if !self.invalid_borrows.contains(&value) {
            self.invalid_borrows.push(value);
        }
    }

    /// Return the known place for one value.
    pub(super) fn value_place(&self, value: mir::Value) -> Option<&mir::Place> {
        self.places
            .get(value.id() as usize)
            .and_then(Option::as_ref)
    }

    /// Return the best known place for one value.
    pub(super) fn place_for_value(&self, value: mir::Value) -> mir::Place {
        self.value_place(value)
            .cloned()
            .unwrap_or_else(|| mir::Place::value(value))
    }

    /// Apply one instruction's place bindings.
    pub(super) fn apply_instruction(&mut self, instruction: &mir::Instruction) {
        match instruction {
            mir::Instruction::LocalAddr {
                destination, local, ..
            } => self.set_place(*destination, mir::Place::local(*local)),
            mir::Instruction::GlobalAddr {
                destination,
                global,
                ..
            } => self.set_place(*destination, mir::Place::global(*global)),
            mir::Instruction::NewZeroed { destination, .. }
            | mir::Instruction::NewUninit { destination, .. }
            | mir::Instruction::NewComplete { destination, .. }
            | mir::Instruction::NewSliceZeroed { destination, .. }
            | mir::Instruction::NewSliceUninit { destination, .. }
            | mir::Instruction::FrameAllocZeroed { destination, .. }
            | mir::Instruction::FrameAllocUninit { destination, .. }
            | mir::Instruction::FunctionPointer { destination, .. }
            | mir::Instruction::FunctionEnvironment { destination, .. }
            | mir::Instruction::FunctionEnvironmentCurrent { destination } => {
                self.set_place(*destination, mir::Place::value(*destination))
            }
            mir::Instruction::FieldAddr {
                destination,
                aggregate,
                field,
                ..
            } => self.set_projected_place(
                *destination,
                *aggregate,
                mir::Projection::Field { index: *field },
            ),
            mir::Instruction::ElementAddr {
                destination,
                base,
                index,
                ..
            } => self.set_projected_place(
                *destination,
                *base,
                mir::Projection::Index { index: *index },
            ),
            mir::Instruction::SliceView {
                destination,
                source,
                start,
                length,
                ..
            } => self.set_projected_place(
                *destination,
                *source,
                mir::Projection::Slice {
                    start: *start,
                    length: *length,
                },
            ),
            mir::Instruction::Cast {
                destination,
                argument,
                ..
            }
            | mir::Instruction::TensorCast {
                destination,
                tensor: argument,
            }
            | mir::Instruction::TensorView {
                destination,
                view: argument,
                ..
            }
            | mir::Instruction::Pin {
                destination,
                value: argument,
                ..
            } => self.copy_place(*destination, *argument),
            _ => {}
        }
    }

    /// Merge places known on every predecessor.
    fn merge_places(predecessors: &[FlowState]) -> Vec<Option<mir::Place>> {
        let Some(first) = predecessors.first() else {
            return Vec::new();
        };
        let value_count = predecessors
            .iter()
            .fold(first.places.len(), |count, predecessor| {
                count.max(predecessor.places.len())
            });
        let mut places = Vec::with_capacity(value_count);

        // keep only places all predecessors agree on
        for index in 0..value_count {
            let Some(place) = first.place_at(index).cloned() else {
                places.push(None);
                continue;
            };
            let is_stable = predecessors[1..]
                .iter()
                .all(|predecessor| predecessor.place_at(index) == Some(&place));

            places.push(is_stable.then_some(place));
        }

        places
    }

    /// Return the place at one dense value index.
    fn place_at(&self, index: usize) -> Option<&mir::Place> {
        self.places.get(index).and_then(Option::as_ref)
    }

    /// Bind a place to a successor parameter.
    fn bind_place(&mut self, argument: mir::Value, parameter: mir::Value) {
        let Some(place) = self.value_place(argument).cloned() else {
            self.clear_place(parameter);
            return;
        };
        if place == mir::Place::value(argument) {
            self.clear_place(parameter);
            return;
        }

        self.set_place(parameter, place);
    }

    /// Clear a value's known place.
    fn clear_place(&mut self, value: mir::Value) {
        let index = self.resize_places(value);

        self.places[index] = None;
    }

    /// Set a value's known place.
    fn set_place(&mut self, value: mir::Value, place: mir::Place) {
        let index = self.resize_places(value);

        self.places[index] = Some(place);
    }

    /// Set a value's projected place.
    fn set_projected_place(
        &mut self,
        value: mir::Value,
        base: mir::Value,
        projection: mir::Projection,
    ) {
        let place = self.place_for_value(base).with_projection(projection);

        self.set_place(value, place);
    }

    /// Copy a value's known place.
    fn copy_place(&mut self, value: mir::Value, source: mir::Value) {
        let Some(place) = self.value_place(source).cloned() else {
            self.clear_place(value);
            return;
        };

        self.set_place(value, place);
    }

    /// Resize place storage for one value.
    fn resize_places(&mut self, value: mir::Value) -> usize {
        let index = value.id() as usize;
        let value_count = index + 1;

        if self.places.len() < value_count {
            self.places.resize(value_count, None);
        }

        index
    }
}
