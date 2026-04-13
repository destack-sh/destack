use std::collections::{HashMap, HashSet};

use destack_mir as mir;

use crate::optimize::{Analysis, AnalysisId, FunctionAnalyses, FunctionAnalysis};

use super::{ControlFlowGraph, DominatorTree};

/// A natural loop in the control flow graph.
///
/// A natural loop has a single entry point (the header) and is defined by one or
/// more back edges. A back edge is an edge from a block to one of its dominators.
/// The loop body consists of all blocks that can reach a back edge source (latch)
/// without going through the header.
///
/// This analysis only identifies natural (reducible) loops. Irreducible control
/// flow (multiple entry points) is not represented as a loop.
#[derive(Debug, Clone)]
pub struct Loop {
    /// The loop header block.
    ///
    /// The single entry point to the loop. All back edges target this block.
    /// The header dominates all blocks in the loop body.
    pub header: mir::LocalNodeId<mir::Block>,

    /// Latch blocks (back edge sources).
    ///
    /// These are the blocks that branch back to the header. A loop with a single
    /// latch is in canonical form and easier to optimize.
    pub latches: Vec<mir::LocalNodeId<mir::Block>>,

    /// All blocks in the loop body, including the header.
    pub blocks: HashSet<mir::LocalNodeId<mir::Block>>,

    /// Exiting blocks (blocks with edges leaving the loop).
    ///
    /// A loop with a single exiting block is easier to transform. The exiting
    /// block is not necessarily the same as the latch.
    pub exiting_blocks: Vec<mir::LocalNodeId<mir::Block>>,

    /// Exit blocks (blocks outside the loop that are targets of exiting edges).
    ///
    /// A loop with a single exit block has simpler control flow for LCSSA form.
    pub exit_blocks: Vec<mir::LocalNodeId<mir::Block>>,

    /// Parent loop index, if this is a nested loop.
    pub parent: Option<usize>,

    /// Nesting depth (0 for outermost loops).
    pub depth: u32,
}

impl Loop {
    /// Check if the loop has a single latch (canonical form).
    pub fn has_single_latch(&self) -> bool {
        self.latches.len() == 1
    }

    /// Check if the loop has a single exiting block.
    pub fn has_single_exit(&self) -> bool {
        self.exiting_blocks.len() == 1
    }

    /// Check if a block is inside this loop.
    pub fn contains(&self, block: mir::LocalNodeId<mir::Block>) -> bool {
        self.blocks.contains(&block)
    }
}

/// Natural loop analysis for a function.
///
/// Identifies loops using dominator-based back edge detection. A back edge is an
/// edge A → B where B dominates A. Each unique header (back edge target) defines
/// one natural loop.
///
/// Multiple back edges to the same header are merged into a single loop with
/// multiple latches. Nested loops are detected and organized into a forest.
///
/// This analysis assumes reducible control flow. Irreducible loops (with multiple
/// entry points) are not detected. Most structured source languages produce only
/// reducible control flow.
#[derive(Debug)]
pub struct LoopAnalysis {
    /// All loops, indexed by loop ID.
    loops: Vec<Loop>,

    /// Map from header block to loop index.
    header_to_loop: HashMap<mir::LocalNodeId<mir::Block>, usize>,

    /// Map from block to its innermost containing loop.
    block_to_loop: HashMap<mir::LocalNodeId<mir::Block>, usize>,
}

impl LoopAnalysis {
    /// Build loop analysis from dominator information.
    fn build(
        function: &mir::Function,
        tree: &mir::NodeTree,
        cfg: &ControlFlowGraph,
        domtree: &DominatorTree,
    ) -> Self {
        // handle functions without bodies (imports)
        if function.entry.is_none() {
            return Self {
                loops: Vec::new(),
                header_to_loop: HashMap::new(),
                block_to_loop: HashMap::new(),
            };
        }

        // collect back edges grouped by header
        let back_edges_by_header = Self::find_back_edges(function, tree, domtree);

        // build loop structures from back edges
        let (mut loops, header_to_loop) =
            Self::build_loops(back_edges_by_header, tree, cfg, domtree);

        // establish parent/child relationships and compute depths
        Self::compute_nesting(&mut loops, &header_to_loop, domtree);

        // map each block to its innermost containing loop
        let block_to_loop = Self::build_block_map(&loops);

        Self {
            loops,
            header_to_loop,
            block_to_loop,
        }
    }

    /// Find all back edges grouped by their target (header).
    fn find_back_edges(
        function: &mir::Function,
        tree: &mir::NodeTree,
        domtree: &DominatorTree,
    ) -> HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>> {
        let mut back_edges: HashMap<
            mir::LocalNodeId<mir::Block>,
            Vec<mir::LocalNodeId<mir::Block>>,
        > = HashMap::new();

        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);

            // check each outgoing edge
            for successor in terminator.successors() {
                let Some(successor) = successor.block() else {
                    continue;
                };

                // back edge: successor dominates the current block
                if domtree.dominates(successor, block_id) {
                    back_edges.entry(successor).or_default().push(block_id);
                }
            }
        }

        back_edges
    }

    /// Build loop structures from back edges.
    fn build_loops(
        back_edges_by_header: HashMap<
            mir::LocalNodeId<mir::Block>,
            Vec<mir::LocalNodeId<mir::Block>>,
        >,
        tree: &mir::NodeTree,
        cfg: &ControlFlowGraph,
        domtree: &DominatorTree,
    ) -> (Vec<Loop>, HashMap<mir::LocalNodeId<mir::Block>, usize>) {
        let mut loops = Vec::new();
        let mut header_to_loop = HashMap::new();

        // sort by header block ID for deterministic iteration order
        let mut sorted_entries: Vec<_> = back_edges_by_header.into_iter().collect();
        sorted_entries.sort_by_key(|(header, _)| *header);

        for (header, latches) in sorted_entries {
            // compute loop body via reverse reachability from latches
            let blocks = Self::compute_loop_body(header, &latches, cfg, domtree);

            // find exiting blocks and exit blocks
            let (exiting_blocks, exit_blocks) = Self::compute_exits(&blocks, tree);

            let loop_index = loops.len();
            loops.push(Loop {
                header,
                latches,
                blocks,
                exiting_blocks,
                exit_blocks,
                parent: None,
                depth: 0,
            });
            header_to_loop.insert(header, loop_index);
        }

        (loops, header_to_loop)
    }

    /// Compute loop body using reverse DFS from latches.
    ///
    /// The body includes all blocks that can reach any latch without passing
    /// through the header, plus the header itself.
    fn compute_loop_body(
        header: mir::LocalNodeId<mir::Block>,
        latches: &[mir::LocalNodeId<mir::Block>],
        cfg: &ControlFlowGraph,
        domtree: &DominatorTree,
    ) -> HashSet<mir::LocalNodeId<mir::Block>> {
        let mut body = HashSet::new();
        body.insert(header);

        let mut worklist: Vec<mir::LocalNodeId<mir::Block>> = Vec::new();

        // seed with latch blocks
        for &latch in latches {
            if body.insert(latch) {
                worklist.push(latch);
            }
        }

        // reverse DFS: add predecessors that aren't the header
        while let Some(block) = worklist.pop() {
            for &predecessor in cfg.predecessors(block) {
                if !domtree.dominates(header, predecessor) {
                    continue;
                }

                if body.insert(predecessor) {
                    worklist.push(predecessor);
                }
            }
        }

        body
    }

    /// Find exiting blocks and exit blocks for a loop.
    fn compute_exits(
        body: &HashSet<mir::LocalNodeId<mir::Block>>,
        tree: &mir::NodeTree,
    ) -> (
        Vec<mir::LocalNodeId<mir::Block>>,
        Vec<mir::LocalNodeId<mir::Block>>,
    ) {
        let mut exiting_blocks = Vec::new();
        let mut exit_blocks_set = HashSet::new();

        for &block_id in body {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);
            let mut is_exiting = false;

            for successor in terminator.successors() {
                let Some(successor) = successor.block() else {
                    continue;
                };

                if !body.contains(&successor) {
                    exit_blocks_set.insert(successor);
                    is_exiting = true;
                }
            }

            if is_exiting {
                exiting_blocks.push(block_id);
            }
        }

        let mut exit_blocks: Vec<_> = exit_blocks_set.into_iter().collect();

        exiting_blocks.sort_by_key(|block| block.id);
        exit_blocks.sort_by_key(|block| block.id);

        (exiting_blocks, exit_blocks)
    }

    /// Compute parent relationships and nesting depths.
    fn compute_nesting(
        loops: &mut [Loop],
        header_to_loop: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
        domtree: &DominatorTree,
    ) {
        let loop_count = loops.len();

        // find parent for each loop by walking up dominator tree
        for i in 0..loop_count {
            let header = loops[i].header;

            // walk up immediate dominators to find containing loop
            let mut current = domtree.immediate_dominator(header);
            while let Some(dominator) = current {
                if let Some(&parent_index) = header_to_loop.get(&dominator) {
                    // verify the header is actually in the parent's body
                    if loops[parent_index].blocks.contains(&header) {
                        loops[i].parent = Some(parent_index);
                        break;
                    }
                }
                current = domtree.immediate_dominator(dominator);
            }
        }

        // compute depths from parent chain
        for i in 0..loop_count {
            let mut depth = 0u32;
            let mut current = loops[i].parent;

            while let Some(parent_index) = current {
                depth += 1;
                current = loops[parent_index].parent;
            }

            loops[i].depth = depth;
        }
    }

    /// Build mapping from blocks to their innermost containing loop.
    fn build_block_map(loops: &[Loop]) -> HashMap<mir::LocalNodeId<mir::Block>, usize> {
        let mut block_to_loop = HashMap::new();

        // process deepest loops first so innermost wins
        let mut indices: Vec<usize> = (0..loops.len()).collect();
        indices.sort_by_key(|&i| std::cmp::Reverse(loops[i].depth));

        for index in indices {
            for &block in &loops[index].blocks {
                block_to_loop.entry(block).or_insert(index);
            }
        }

        block_to_loop
    }

    /// Get all loops.
    pub fn loops(&self) -> &[Loop] {
        &self.loops
    }

    /// Get the number of loops.
    pub fn num_loops(&self) -> usize {
        self.loops.len()
    }

    /// Check if a block is a loop header.
    pub fn is_loop_header(&self, block: mir::LocalNodeId<mir::Block>) -> bool {
        self.header_to_loop.contains_key(&block)
    }

    /// Get the loop with the given header.
    pub fn loop_for_header(&self, header: mir::LocalNodeId<mir::Block>) -> Option<&Loop> {
        self.header_to_loop.get(&header).map(|&i| &self.loops[i])
    }

    /// Get the innermost loop containing a block.
    pub fn innermost_loop(&self, block: mir::LocalNodeId<mir::Block>) -> Option<&Loop> {
        self.block_to_loop.get(&block).map(|&i| &self.loops[i])
    }

    /// Get the nesting depth for a block (0 if not in any loop).
    pub fn loop_depth(&self, block: mir::LocalNodeId<mir::Block>) -> u32 {
        self.block_to_loop
            .get(&block)
            .map(|&i| self.loops[i].depth + 1)
            .unwrap_or(0)
    }

    /// Check if a block is inside any loop.
    pub fn is_in_loop(&self, block: mir::LocalNodeId<mir::Block>) -> bool {
        self.block_to_loop.contains_key(&block)
    }

    /// Iterate over loops at a specific nesting depth.
    pub fn loops_at_depth(&self, depth: u32) -> impl Iterator<Item = &Loop> {
        self.loops.iter().filter(move |lp| lp.depth == depth)
    }

    /// Iterate over top-level (outermost) loops.
    pub fn top_level_loops(&self) -> impl Iterator<Item = &Loop> {
        self.loops_at_depth(0)
    }

    /// Get the child loops of a given loop.
    pub fn child_loops(&self, loop_index: usize) -> impl Iterator<Item = (usize, &Loop)> {
        self.loops
            .iter()
            .enumerate()
            .filter(move |(_, lp)| lp.parent == Some(loop_index))
    }

    /// Get a loop by index.
    pub fn get_loop(&self, index: usize) -> Option<&Loop> {
        self.loops.get(index)
    }

    /// Get the loop index for a header.
    pub fn loop_index(&self, header: mir::LocalNodeId<mir::Block>) -> Option<usize> {
        self.header_to_loop.get(&header).copied()
    }
}

impl Analysis for LoopAnalysis {
    const ID: AnalysisId = AnalysisId("loops");
    const DEPENDENCIES: &'static [AnalysisId] = &[DominatorTree::ID];
}

impl FunctionAnalysis for LoopAnalysis {
    fn compute(
        function: &mir::Function,
        tree: &mir::NodeTree,
        analyses: &FunctionAnalyses<'_>,
    ) -> Self {
        let cfg = analyses.get::<ControlFlowGraph>();
        let domtree = analyses.get::<DominatorTree>();
        Self::build(function, tree, &cfg, &domtree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Self-loop (single block looping to itself) is detected.
    #[test]
    fn test_detect_self_loop() {
        let test = TestProgram::new(
            r#"
function selfLoop(v0: boolean): void {
b0(v0: boolean):
    branch v0, b0(v0), b1
b1:
    return
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let analysis = analyses.get::<LoopAnalysis>();

        assert_eq!(analysis.num_loops(), 1);

        let lp = &analysis.loops()[0];
        assert_eq!(lp.header, function.blocks[0]);
        assert_eq!(lp.latches.len(), 1);
        assert_eq!(lp.blocks.len(), 1);
        assert_eq!(lp.depth, 0);
        assert!(lp.has_single_latch());
    }

    /// Self-loop with an outside predecessor does not pull the predecessor into the loop body.
    #[test]
    fn test_self_loop_excludes_predecessor() {
        let test = TestProgram::new(
            r#"
function selfLoopEntry(v0: boolean): void {
b0(v0: boolean):
    jump b1
b1:
    branch v0, b1, b2
b2:
    return
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let analysis = analyses.get::<LoopAnalysis>();

        assert_eq!(analysis.num_loops(), 1);

        let block0 = function.blocks[0];
        let block1 = function.blocks[1];

        let lp = analysis.loop_for_header(block1).unwrap();
        assert!(lp.contains(block1));
        assert!(!lp.contains(block0));
    }

    /// While-style loop with separate header and latch is detected.
    #[test]
    fn test_detect_while_loop() {
        let test = TestProgram::new(
            r#"
function whileLoop(v0: boolean): void {
b0(v0: boolean):
    jump b1(v0)
b1(v1: boolean):
    branch v1, b2, b3
b2:
    jump b1(v1)
b3:
    return
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let analysis = analyses.get::<LoopAnalysis>();

        assert_eq!(analysis.num_loops(), 1);

        let block1 = function.blocks[1];
        let block2 = function.blocks[2];

        let lp = analysis.loop_for_header(block1).unwrap();

        // block1 is header, block2 is latch
        assert_eq!(lp.header, block1);
        assert_eq!(lp.latches, vec![block2]);
        assert!(lp.blocks.contains(&block1));
        assert!(lp.blocks.contains(&block2));
        assert_eq!(lp.blocks.len(), 2);

        // block1 is the exiting block (branches to block3)
        assert_eq!(lp.exiting_blocks, vec![block1]);
        assert!(lp.has_single_exit());
    }

    /// Nested loops have correct parent relationships and depths.
    #[test]
    fn test_detect_nested_loops() {
        let test = TestProgram::new(
            r#"
function nested(v0: boolean, v1: boolean): void {
b0(v0: boolean, v1: boolean):
    jump b1(v0, v1)
b1(v2: boolean, v3: boolean):
    branch v2, b2(v3), b4
b2(v4: boolean):
    branch v4, b3, b1(v2, v4)
b3:
    jump b2(v4)
b4:
    return
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let analysis = analyses.get::<LoopAnalysis>();

        assert_eq!(analysis.num_loops(), 2);

        let block1 = function.blocks[1];
        let block2 = function.blocks[2];
        let block3 = function.blocks[3];

        let outer = analysis.loop_for_header(block1).unwrap();
        let inner = analysis.loop_for_header(block2).unwrap();

        // outer loop
        assert!(outer.parent.is_none());
        assert_eq!(outer.depth, 0);
        assert!(outer.contains(block1));
        assert!(outer.contains(block2));
        assert!(outer.contains(block3));

        // inner loop
        assert!(inner.parent.is_some());
        assert_eq!(inner.depth, 1);
        assert!(inner.contains(block2));
        assert!(inner.contains(block3));
        assert!(!inner.contains(block1));
    }

    /// Loop depth query returns correct values.
    #[test]
    fn test_compute_loop_depth() {
        let test = TestProgram::new(
            r#"
function depth(v0: boolean): void {
b0(v0: boolean):
    jump b1(v0)
b1(v1: boolean):
    branch v1, b2(v1), b3
b2(v2: boolean):
    branch v2, b2(v2), b1(v2)
b3:
    return
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let analysis = analyses.get::<LoopAnalysis>();

        let block0 = function.blocks[0];
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];
        let block3 = function.blocks[3];

        // blocks outside loops have depth 0
        assert_eq!(analysis.loop_depth(block0), 0);
        assert_eq!(analysis.loop_depth(block3), 0);
        assert!(!analysis.is_in_loop(block0));
        assert!(!analysis.is_in_loop(block3));

        // block1 is in outer loop only (depth 1)
        assert_eq!(analysis.loop_depth(block1), 1);
        assert!(analysis.is_in_loop(block1));

        // block2 is in nested loop (depth 2)
        assert_eq!(analysis.loop_depth(block2), 2);
        assert!(analysis.is_in_loop(block2));
    }

    /// Function without loops returns empty analysis.
    #[test]
    fn test_handle_no_loops() {
        let test = TestProgram::new(
            r#"
function noLoops(v0: boolean): void {
b0(v0: boolean):
    branch v0, b1, b2
b1:
    jump b3
b2:
    jump b3
b3:
    return
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let analysis = analyses.get::<LoopAnalysis>();

        assert_eq!(analysis.num_loops(), 0);
        assert!(analysis.loops().is_empty());

        for &block in &function.blocks {
            assert!(!analysis.is_in_loop(block));
            assert_eq!(analysis.loop_depth(block), 0);
        }
    }

    /// Multiple back edges to the same header create a single loop.
    #[test]
    fn test_merge_multiple_latches() {
        let test = TestProgram::new(
            r#"
function multiLatch(v0: boolean): void {
b0(v0: boolean):
    jump b1(v0)
b1(v1: boolean):
    branch v1, b2, b3
b2:
    jump b1(v1)
b3:
    jump b1(v1)
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let analysis = analyses.get::<LoopAnalysis>();

        assert_eq!(analysis.num_loops(), 1);

        let lp = &analysis.loops()[0];
        assert_eq!(lp.header, function.blocks[1]);
        assert_eq!(lp.latches.len(), 2);
        assert!(!lp.has_single_latch());
    }

    /// Exiting blocks and exit blocks are computed correctly.
    #[test]
    fn test_compute_exit_info() {
        let test = TestProgram::new(
            r#"
function exits(v0: boolean, v1: boolean): void {
b0(v0: boolean, v1: boolean):
    jump b1(v0, v1)
b1(v2: boolean, v3: boolean):
    branch v2, b2(v3), b4
b2(v4: boolean):
    branch v4, b1(v2, v4), b3
b3:
    return
b4:
    return
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let analysis = analyses.get::<LoopAnalysis>();

        assert_eq!(analysis.num_loops(), 1);

        let lp = &analysis.loops()[0];
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];
        let block3 = function.blocks[3];
        let block4 = function.blocks[4];

        // two exiting blocks: block1 and block2
        assert_eq!(lp.exiting_blocks.len(), 2);
        assert!(lp.exiting_blocks.contains(&block1));
        assert!(lp.exiting_blocks.contains(&block2));

        // two exit blocks: block3 and block4
        assert_eq!(lp.exit_blocks.len(), 2);
        assert!(lp.exit_blocks.contains(&block3));
        assert!(lp.exit_blocks.contains(&block4));

        assert!(!lp.has_single_exit());
    }

    /// Innermost loop is returned for blocks in nested loops.
    #[test]
    fn test_return_innermost_loop() {
        let test = TestProgram::new(
            r#"
function innermost(v0: boolean): void {
b0(v0: boolean):
    jump b1(v0)
b1(v1: boolean):
    branch v1, b2(v1), b3
b2(v2: boolean):
    branch v2, b2(v2), b1(v2)
b3:
    return
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let analysis = analyses.get::<LoopAnalysis>();

        let block1 = function.blocks[1];
        let block2 = function.blocks[2];

        // block2's innermost loop has block2 as header
        let inner = analysis.innermost_loop(block2).unwrap();
        assert_eq!(inner.header, block2);

        // block1's innermost loop has block1 as header
        let outer = analysis.innermost_loop(block1).unwrap();
        assert_eq!(outer.header, block1);
    }

    /// Top-level loops iterator returns only outermost loops.
    #[test]
    fn test_iterate_top_level_loops() {
        let test = TestProgram::new(
            r#"
function twoOuter(v0: boolean, v1: boolean): void {
b0(v0: boolean, v1: boolean):
    jump b1(v0)
b1(v2: boolean):
    branch v2, b1(v2), b2(v1)
b2(v3: boolean):
    branch v3, b2(v3), b3
b3:
    return
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let analysis = analyses.get::<LoopAnalysis>();

        let top_level: Vec<_> = analysis.top_level_loops().collect();
        assert_eq!(top_level.len(), 2);
        assert!(top_level.iter().all(|lp| lp.depth == 0));
        assert!(top_level.iter().all(|lp| lp.parent.is_none()));
    }

    /// Child loops iterator returns direct children only.
    #[test]
    fn test_iterate_child_loops() {
        let test = TestProgram::new(
            r#"
function parentChild(v0: boolean, v1: boolean): void {
b0(v0: boolean, v1: boolean):
    jump b1(v0, v1)
b1(v2: boolean, v3: boolean):
    branch v2, b2(v3), b3
b2(v4: boolean):
    branch v4, b2(v4), b1(v2, v4)
b3:
    return
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let analysis = analyses.get::<LoopAnalysis>();

        let block1 = function.blocks[1];
        let outer_index = analysis.loop_index(block1).unwrap();

        let children: Vec<_> = analysis.child_loops(outer_index).collect();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].1.depth, 1);
    }
}
