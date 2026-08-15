use super::{Analysis, FunctionCache, Mutation};
use crate::{Block, Function, LocalNodeId, NodeTable, Tree};

/// Control flow graph for one function.
#[derive(Debug, Clone)]
pub struct ControlTable {
    /// Compact index for each function block.
    indices: NodeTable<Block, u32>,
    /// First predecessor offset for each block and the final predecessor count.
    offsets: Vec<u32>,
    /// Predecessors grouped by block id.
    predecessors: Vec<LocalNodeId<Block>>,
}

impl ControlTable {
    /// Build the control flow graph for one function.
    pub fn build(function: &Function, tree: &Tree) -> Self {
        let mut blocks = function.blocks().to_vec();
        blocks.sort_unstable_by_key(|block| block.get());

        let mut indices = NodeTable::from_nodes(&blocks, || 0);
        for (index, block) in blocks.iter().copied().enumerate() {
            *indices.get_mut(block) = index as u32;
        }

        let block_count = blocks.len();
        let mut edges = Vec::new();

        // collect predecessor pairs
        for &block_id in function.blocks() {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);

            for successor in terminator.successors(tree) {
                edges.push((successor, block_id));
            }
        }
        edges.sort_unstable_by_key(|(successor, predecessor)| (successor.get(), predecessor.get()));
        edges.dedup();

        let mut offsets = vec![0u32; block_count + 1];

        // count predecessors per block
        for (successor, _) in &edges {
            let index = *indices.get(*successor) as usize;
            offsets[index + 1] += 1;
        }

        // convert predecessor counts into ranges
        for block in 0..block_count {
            offsets[block + 1] += offsets[block];
        }

        let predecessors = edges
            .into_iter()
            .map(|(_, predecessor)| predecessor)
            .collect();

        Self {
            indices,
            offsets,
            predecessors,
        }
    }

    /// Return the predecessors of one block.
    pub fn predecessors(&self, block: LocalNodeId<Block>) -> &[LocalNodeId<Block>] {
        let index = *self.indices.get(block) as usize;
        let range = &self.offsets[index..=index + 1];
        let start = range[0] as usize;
        let end = range[1] as usize;

        &self.predecessors[start..end]
    }

    /// Return whether one block is reachable from the entry block.
    pub fn is_reachable(&self, block: LocalNodeId<Block>, entry: LocalNodeId<Block>) -> bool {
        // validate query blocks
        self.predecessors(block);
        self.predecessors(entry);

        // accept the entry block itself
        if block == entry {
            return true;
        }

        let mut worklist = vec![block];
        let mut visited = vec![false; self.offsets.len() - 1];

        // walk backward through predecessors until we find the entry
        while let Some(current) = worklist.pop() {
            let index = *self.indices.get(current) as usize;
            let is_visited = &mut visited[index];
            if *is_visited {
                continue;
            }
            *is_visited = true;

            if current == entry {
                return true;
            }

            worklist.extend(self.predecessors(current).iter().copied());
        }

        false
    }
}

impl Analysis for ControlTable {
    const INVALIDATED_BY: Mutation = Mutation::CONTROL;
}

impl ControlTable {
    /// Compute control flow for one function.
    pub(crate) fn compute(function: &Function, tree: &Tree, _analyses: &mut FunctionCache) -> Self {
        Self::build(function, tree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestProgram;

    #[test]
    fn test_build_predecessors_for_linear_flow() {
        let (tree, function_id) = TestProgram::parse_function(
            r#"
function linear(): void {
entry:
    jump b1

b1:
    jump b2

b2:
    return
}
"#,
        );

        let function = tree.get(function_id);
        let cfg = ControlTable::build(function, &tree);

        let block0 = function.entry().expect("missing entry");
        let block1 = function.block(1);
        let block2 = function.block(2);

        assert!(cfg.predecessors(block0).is_empty());
        assert_eq!(cfg.predecessors(block1).len(), 1);
        assert_eq!(cfg.predecessors(block2).len(), 1);
    }

    #[test]
    fn test_build_predecessors_for_branch() {
        let (tree, function_id) = TestProgram::parse_function(
            r#"
function testBranch(v0: boolean): void {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    return

b2:
    return
}
"#,
        );

        let function = tree.get(function_id);
        let cfg = ControlTable::build(function, &tree);

        let block1 = function.block(1);
        let block2 = function.block(2);

        assert_eq!(cfg.predecessors(block1).len(), 1);
        assert_eq!(cfg.predecessors(block2).len(), 1);
    }

    #[test]
    fn test_build_predecessors_for_diamond() {
        let (tree, function_id) = TestProgram::parse_function(
            r#"
function diamond(v0: boolean): void {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    jump b3

b2:
    jump b3

b3:
    return
}
"#,
        );

        let function = tree.get(function_id);
        let cfg = ControlTable::build(function, &tree);

        let block3 = function.block(3);

        assert_eq!(cfg.predecessors(block3).len(), 2);
    }

    #[test]
    fn test_build_predecessors_for_loop() {
        let (tree, function_id) = TestProgram::parse_function(
            r#"
function loop(v0: boolean): void {
entry(v0: boolean):
    jump b1(v0)

b1(v1: boolean):
    branch v1 => b1(v1) | b2

b2:
    return
}
"#,
        );

        let function = tree.get(function_id);
        let cfg = ControlTable::build(function, &tree);

        let block1 = function.block(1);

        assert_eq!(cfg.predecessors(block1).len(), 2);
    }

    #[test]
    fn test_check_reachability() {
        let (tree, function_id) = TestProgram::parse_function(
            r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: int32 = 1
    return v1

b2:
    unreachable

b3:
    v2: int32 = 2
    return v2
}
"#,
        );

        let function = tree.get(function_id);
        let cfg = ControlTable::build(function, &tree);
        let entry = function.entry().expect("missing entry");

        let reachable_block = function.block(1);
        let unreachable_block = function.block(3);

        assert!(cfg.is_reachable(reachable_block, entry));
        assert!(!cfg.is_reachable(unreachable_block, entry));
    }
}
