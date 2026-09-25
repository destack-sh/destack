use tspp_mir::{
    Access, Block, Initialization, InitializationState, Instruction, LocalNodeId, LocalNodeIdAny,
    Place, PlaceOrigin, PlaceType, Projection, Reference, Terminator, Unavailability,
};

use crate::verify::VerifyError;

use super::checker::FunctionChecker;

/// How a move out of one place is checked, by the storage the place addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Movability {
    /// Local storage or its unique pointees, whose moved paths initialization tracks.
    Owned {
        /// Whether a destructor of a containing value keeps the place in use.
        has_drop: bool,
    },
    /// Storage other roots may address: a borrow, a managed object, or a global.
    Aliased,
    /// Storage behind a raw pointer, moved at the author's word.
    Unchecked,
}

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
            self.check_move_terminator(block_id, block.terminator, terminator, &state);
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

        // check the ownership along a moved projection or load
        if let Some(place) = self.moved_place(instruction) {
            self.check_move_place(&place, anchor);
        }
    }

    /// Return the place one instruction moves out of by projection or load.
    pub(super) fn moved_place(&self, instruction: &Instruction) -> Option<Place> {
        match instruction {
            // move a field, element, or payload out of an aggregate
            Instruction::FieldGet {
                destination,
                aggregate,
                field,
            } if self.is_moved_on_use(*destination) => {
                Some(Place::value(*aggregate).with_projection(Projection::Field { index: *field }))
            }
            Instruction::ElementGet {
                destination,
                aggregate,
                index,
            } if self.is_moved_on_use(*destination) => Some(
                Place::value(*aggregate).with_projection(Projection::Element { index: *index }),
            ),
            Instruction::VariantPayload {
                destination,
                variant,
                case,
                ..
            } if self.is_moved_on_use(*destination) => {
                Some(Place::value(*variant).with_projection(Projection::Variant { case: *case }))
            }
            // move a value out of its storage
            Instruction::Load {
                destination, place, ..
            } if self.is_moved_on_use(*destination) => Some(place.clone()),
            // leave copies and every other instruction alone
            _ => None,
        }
    }

    /// Check ownership and destructor restrictions along one moved place.
    fn check_move_place(&mut self, place: &Place, anchor: LocalNodeIdAny) {
        let error = match self.movability(place) {
            Movability::Aliased => Some(VerifyError::MoveOutOfReference {
                anchor: self.anchor(anchor),
            }),
            Movability::Owned { has_drop: true } => Some(VerifyError::MoveOutOfDrop {
                anchor: self.anchor(anchor),
            }),
            Movability::Owned { has_drop: false } | Movability::Unchecked => None,
        };
        if let Some(error) = error {
            self.verification.emit_error(error);
        }
    }

    /// Return how a move out of one place is checked.
    pub(super) fn movability(&self, place: &Place) -> Movability {
        let mut movability = match place.origin {
            PlaceOrigin::Global(_) => Movability::Aliased,
            PlaceOrigin::Local(_) | PlaceOrigin::Value(_) => Movability::Owned { has_drop: false },
        };

        // track a move through an exclusive borrow as a move of the frame storage it resolves to
        let is_tracked = self
            .moves
            .place(&self.places.resolve_place(place))
            .is_some();

        // preserve ownership through unique references and honor explicit raw access
        for (length, ty) in place.prefix_types(self.function_id, self.tree) {
            if place.path.projections[length] == Projection::Deref {
                let PlaceType::Value(reference) = ty else {
                    unreachable!("a place dereferences a non-value");
                };
                let reference = self.tree.type_definition(self.tree.storage_type(reference));
                match reference.dereference_kind() {
                    Reference::Unique => {}
                    Reference::Borrowed
                        if is_tracked
                            && length == 0
                            && reference.reference_access() == Some(Access::Exclusive) => {}
                    Reference::Borrowed | Reference::Managed(_) => {
                        movability = Movability::Aliased;
                    }
                    Reference::Raw => movability = Movability::Unchecked,
                }
            }
            // keep every field available to its containing value's destructor
            else if let Movability::Owned { has_drop } = &mut movability
                && let PlaceType::Value(ty) = ty
            {
                *has_drop |= self.verification.drops.has_hook(ty);
            }
        }

        movability
    }

    /// Check initialized storage read by one terminator.
    fn check_move_terminator(
        &mut self,
        block_id: LocalNodeId<Block>,
        terminator_id: LocalNodeId<Terminator>,
        terminator: &Terminator,
        state: &InitializationState,
    ) {
        let anchor = terminator_id.into_any();

        // report unavailable move paths
        for unavailable in self
            .initialization
            .terminator_unavailability(block_id, terminator, state, self.tree)
        {
            self.emit_unavailability(unavailable, anchor);
        }
    }

    /// Emit one unavailable move-path error.
    fn emit_unavailability(&mut self, unavailable: Unavailability, anchor: LocalNodeIdAny) {
        match (unavailable.initialization, unavailable.moved_at) {
            (Initialization::Uninitialized, Some(moved_at)) => {
                let moved_at = self.anchor(moved_at);
                self.verification.emit_error(
                    VerifyError::UseAfterMove {
                        anchor: self.anchor(anchor),
                        moved_at: moved_at.clone(),
                    }
                    .label(moved_at, "value moved here"),
                );
            }
            (Initialization::MaybeInitialized, Some(moved_at)) => {
                let moved_at = self.anchor(moved_at);
                self.verification.emit_error(
                    VerifyError::MaybeUseAfterMove {
                        anchor: self.anchor(anchor),
                        moved_at: moved_at.clone(),
                    }
                    .label(moved_at, "value moved on this path"),
                );
            }
            (Initialization::Uninitialized, None) => {
                self.verification
                    .emit_error(VerifyError::UseOfUninitializedPlace {
                        anchor: self.anchor(anchor),
                    });
            }
            (Initialization::MaybeInitialized, None) => {
                self.verification
                    .emit_error(VerifyError::MaybeUseOfUninitializedPlace {
                        anchor: self.anchor(anchor),
                    });
            }
            (Initialization::Initialized, _) => {
                unreachable!("initialized path produced an unavailable use")
            }
        }
    }
}
