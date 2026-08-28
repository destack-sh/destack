use std::sync::Arc;

use destack_mir::{
    Function, FunctionCache, Initialization, InitializationState, InitializationTable, Instruction,
    LocalNodeId, LocalNodeIdAny, Terminator, Tree, Type, Unavailability, Value,
};

use crate::verify::{VerifyError, VerifyState};

/// Move checker for one MIR function.
pub(in crate::verify) struct MoveChecker<'a, 'b> {
    /// The function being checked.
    function: &'a Function,
    /// The MIR tree.
    tree: &'a Tree,
    /// Module verification state.
    verification: &'a mut VerifyState<'b>,
    /// Move-path initialization.
    initialization: Arc<InitializationTable>,
}

impl<'a, 'b> MoveChecker<'a, 'b> {
    /// Create one function move checker.
    pub(in crate::verify) fn new(
        function: &'a Function,
        tree: &'a Tree,
        verification: &'a mut VerifyState<'b>,
        analyses: &mut FunctionCache,
    ) -> Self {
        let initialization = analyses.initialization(function, tree);

        Self {
            function,
            tree,
            verification,
            initialization,
        }
    }

    /// Check move legality across the function.
    pub(in crate::verify) fn check(mut self) {
        for &block_id in self.function.blocks() {
            let Some(mut state) = self.initialization.entry(block_id).cloned() else {
                continue;
            };
            let block = self.tree.get(block_id);

            // check and transfer instructions in execution order
            for &instruction_id in &block.instructions {
                let instruction = self.tree.get(instruction_id);
                self.check_instruction(instruction_id, instruction, &state);
                self.initialization
                    .transfer_instruction(instruction_id, &mut state, self.tree);
            }

            // check the terminating operation
            let terminator = self.tree.get(block.terminator);
            self.check_terminator(block.terminator, terminator, &state);
        }
    }

    /// Check initialized storage and move rules for one instruction.
    fn check_instruction(
        &mut self,
        instruction_id: LocalNodeId<Instruction>,
        instruction: &Instruction,
        state: &InitializationState,
    ) {
        let anchor = instruction_id.into_any();

        // report unavailable move paths
        for unavailable in
            self.initialization
                .instruction_unavailability(instruction, state, self.tree)
        {
            self.emit_unavailability(unavailable, anchor);
        }

        // enforce instruction-specific move rules
        match instruction {
            Instruction::Select { destination, .. } if self.is_move_only(*destination) => {
                self.verification
                    .emit_error(VerifyError::SelectOfMoveOnlyValue {
                        anchor: self.verification.anchor(anchor),
                    });
            }
            Instruction::FieldGet {
                destination,
                aggregate,
                ..
            }
            | Instruction::ElementGet {
                destination,
                aggregate,
                ..
            } if self.is_move_only(*destination) && self.has_drop_hook(*aggregate) => {
                self.verification.emit_error(VerifyError::MoveOutOfDrop {
                    anchor: self.verification.anchor(anchor),
                });
            }
            Instruction::VariantPayload {
                destination,
                variant,
                ..
            } if self.is_move_only(*destination) && self.has_drop_hook(*variant) => {
                self.verification.emit_error(VerifyError::MoveOutOfDrop {
                    anchor: self.verification.anchor(anchor),
                });
            }
            Instruction::Load {
                destination,
                pointer,
                ..
            } if !self.is_pointer(*pointer) && self.is_move_only(*destination) => {
                self.verification
                    .emit_error(VerifyError::MoveOutOfReference {
                        anchor: self.verification.anchor(anchor),
                    });
            }
            Instruction::Store { pointer, value }
                if !self.is_pointer(*pointer)
                    && self.is_move_only(*value)
                    && !self.points_to_uninitialized(*pointer) =>
            {
                self.verification
                    .emit_error(VerifyError::OverwriteOfMoveOnlyPlace {
                        anchor: self.verification.anchor(anchor),
                    });
            }
            _ => {}
        }
    }

    /// Check initialized storage read by one terminator.
    fn check_terminator(
        &mut self,
        terminator_id: LocalNodeId<Terminator>,
        terminator: &Terminator,
        state: &InitializationState,
    ) {
        let anchor = terminator_id.into_any();

        // report unavailable move paths
        for unavailable in self
            .initialization
            .terminator_unavailability(terminator, state, self.tree)
        {
            self.emit_unavailability(unavailable, anchor);
        }
    }

    /// Emit one unavailable move-path error.
    fn emit_unavailability(&mut self, unavailable: Unavailability, anchor: LocalNodeIdAny) {
        match (unavailable.initialization, unavailable.moved_at) {
            (Initialization::Uninitialized, Some(moved_at)) => {
                let moved_at = self.verification.anchor(moved_at);
                self.verification.emit_error(
                    VerifyError::UseAfterMove {
                        anchor: self.verification.anchor(anchor),
                        moved_at: moved_at.clone(),
                    }
                    .label(moved_at, "value moved here"),
                );
            }
            (Initialization::MaybeInitialized, Some(moved_at)) => {
                let moved_at = self.verification.anchor(moved_at);
                self.verification.emit_error(
                    VerifyError::MaybeUseAfterMove {
                        anchor: self.verification.anchor(anchor),
                        moved_at: moved_at.clone(),
                    }
                    .label(moved_at, "value moved on this path"),
                );
            }
            (Initialization::Uninitialized, None) => {
                self.verification
                    .emit_error(VerifyError::UseOfUninitializedPlace {
                        anchor: self.verification.anchor(anchor),
                    });
            }
            (Initialization::MaybeInitialized, None) => {
                self.verification
                    .emit_error(VerifyError::MaybeUseOfUninitializedPlace {
                        anchor: self.verification.anchor(anchor),
                    });
            }
            (Initialization::Initialized, _) => {
                unreachable!("initialized path produced an unavailable use")
            }
        }
    }

    /// Return whether one value is move-only.
    fn is_move_only(&self, value: Value) -> bool {
        let ty = self.function.expect_value_type(value);

        self.tree.ty(ty).copy(self.tree).is_no()
    }

    /// Return whether one value has a user drop hook.
    fn has_drop_hook(&self, value: Value) -> bool {
        let ty = self.function.expect_value_type(value);

        self.verification.drops.has_hook(ty)
    }

    /// Return whether one value has an unchecked pointer type.
    fn is_pointer(&self, value: Value) -> bool {
        let ty = self.function.expect_value_type(value);

        self.tree.ty(ty).is_pointer()
    }

    /// Return whether one reference addresses uninitialized storage.
    fn points_to_uninitialized(&self, value: Value) -> bool {
        let ty = self.function.expect_value_type(value);
        let Type::Reference { pointee, .. } = self.tree.ty(ty) else {
            return false;
        };

        matches!(self.tree.ty(*pointee), Type::Uninit { .. })
    }
}
