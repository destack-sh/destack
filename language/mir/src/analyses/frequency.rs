use destack_core::{FxIndexMap, FxIndexSet};

use super::{Analysis, ExecutionFrequencyOptions, FunctionCache, LoopTable, Mutation};
use crate::{
    Block, Edge, Function, FunctionProfile, LocalNodeId, NodeTable, Profile, Terminator, Tree,
};

/// Relative execution frequencies for blocks and edges.
#[derive(Debug, Clone)]
pub struct FrequencyTable {
    /// Frequency of each block relative to the function entry.
    blocks: NodeTable<Block, f64>,
    /// Frequency of each control-flow edge relative to the function entry.
    edges: FxIndexMap<Edge, f64>,
}

impl FrequencyTable {
    /// Compute execution frequency with an explicit function profile.
    pub fn compute_profiled(
        function: &Function,
        tree: &Tree,
        analyses: &mut FunctionCache,
        profile: Option<&FunctionProfile>,
    ) -> Self {
        let loops = analyses.loops(function, tree);

        let options = analyses.options().execution_frequency;

        Frequencies::new(function, tree, &loops, profile, options).run()
    }

    /// Return one block's frequency relative to the entry, or zero when unknown.
    pub fn block(&self, block: LocalNodeId<Block>) -> f64 {
        if self.blocks.is_empty() {
            0.0
        } else {
            *self.blocks.get(block)
        }
    }

    /// Return one edge's frequency relative to the entry, or zero when unknown.
    pub fn edge(&self, edge: Edge) -> f64 {
        self.edges.get(&edge).copied().unwrap_or(0.0)
    }

    /// Return true when no branch weights drove this frequency model.
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Absolute block execution counts, scaling each frequency by the entry count.
    pub fn block_counts(&self, entry_count: u64) -> FxIndexMap<LocalNodeId<Block>, u64> {
        self.blocks
            .iter_nodes()
            .filter_map(|(block, frequency)| {
                let count = (entry_count as f64 * frequency).round() as u64;
                (count > 0).then_some((block, count))
            })
            .collect()
    }

    /// Return relative successor probabilities for one terminator's edges.
    pub fn successor_edge_probabilities(
        tree: &Tree,
        source: LocalNodeId<Block>,
        terminator: &Terminator,
        profile: Option<&FunctionProfile>,
    ) -> Vec<(Edge, f64)> {
        let targets = terminator.targets(tree, source);
        if targets.is_empty() {
            return Vec::new();
        }

        // sum profiled edge counts to normalize against
        let counts = targets
            .iter()
            .map(|(edge, _)| profile.map_or(0, |profile| profile.edge(*edge)))
            .collect::<Vec<_>>();
        let total: u64 = counts.iter().copied().sum();
        let count = targets.len() as f64;

        // each edge takes its profiled share, or an even split when unweighted
        targets
            .iter()
            .enumerate()
            .map(|(index, (edge, _))| {
                let probability = if total > 0 {
                    counts[index] as f64 / total as f64
                } else {
                    1.0 / count
                };
                (*edge, probability)
            })
            .collect()
    }
}

/// Absolute execution counts derived from a profile.
#[derive(Debug, Clone, Default)]
pub struct ExecutionCounts {
    /// Block execution counts.
    blocks: FxIndexMap<LocalNodeId<Block>, u64>,
    /// Edge execution counts.
    edges: FxIndexMap<Edge, u64>,
}

impl ExecutionCounts {
    /// Build absolute execution counts for one function.
    pub fn new(
        function: &Function,
        tree: &Tree,
        profile: Option<&Profile>,
        analyses: &mut FunctionCache,
    ) -> Self {
        let Some(profile) = profile else {
            return Self::default();
        };

        let Some(function_profile) = profile.function(function.symbol) else {
            return Self::default();
        };

        let frequency =
            FrequencyTable::compute_profiled(function, tree, analyses, Some(function_profile));
        let blocks = frequency.block_counts(function_profile.entry.get());
        let edges = Self::edge_counts(function, tree, Some(function_profile), &blocks);

        Self { blocks, edges }
    }

    /// Return block execution counts.
    pub fn blocks(&self) -> &FxIndexMap<LocalNodeId<Block>, u64> {
        &self.blocks
    }

    /// Return edge execution counts.
    pub fn edges(&self) -> &FxIndexMap<Edge, u64> {
        &self.edges
    }

    /// Return one block's execution count.
    pub fn block(&self, block: LocalNodeId<Block>) -> u64 {
        self.blocks.get(&block).copied().unwrap_or(0)
    }

    /// Return one edge's execution count.
    pub fn edge(&self, edge: Edge) -> u64 {
        self.edges.get(&edge).copied().unwrap_or(0)
    }

    /// Return true when no profile counts are available.
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty() && self.edges.is_empty()
    }

    /// Return absolute edge execution counts from block counts.
    fn edge_counts(
        function: &Function,
        tree: &Tree,
        profile: Option<&FunctionProfile>,
        block_counts: &FxIndexMap<LocalNodeId<Block>, u64>,
    ) -> FxIndexMap<Edge, u64> {
        let mut counts = FxIndexMap::default();

        for &block in function.blocks() {
            // split this block's count across its successors
            let source_count = block_counts.get(&block).copied().unwrap_or(0);
            if source_count == 0 {
                continue;
            }

            let terminator = tree.get(tree.get(block).terminator);
            for (edge, probability) in
                FrequencyTable::successor_edge_probabilities(tree, block, terminator, profile)
            {
                let count = (source_count as f64 * probability).round() as u64;
                if count > 0 {
                    counts.insert(edge, count);
                }
            }
        }

        counts
    }
}

impl Analysis for FrequencyTable {
    const INVALIDATED_BY: Mutation = Mutation::CONTROL;
}

impl FrequencyTable {
    /// Compute execution frequencies for one function.
    pub(crate) fn compute(function: &Function, tree: &Tree, analyses: &mut FunctionCache) -> Self {
        Self::compute_profiled(function, tree, analyses, None)
    }
}

/// A control-flow node resolved at one loop nesting level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LevelTarget {
    /// A block or packaged child loop at this level.
    Node(LocalNodeId<Block>),
    /// A target outside the current loop.
    Exit,
}

/// Mass distribution computed for one loop, relative to its header at 1.0.
#[derive(Debug, Clone, Default)]
struct LoopMass {
    /// Local mass reaching each member per header entry.
    local: FxIndexMap<LocalNodeId<Block>, f64>,
    /// Loop scale, the back-edge geometric series sum.
    scale: f64,
    /// Per-iteration exit distribution leaving the loop, summing to one.
    exits: Vec<(Edge, f64)>,
}

/// Working state for one function's frequency analysis.
struct Frequencies<'a> {
    /// The function being analyzed.
    function: &'a Function,
    /// The tree the function belongs to.
    tree: &'a Tree,
    /// Loop nesting forest for the function.
    loops: &'a LoopTable,
    /// Innermost containing loop index for each block.
    innermost: NodeTable<Block, Option<usize>>,
    /// Successor probability for each control-flow edge.
    probabilities: FxIndexMap<Edge, f64>,
    /// Whether any branch weight is present.
    weighted: bool,
    /// Options for execution frequency analysis.
    options: ExecutionFrequencyOptions,
}

impl<'a> Frequencies<'a> {
    /// Seed branch probabilities and loop membership.
    fn new(
        function: &'a Function,
        tree: &'a Tree,
        loops: &'a LoopTable,
        profile: Option<&'a FunctionProfile>,
        options: ExecutionFrequencyOptions,
    ) -> Self {
        let (probabilities, weighted) = Self::branch_probabilities(function, tree, profile);

        // record the innermost loop containing each block
        let mut innermost = NodeTable::from_nodes(function.blocks(), || None);
        for &block in function.blocks() {
            let index = loops
                .innermost_loop(block)
                .and_then(|containing| loops.loop_index(containing.header));
            *innermost.get_mut(block) = index;
        }

        Self {
            function,
            tree,
            loops,
            innermost,
            probabilities,
            weighted,
            options,
        }
    }

    /// Compute block and edge frequencies for the function.
    fn run(&self) -> FrequencyTable {
        let empty = FrequencyTable {
            blocks: NodeTable::new(),
            edges: FxIndexMap::default(),
        };

        // skip unweighted functions
        let Some(entry) = self.function.entry() else {
            return empty;
        };
        if !self.weighted {
            return empty;
        }

        // distribute mass within each loop, innermost first, packaging children
        let loop_count = self.loops.num_loops();
        let mut masses = vec![LoopMass::default(); loop_count];
        let mut inner_first: Vec<usize> = (0..loop_count).collect();
        inner_first.sort_by_key(|&index| std::cmp::Reverse(self.loop_depth(index)));
        for &index in &inner_first {
            let header = self
                .loops
                .get_loop(index)
                .unwrap_or_else(|| panic!("missing loop for frequency index: {index}"))
                .header;
            let (local, backedge, exits) = self.distribute(Some(index), header, &masses);
            let scale = Self::loop_scale(backedge, self.options.max_loop_scale);
            masses[index] = LoopMass {
                local,
                scale,
                exits: exits
                    .into_iter()
                    .map(|(edge, mass)| (edge, mass * scale))
                    .collect(),
            };
        }

        // distribute mass across the top level, with packaged loops as single nodes
        let top_entry = self.resolve(entry, None).node().unwrap_or(entry);
        let (top_local, _, _) = self.distribute(None, top_entry, &masses);

        // frequency of each loop header as a packaged node, outermost first
        let mut entry_freq = vec![0.0; loop_count];
        let mut outer_first: Vec<usize> = (0..loop_count).collect();
        outer_first.sort_by_key(|&index| self.loop_depth(index));
        for &index in &outer_first {
            let this = self
                .loops
                .get_loop(index)
                .unwrap_or_else(|| panic!("missing loop for frequency index: {index}"));
            entry_freq[index] = match this.parent {
                None => top_local.get(&this.header).copied().unwrap_or(0.0),
                Some(parent) => {
                    entry_freq[parent]
                        * masses[parent]
                            .local
                            .get(&this.header)
                            .copied()
                            .unwrap_or(0.0)
                        * masses[parent].scale
                }
            };
        }

        // assemble global block frequencies from the loop nest
        let mut blocks = NodeTable::from_nodes(self.function.blocks(), || 0.0);
        for &block in self.function.blocks() {
            let frequency = match *self.innermost.get(block) {
                None => top_local.get(&block).copied().unwrap_or(0.0),
                Some(index) => {
                    entry_freq[index]
                        * masses[index].local.get(&block).copied().unwrap_or(0.0)
                        * masses[index].scale
                }
            };
            *blocks.get_mut(block) = frequency;
        }

        // edge frequency is the source frequency split by branch probability
        let mut edges = FxIndexMap::default();
        for &block in self.function.blocks() {
            let source_frequency = *blocks.get(block);
            if source_frequency == 0.0 {
                continue;
            }
            let terminator = self.tree.get(self.tree.get(block).terminator);
            for (edge, _) in terminator.targets(self.tree, block) {
                let probability = self.probabilities.get(&edge).copied().unwrap_or(0.0);
                let frequency = source_frequency * probability;
                if frequency != 0.0 {
                    edges.insert(edge, frequency);
                }
            }
        }

        FrequencyTable { blocks, edges }
    }

    /// Distribute one header entry's mass across one nesting level.
    fn distribute(
        &self,
        level: Option<usize>,
        entry: LocalNodeId<Block>,
        masses: &[LoopMass],
    ) -> (FxIndexMap<LocalNodeId<Block>, f64>, f64, Vec<(Edge, f64)>) {
        let order = self.level_order(level, entry, masses);

        let mut local = FxIndexMap::default();
        local.insert(entry, 1.0);
        let mut backedge = 0.0;
        let mut exits: FxIndexMap<Edge, f64> = FxIndexMap::default();

        // forward order guarantees every predecessor is settled before its target
        for &node in &order {
            let mass = local.get(&node).copied().unwrap_or(0.0);
            if mass == 0.0 {
                continue;
            }

            for (edge, probability, target) in self.out_edges(node, level, masses) {
                let flow = mass * probability;
                match target {
                    LevelTarget::Exit => *exits.entry(edge).or_insert(0.0) += flow,
                    LevelTarget::Node(node) if node == entry => backedge += flow,
                    LevelTarget::Node(node) => *local.entry(node).or_insert(0.0) += flow,
                }
            }
        }

        (local, backedge, exits.into_iter().collect())
    }

    /// Order one level's nodes so predecessors precede targets, ignoring back edges.
    fn level_order(
        &self,
        level: Option<usize>,
        entry: LocalNodeId<Block>,
        masses: &[LoopMass],
    ) -> Vec<LocalNodeId<Block>> {
        let successors = |node: LocalNodeId<Block>| -> Vec<LocalNodeId<Block>> {
            self.out_edges(node, level, masses)
                .into_iter()
                .filter_map(|(_, _, target)| match target {
                    LevelTarget::Node(next) if next != entry => Some(next),
                    _ => None,
                })
                .collect()
        };

        let mut visited = FxIndexSet::from_iter([entry]);
        let mut stack = vec![(entry, successors(entry), 0usize)];
        let mut postorder = Vec::new();

        // iterative depth-first postorder over the acyclic level graph
        while let Some((node, children, index)) = stack.last_mut() {
            if *index < children.len() {
                let next = children[*index];
                *index += 1;
                if visited.insert(next) {
                    let next_children = successors(next);
                    stack.push((next, next_children, 0));
                }
            } else {
                postorder.push(*node);
                stack.pop();
            }
        }

        postorder.reverse();
        postorder
    }

    /// Enumerate one node's outgoing edges with probability and resolved target.
    fn out_edges(
        &self,
        node: LocalNodeId<Block>,
        level: Option<usize>,
        masses: &[LoopMass],
    ) -> Vec<(Edge, f64, LevelTarget)> {
        // a plain block at this level distributes by its terminator probabilities
        if *self.innermost.get(node) == level {
            let terminator = self.tree.get(self.tree.get(node).terminator);
            return terminator
                .targets(self.tree, node)
                .into_iter()
                .map(|(edge, _)| {
                    let probability = self.probabilities.get(&edge).copied().unwrap_or(0.0);
                    (edge, probability, self.resolve(edge.target, level))
                })
                .collect();
        }

        // a packaged child loop distributes by its per-iteration exit distribution
        let child = match *self.innermost.get(node) {
            Some(child) => child,
            None => unreachable!("level node lacks a containing loop"),
        };
        masses[child]
            .exits
            .iter()
            .map(|&(edge, fraction)| (edge, fraction, self.resolve(edge.target, level)))
            .collect()
    }

    /// Resolve a block to the node that represents it at one nesting level.
    fn resolve(&self, block: LocalNodeId<Block>, level: Option<usize>) -> LevelTarget {
        let inner = *self.innermost.get(block);

        // a target outside the current loop leaves this level
        if let Some(level) = level
            && !self.contains(level, block)
        {
            return LevelTarget::Exit;
        }

        // a block whose innermost loop is this level appears directly
        if inner == level {
            return LevelTarget::Node(block);
        }

        // otherwise the block sits inside a child loop, represented by its header
        let mut current = match inner {
            Some(current) => current,
            None => unreachable!("nested block lacks a containing loop"),
        };
        loop {
            let parent = self.loops.get_loop(current).and_then(|l| l.parent);
            if parent == level {
                break;
            }
            current = match parent {
                Some(parent) => parent,
                None => unreachable!("walked past the forest root without reaching the level"),
            };
        }
        LevelTarget::Node(
            match self.loops.get_loop(current) {
                Some(child) => child,
                None => unreachable!("missing child loop"),
            }
            .header,
        )
    }

    /// Return whether one loop contains a block.
    fn contains(&self, index: usize, block: LocalNodeId<Block>) -> bool {
        let mut current = *self.innermost.get(block);
        while let Some(loop_index) = current {
            if loop_index == index {
                return true;
            }
            current = self.loops.get_loop(loop_index).and_then(|l| l.parent);
        }
        false
    }

    /// Return one loop's nesting depth.
    fn loop_depth(&self, index: usize) -> u32 {
        self.loops
            .get_loop(index)
            .unwrap_or_else(|| panic!("missing loop for frequency index: {index}"))
            .depth
    }

    /// Return successor probabilities for every edge and whether any profile edge count is present.
    fn branch_probabilities(
        function: &Function,
        tree: &Tree,
        profile: Option<&FunctionProfile>,
    ) -> (FxIndexMap<Edge, f64>, bool) {
        let mut probabilities = FxIndexMap::default();
        let mut weighted = false;

        for &block in function.blocks() {
            let terminator = tree.get(tree.get(block).terminator);

            // a profiled edge marks the function as weighted
            let targets = terminator.targets(tree, block);
            if targets
                .iter()
                .any(|(edge, _)| profile.is_some_and(|profile| profile.edge(*edge) > 0))
            {
                weighted = true;
            }

            probabilities.extend(FrequencyTable::successor_edge_probabilities(
                tree, block, terminator, profile,
            ));
        }

        (probabilities, weighted)
    }

    /// Return the geometric series sum for one loop's back-edge mass.
    fn loop_scale(backedge_mass: f64, maximum: f64) -> f64 {
        if backedge_mass <= 0.0 {
            return 1.0;
        }
        if backedge_mass >= 1.0 - 1.0 / maximum {
            return maximum;
        }

        1.0 / (1.0 - backedge_mass)
    }
}

impl LevelTarget {
    /// Return the node when this target stays at the current level.
    fn node(self) -> Option<LocalNodeId<Block>> {
        match self {
            LevelTarget::Node(block) => Some(block),
            LevelTarget::Exit => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestProgram;
    use crate::{Count, FunctionHash, FunctionProfile, SamplerId, Successor, ValueProfile};

    /// Build a function profile with branch edge counts.
    fn branch_profile(
        tree: &Tree,
        block: LocalNodeId<Block>,
        then_count: u64,
        else_count: u64,
    ) -> FunctionProfile {
        let terminator = tree.get(tree.get(block).terminator);
        let mut edges = FxIndexMap::default();

        // attach counts to the structural branch successors
        for (edge, _) in terminator.targets(tree, block) {
            match edge.successor {
                Successor::BranchThen => {
                    edges.insert(edge, Count::new(then_count));
                }
                Successor::BranchElse => {
                    edges.insert(edge, Count::new(else_count));
                }
                _ => {}
            }
        }

        FunctionProfile {
            hash: FunctionHash(0),
            entry: Count::new(1),
            edges,
            counts: Vec::new(),
            values: FxIndexMap::<SamplerId, ValueProfile>::default(),
        }
    }

    /// Compute block frequencies for a function with fresh analyses.
    fn frequencies(
        tree: &Tree,
        function_id: LocalNodeId<Function>,
        profile: Option<&FunctionProfile>,
    ) -> FrequencyTable {
        let function = tree.get(function_id);
        let mut analyses = FunctionCache::new();

        FrequencyTable::compute_profiled(function, tree, &mut analyses, profile)
    }

    #[test]
    fn test_no_weights_is_empty() {
        let (tree, function_id) = TestProgram::parse_function(
            r#"
function diamond(v0: boolean): void {
b0(v0: boolean):
    branch v0 => b1 | b2
b1:
    jump b3
b2:
    jump b3
b3:
    return
}"#,
        );

        assert!(frequencies(&tree, function_id, None).is_empty());
    }

    #[test]
    fn test_diamond_splits_by_weight() {
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
        let blocks = function.blocks().to_vec();
        let profile = branch_profile(&tree, blocks[0], 3, 1);

        let frequency = frequencies(&tree, function_id, Some(&profile));
        assert!((frequency.block(blocks[0]) - 1.0).abs() < 1e-9);
        assert!((frequency.block(blocks[1]) - 0.75).abs() < 1e-9);
        assert!((frequency.block(blocks[2]) - 0.25).abs() < 1e-9);
        assert!((frequency.block(blocks[3]) - 1.0).abs() < 1e-9);
    }

    /// Absolute execution counts scale normalized frequencies by entry count.
    #[test]
    fn test_execution_counts_scale_frequency_by_entry_count() {
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
        let blocks = function.blocks().to_vec();
        let mut function_profile = branch_profile(&tree, blocks[0], 3, 1);
        function_profile.entry = Count::new(100);

        let profile = Profile {
            functions: FxIndexMap::from_iter([(function.symbol, function_profile)]),
            globals: FxIndexMap::default(),
        };
        let mut analyses = FunctionCache::new();
        let counts = ExecutionCounts::new(function, &tree, Some(&profile), &mut analyses);

        assert_eq!(counts.block(blocks[0]), 100);
        assert_eq!(counts.block(blocks[1]), 75);
        assert_eq!(counts.block(blocks[2]), 25);
        assert_eq!(counts.block(blocks[3]), 100);

        // edge counts scale from source counts and profiled branch probabilities
        let terminator = tree.get(tree.get(blocks[0]).terminator);
        for (edge, _) in terminator.targets(&tree, blocks[0]) {
            if edge.target == blocks[1] {
                assert_eq!(counts.edge(edge), 75);
            } else if edge.target == blocks[2] {
                assert_eq!(counts.edge(edge), 25);
            }
        }
    }

    #[test]
    fn test_loop_scales_by_back_edge() {
        let (tree, function_id) = TestProgram::parse_function(
            r#"
function counted(v0: boolean): void {
entry:
    jump b1

b1(v1: boolean):
    branch v1 => b2 | b3

b2:
    jump b1(v1)

b3:
    return
}
"#,
        );

        let function = tree.get(function_id);
        let blocks = function.blocks().to_vec();
        let profile = branch_profile(&tree, blocks[1], 9, 1);

        let frequency = frequencies(&tree, function_id, Some(&profile));
        assert!((frequency.block(blocks[0]) - 1.0).abs() < 1e-9);
        assert!((frequency.block(blocks[1]) - 10.0).abs() < 1e-6);
        assert!((frequency.block(blocks[2]) - 9.0).abs() < 1e-6);
        assert!((frequency.block(blocks[3]) - 1.0).abs() < 1e-9);

        // the back edge carries the body frequency, the exit carries one entry
        let header = blocks[1];
        let terminator = tree.get(tree.get(header).terminator);
        for (edge, _) in terminator.targets(&tree, header) {
            if edge.target == blocks[2] {
                assert!((frequency.edge(edge) - 9.0).abs() < 1e-6);
            } else if edge.target == blocks[3] {
                assert!((frequency.edge(edge) - 1.0).abs() < 1e-6);
            }
        }
    }
}
