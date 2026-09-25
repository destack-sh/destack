use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{Block, Instruction, LocalNodeId, MovePathId};

/// Ownership retention at verified MIR program points.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Reflect)]
pub struct RetentionTable {
    /// Flattened retained move paths.
    paths: Vec<MovePathId>,
    /// Retained paths at block entries.
    blocks: Vec<RetentionPoint>,
    /// Retained paths after instructions.
    instructions: Vec<RetentionPoint>,
}

impl RetentionTable {
    /// Insert paths retained at one block entry.
    pub fn insert_block(&mut self, block: LocalNodeId<Block>, paths: Vec<MovePathId>) {
        if let Some(point) = self.append(block.id, paths) {
            self.blocks.push(point);
        }
    }

    /// Insert paths retained after one instruction.
    pub fn insert_instruction(
        &mut self,
        instruction: LocalNodeId<Instruction>,
        paths: Vec<MovePathId>,
    ) {
        if let Some(point) = self.append(instruction.id, paths) {
            self.instructions.push(point);
        }
    }

    /// Append another retention table.
    pub fn extend(&mut self, other: Self) {
        let offset = self.paths.len() as u32;
        self.paths.extend(other.paths);

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

    /// Return paths retained at one block entry.
    pub fn block(&self, block: LocalNodeId<Block>) -> &[MovePathId] {
        self.get(block.id, &self.blocks)
    }

    /// Return paths retained after one instruction.
    pub fn instruction(&self, instruction: LocalNodeId<Instruction>) -> &[MovePathId] {
        self.get(instruction.id, &self.instructions)
    }

    /// Append one point to flattened entry storage.
    fn append(&mut self, node: u32, paths: Vec<MovePathId>) -> Option<RetentionPoint> {
        if paths.is_empty() {
            return None;
        }

        let start = self.paths.len() as u32;
        let length = paths.len() as u32;
        self.paths.extend(paths);
        Some(RetentionPoint {
            node,
            start,
            length,
        })
    }

    /// Return retained paths for one sorted program point.
    fn get(&self, node: u32, points: &[RetentionPoint]) -> &[MovePathId] {
        let Ok(index) = points.binary_search_by_key(&node, |point| point.node) else {
            return &[];
        };
        let point = points[index];
        let start = point.start as usize;
        let end = start + point.length as usize;

        &self.paths[start..end]
    }
}

/// Retained path range for one MIR program point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct RetentionPoint {
    /// The local MIR node identifier.
    node: u32,
    /// The first retained path.
    start: u32,
    /// The number of retained paths.
    length: u32,
}

impl RetentionPoint {
    /// Shift this point into appended entry storage.
    fn with_offset(mut self, offset: u32) -> Self {
        self.start += offset;

        self
    }
}
