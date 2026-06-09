use std::collections::HashMap;

use destack_mir as mir;

/// Conservative place alias relation for one function.
pub(super) struct PlaceAlias {
    /// Integer constants by SSA value.
    constants: HashMap<mir::Value, u128>,
}

impl PlaceAlias {
    /// Build place alias relation for one function.
    pub(super) fn new(function: &mir::Function, tree: &mir::Tree) -> Self {
        let constants = Self::integer_constants(function, tree);

        Self { constants }
    }

    /// Return whether two places may alias.
    pub(super) fn may_alias(&self, left: &mir::Place, right: &mir::Place) -> bool {
        if left.origin != right.origin {
            return Self::origins_may_alias(left.origin, right.origin);
        }

        self.same_origin_may_alias(left, right)
    }

    /// Return whether two moved places may alias.
    pub(super) fn moved_may_alias(&self, left: &mir::Place, right: &mir::Place) -> bool {
        if left.origin != right.origin {
            return false;
        }

        self.same_origin_may_alias(left, right)
    }

    /// Return whether two places with the same origin may alias.
    fn same_origin_may_alias(&self, left: &mir::Place, right: &mir::Place) -> bool {
        // stop once any shared projection proves disjointness
        for (left, right) in left.path.projections.iter().zip(&right.path.projections) {
            if self.projections_are_disjoint(left, right) {
                return false;
            }
        }

        true
    }

    /// Return whether two place origins may alias.
    fn origins_may_alias(left: mir::PlaceOrigin, right: mir::PlaceOrigin) -> bool {
        matches!(left, mir::PlaceOrigin::Value(_)) || matches!(right, mir::PlaceOrigin::Value(_))
    }

    /// Return whether two projections are proven disjoint.
    fn projections_are_disjoint(&self, left: &mir::Projection, right: &mir::Projection) -> bool {
        let Some(left) = self.projection_interval(left) else {
            return false;
        };
        let Some(right) = self.projection_interval(right) else {
            return false;
        };

        left.1 <= right.0 || right.1 <= left.0
    }

    /// Return the covered half-open index interval for one projection.
    fn projection_interval(&self, projection: &mir::Projection) -> Option<(u128, u128)> {
        match projection {
            mir::Projection::Field { index } => {
                let start = u128::from(*index);
                let end = start.checked_add(1)?;

                Some((start, end))
            }
            mir::Projection::Element { index } => {
                let start = u128::from(*index);
                let end = start.checked_add(1)?;

                Some((start, end))
            }
            mir::Projection::Index { index } => {
                let start = self.constant_index(*index)?;
                let end = start.checked_add(1)?;

                Some((start, end))
            }
            mir::Projection::AnyElement | mir::Projection::Variant { .. } => None,
            mir::Projection::Slice { start, length } => {
                let start = self.constant_index(*start)?;
                let length = self.constant_index(*length)?;
                let end = start.checked_add(length)?;

                Some((start, end))
            }
        }
    }

    /// Return the integer constant behind one value reference.
    fn constant_index(&self, value: mir::ValueReference) -> Option<u128> {
        let value = value.value()?;

        self.constants.get(&value).copied()
    }

    /// Collect integer constants used by place projections.
    fn integer_constants(function: &mir::Function, tree: &mir::Tree) -> HashMap<mir::Value, u128> {
        let mut constants = HashMap::new();

        // scan instruction constants once per function
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            for &instruction_id in &block.instructions {
                let mir::Instruction::Const { destination, value } = tree.get(instruction_id)
                else {
                    continue;
                };
                let Some(destination) = destination.value() else {
                    continue;
                };
                let Some(value) = Self::integer_constant(value) else {
                    continue;
                };

                constants.insert(destination, value);
            }
        }

        constants
    }

    /// Return an unsigned integer value for one constant.
    fn integer_constant(value: &mir::Constant) -> Option<u128> {
        match value {
            mir::Constant::Int { value, .. } => u128::try_from(*value).ok(),
            mir::Constant::UInt { value, .. } => Some(*value),
            _ => None,
        }
    }
}
