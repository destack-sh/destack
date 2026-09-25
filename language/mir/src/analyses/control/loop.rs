use tspp_core::{DenseGraph, FxIndexSet};

use crate as mir;

use crate::{Analysis, Mutation, NodeTable};

use super::{ControlTable, DominatorTable};

/// Natural loops for one function.
#[derive(Debug)]
pub struct LoopTable {
    /// All loops, indexed by loop ID.
    loops: Vec<Loop>,
    /// Loop index for each header block.
    header_to_loop: NodeTable<mir::Block, Option<usize>>,
    /// Innermost loop index for each block.
    block_to_loop: NodeTable<mir::Block, Option<usize>>,
    /// Whether each block belongs to a cycle after removing natural backedges.
    irreducible: NodeTable<mir::Block, bool>,
}

/// One natural loop in a MIR function.
#[derive(Debug, Clone)]
pub struct Loop {
    /// The loop header.
    pub header: mir::BlockId,
    /// The back edge sources.
    pub latches: Vec<mir::BlockId>,
    /// All blocks in the loop body, including the header.
    pub blocks: FxIndexSet<mir::BlockId>,

    /// The blocks with outgoing loop exits.
    pub exiting_blocks: Vec<mir::BlockId>,
    /// The blocks reached by loop exits.
    pub exit_blocks: Vec<mir::BlockId>,

    /// Parent loop index, if this is a nested loop.
    pub parent: Option<usize>,
    /// Nesting depth (0 for outermost loops).
    pub depth: u32,
}

impl Loop {
    /// Return whether the loop has one latch.
    pub fn has_single_latch(&self) -> bool {
        self.latches.len() == 1
    }

    /// Return whether the loop has one exit destination.
    pub fn has_single_exit(&self) -> bool {
        self.exit_blocks.len() == 1
    }

    /// Return whether a block is inside this loop.
    pub fn contains(&self, block: mir::BlockId) -> bool {
        self.blocks.contains(&block)
    }
}

impl LoopTable {
    /// Analyse natural loops from reachable backedges and dominators.
    pub fn analyse(
        function: &mir::Function,
        graph: &ControlTable,
        dominators: &DominatorTable,
    ) -> Self {
        // return empty tables for imported functions
        if function.entry().is_none() {
            return Self {
                loops: Vec::new(),
                header_to_loop: NodeTable::new(),
                block_to_loop: NodeTable::new(),
                irreducible: NodeTable::new(),
            };
        }

        // build natural loops from each header's backedges
        let (mut loops, header_to_loop) = Self::build_loops(function, graph, dominators);

        // set parent relationships and nesting depths
        Self::build_nesting(&mut loops, &header_to_loop, dominators);

        // map each block to its innermost containing loop
        let block_to_loop = Self::build_block_map(function.blocks(), &loops);
        let irreducible = Self::find_irreducible(function, graph, dominators);

        Self {
            loops,
            header_to_loop,
            block_to_loop,
            irreducible,
        }
    }

    /// Identify cycles that remain after removing natural loop backedges.
    fn find_irreducible(
        function: &mir::Function,
        graph: &ControlTable,
        dominators: &DominatorTable,
    ) -> NodeTable<mir::Block, bool> {
        let blocks = function.blocks();
        let indices = NodeTable::from_entries(
            blocks
                .iter()
                .enumerate()
                .map(|(index, &block)| (block, index as u32))
                .collect(),
        );
        let mut offsets = vec![0];
        let mut targets = Vec::new();

        // retain reachable edges that do not return to a dominating header
        for &block in blocks {
            if graph.is_reachable(block) {
                for successor in graph.successors(block) {
                    if !dominators.dominates(successor, block) {
                        targets.push(*indices.get(successor));
                    }
                }
            }
            offsets.push(targets.len() as u32);
        }

        // mark the remaining cyclic components
        let remaining = DenseGraph::new(&offsets, &targets);
        let components = remaining.strongly_connected_components();
        let mut sizes = vec![0; components.component_count() as usize];
        for &component in components.components() {
            sizes[component as usize] += 1;
        }
        let entries = blocks
            .iter()
            .enumerate()
            .map(|(index, &block)| (block, sizes[components.component(index) as usize] > 1));

        NodeTable::from_entries(entries.collect())
    }

    /// Return whether a block belongs to a cycle that has no dominating loop header.
    pub fn is_irreducible(&self, block: mir::BlockId) -> bool {
        *self.irreducible.get(block)
    }

    /// Collect natural loops and index them by header.
    fn build_loops(
        function: &mir::Function,
        graph: &ControlTable,
        dominators: &DominatorTable,
    ) -> (Vec<Loop>, NodeTable<mir::Block, Option<usize>>) {
        // collect the latches of each reachable header
        let mut loops = Vec::new();
        for header in graph.reachable_blocks() {
            let latches = graph
                .predecessors(header)
                .filter(|&predecessor| dominators.dominates(header, predecessor))
                .collect::<Vec<_>>();
            if latches.is_empty() {
                continue;
            }

            // collect the body and exits of this natural loop
            let blocks = Self::collect_blocks(header, &latches, graph, dominators);
            let (exiting_blocks, exit_blocks) = Self::collect_exits(&blocks, graph);
            loops.push(Loop {
                header,
                latches,
                blocks,
                exiting_blocks,
                exit_blocks,
                parent: None,
                depth: 0,
            });
        }

        // preserve header order independently of control flow traversal
        loops.sort_unstable_by_key(|natural_loop| natural_loop.header);
        let mut header_to_loop = NodeTable::from_nodes(function.blocks(), || None);
        for (index, natural_loop) in loops.iter().enumerate() {
            *header_to_loop.get_mut(natural_loop.header) = Some(index);
        }

        (loops, header_to_loop)
    }

    /// Collect a loop body from its latches.
    fn collect_blocks(
        header: mir::BlockId,
        latches: &[mir::BlockId],
        graph: &ControlTable,
        dominators: &DominatorTable,
    ) -> FxIndexSet<mir::BlockId> {
        // start with the header and its distinct latches
        let mut body = FxIndexSet::from_iter([header]);
        let mut worklist = Vec::new();
        for &latch in latches {
            if body.insert(latch) {
                worklist.push(latch);
            }
        }

        // collect dominated predecessors until the walk returns to the header
        while let Some(block) = worklist.pop() {
            for predecessor in graph.predecessors(block) {
                if dominators.dominates(header, predecessor) && body.insert(predecessor) {
                    worklist.push(predecessor);
                }
            }
        }

        body
    }

    /// Find exiting blocks and exit blocks for a loop.
    fn collect_exits(
        body: &FxIndexSet<mir::BlockId>,
        graph: &ControlTable,
    ) -> (Vec<mir::BlockId>, Vec<mir::BlockId>) {
        let mut exiting_blocks = Vec::new();
        let mut exit_blocks_set = FxIndexSet::default();

        for &block_id in body {
            let mut is_exiting = false;

            for successor in graph.successors(block_id) {
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

    /// Set parent relationships and nesting depths.
    fn build_nesting(
        loops: &mut [Loop],
        header_to_loop: &NodeTable<mir::Block, Option<usize>>,
        dominators: &DominatorTable,
    ) {
        let loop_count = loops.len();

        // find each loop's parent through the dominator tree
        for i in 0..loop_count {
            let header = loops[i].header;

            // walk up immediate dominators to find containing loop
            let mut current = dominators.immediate_dominator(header);
            while let Some(block) = current {
                if let Some(parent_index) = *header_to_loop.get(block) {
                    // verify the header is actually in the parent's body
                    if loops[parent_index].blocks.contains(&header) {
                        loops[i].parent = Some(parent_index);
                        break;
                    }
                }
                current = dominators.immediate_dominator(block);
            }
        }

        // count each loop's ancestors
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
    fn build_block_map(
        blocks: &[mir::BlockId],
        loops: &[Loop],
    ) -> NodeTable<mir::Block, Option<usize>> {
        let mut block_to_loop = NodeTable::from_nodes(blocks, || None::<usize>);

        // retain the deepest containing loop for each block
        for (index, natural_loop) in loops.iter().enumerate() {
            for &block in &natural_loop.blocks {
                let current = block_to_loop.get_mut(block);
                if current.is_none_or(|current| loops[current].depth < natural_loop.depth) {
                    *current = Some(index);
                }
            }
        }

        block_to_loop
    }

    /// Return every loop.
    pub fn loops(&self) -> &[Loop] {
        &self.loops
    }

    /// Return the number of loops.
    pub fn len(&self) -> usize {
        self.loops.len()
    }

    /// Return whether the function has no natural loops.
    pub fn is_empty(&self) -> bool {
        self.loops.is_empty()
    }

    /// Return whether a block is a loop header.
    pub fn is_loop_header(&self, block: mir::BlockId) -> bool {
        self.header_to_loop.get(block).is_some()
    }

    /// Return the loop with one header.
    pub fn header_loop(&self, header: mir::BlockId) -> Option<&Loop> {
        let index = (*self.header_to_loop.get(header))?;

        Some(&self.loops[index])
    }

    /// Return the innermost loop containing one block.
    pub fn innermost_loop(&self, block: mir::BlockId) -> Option<&Loop> {
        let index = (*self.block_to_loop.get(block))?;

        Some(&self.loops[index])
    }

    /// Return the nesting depth for one block.
    pub fn loop_depth(&self, block: mir::BlockId) -> u32 {
        if let Some(index) = *self.block_to_loop.get(block) {
            self.loops[index].depth + 1
        } else {
            0
        }
    }

    /// Return whether a block is inside any loop.
    pub fn is_in_loop(&self, block: mir::BlockId) -> bool {
        self.block_to_loop.get(block).is_some()
    }

    /// Iterate over loops at a specific nesting depth.
    pub fn loops_at_depth(&self, depth: u32) -> impl Iterator<Item = &Loop> {
        self.loops
            .iter()
            .filter(move |natural_loop| natural_loop.depth == depth)
    }

    /// Iterate over top-level (outermost) loops.
    pub fn top_level_loops(&self) -> impl Iterator<Item = &Loop> {
        self.loops_at_depth(0)
    }

    /// Return the direct children of one loop.
    pub fn child_loops(&self, loop_index: usize) -> impl Iterator<Item = (usize, &Loop)> {
        self.loops
            .iter()
            .enumerate()
            .filter(move |(_, natural_loop)| natural_loop.parent == Some(loop_index))
    }

    /// Return one loop by index.
    pub fn get(&self, index: usize) -> Option<&Loop> {
        self.loops.get(index)
    }

    /// Return the loop index for one header.
    pub fn loop_index(&self, header: mir::BlockId) -> Option<usize> {
        *self.header_to_loop.get(header)
    }
}

impl Analysis for LoopTable {
    const INVALIDATED_BY: Mutation = Mutation::CONTROL;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestModule;

    /// Self-loop (single block looping to itself) is detected.
    #[test]
    fn test_detect_self_loop() {
        let test = TestModule::new(
            r#"
function selfLoop(v0: boolean): void {
entry(v0: boolean):
    branch v0 => entry(v0) | b1

b1:
    return
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let analysis = analyses.loops(function_id, &test.tree);

        assert_eq!(analysis.len(), 1);

        let natural_loop = &analysis.loops()[0];
        assert_eq!(natural_loop.header, function.block(0));
        assert_eq!(natural_loop.latches, vec![function.block(0)]);
        assert_eq!(
            natural_loop.blocks,
            FxIndexSet::from_iter([function.block(0)])
        );
        assert_eq!(natural_loop.exiting_blocks, vec![function.block(0)]);
        assert_eq!(natural_loop.exit_blocks, vec![function.block(1)]);
        assert_eq!(natural_loop.parent, None);
        assert_eq!(natural_loop.depth, 0);
        assert!(natural_loop.has_single_latch());
    }

    /// Self-loop with an outside predecessor does not pull the predecessor into the loop body.
    #[test]
    fn test_exclude_predecessor_from_self_loop() {
        let test = TestModule::new(
            r#"
function selfLoopEntry(v0: boolean): void {
entry(v0: boolean):
    jump b1

b1:
    branch v0 => b1 | b2

b2:
    return
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let analysis = analyses.loops(function_id, &test.tree);

        assert_eq!(analysis.len(), 1);

        let block0 = function.block(0);
        let block1 = function.block(1);

        let natural_loop = analysis.header_loop(block1).unwrap();
        assert_eq!(natural_loop.blocks, FxIndexSet::from_iter([block1]));
        assert_eq!(
            analysis
                .innermost_loop(block0)
                .map(|natural_loop| natural_loop.header),
            None
        );
    }

    /// While-style loop with separate header and latch is detected.
    #[test]
    fn test_detect_while_loop() {
        let test = TestModule::new(
            r#"
function whileLoop(v0: boolean): void {
entry(v0: boolean):
    jump b1(v0)

b1(v1: boolean):
    branch v1 => b2 | b3

b2:
    jump b1(v1)

b3:
    return
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let analysis = analyses.loops(function_id, &test.tree);

        assert_eq!(analysis.len(), 1);

        let block1 = function.block(1);
        let block2 = function.block(2);

        let natural_loop = analysis.header_loop(block1).unwrap();

        // identify the header and latch
        assert_eq!(natural_loop.header, block1);
        assert_eq!(natural_loop.latches, vec![block2]);
        assert_eq!(natural_loop.blocks, FxIndexSet::from_iter([block1, block2]));

        // identify the block that exits the loop
        assert_eq!(natural_loop.exiting_blocks, vec![block1]);
        assert!(natural_loop.has_single_exit());
    }

    /// Nested loops have correct parent relationships and depths.
    #[test]
    fn test_detect_nested_loops() {
        let test = TestModule::new(
            r#"
function nested(v0: boolean, v1: boolean): void {
entry(v0: boolean, v1: boolean):
    jump b1(v0, v1)

b1(v2: boolean, v3: boolean):
    branch v2 => b2(v3) | b4

b2(v4: boolean):
    branch v4 => b3 | b1(v2, v4)

b3:
    jump b2(v4)

b4:
    return
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let analysis = analyses.loops(function_id, &test.tree);

        assert_eq!(analysis.len(), 2);

        let block1 = function.block(1);
        let block2 = function.block(2);
        let block3 = function.block(3);

        let outer = analysis.header_loop(block1).unwrap();
        let inner = analysis.header_loop(block2).unwrap();

        // outer loop
        assert_eq!(outer.parent, None);
        assert_eq!(outer.depth, 0);
        assert_eq!(
            outer.blocks,
            FxIndexSet::from_iter([block1, block2, block3])
        );

        // inner loop
        assert_eq!(inner.parent, analysis.loop_index(block1));
        assert_eq!(inner.depth, 1);
        assert_eq!(inner.blocks, FxIndexSet::from_iter([block2, block3]));
    }

    /// Loop depth query returns correct values.
    #[test]
    fn test_measure_loop_depth() {
        let test = TestModule::new(
            r#"
function depth(v0: boolean): void {
entry(v0: boolean):
    jump b1(v0)

b1(v1: boolean):
    branch v1 => b2(v1) | b3

b2(v2: boolean):
    branch v2 => b2(v2) | b1(v2)

b3:
    return
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let analysis = analyses.loops(function_id, &test.tree);

        let block0 = function.block(0);
        let block1 = function.block(1);
        let block2 = function.block(2);
        let block3 = function.block(3);

        // assign depth zero outside loops
        assert_eq!(analysis.loop_depth(block0), 0);
        assert_eq!(analysis.loop_depth(block3), 0);
        assert!(!analysis.is_in_loop(block0));
        assert!(!analysis.is_in_loop(block3));

        // count the outer loop around block1
        assert_eq!(analysis.loop_depth(block1), 1);
        assert!(analysis.is_in_loop(block1));

        // count both loops around block2
        assert_eq!(analysis.loop_depth(block2), 2);
        assert!(analysis.is_in_loop(block2));

        // identify each innermost loop and the direct nesting relationship
        assert_eq!(
            [block0, block1, block2, block3].map(|block| analysis
                .innermost_loop(block)
                .map(|natural_loop| natural_loop.header)),
            [None, Some(block1), Some(block2), None],
        );
        let outer = analysis.loop_index(block1).unwrap();
        assert_eq!(
            analysis
                .child_loops(outer)
                .map(|(_, natural_loop)| natural_loop.header)
                .collect::<Vec<_>>(),
            vec![block2]
        );
        assert_eq!(
            analysis
                .top_level_loops()
                .map(|natural_loop| natural_loop.header)
                .collect::<Vec<_>>(),
            vec![block1]
        );
    }

    /// Function without loops returns empty analysis.
    #[test]
    fn test_handle_no_loops() {
        let test = TestModule::new(
            r#"
function noLoops(v0: boolean): void {
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

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let analysis = analyses.loops(function_id, &test.tree);

        assert_eq!(analysis.len(), 0);
        assert!(analysis.loops().is_empty());

        for &block in function.blocks() {
            assert!(!analysis.is_in_loop(block));
            assert_eq!(analysis.loop_depth(block), 0);
        }
    }

    /// Multiple back edges to the same header create a single loop.
    #[test]
    fn test_merge_multiple_latches() {
        let test = TestModule::new(
            r#"
function multiLatch(v0: boolean): void {
entry(v0: boolean):
    jump b1(v0)

b1(v1: boolean):
    branch v1 => b2 | b3

b2:
    jump b1(v1)

b3:
    jump b1(v1)
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let analysis = analyses.loops(function_id, &test.tree);

        assert_eq!(analysis.len(), 1);

        let natural_loop = &analysis.loops()[0];
        assert_eq!(natural_loop.header, function.block(1));
        assert_eq!(
            natural_loop.latches,
            vec![function.block(2), function.block(3)]
        );
        assert_eq!(
            natural_loop.blocks,
            FxIndexSet::from_iter([function.block(1), function.block(2), function.block(3)])
        );
        assert!(!natural_loop.has_single_latch());
    }

    /// Exiting blocks and exit blocks are computed correctly.
    #[test]
    fn test_identify_loop_exits() {
        let test = TestModule::new(
            r#"
function exits(v0: boolean, v1: boolean): void {
entry(v0: boolean, v1: boolean):
    jump b1(v0, v1)

b1(v2: boolean, v3: boolean):
    branch v2 => b2(v3) | b4

b2(v4: boolean):
    branch v4 => b1(v2, v4) | b3

b3:
    return

b4:
    return
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let analysis = analyses.loops(function_id, &test.tree);

        assert_eq!(analysis.len(), 1);

        let natural_loop = &analysis.loops()[0];
        let block1 = function.block(1);
        let block2 = function.block(2);
        let block3 = function.block(3);
        let block4 = function.block(4);

        // identify both blocks that exit the loop
        assert_eq!(natural_loop.exiting_blocks, vec![block1, block2]);

        // identify both destinations outside the loop
        assert_eq!(natural_loop.exit_blocks, vec![block3, block4]);

        assert!(!natural_loop.has_single_exit());
    }

    /// Top-level loops iterator returns only outermost loops.
    #[test]
    fn test_iterate_top_level_loops() {
        let test = TestModule::new(
            r#"
function twoOuter(v0: boolean, v1: boolean): void {
entry(v0: boolean, v1: boolean):
    jump b1(v0)

b1(v2: boolean):
    branch v2 => b1(v2) | b2(v1)

b2(v3: boolean):
    branch v3 => b2(v3) | b3

b3:
    return
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let analysis = analyses.loops(function_id, &test.tree);

        let top_level = analysis
            .top_level_loops()
            .map(|natural_loop| (natural_loop.header, natural_loop.depth, natural_loop.parent))
            .collect::<Vec<_>>();
        assert_eq!(
            top_level,
            [(function.block(1), 0, None), (function.block(2), 0, None)]
        );
    }

    /// Count a latch once when both branches return to the loop header.
    #[test]
    fn test_deduplicate_latches() {
        let program = TestModule::new(
            r#"
function test(v0: boolean): void {
entry(v0: boolean):
    jump header

header:
    jump latch

latch:
    branch v0 => header | header
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let table = analyses.loops(program.entry_function_id(), &program.tree);
        let header = function.block(1);
        let latch = function.block(2);
        let natural_loop = table.header_loop(header).expect("loop header");

        assert_eq!(table.len(), 1);
        assert_eq!(natural_loop.latches, vec![latch]);
        assert_eq!(natural_loop.blocks, FxIndexSet::from_iter([header, latch]));
        assert_eq!(natural_loop.exiting_blocks, vec![]);
        assert_eq!(natural_loop.exit_blocks, vec![]);
        assert!(natural_loop.has_single_latch());
    }

    /// Recognize one exit destination reached from two different loop blocks.
    #[test]
    fn test_distinguish_exiting_blocks_from_exit_blocks() {
        let program = TestModule::new(
            r#"
function test(v0: boolean, v1: boolean): void {
entry(v0: boolean, v1: boolean):
    jump header

header:
    branch v0 => latch | exit

latch:
    branch v1 => header | exit

exit:
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let table = analyses.loops(program.entry_function_id(), &program.tree);
        let header = function.block(1);
        let latch = function.block(2);
        let exit = function.block(3);
        let natural_loop = table.header_loop(header).expect("loop header");

        assert_eq!(natural_loop.exiting_blocks, vec![header, latch]);
        assert_eq!(natural_loop.exit_blocks, vec![exit]);
        assert!(natural_loop.has_single_exit());
    }

    /// Exclude irreducible cycles from the natural loop forest.
    #[test]
    fn test_exclude_irreducible_cycles() {
        let program = TestModule::new(
            r#"
function test(v0: boolean): void {
entry(v0: boolean):
    branch v0 => left | right

left:
    jump right

right:
    branch v0 => left | exit

exit:
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let table = analyses.loops(program.entry_function_id(), &program.tree);

        assert!(table.loops().is_empty());
        assert_eq!(
            function
                .blocks()
                .iter()
                .map(|&block| table.is_irreducible(block))
                .collect::<Vec<_>>(),
            vec![false, true, true, false]
        );
        assert_eq!(
            function
                .blocks()
                .iter()
                .map(|&block| table.loop_depth(block))
                .collect::<Vec<_>>(),
            vec![0, 0, 0, 0]
        );
    }
}
