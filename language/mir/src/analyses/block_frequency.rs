use std::collections::{HashMap, HashSet};

use super::{
    Analysis, AnalysisId, Edge, FunctionAnalyses, FunctionAnalysis, LoopAnalysis, Mutation,
    terminator_targets,
};
use crate::{Block, Function, LocalNodeId, Profile, Terminator, Tree};

/// Largest loop scale, bounding the geometric series for near-certain back edges.
const MAX_LOOP_SCALE: f64 = 4096.0;

/// Relative execution frequency of each block and edge, with the entry block at 1.0.
///
/// Frequencies follow LLVM's `BlockFrequencyInfo` loop-nest mass model: branch
/// weights give per-edge probabilities, mass flows from the entry, and each loop
/// scales by its back-edge geometric series. The model is empty when no branch
/// weights are present, so consumers see no profile signal until apply-profile runs.
#[derive(Debug, Clone)]
pub struct BlockFrequency {
    /// Frequency of each block relative to the function entry.
    blocks: HashMap<LocalNodeId<Block>, f64>,
    /// Frequency of each control-flow edge relative to the function entry.
    edges: HashMap<Edge, f64>,
}

impl BlockFrequency {
    /// Return one block's frequency relative to the entry, or zero when unknown.
    pub fn block(&self, block: LocalNodeId<Block>) -> f64 {
        self.blocks.get(&block).copied().unwrap_or(0.0)
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
    pub fn block_counts(&self, entry_count: u64) -> HashMap<LocalNodeId<Block>, u64> {
        self.blocks
            .iter()
            .filter_map(|(&block, &frequency)| {
                let count = (entry_count as f64 * frequency).round() as u64;
                (count > 0).then_some((block, count))
            })
            .collect()
    }
}

impl Analysis for BlockFrequency {
    const ID: AnalysisId = AnalysisId("blockfreq");
    const INVALIDATED_BY: Mutation = Mutation::CONTROL_FLOW;
}

impl FunctionAnalysis for BlockFrequency {
    fn compute(function: &Function, tree: &Tree, analyses: &FunctionAnalyses) -> Self {
        let loops = analyses.get::<LoopAnalysis>(function, tree);
        Frequencies::new(function, tree, &loops).run()
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
    local: HashMap<LocalNodeId<Block>, f64>,
    /// Loop scale, the back-edge geometric series sum.
    scale: f64,
    /// Per-iteration exit distribution leaving the loop, summing to one.
    exits: Vec<(Edge, f64)>,
}

/// Working state for one function's block-frequency computation.
struct Frequencies<'a> {
    /// The function being analyzed.
    function: &'a Function,
    /// The tree the function belongs to.
    tree: &'a Tree,
    /// Loop nesting forest for the function.
    loops: &'a LoopAnalysis,
    /// Innermost containing loop index for each block.
    innermost: HashMap<LocalNodeId<Block>, Option<usize>>,
    /// Successor probability for each control-flow edge.
    probabilities: HashMap<Edge, f64>,
    /// Whether any branch weight is present.
    weighted: bool,
}

impl<'a> Frequencies<'a> {
    /// Seed the computation with branch probabilities and loop membership.
    fn new(function: &'a Function, tree: &'a Tree, loops: &'a LoopAnalysis) -> Self {
        let (probabilities, weighted) = branch_probabilities(function, tree);

        // record the innermost loop containing each block
        let mut innermost = HashMap::new();
        for &block in &function.blocks {
            let index = loops
                .innermost_loop(block)
                .and_then(|containing| loops.loop_index(containing.header));
            innermost.insert(block, index);
        }

        Self {
            function,
            tree,
            loops,
            innermost,
            probabilities,
            weighted,
        }
    }

    /// Compute block and edge frequencies for the function.
    fn run(&self) -> BlockFrequency {
        let empty = BlockFrequency {
            blocks: HashMap::new(),
            edges: HashMap::new(),
        };

        // without branch weights there is no profile signal to propagate
        let Some(entry) = self.function.entry else {
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
            let header = self.loops.get_loop(index).map(|l| l.header);
            let Some(header) = header else { continue };
            let (local, backedge, exits) = self.distribute(Some(index), header, &masses);
            let scale = loop_scale(backedge);
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
            let Some(this) = self.loops.get_loop(index) else {
                continue;
            };
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
        let mut blocks = HashMap::new();
        for &block in &self.function.blocks {
            let frequency = match self.innermost[&block] {
                None => top_local.get(&block).copied().unwrap_or(0.0),
                Some(index) => {
                    entry_freq[index]
                        * masses[index].local.get(&block).copied().unwrap_or(0.0)
                        * masses[index].scale
                }
            };
            if frequency != 0.0 {
                blocks.insert(block, frequency);
            }
        }

        // edge frequency is the source frequency split by branch probability
        let mut edges = HashMap::new();
        for &block in &self.function.blocks {
            let source_frequency = blocks.get(&block).copied().unwrap_or(0.0);
            if source_frequency == 0.0 {
                continue;
            }
            let terminator = self.tree.get(self.tree.get(block).terminator);
            for (edge, _) in terminator_targets(block, terminator) {
                let probability = self.probabilities.get(&edge).copied().unwrap_or(0.0);
                let frequency = source_frequency * probability;
                if frequency != 0.0 {
                    edges.insert(edge, frequency);
                }
            }
        }

        BlockFrequency { blocks, edges }
    }

    /// Distribute one header entry's mass across one nesting level.
    fn distribute(
        &self,
        level: Option<usize>,
        entry: LocalNodeId<Block>,
        masses: &[LoopMass],
    ) -> (HashMap<LocalNodeId<Block>, f64>, f64, Vec<(Edge, f64)>) {
        let order = self.level_order(level, entry, masses);

        let mut local = HashMap::new();
        local.insert(entry, 1.0);
        let mut backedge = 0.0;
        let mut exits: HashMap<Edge, f64> = HashMap::new();

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

        let mut visited = HashSet::from([entry]);
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
        if self.innermost[&node] == level {
            let terminator = self.tree.get(self.tree.get(node).terminator);
            return terminator_targets(node, terminator)
                .into_iter()
                .map(|(edge, _)| {
                    let probability = self.probabilities.get(&edge).copied().unwrap_or(0.0);
                    (edge, probability, self.resolve(edge.target, level))
                })
                .collect();
        }

        // a packaged child loop distributes by its per-iteration exit distribution
        let child = self.innermost[&node].expect("level node lacks a containing loop");
        masses[child]
            .exits
            .iter()
            .map(|&(edge, fraction)| (edge, fraction, self.resolve(edge.target, level)))
            .collect()
    }

    /// Resolve a block to the node that represents it at one nesting level.
    fn resolve(&self, block: LocalNodeId<Block>, level: Option<usize>) -> LevelTarget {
        let inner = self.innermost.get(&block).copied().flatten();

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
        let mut current = inner.expect("nested block lacks a containing loop");
        loop {
            let parent = self.loops.get_loop(current).and_then(|l| l.parent);
            if parent == level {
                break;
            }
            current = parent.expect("walked past the forest root without reaching the level");
        }
        LevelTarget::Node(
            self.loops
                .get_loop(current)
                .expect("missing child loop")
                .header,
        )
    }

    /// Return whether one loop contains a block.
    fn contains(&self, index: usize, block: LocalNodeId<Block>) -> bool {
        let mut current = self.innermost.get(&block).copied().flatten();
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
        self.loops.get_loop(index).map(|l| l.depth).unwrap_or(0)
    }
}

/// Successor probabilities for every edge, and whether any branch weight is present.
fn branch_probabilities(function: &Function, tree: &Tree) -> (HashMap<Edge, f64>, bool) {
    let mut probabilities = HashMap::new();
    let mut weighted = false;

    for &block in &function.blocks {
        let terminator = tree.get(tree.get(block).terminator);

        // a known weight on any successor marks the function as profiled
        let targets = terminator_targets(block, terminator);
        if targets
            .iter()
            .any(|(_, target)| target.weight.known().is_some())
        {
            weighted = true;
        }

        probabilities.extend(successor_probabilities(block, terminator));
    }

    (probabilities, weighted)
}

/// Relative successor probabilities for one terminator's edges.
///
/// Probabilities follow the successors' known branch weights, or split evenly
/// when none are known. Reading directly from the terminator keeps this correct
/// for blocks a pass has just built, whose cloned terminators carry the weights.
pub fn successor_probabilities(
    source: LocalNodeId<Block>,
    terminator: &Terminator,
) -> Vec<(Edge, f64)> {
    let targets = terminator_targets(source, terminator);
    if targets.is_empty() {
        return Vec::new();
    }

    // sum the known weights to normalize against
    let weights: Vec<Option<u32>> = targets
        .iter()
        .map(|(_, target)| target.weight.known())
        .collect();
    let total: u64 = weights.iter().flatten().map(|&weight| weight as u64).sum();
    let count = targets.len() as f64;

    // each edge takes its weighted share, or an even split when unweighted
    targets
        .iter()
        .enumerate()
        .map(|(index, (edge, _))| {
            let probability = if total > 0 {
                weights[index].map_or(0.0, |weight| weight as f64 / total as f64)
            } else {
                1.0 / count
            };
            (*edge, probability)
        })
        .collect()
}

/// Absolute block execution counts from a function's profile entry count.
///
/// Bridges loaded profile data to the frequency model: the entry count scales
/// the relative block frequencies into absolute counts, or none without a profile.
pub fn profile_block_counts(
    function: &Function,
    tree: &Tree,
    profile: Option<&Profile>,
    analyses: &FunctionAnalyses,
) -> HashMap<LocalNodeId<Block>, u64> {
    let Some(profile) = profile else {
        return HashMap::new();
    };

    let entry_count = profile
        .function(function.symbol)
        .map_or(0, |function_profile| function_profile.entry.get());
    analyses
        .get::<BlockFrequency>(function, tree)
        .block_counts(entry_count)
}

/// Edge execution counts, splitting each block count by its branch probabilities.
///
/// This is the absolute form of the `edge = block * probability` identity, valid
/// on a working count map a pass has mutated and on terminators it has just built.
pub fn edge_counts(
    function: &Function,
    tree: &Tree,
    block_counts: &HashMap<LocalNodeId<Block>, u64>,
) -> HashMap<Edge, u64> {
    let mut counts = HashMap::new();

    for &block in &function.blocks {
        // split this block's count across its successors
        let source_count = block_counts.get(&block).copied().unwrap_or(0);
        if source_count == 0 {
            continue;
        }

        let terminator = tree.get(tree.get(block).terminator);
        for (edge, probability) in successor_probabilities(block, terminator) {
            let count = (source_count as f64 * probability).round() as u64;
            if count > 0 {
                counts.insert(edge, count);
            }
        }
    }

    counts
}

/// The loop scale: the geometric series sum for one loop's back-edge mass.
fn loop_scale(backedge_mass: f64) -> f64 {
    if backedge_mass <= 0.0 {
        return 1.0;
    }
    if backedge_mass >= 1.0 - 1.0 / MAX_LOOP_SCALE {
        return MAX_LOOP_SCALE;
    }
    1.0 / (1.0 - backedge_mass)
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
    use crate::analyses::tests::parse_test_function;
    use crate::{EdgeWeight, Terminator};

    /// Set the then and else branch weights on one block's terminator.
    fn set_branch_weights(
        tree: &mut Tree,
        block: LocalNodeId<Block>,
        then_weight: u32,
        else_weight: u32,
    ) {
        let terminator_id = tree.get(block).terminator;
        let Terminator::Branch {
            then_target,
            else_target,
            ..
        } = tree.get_mut(terminator_id)
        else {
            panic!("expected a branch terminator");
        };
        then_target.weight = EdgeWeight::Known(then_weight);
        else_target.weight = EdgeWeight::Known(else_weight);
    }

    /// Compute block frequencies for a function with a fresh analysis cache.
    fn frequencies(tree: &Tree, function_id: LocalNodeId<Function>) -> BlockFrequency {
        let function = tree.get(function_id);
        let analyses = FunctionAnalyses::new();
        BlockFrequency::compute(function, tree, &analyses)
    }

    #[test]
    fn test_no_weights_is_empty() {
        let (tree, function_id) = parse_test_function(
            r#"
function diamond(v0: boolean): void {
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

        assert!(frequencies(&tree, function_id).is_empty());
    }

    #[test]
    fn test_diamond_splits_by_weight() {
        let (mut tree, function_id) = parse_test_function(
            r#"
function diamond(v0: boolean): void {
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

        let function = tree.get(function_id);
        let blocks = function.blocks.clone();
        set_branch_weights(&mut tree, blocks[0], 3, 1);

        let frequency = frequencies(&tree, function_id);
        assert!((frequency.block(blocks[0]) - 1.0).abs() < 1e-9);
        assert!((frequency.block(blocks[1]) - 0.75).abs() < 1e-9);
        assert!((frequency.block(blocks[2]) - 0.25).abs() < 1e-9);
        assert!((frequency.block(blocks[3]) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_loop_scales_by_back_edge() {
        let (mut tree, function_id) = parse_test_function(
            r#"
function counted(v0: boolean): void {
b0:
    jump b1
b1(v1: boolean):
    branch v1, b2, b3
b2:
    jump b1(v1)
b3:
    return
}"#,
        );

        let function = tree.get(function_id);
        let blocks = function.blocks.clone();
        // take the back edge nine times out of ten
        set_branch_weights(&mut tree, blocks[1], 9, 1);

        let frequency = frequencies(&tree, function_id);
        assert!((frequency.block(blocks[0]) - 1.0).abs() < 1e-9);
        assert!((frequency.block(blocks[1]) - 10.0).abs() < 1e-6);
        assert!((frequency.block(blocks[2]) - 9.0).abs() < 1e-6);
        assert!((frequency.block(blocks[3]) - 1.0).abs() < 1e-9);

        // the back edge carries the body frequency, the exit carries one entry
        let header = blocks[1];
        let terminator = tree.get(tree.get(header).terminator);
        for (edge, _) in terminator_targets(header, terminator) {
            if edge.target == blocks[2] {
                assert!((frequency.edge(edge) - 9.0).abs() < 1e-6);
            } else if edge.target == blocks[3] {
                assert!((frequency.edge(edge) - 1.0).abs() < 1e-6);
            }
        }
    }
}
