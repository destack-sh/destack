use destack_core::{FxIndexMap, FxIndexSet};
use destack_mir::{BlockId, Function, Instruction, Terminator, Tree, Type, Value};

use crate::verify::{VerifyError, VerifyState};

/// Definite initialization checker for one constructor receiver.
pub(in crate::verify) struct InitializationChecker<'a, 'b> {
    /// The function being checked.
    function: &'a Function,
    /// The MIR tree.
    tree: &'a Tree,
    /// Module verification state.
    verification: &'a mut VerifyState<'b>,
}

/// The field initialization state entering one block.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FieldState {
    /// Fields initialized on every path.
    must: Vec<bool>,
    /// Fields initialized on some path.
    may: Vec<bool>,
}

impl FieldState {
    /// Create the state with every field uninitialized.
    fn new(count: usize) -> Self {
        Self {
            must: vec![false; count],
            may: vec![false; count],
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

        is_changed
    }

    /// Mark every field initialized.
    fn fill(&mut self) {
        self.must.iter_mut().for_each(|field| *field = true);
        self.may.iter_mut().for_each(|field| *field = true);
    }
}

impl<'a, 'b> InitializationChecker<'a, 'b> {
    /// Create one constructor initialization checker.
    pub(in crate::verify) fn new(
        function: &'a Function,
        tree: &'a Tree,
        verification: &'a mut VerifyState<'b>,
    ) -> Self {
        Self {
            function,
            tree,
            verification,
        }
    }

    /// Check definite initialization of the constructor receiver.
    pub(in crate::verify) fn check(mut self) {
        // constructors receive exclusive uninitialized field storage
        let Some((receiver, field_count)) = self.uninitialized_receiver() else {
            return;
        };

        // collect the receiver's aliases and projected field addresses
        let (aliases, addresses) = self.collect_receiver_addresses(receiver, field_count);

        // require every field initialized once on every path to return
        self.check_field_flow(field_count, &aliases, &addresses);
    }

    /// Return the receiver value and field count of one constructor.
    fn uninitialized_receiver(&self) -> Option<(Value, usize)> {
        // read the struct behind the first parameter's uninitialized pointee
        let receiver = self.function.parameters.first()?;
        let Type::Reference { pointee, .. } = self.tree.ty(receiver.ty) else {
            return None;
        };
        let Type::Uninit { value } = self.tree.ty(*pointee) else {
            return None;
        };
        let Type::Struct { fields, .. } = self.tree.ty(*value) else {
            return None;
        };

        Some((receiver.value, fields.len()))
    }

    /// Collect the receiver's alias values and projected field addresses.
    fn collect_receiver_addresses(
        &self,
        receiver: Value,
        field_count: usize,
    ) -> (FxIndexSet<Value>, FxIndexMap<Value, u32>) {
        let mut aliases = FxIndexSet::default();
        let mut addresses = FxIndexMap::default();
        aliases.insert(receiver);

        // walk every instruction for casts and field addresses off the receiver
        for &block in self.function.blocks() {
            for &instruction in &self.tree.get(block).instructions {
                match self.tree.get(instruction) {
                    // carry the receiver through its casts
                    Instruction::Cast {
                        destination,
                        argument,
                        ..
                    } if aliases.contains(argument) => {
                        aliases.insert(*destination);
                    }
                    // record the field each projected address names
                    Instruction::FieldAddr {
                        destination,
                        aggregate,
                        field,
                        ..
                    } if aliases.contains(aggregate) && (*field as usize) < field_count => {
                        addresses.insert(*destination, *field);
                    }
                    // leave every other instruction alone
                    _ => {}
                }
            }
        }

        (aliases, addresses)
    }

    /// Run the field initialization dataflow, reporting violations.
    fn check_field_flow(
        &mut self,
        field_count: usize,
        aliases: &FxIndexSet<Value>,
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
            self.transfer_block(block, &mut state, aliases, addresses, false);

            // join this exit into every successor entry, requeueing what moved
            for successor in self
                .tree
                .get(self.tree.get(block).terminator)
                .successors(self.tree)
            {
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
            self.transfer_block(block, &mut state, aliases, addresses, true);
        }
    }

    /// Transfer one block's instructions, reporting when requested.
    fn transfer_block(
        &mut self,
        block: BlockId,
        state: &mut FieldState,
        aliases: &FxIndexSet<Value>,
        addresses: &FxIndexMap<Value, u32>,
        is_reporting: bool,
    ) {
        let block = self.tree.get(block);
        for &instruction_id in &block.instructions {
            match self.tree.get(instruction_id) {
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
                        // a delegated receiver initializes every field
                        if aliases.contains(argument) {
                            state.fill();
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
