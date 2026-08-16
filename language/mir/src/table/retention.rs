use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Block, Instruction, LocalNodeId, MovePathId, PlaceOrigin};

/// Ownership retention at verified MIR program points.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Reflect)]
pub struct RetentionTable {
    /// Flattened retention entries.
    entries: Vec<Retention>,
    /// Retention live at block entries.
    blocks: Vec<RetentionPoint>,
    /// Retention live after instructions.
    instructions: Vec<RetentionPoint>,
}

impl RetentionTable {
    /// Insert retention live at one block entry.
    pub fn insert_block(&mut self, block: LocalNodeId<Block>, entries: Vec<Retention>) {
        if let Some(point) = self.append(block.id, entries) {
            self.blocks.push(point);
        }
    }

    /// Insert retention live after one instruction.
    pub fn insert_instruction(
        &mut self,
        instruction: LocalNodeId<Instruction>,
        entries: Vec<Retention>,
    ) {
        if let Some(point) = self.append(instruction.id, entries) {
            self.instructions.push(point);
        }
    }

    /// Append another retention table.
    pub fn extend(&mut self, other: Self) {
        let offset = self.entries.len() as u32;
        self.entries.extend(other.entries);

        // append point ranges adjusted to the combined entry storage
        self.blocks.extend(
            other
                .blocks
                .into_iter()
                .map(|point| point.with_offset(offset)),
        );
        self.instructions.extend(
            other
                .instructions
                .into_iter()
                .map(|point| point.with_offset(offset)),
        );
    }

    /// Sort program points for direct lookup.
    pub fn sort(&mut self) {
        self.blocks.sort_unstable_by_key(|point| point.node);
        self.instructions.sort_unstable_by_key(|point| point.node);
    }

    /// Return retention live at one block entry.
    pub fn block(&self, block: LocalNodeId<Block>) -> &[Retention] {
        self.get(block.id, &self.blocks)
    }

    /// Return retention live after one instruction.
    pub fn instruction(&self, instruction: LocalNodeId<Instruction>) -> &[Retention] {
        self.get(instruction.id, &self.instructions)
    }

    /// Append one point to flattened entry storage.
    fn append(&mut self, node: u32, entries: Vec<Retention>) -> Option<RetentionPoint> {
        if entries.is_empty() {
            return None;
        }

        let start = self.entries.len() as u32;
        let length = entries.len() as u32;
        self.entries.extend(entries);
        Some(RetentionPoint {
            node,
            range: RetentionRange { start, length },
        })
    }

    /// Return retention for one sorted program point.
    fn get(&self, node: u32, points: &[RetentionPoint]) -> &[Retention] {
        let Ok(index) = points.binary_search_by_key(&node, |point| point.node) else {
            return &[];
        };
        let range = points[index].range;
        let start = range.start as usize;
        let end = start + range.length as usize;

        &self.entries[start..end]
    }
}

/// One carrier keeping borrowed storage alive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Retention {
    /// The lifetime carrier for the borrow.
    pub carrier: RetentionCarrier,
    /// The move-only path kept initialized by the borrow.
    pub owner: MovePathId,
}

impl Retention {
    /// Create one retention entry.
    pub fn new(carrier: RetentionCarrier, owner: MovePathId) -> Self {
        Self { carrier, owner }
    }
}

/// One carrier keeping a borrowed owner alive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum RetentionCarrier {
    /// A live value or storage place.
    Place(PlaceOrigin),
    /// The current function frame.
    Frame,
}

/// Retention range for one MIR program point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct RetentionPoint {
    /// The local MIR node identifier.
    node: u32,
    /// The retention live at this point.
    range: RetentionRange,
}

impl RetentionPoint {
    /// Shift this point into appended entry storage.
    fn with_offset(mut self, offset: u32) -> Self {
        self.range.start += offset;

        self
    }
}

/// One range in flattened retention storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct RetentionRange {
    /// The first entry index.
    start: u32,
    /// The number of entries.
    length: u32,
}
