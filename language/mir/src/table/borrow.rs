use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Block, Instruction, LocalNodeId, PlaceOrigin};

/// Borrow dependencies at verified MIR program points.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Reflect)]
pub struct BorrowTable {
    /// Flattened borrow dependencies.
    dependencies: Vec<BorrowDependency>,
    /// Dependencies live at block entries.
    blocks: Vec<BorrowPoint>,
    /// Dependencies live after instructions.
    instructions: Vec<BorrowPoint>,
}

impl BorrowTable {
    /// Insert dependencies live at one block entry.
    pub fn insert_block(&mut self, block: LocalNodeId<Block>, dependencies: Vec<BorrowDependency>) {
        if let Some(point) = self.append(block.id, dependencies) {
            self.blocks.push(point);
        }
    }

    /// Insert dependencies live after one instruction.
    pub fn insert_instruction(
        &mut self,
        instruction: LocalNodeId<Instruction>,
        dependencies: Vec<BorrowDependency>,
    ) {
        if let Some(point) = self.append(instruction.id, dependencies) {
            self.instructions.push(point);
        }
    }

    /// Append another verified borrow table.
    pub fn extend(&mut self, other: Self) {
        let offset = self.dependencies.len() as u32;
        self.dependencies.extend(other.dependencies);

        // append point ranges adjusted to the combined dependency storage
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

    /// Return dependencies live at one block entry.
    pub fn block(&self, block: LocalNodeId<Block>) -> &[BorrowDependency] {
        self.get(block.id, &self.blocks)
    }

    /// Return dependencies live after one instruction.
    pub fn instruction(&self, instruction: LocalNodeId<Instruction>) -> &[BorrowDependency] {
        self.get(instruction.id, &self.instructions)
    }

    /// Append one point to flattened dependency storage.
    fn append(&mut self, node: u32, dependencies: Vec<BorrowDependency>) -> Option<BorrowPoint> {
        if dependencies.is_empty() {
            return None;
        }

        let start = self.dependencies.len() as u32;
        let length = dependencies.len() as u32;
        self.dependencies.extend(dependencies);
        Some(BorrowPoint {
            node,
            range: BorrowRange { start, length },
        })
    }

    /// Return dependencies for one sorted program point.
    fn get(&self, node: u32, points: &[BorrowPoint]) -> &[BorrowDependency] {
        let Ok(index) = points.binary_search_by_key(&node, |point| point.node) else {
            return &[];
        };
        let range = points[index].range;
        let start = range.start as usize;
        let end = start + range.length as usize;

        &self.dependencies[start..end]
    }
}

/// One carrier keeping a move-only owner alive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct BorrowDependency {
    /// The value or storage carrying the borrow.
    pub carrier: PlaceOrigin,
    /// The borrowed move-only owner.
    pub owner: PlaceOrigin,
}

impl BorrowDependency {
    /// Create one borrow dependency.
    pub fn new(carrier: PlaceOrigin, owner: PlaceOrigin) -> Self {
        Self { carrier, owner }
    }
}

/// Dependency range for one MIR program point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct BorrowPoint {
    /// The local MIR node identifier.
    node: u32,
    /// The dependencies live at this point.
    range: BorrowRange,
}

impl BorrowPoint {
    /// Shift this point into appended dependency storage.
    fn with_offset(mut self, offset: u32) -> Self {
        self.range.start += offset;

        self
    }
}

/// One range in flattened borrow dependency storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct BorrowRange {
    /// The first dependency index.
    start: u32,
    /// The number of dependencies.
    length: u32,
}
