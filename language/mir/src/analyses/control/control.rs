use crate::{Analysis, Block, BlockTarget, Edge, Function, LocalNodeId, Mutation, NodeTable, Tree};

/// Control flow and entry reachability for one function.
#[derive(Debug, Clone)]
pub struct ControlTable {
    /// Function blocks in their original order.
    blocks: Vec<LocalNodeId<Block>>,
    /// Compact index for each function block.
    indices: NodeTable<Block, u32>,
    /// Successor edges and distinct predecessor blocks in compact indices.
    pub(super) graph: ControlGraph,
    /// Entry block index, absent for declarations.
    pub(super) entry: Option<u32>,
    /// Reachable block indices in depth first postorder.
    pub(super) postorder: Vec<u32>,
    /// Entry reachability indexed by compact block index.
    reachable: Vec<bool>,
}

/// Directed graph with compact node indices and contiguous adjacency lists.
#[derive(Debug, Clone)]
pub(super) struct ControlGraph {
    /// First successor offset for each node and the final edge count.
    successor_offsets: Vec<u32>,
    /// Successors in edge order, including repeated targets.
    successors: Vec<u32>,
    /// First predecessor offset for each node and the final predecessor count.
    predecessor_offsets: Vec<u32>,
    /// Distinct predecessor nodes in node order.
    predecessors: Vec<u32>,
    /// First incoming edge offset for each node and the final edge count.
    incoming_offsets: Vec<u32>,
    /// Source node and successor position for each incoming edge.
    incoming: Vec<(u32, u32)>,
}

/// Node and next successor in an unfinished depth first visit.
#[derive(Debug)]
struct DepthFirstFrame {
    /// Node being visited.
    node: u32,
    /// Next successor position within the node's adjacency list.
    successor: usize,
}

impl ControlTable {
    /// Analyse a function's terminators to record control flow and traversal order.
    pub fn analyse(function: &Function, tree: &Tree) -> Self {
        // map sparse MIR ids to compact function indices
        let blocks = function.blocks().to_vec();
        let entries = blocks
            .iter()
            .enumerate()
            .map(|(index, &block)| (block, index as u32));
        let indices = NodeTable::from_entries(entries.collect());
        let entry = function.entry().map(|block| *indices.get(block));

        // preserve each successor occurrence in terminator order
        let mut offsets = Vec::with_capacity(blocks.len() + 1);
        let mut successors = Vec::new();
        offsets.push(0);

        // append each terminator's successor positions
        for &block in &blocks {
            let block = tree.get(block);
            let terminator = tree.get(block.terminator);
            let targets = terminator.successors(tree);
            successors.extend(targets.iter().map(|&block| *indices.get(block)));
            offsets.push(successors.len() as u32);
        }

        // traverse the graph once to record order and reachability
        let graph = ControlGraph::new(offsets, successors);
        let postorder = match entry {
            Some(entry) => graph.postorder(entry),
            None => Vec::new(),
        };

        // mark the blocks visited by the entry traversal
        let mut reachable = vec![false; blocks.len()];
        for &block in &postorder {
            reachable[block as usize] = true;
        }

        Self {
            blocks,
            indices,
            graph,
            entry,
            postorder,
            reachable,
        }
    }

    /// Return distinct predecessor blocks in function order.
    pub fn predecessors(
        &self,
        block: LocalNodeId<Block>,
    ) -> impl ExactSizeIterator<Item = LocalNodeId<Block>> + DoubleEndedIterator + '_ {
        // translate the compact predecessor list to MIR ids
        let index = self.index(block);

        self.graph
            .predecessors(index)
            .iter()
            .map(|&index| self.block(index))
    }

    /// Return successor blocks in terminator order, including repeated targets.
    pub fn successors(
        &self,
        block: LocalNodeId<Block>,
    ) -> impl ExactSizeIterator<Item = LocalNodeId<Block>> + DoubleEndedIterator + '_ {
        // translate the compact successor list to MIR ids
        let index = self.index(block);

        self.graph
            .successors(index)
            .iter()
            .map(|&index| self.block(index))
    }

    /// Return each incoming edge with its arguments, preserving repeated source and target pairs.
    pub fn incoming_edges<'a>(
        &'a self,
        block: LocalNodeId<Block>,
        tree: &'a Tree,
    ) -> impl ExactSizeIterator<Item = (Edge, &'a BlockTarget)> + DoubleEndedIterator + 'a {
        // resolve each cached position against the current terminator arguments
        let index = self.index(block);

        self.graph
            .incoming(index)
            .iter()
            .map(move |&(source, successor)| {
                // locate the source terminator
                let source = self.block(source);
                let block = tree.get(source);
                let terminator = tree.get(block.terminator);

                terminator
                    .target_at(successor as usize, source, tree)
                    .unwrap_or_else(|| {
                        unreachable!("cached control edge is absent from its terminator")
                    })
            })
    }

    /// Return whether a block is reachable from the function entry.
    pub fn is_reachable(&self, block: LocalNodeId<Block>) -> bool {
        let index = self.index(block) as usize;

        self.reachable[index]
    }

    /// Iterate over reachable blocks in function order.
    pub fn reachable_blocks(&self) -> impl Iterator<Item = LocalNodeId<Block>> + '_ {
        // retain reachable blocks in function order
        self.blocks
            .iter()
            .copied()
            .zip(&self.reachable)
            .filter_map(|(block, &reachable)| reachable.then_some(block))
    }

    /// Iterate over reachable blocks in depth first postorder.
    pub fn postorder(
        &self,
    ) -> impl ExactSizeIterator<Item = LocalNodeId<Block>> + DoubleEndedIterator + '_ {
        self.postorder.iter().map(|&index| self.block(index))
    }

    /// Iterate over reachable blocks in reverse postorder.
    pub fn reverse_postorder(
        &self,
    ) -> impl ExactSizeIterator<Item = LocalNodeId<Block>> + DoubleEndedIterator + '_ {
        self.postorder().rev()
    }

    /// Translate a MIR block id to its compact index.
    pub(super) fn index(&self, block: LocalNodeId<Block>) -> u32 {
        *self.indices.get(block)
    }

    /// Translate a compact index to its MIR block id.
    pub(super) fn block(&self, index: u32) -> LocalNodeId<Block> {
        self.blocks[index as usize]
    }
}

impl ControlGraph {
    /// Build predecessor and incoming-edge lists from valid successor offsets and node indices.
    pub(super) fn new(successor_offsets: Vec<u32>, successors: Vec<u32>) -> Self {
        // allocate predecessor and incoming-edge counts
        let count = successor_offsets.len() - 1;
        let mut predecessor_offsets = vec![0; count + 1];
        let mut incoming_offsets = vec![0; count + 1];
        let mut previous = vec![None; count];

        // count every edge and each distinct source per target
        for source in 0..count {
            let start = successor_offsets[source] as usize;
            let end = successor_offsets[source + 1] as usize;

            // count each occurrence and record its source once per target
            for &target in &successors[start..end] {
                incoming_offsets[target as usize + 1] += 1;
                if previous[target as usize] != Some(source as u32) {
                    predecessor_offsets[target as usize + 1] += 1;
                    previous[target as usize] = Some(source as u32);
                }
            }
        }

        // convert counts into contiguous predecessor ranges
        for node in 0..count {
            predecessor_offsets[node + 1] += predecessor_offsets[node];
            incoming_offsets[node + 1] += incoming_offsets[node];
        }

        // allocate contiguous edge lists and reset the write positions
        let mut predecessors = vec![0; predecessor_offsets[count] as usize];
        let mut cursors = predecessor_offsets[..count].to_vec();
        let mut incoming = vec![(0, 0); successors.len()];
        let mut incoming_cursors = incoming_offsets[..count].to_vec();
        previous.fill(None);

        // fill each target's list in source order
        for source in 0..count {
            let start = successor_offsets[source] as usize;
            let end = successor_offsets[source + 1] as usize;

            // assign each edge to its target's contiguous range
            for (successor, &target) in successors[start..end].iter().enumerate() {
                // retain every edge occurrence for direct argument lookup
                incoming[incoming_cursors[target as usize] as usize] =
                    (source as u32, successor as u32);
                incoming_cursors[target as usize] += 1;

                // retain each predecessor block once
                if previous[target as usize] != Some(source as u32) {
                    predecessors[cursors[target as usize] as usize] = source as u32;
                    cursors[target as usize] += 1;
                    previous[target as usize] = Some(source as u32);
                }
            }
        }

        Self {
            successor_offsets,
            successors,
            predecessor_offsets,
            predecessors,
            incoming_offsets,
            incoming,
        }
    }

    /// Return the number of nodes.
    pub(super) fn len(&self) -> usize {
        self.successor_offsets.len() - 1
    }

    /// Return successor occurrences for one node.
    pub(super) fn successors(&self, node: u32) -> &[u32] {
        let start = self.successor_offsets[node as usize] as usize;
        let end = self.successor_offsets[node as usize + 1] as usize;

        &self.successors[start..end]
    }

    /// Return distinct predecessor nodes.
    pub(super) fn predecessors(&self, node: u32) -> &[u32] {
        let start = self.predecessor_offsets[node as usize] as usize;
        let end = self.predecessor_offsets[node as usize + 1] as usize;

        &self.predecessors[start..end]
    }

    /// Return the source node and successor position of each incoming edge.
    fn incoming(&self, node: u32) -> &[(u32, u32)] {
        let start = self.incoming_offsets[node as usize] as usize;
        let end = self.incoming_offsets[node as usize + 1] as usize;

        &self.incoming[start..end]
    }

    /// Compute reachable postorder with an iterative depth first traversal.
    pub(super) fn postorder(&self, entry: u32) -> Vec<u32> {
        // seed the explicit DFS stack at the entry
        let mut seen = vec![false; self.len()];
        let mut order = Vec::with_capacity(self.len());
        let mut stack = vec![DepthFirstFrame {
            node: entry,
            successor: 0,
        }];
        seen[entry as usize] = true;

        // finish each successor before recording its source
        while let Some(frame) = stack.last_mut() {
            let node = frame.node;
            let successors = self.successors(node);

            // advance the source before descending to an unvisited successor
            if let Some(&successor) = successors.get(frame.successor) {
                frame.successor += 1;
                if !seen[successor as usize] {
                    seen[successor as usize] = true;
                    stack.push(DepthFirstFrame {
                        node: successor,
                        successor: 0,
                    });
                }
            }
            // record the node after visiting all its successors
            else {
                stack.pop();
                order.push(node);
            }
        }

        order
    }
}

impl Analysis for ControlTable {
    const INVALIDATED_BY: Mutation = Mutation::CONTROL;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestModule;
    use crate::{Successor, Terminator, Value};

    /// Collect the predecessor of each block in a chain.
    #[test]
    fn test_find_predecessors_in_chain() {
        let (tree, function_id) = TestModule::parse_function(
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
        let cfg = ControlTable::analyse(function, &tree);

        let block0 = function.entry().expect("missing entry");
        let block1 = function.block(1);
        let block2 = function.block(2);

        assert_eq!(cfg.predecessors(block0).collect::<Vec<_>>(), vec![]);
        assert_eq!(cfg.predecessors(block1).collect::<Vec<_>>(), vec![block0]);
        assert_eq!(cfg.predecessors(block2).collect::<Vec<_>>(), vec![block1]);
    }

    /// Collect the common predecessor of both branch targets.
    #[test]
    fn test_find_branch_predecessors() {
        let (tree, function_id) = TestModule::parse_function(
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
        let cfg = ControlTable::analyse(function, &tree);

        let block0 = function.block(0);
        let block1 = function.block(1);
        let block2 = function.block(2);

        assert_eq!(cfg.predecessors(block1).collect::<Vec<_>>(), vec![block0]);
        assert_eq!(cfg.predecessors(block2).collect::<Vec<_>>(), vec![block0]);
    }

    /// Collect both predecessors of a diamond join.
    #[test]
    fn test_find_join_predecessors() {
        let (tree, function_id) = TestModule::parse_function(
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
        let cfg = ControlTable::analyse(function, &tree);

        let block3 = function.block(3);

        assert_eq!(
            cfg.predecessors(block3).collect::<Vec<_>>(),
            vec![function.block(1), function.block(2)]
        );
    }

    /// Include the header itself as a predecessor of a self-loop.
    #[test]
    fn test_find_loop_predecessors() {
        let (tree, function_id) = TestModule::parse_function(
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
        let cfg = ControlTable::analyse(function, &tree);

        let block1 = function.block(1);

        assert_eq!(
            cfg.predecessors(block1).collect::<Vec<_>>(),
            vec![function.block(0), block1]
        );
    }

    /// Exclude disconnected blocks from entry reachability.
    #[test]
    fn test_exclude_disconnected_blocks() {
        let (tree, function_id) = TestModule::parse_function(
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
        let cfg = ControlTable::analyse(function, &tree);

        let reachable_block = function.block(1);
        let unreachable_block = function.block(3);

        assert!(cfg.is_reachable(reachable_block));
        assert!(!cfg.is_reachable(unreachable_block));
    }

    /// Retain both branch edges and their distinct arguments for a shared target.
    #[test]
    fn test_preserve_duplicate_branch_edges() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: boolean, v1: int32, v2: int32): int32 {
entry(v0: boolean, v1: int32, v2: int32):
    branch v0 => join(v1) | join(v2)

join(v3: int32):
    return v3
}
"#,
        );
        let function = tree.get(function_id);
        let [entry, join]: [_; 2] = function.blocks().try_into().unwrap();
        let control = ControlTable::analyse(function, &tree);

        assert_eq!(control.predecessors(join).collect::<Vec<_>>(), [entry]);
        assert_eq!(control.successors(entry).collect::<Vec<_>>(), [join, join]);
        let incoming: Vec<_> = control
            .incoming_edges(join, &tree)
            .map(|(edge, target)| (edge, target.arguments(&tree).to_vec()))
            .collect();
        let expected = [
            (
                Edge::new(entry, Successor::BranchThen, join),
                vec![Value::new(1)],
            ),
            (
                Edge::new(entry, Successor::BranchElse, join),
                vec![Value::new(2)],
            ),
        ];
        assert_eq!(incoming, expected);
    }

    /// Read changed arguments through the existing cached edge position.
    #[test]
    fn test_read_updated_edge_arguments() {
        let (mut tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    jump done(v0)

done(v2: int32):
    return v2
}
"#,
        );
        let function = tree.get(function_id);
        let [entry, done]: [_; 2] = function.blocks().try_into().unwrap();
        let control = ControlTable::analyse(function, &tree);

        // replace the argument after caching the graph
        let arguments = tree.add_values(&[Value::new(1)]);
        let terminator = tree.get(entry).terminator;
        let Terminator::Jump { target } = tree.get_mut(terminator) else {
            panic!("expected entry jump");
        };
        target.arguments = arguments;

        let incoming: Vec<_> = control
            .incoming_edges(done, &tree)
            .map(|(edge, target)| (edge, target.arguments(&tree).to_vec()))
            .collect();
        assert_eq!(
            incoming,
            [(Edge::new(entry, Successor::Jump, done), vec![Value::new(1)])]
        );
    }

    /// Visit each reachable block once when the function entry is last in block order.
    #[test]
    fn test_traverse_reordered_blocks() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: boolean): void {
entry(v0: boolean):
    branch v0 => left | right

left:
    jump join

right:
    jump join

join:
    return

dead:
    jump join
}
"#,
        );
        let mut function = tree.get(function_id).clone();
        let [entry, left, right, join, dead]: [_; 5] = function.blocks().try_into().unwrap();
        function.replace_blocks(vec![dead, join, right, left, entry], &tree);
        let control = ControlTable::analyse(&function, &tree);

        assert_eq!(
            control.predecessors(join).collect::<Vec<_>>(),
            [dead, right, left]
        );
        assert_eq!(
            control.reachable_blocks().collect::<Vec<_>>(),
            [join, right, left, entry]
        );
        assert_eq!(
            control.postorder().collect::<Vec<_>>(),
            [join, left, right, entry]
        );
        assert_eq!(
            control.reverse_postorder().collect::<Vec<_>>(),
            [entry, right, left, join]
        );
    }

    /// Resolve repeated switch targets by their default and case positions.
    #[test]
    fn test_preserve_switch_edges() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: int32, v1: int32, v2: int32): int32 {
entry(v0: int32, v1: int32, v2: int32):
    switch v0, join(v1), 0 => join(v2), 1 => done(v2)

join(v3: int32):
    return v3

done(v4: int32):
    return v4

dead:
    jump join(v1)
}
"#,
        );
        let function = tree.get(function_id);
        let [entry, join, done, dead]: [_; 4] = function.blocks().try_into().unwrap();
        let control = ControlTable::analyse(function, &tree);

        let incoming: Vec<_> = control
            .incoming_edges(join, &tree)
            .map(|(edge, target)| (edge, target.arguments(&tree).to_vec()))
            .collect();
        let expected = [
            (
                Edge::new(entry, Successor::SwitchDefault, join),
                vec![Value::new(1)],
            ),
            (
                Edge::new(entry, Successor::SwitchCase { value: 0 }, join),
                vec![Value::new(2)],
            ),
            (Edge::new(dead, Successor::Jump, join), vec![Value::new(1)]),
        ];
        assert_eq!(incoming, expected);
        let incoming: Vec<_> = control
            .incoming_edges(done, &tree)
            .map(|(edge, target)| (edge, target.arguments(&tree).to_vec()))
            .collect();
        assert_eq!(
            incoming,
            [(
                Edge::new(entry, Successor::SwitchCase { value: 1 }, done),
                vec![Value::new(2)]
            ),]
        );
    }

    /// Return empty traversals for a function declaration.
    #[test]
    fn test_traverse_declaration() {
        let (tree, function_id) = TestModule::parse_function("external function test(): void");
        let function = tree.get(function_id);
        let control = ControlTable::analyse(function, &tree);

        assert_eq!(control.postorder().collect::<Vec<_>>(), []);
        assert_eq!(control.reachable_blocks().collect::<Vec<_>>(), []);
        assert_eq!(control.graph.len(), 0);
    }

    /// Resolve variant case positions with and without a default target.
    #[test]
    fn test_preserve_variant_switch_edges() {
        let (tree, function_id) = TestModule::parse_function(
            r#"
type Choice = variant<uint1> { 0uint1 = void; 1uint1 = void; }

function test(v0: Choice): void {
entry(v0: Choice):
    variant.switch v0, 0 => first, 1 => second

first:
    variant.switch v0, 0 => done, else other

second:
    return

done:
    return

other:
    return
}
"#,
        );
        let function = tree.get(function_id);
        let [entry, first, second, done, other]: [_; 5] = function.blocks().try_into().unwrap();
        let blocks = [entry, first, second, done, other];
        let control = ControlTable::analyse(function, &tree);

        // collect the exact incoming edges of every block
        let actual = blocks.map(|block| {
            let edges = control
                .incoming_edges(block, &tree)
                .map(|(edge, _)| edge)
                .collect::<Vec<_>>();

            (block, edges)
        });
        let expected = [
            (entry, vec![]),
            (
                first,
                vec![Edge::new(entry, Successor::SwitchCase { value: 0 }, first)],
            ),
            (
                second,
                vec![Edge::new(entry, Successor::SwitchCase { value: 1 }, second)],
            ),
            (
                done,
                vec![Edge::new(first, Successor::SwitchCase { value: 0 }, done)],
            ),
            (
                other,
                vec![Edge::new(first, Successor::SwitchDefault, other)],
            ),
        ];
        assert_eq!(actual, expected);
    }
}
