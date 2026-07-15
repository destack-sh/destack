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
    /// The aggregate decompositions in progress.
    decompositions: Vec<Decomposition>,
}

/// Result of checking a place use against moved places.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MoveUse {
    /// Move state to report.
    pub(super) state: MoveState,
    /// MIR node for diagnostics.
    pub(super) at: mir::LocalNodeIdAny,
}

/// One aggregate decomposition in progress.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Decomposition {
    /// The aggregate being decomposed.
    parent: mir::Place,
    /// The move-only children not yet extracted.
    remaining: u64,
    /// The first extraction for diagnostics.
    at: mir::LocalNodeIdAny,
}

impl MoveSet {
    /// Collapse incomplete aggregate moves after reporting them.
    pub(super) fn collapse_partial_moves(&mut self) -> Option<MoveUse> {
        let partial_move = self.decompositions.first().map(|decomposition| MoveUse {
            state: MoveState::Moved,
            at: decomposition.at,
        });
        let roots = self
            .decompositions
            .iter()
            .map(|decomposition| (decomposition.parent.clone(), decomposition.at))
            .collect::<Vec<_>>();
        self.decompositions.clear();

        // consume each invalid aggregate to keep later diagnostics focused
        for (root, at) in roots {
            self.move_place(root, at);
        }

        partial_move
    }

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
    pub(super) fn move_place(&mut self, place: mir::Place, at: mir::LocalNodeIdAny) -> bool {
        // skip places already covered by parent moves
        if self.moves.iter().any(|moved| moved.place.contains(&place)) {
            return false;
        }

        // remove tracked children superseded by this move
        self.moves.retain(|moved| !place.contains(&moved.place));

        self.moves.push(Move {
            place,
            state: MoveState::Moved,
            at,
        });

        true
    }

    /// Begin one aggregate decomposition after its first child move.
    pub(super) fn begin_decomposition(
        &mut self,
        parent: mir::Place,
        child_count: u64,
        at: mir::LocalNodeIdAny,
    ) {
        if child_count == 1 {
            self.move_place(parent, at);
        } else {
            self.decompositions.push(Decomposition {
                parent,
                remaining: child_count - 1,
                at,
            });
        }
    }

    /// Advance one aggregate decomposition after another child move.
    pub(super) fn step_decomposition(
        &mut self,
        parent: &mir::Place,
        at: mir::LocalNodeIdAny,
    ) -> bool {
        let index = self
            .decompositions
            .iter()
            .position(|decomposition| decomposition.parent == *parent);
        let Some(index) = index else {
            return false;
        };

        let decomposition = &mut self.decompositions[index];
        decomposition.remaining -= 1;

        // replace child moves with the complete aggregate move
        if decomposition.remaining == 0 {
            self.decompositions.remove(index);
            self.move_place(parent.clone(), at);
        }

        true
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

        Self {
            moves,
            decompositions: Vec::new(),
        }
    }
}
