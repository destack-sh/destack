use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Point, Value};

/// The safepoints of verified MIR.
///
/// Each safepoint is one point where the collector may run, with the handles held live there.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Reflect)]
pub struct SafepointTable {
    /// The safepoints sorted by point.
    points: Vec<Safepoint>,
}

/// One point where the collector may run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Safepoint {
    /// The instruction point of the safepoint.
    pub point: Point,
    /// How the collector gets to run at the point.
    pub kind: SafepointKind,
    /// The managed handles whose borrows are live at the point, held live past it.
    pub live: Vec<Value>,
}

/// How the collector gets to run at one safepoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum SafepointKind {
    /// A call that may park the fiber.
    Park,
    /// A point elaboration polls the runtime at.
    Poll,
}

impl SafepointTable {
    /// Insert one safepoint.
    pub fn insert(&mut self, point: Point, kind: SafepointKind, live: Vec<Value>) {
        self.points.push(Safepoint { point, kind, live });
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
