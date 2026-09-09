use destack_mir::{
    Initialization, InitializationState, Instruction, LocalNodeId, LocalNodeIdAny, Terminator,
    Unavailability,
};

use crate::verify::VerifyError;

use super::checker::FunctionChecker;

impl FunctionChecker<'_, '_> {
    /// Check move legality across the function.
    pub(super) fn check_moves(&mut self) {
        for &block_id in self.function.blocks() {
            let Some(mut state) = self.initialization.entry(block_id).cloned() else {
                continue;
            };
            let block = self.tree.get(block_id);

            // check and transfer instructions in execution order
            for &instruction_id in &block.instructions {
                let instruction = self.tree.get(instruction_id);
                self.check_move_instruction(instruction_id, instruction, &state);
                self.initialization
                    .transfer_instruction(instruction_id, &mut state, self.tree);
            }

            // check the terminating operation
            let terminator = self.tree.get(block.terminator);
            self.check_move_terminator(block.terminator, terminator, &state);
        }
    }

    /// Check initialized storage and move rules for one instruction.
    fn check_move_instruction(
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
            } if !self.is_pointer(*pointer)
                && !self.points_to_uninitialized(*pointer)
                && self.is_move_only(*destination)
                && self.moves.pointee(*pointer).is_none() =>
            {
                self.verification
                    .emit_error(VerifyError::MoveOutOfReference {
                        anchor: self.verification.anchor(anchor),
                    });
            }
            _ => {}
        }
    }

    /// Check initialized storage read by one terminator.
    fn check_move_terminator(
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
}
