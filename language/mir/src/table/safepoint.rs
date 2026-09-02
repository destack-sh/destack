use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Point, Value};

/// The safepoints of verified MIR: the points where the collector may run, each with the references pinned there.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Reflect)]
pub struct SafepointTable {
    /// The safepoints sorted by point.
    points: Vec<Safepoint>,
}

/// One point where the collector may run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Safepoint {
    /// The point: a parking call, or a loop header or tail call that elaboration polls at.
    pub point: Point,
    /// The references into managed storage live across a parking call, pinned over it.
    pub pins: Vec<Value>,
}

impl SafepointTable {
    /// Insert one safepoint.
    pub fn insert(&mut self, point: Point, pins: Vec<Value>) {
        self.points.push(Safepoint { point, pins });
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
