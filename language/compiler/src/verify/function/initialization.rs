use destack_core::{FxIndexMap, FxIndexSet};
use destack_mir::{BlockId, Instruction, PlaceOrigin, Projection, Terminator, Type, Value};

use crate::verify::VerifyError;

use super::checker::FunctionChecker;

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

        // collect the receiver's aliases, its handles, and its projected field addresses
        let (aliases, handles, addresses) = self.collect_receiver_addresses(receiver, field_count);

        // require every field initialized once on every path to return
        self.check_field_flow(receiver, field_count, &aliases, &handles, &addresses);
    }

    /// Return the number of leading fields one delegated receiver's base constructor initializes.
    fn delegated_field_count(&self, receiver: Value) -> usize {
        let ty = self.function.expect_value_type(receiver);
        let Type::Reference { pointee, .. } = self.tree.get(ty) else {
            return 0;
        };
        let Type::Uninit { value } = self.tree.get(*pointee) else {
            return 0;
        };
        match self.tree.get(self.tree.represented(*value)) {
            Type::Struct { fields, .. } => fields.len(),
            _ => 0,
        }
    }

    /// Return the receiver value and field count of one constructor.
    fn uninitialized_receiver(&self) -> Option<(Value, usize)> {
        // read the struct behind the first parameter's uninitialized pointee
        let receiver = self.function.parameters.first()?;
        let Type::Reference { pointee, .. } = self.tree.get(receiver.ty) else {
            return None;
        };
        let Type::Uninit { value } = self.tree.get(*pointee) else {
            return None;
        };
        let Type::Struct { fields, .. } = self.tree.get(self.tree.represented(*value)) else {
            return None;
        };

        Some((receiver.value, fields.len()))
    }

    /// Collect the values naming the receiver's uninitialized storage, the values reading the
    /// initialized object out of it, and the values addressing its fields.
    fn collect_receiver_addresses(
        &self,
        receiver: Value,
        field_count: usize,
    ) -> (FxIndexSet<Value>, FxIndexSet<Value>, FxIndexMap<Value, u32>) {
        let mut aliases = FxIndexSet::default();
        let mut handles = FxIndexSet::default();
        let mut addresses = FxIndexMap::default();
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
                [] if self.points_to_uninitialized(value) => {
                    aliases.insert(value);
                }
                [] => {
                    handles.insert(value);
                }
                [Projection::Field { index }] if (*index as usize) < field_count => {
                    addresses.insert(value, *index);
                }
                _ => {}
            }
        }

        (aliases, handles, addresses)
    }

    /// Run the field initialization dataflow, reporting violations.
    fn check_field_flow(
        &mut self,
        receiver: Value,
        field_count: usize,
        aliases: &FxIndexSet<Value>,
        handles: &FxIndexSet<Value>,
        addresses: &FxIndexMap<Value, u32>,
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
            self.transfer_block(
                block, receiver, &mut state, aliases, handles, addresses, false,
            );

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
            self.transfer_block(
                block, receiver, &mut state, aliases, handles, addresses, true,
            );
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
        addresses: &FxIndexMap<Value, u32>,
        is_reporting: bool,
    ) {
        let block = self.tree.get(block);
        for &instruction_id in &block.instructions {
            let instruction = self.tree.get(instruction_id);

            // let a handle escape once every field initializes
            let escapes = !matches!(instruction, Instruction::FieldAddr { .. })
                && instruction
                    .reads(self.tree)
                    .iter()
                    .any(|value| handles.contains(value));
            if escapes && is_reporting && !state.is_complete() {
                self.verification
                    .emit_error(VerifyError::ReceiverBeforeInitialization {
                        anchor: self.verification.anchor(instruction_id.into_any()),
                    });
            }
            match instruction {
                // initialize the field one store names, rejecting a repeat
                Instruction::Store { pointer, .. } => {
                    let Some(field) = addresses.get(pointer).copied() else {
                        continue;
                    };
                    if is_reporting && state.may[field as usize] {
                        self.verification
                            .emit_error(VerifyError::FieldInitializedTwice {
                                anchor: self.verification.anchor(instruction_id.into_any()),
                                field: field.to_string(),
                            });
                    }

                    state.must[field as usize] = true;
                    state.may[field as usize] = true;
                }
                // initialize the fields one call receives
                Instruction::Call { call, .. } => {
                    for argument in self.tree.get_values(call.arguments) {
                        // a narrowed receiver delegates to the base constructor, initializing the
                        // base's leading fields once
                        if *argument != receiver
                            && aliases.contains(argument)
                            && let count = self.delegated_field_count(*argument)
                            && count <= state.must.len()
                        {
                            if is_reporting && state.may_delegate {
                                self.verification.emit_error(VerifyError::SuperCalledTwice {
                                    anchor: self.verification.anchor(instruction_id.into_any()),
                                });
                            }
                            state.fill(count);
                        }
                        // a passed field address initializes its field
                        if let Some(field) = addresses.get(argument).copied() {
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
                            anchor: self.verification.anchor(block.terminator.into_any()),
                            field: field.to_string(),
                        });
                }
            }
        }
    }
}
