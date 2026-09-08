use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::Point;

/// The safepoints of verified MIR: the points elaboration polls the runtime at.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Reflect)]
pub struct SafepointTable {
    /// The safepoints sorted by point.
    points: Vec<Safepoint>,
}

/// One point elaboration polls the runtime at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Safepoint {
    /// The instruction point of the safepoint.
    pub point: Point,
}

impl SafepointTable {
    /// Insert one safepoint.
    pub fn insert(&mut self, point: Point) {
        self.points.push(Safepoint { point });
    }

    /// Append another safepoint table.
    pub fn extend(&mut self, other: Self) {
        self.points.extend(other.points);
    }

    /// Sort the safepoints for direct lookup.
    pub fn sort(&mut self) {
        self.points.sort_by_key(|safepoint| safepoint.point);
    }

    /// Return the safepoint at one point.
    pub fn get(&self, point: Point) -> Option<&Safepoint> {
        self.points
            .binary_search_by_key(&point, |safepoint| safepoint.point)
            .ok()
            .map(|index| &self.points[index])
    }

    /// Iterate every safepoint.
    pub fn iter(&self) -> impl Iterator<Item = &Safepoint> {
        self.points.iter()
    }

    /// Return whether no safepoint is recorded.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}
