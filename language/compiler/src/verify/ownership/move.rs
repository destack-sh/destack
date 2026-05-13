use super::flow::FlowState;
use destack_mir as mir;

/// Move state for one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MoveState {
    /// Definitely moved on all paths.
    Moved,
    /// Moved on at least one predecessor path.
    MaybeMoved,
}

/// One moved place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Move {
    /// The moved place.
    pub(super) place: mir::Place,
    /// The move state.
    pub(super) state: MoveState,
    /// MIR node for diagnostics.
    pub(super) at: mir::LocalNodeIdAny,
}

/// Moved places at one program point.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct MoveSet {
    /// The moved places.
    pub(super) moves: Vec<Move>,
}

/// Result of checking a place use against moved places.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MoveUse {
    /// Move state to report.
    pub(super) state: MoveState,
    /// MIR node for diagnostics.
    pub(super) at: mir::LocalNodeIdAny,
}

impl MoveSet {
    /// Check one place use.
    pub(super) fn check_use(
        &self,
        place: &mir::Place,
        mut may_overlap: impl FnMut(&mir::Place, &mir::Place) -> bool,
    ) -> Option<MoveUse> {
        self.moves.iter().find_map(|moved| {
            // reject exact child use after parent move
            if moved.place.contains(place) {
                return Some(MoveUse {
                    state: moved.state,
                    at: moved.at,
                });
            }

            // report overlapping sibling use as path sensitive
            if may_overlap(&moved.place, place) {
                return Some(MoveUse {
                    state: MoveState::MaybeMoved,
                    at: moved.at,
                });
            }

            None
        })
    }

    /// Mark one place as moved.
    pub(super) fn move_place(&mut self, place: mir::Place, at: mir::LocalNodeIdAny) {
        // skip places already covered by parent moves
        if self.moves.iter().any(|moved| moved.place.contains(&place)) {
            return;
        }

        // remove tracked children superseded by this move
        self.moves.retain(|moved| !place.contains(&moved.place));

        self.moves.push(Move {
            place,
            state: MoveState::Moved,
            at,
        });
    }

    /// Mark one place as initialized.
    pub(super) fn assign_place(&mut self, place: &mir::Place) {
        self.moves.retain(|moved| !place.contains(&moved.place));
    }

    /// Bind successor parameter moves.
    pub(super) fn bind(&mut self, argument: mir::Value, parameter: mir::Value) {
        for moved in &mut self.moves {
            moved.place.replace_value(argument, parameter);
        }
    }

    /// Merge predecessor move sets.
    pub(super) fn merge_predecessors(predecessors: &[FlowState]) -> Self {
        let mut places: Vec<(mir::Place, mir::LocalNodeIdAny)> = Vec::new();

        // collect places moved by at least one predecessor
        for predecessor in predecessors {
            for moved in &predecessor.moves.moves {
                if !places.iter().any(|(place, _)| place == &moved.place) {
                    places.push((moved.place.clone(), moved.at));
                }
            }
        }

        let mut moves = Vec::new();
        for (place, at) in places {
            let mut state = MoveState::Moved;

            // downgrade when any predecessor keeps the place initialized
            for predecessor in predecessors {
                let moved = predecessor
                    .moves
                    .moves
                    .iter()
                    .find(|moved| moved.place == place);

                match moved {
                    Some(moved) => {
                        if moved.state == MoveState::MaybeMoved {
                            state = MoveState::MaybeMoved;
                        }
                    }
                    None => state = MoveState::MaybeMoved,
                }
            }

            moves.push(Move { place, state, at });
        }

        Self { moves }
    }
}
