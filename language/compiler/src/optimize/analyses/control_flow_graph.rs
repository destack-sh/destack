use std::collections::HashMap;
use std::sync::Arc;

use destack_mir as mir;

use crate::optimize::{Analysis, AnalysisCache, AnalysisKind};

/// Control flow graph for a function.
///
/// Maps each block to its predecessors (blocks that can jump to it).
/// Successors are already available via `Block::terminator.successors()`.
#[derive(Debug)]
pub struct ControlFlowGraph {
    /// Predecessors for each block.
    predecessors: HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>>,
}

impl ControlFlowGraph {
    /// Build a control flow graph for a function.
    fn build(function: &mir::Function, tree: &mir::NodeTree) -> Self {
        let mut predecessors: HashMap<
            mir::LocalNodeId<mir::Block>,
            Vec<mir::LocalNodeId<mir::Block>>,
        > = HashMap::new();

        // initialize all blocks with empty predecessor lists
        for &block_id in &function.blocks {
            predecessors.insert(block_id, Vec::new());
        }

        // compute predecessors from successors
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            for successor_id in block.terminator.successors() {
                if let Some(preds) = predecessors.get_mut(&successor_id) {
                    preds.push(block_id);
                }
            }
        }

        Self { predecessors }
    }

    /// Get the predecessors of a block.
    pub fn predecessors(
        &self,
        block: mir::LocalNodeId<mir::Block>,
    ) -> &[mir::LocalNodeId<mir::Block>] {
        self.predecessors
            .get(&block)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Check if a block is reachable (has entry as ancestor or is entry).
    pub fn is_reachable(
        &self,
        block: mir::LocalNodeId<mir::Block>,
        entry: mir::LocalNodeId<mir::Block>,
    ) -> bool {
        if block == entry {
            return true;
        }
        !self.predecessors(block).is_empty()
    }
}

impl Analysis for ControlFlowGraph {
    const KIND: AnalysisKind = AnalysisKind::ControlFlowGraph;

    fn compute(
        function: &mir::Function,
        tree: &mir::NodeTree,
        _cache: &AnalysisCache,
    ) -> Arc<Self> {
        Arc::new(Self::build(function, tree))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    #[test]
    fn test_cfg_linear_flow() {
        // linear flow: block0 -> block1 -> block2
        let program = TestProgram::new(
            r#"function @linear() -> void {
block0:
    jump block1
block1:
    jump block2
block2:
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);

        let cache = AnalysisCache::new();
        let cfg = cache.get::<ControlFlowGraph>(function, &program.tree);

        // block0 has no predecessors (entry)
        let block0 = function.entry.unwrap();
        assert!(cfg.predecessors(block0).is_empty());

        // block1 has block0 as predecessor
        let block1 = function.blocks[1];
        assert_eq!(cfg.predecessors(block1).len(), 1);

        // block2 has block1 as predecessor
        let block2 = function.blocks[2];
        assert_eq!(cfg.predecessors(block2).len(), 1);
    }

    #[test]
    fn test_cfg_branch() {
        // branch: block0 -> block1, block0 -> block2
        let program = TestProgram::new(
            r#"function @test_branch(v0: bool) -> void {
block0(v0: bool):
    branch v0, block1, block2
block1:
    return
block2:
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);

        let cache = AnalysisCache::new();
        let cfg = cache.get::<ControlFlowGraph>(function, &program.tree);

        // block1 and block2 each have block0 as predecessor
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];
        assert_eq!(cfg.predecessors(block1).len(), 1);
        assert_eq!(cfg.predecessors(block2).len(), 1);
    }

    #[test]
    fn test_cfg_diamond() {
        // diamond: block0 -> block1, block0 -> block2, block1 -> block3, block2 -> block3
        let program = TestProgram::new(
            r#"function @diamond(v0: bool) -> void {
block0(v0: bool):
    branch v0, block1, block2
block1:
    jump block3
block2:
    jump block3
block3:
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);

        let cache = AnalysisCache::new();
        let cfg = cache.get::<ControlFlowGraph>(function, &program.tree);

        // block3 has two predecessors: block1 and block2
        let block3 = function.blocks[3];
        assert_eq!(cfg.predecessors(block3).len(), 2);
    }

    #[test]
    fn test_cfg_loop() {
        // loop: block0 -> block1, block1 -> block1, block1 -> block2
        let program = TestProgram::new(
            r#"function @loop(v0: bool) -> void {
block0(v0: bool):
    jump block1(v0)
block1(v1: bool):
    branch v1, block1(v1), block2
block2:
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);

        let cache = AnalysisCache::new();
        let cfg = cache.get::<ControlFlowGraph>(function, &program.tree);

        // block1 has two predecessors: block0 and block1 (self-loop)
        let block1 = function.blocks[1];
        assert_eq!(cfg.predecessors(block1).len(), 2);
    }
}
