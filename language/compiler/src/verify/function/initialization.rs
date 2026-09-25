use tspp_core::{FxIndexMap, FxIndexSet};
use tspp_mir::{
    BlockId, FunctionKind, Instruction, Place, PlaceOrigin, Projection, Terminator, Value,
};

use crate::verify::VerifyError;

use super::checker::FunctionChecker;
use super::validate::{struct_field_count, uninitialized_value};

/// The field initialization state entering one block.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FieldState {
    /// Fields initialized on every path.
    must: Vec<bool>,
    /// Fields initialized on some path.
    may: Vec<bool>,
    /// Whether the base constructor ran on some path.
    may_delegate: bool,
}

impl FieldState {
    /// Create the state with every field uninitialized.
    fn new(count: usize) -> Self {
        Self {
            must: vec![false; count],
            may: vec![false; count],
            may_delegate: false,
        }
    }

    /// Join one predecessor exit into this entry.
    fn join(&mut self, other: &FieldState) -> bool {
        // intersect the must set, since a field must initialize on every path
        let mut is_changed = false;
        for (must, joined) in self.must.iter_mut().zip(&other.must) {
            if *must && !joined {
                *must = false;
                is_changed = true;
            }
        }

        // union the may set, since a field may initialize on any path
        for (may, joined) in self.may.iter_mut().zip(&other.may) {
            if !*may && *joined {
                *may = true;
                is_changed = true;
            }
        }
        if !self.may_delegate && other.may_delegate {
            self.may_delegate = true;
            is_changed = true;
        }

        is_changed
    }

    /// Mark the leading fields the base constructor initializes.
    fn fill(&mut self, count: usize) {
        self.must
            .iter_mut()
            .take(count)
            .for_each(|field| *field = true);
        self.may
            .iter_mut()
            .take(count)
            .for_each(|field| *field = true);
        self.may_delegate = true;
    }

    /// Return whether every field is initialized on every path.
    fn is_complete(&self) -> bool {
        self.must.iter().all(|field| *field)
    }
}

impl FunctionChecker<'_, '_> {
    /// Check definite initialization of a constructor's receiver.
    pub(super) fn check_initialization(&mut self) {
        // constructors receive exclusive uninitialized field storage
        let Some((receiver, field_count)) = self.uninitialized_receiver() else {
            return;
        };

        // collect aliases to the receiver storage and initialized object
        let (aliases, handles) = self.collect_receiver_aliases(receiver);

        // require every field initialized once on every path to return
        self.check_field_flow(receiver, field_count, &aliases, &handles);
    }

    /// Return the receiver value and field count of one constructor.
    fn uninitialized_receiver(&self) -> Option<(Value, usize)> {
        if self.function.kind != FunctionKind::Constructor {
            return None;
        }
        let receiver = self.function.parameters.first()?;
        let value = uninitialized_value(self.tree, receiver.ty)?;
        let count = struct_field_count(self.tree, value)?;

        Some((receiver.value, count))
    }

    /// Collect aliases to the receiver's uninitialized storage and initialized object.
    fn collect_receiver_aliases(&self, receiver: Value) -> (FxIndexSet<Value>, FxIndexSet<Value>) {
        let mut aliases = FxIndexSet::default();
        let mut handles = FxIndexSet::default();
        for (index, ty) in self.function.value_types().iter().enumerate() {
            if ty.is_none() {
                continue;
            }
            let value = Value::new(index as u32);
            let place = self.places.get(value);
            if place.origin != PlaceOrigin::Value(receiver) {
                continue;
            }
            match place.path.projections.as_slice() {
                // alias the uninitialized storage, a read as the initialized object a handle
                [Projection::Deref] if self.points_to_uninitialized(value) => {
                    aliases.insert(value);
                }
                [Projection::Deref] => {
                    handles.insert(value);
                }
                _ => {}
            }
        }

        (aliases, handles)
    }

    /// Return the receiver field selected by a canonical place.
    fn receiver_field(&self, receiver: Value, place: &Place) -> Option<u32> {
        match (place.origin, place.path.projections.as_slice()) {
            (PlaceOrigin::Value(value), [Projection::Deref, Projection::Field { index }])
                if value == receiver =>
            {
                Some(*index)
            }
            _ => None,
        }
    }

    /// Run the field initialization dataflow, reporting violations.
    fn check_field_flow(
        &mut self,
        receiver: Value,
        field_count: usize,
        aliases: &FxIndexSet<Value>,
        handles: &FxIndexSet<Value>,
    ) {
        // seed every block entry, starting the entry block uninitialized
        let mut entries: FxIndexMap<BlockId, FieldState> = FxIndexMap::default();
        let Some(first) = self.function.blocks().first().copied() else {
            return;
        };
        entries.insert(first, FieldState::new(field_count));

        // propagate to a fixpoint before reporting
        let mut pending = vec![first];
        while let Some(block) = pending.pop() {
            let Some(mut state) = entries.get(&block).cloned() else {
                continue;
            };
            self.transfer_block(block, receiver, &mut state, aliases, handles, false);

            // join this exit into every successor entry, the entry block keeping its seed
            for successor in self
                .tree
                .get(self.tree.get(block).terminator)
                .successors(self.tree)
            {
                if successor == first {
                    continue;
                }
                match entries.get_mut(&successor) {
                    Some(entry) => {
                        if entry.join(&state) {
                            pending.push(successor);
                        }
                    }
                    None => {
                        entries.insert(successor, state.clone());
                        pending.push(successor);
                    }
                }
            }
        }

        // report violations over the settled states
        for &block in self.function.blocks() {
            let Some(mut state) = entries.get(&block).cloned() else {
                continue;
            };
            self.transfer_block(block, receiver, &mut state, aliases, handles, true);
        }
    }

    /// Transfer one block's instructions, reporting when requested.
    fn transfer_block(
        &mut self,
        block: BlockId,
        receiver: Value,
        state: &mut FieldState,
        aliases: &FxIndexSet<Value>,
        handles: &FxIndexSet<Value>,
        is_reporting: bool,
    ) {
        let block = self.tree.get(block);
        for &instruction_id in &block.instructions {
            let instruction = self.tree.get(instruction_id);

            // let a handle escape once every field initializes
            let escapes = !matches!(instruction, Instruction::Address { .. })
                && instruction
                    .reads(self.tree)
                    .iter()
                    .any(|value| handles.contains(value));
            if escapes && is_reporting && !state.is_complete() {
                self.verification
                    .emit_error(VerifyError::ReceiverBeforeInitialization {
                        anchor: self.anchor(instruction_id.into_any()),
                    });
            }
            match instruction {
                // initialize the field one store names, rejecting a repeat
                Instruction::Store { place, .. } => {
                    let place = self.places.resolve_place(place);
                    let Some(field) = self.receiver_field(receiver, &place) else {
                        continue;
                    };
                    if is_reporting && state.may[field as usize] {
                        self.verification
                            .emit_error(VerifyError::FieldInitializedTwice {
                                anchor: self.anchor(instruction_id.into_any()),
                                field: field.to_string(),
                            });
                    }

                    state.must[field as usize] = true;
                    state.may[field as usize] = true;
                }
                // initialize the uninitialized storage each call argument fills in place
                Instruction::Call { call, .. } => {
                    let Some((_, parameters, _)) = self
                        .tree
                        .type_definition(call.signature)
                        .function_signature_parts()
                    else {
                        unreachable!("a validated call has a function signature");
                    };
                    let arguments = self.tree.get_values(call.arguments);
                    for (parameter, &argument) in parameters.iter().zip(arguments) {
                        if uninitialized_value(self.tree, parameter.ty).is_none() {
                            continue;
                        }

                        // complete the base constructor's leading fields once
                        if argument != receiver && aliases.contains(&argument) {
                            let argument_type = self.function.expect_value_type(argument);
                            let Some(count) = uninitialized_value(self.tree, argument_type)
                                .and_then(|value| struct_field_count(self.tree, value))
                                .filter(|count| *count <= state.must.len())
                            else {
                                unreachable!("a validated view of the receiver holds its fields");
                            };
                            if is_reporting && state.may_delegate {
                                self.verification.emit_error(VerifyError::SuperCalledTwice {
                                    anchor: self.anchor(instruction_id.into_any()),
                                });
                            }
                            state.fill(count);
                        }
                        // complete one field constructed in place
                        else if let Some(field) =
                            self.receiver_field(receiver, self.places.get(argument))
                        {
                            if is_reporting && state.may[field as usize] {
                                self.verification
                                    .emit_error(VerifyError::FieldInitializedTwice {
                                        anchor: self.anchor(instruction_id.into_any()),
                                        field: field.to_string(),
                                    });
                            }
                            state.must[field as usize] = true;
                            state.may[field as usize] = true;
                        }
                    }
                }
                // leave every other instruction alone
                _ => {}
            }
        }

        // require every field initialized at return
        if is_reporting && let Terminator::Return { .. } = self.tree.get(block.terminator) {
            for (field, initialized) in state.must.iter().enumerate() {
                if !initialized {
                    self.verification
                        .emit_error(VerifyError::FieldLeftUninitialized {
                            anchor: self.anchor(block.terminator.into_any()),
                            field: field.to_string(),
                        });
                }
            }
        }
    }
}
