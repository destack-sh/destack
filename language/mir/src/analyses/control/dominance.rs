use super::control::ControlGraph;

/// Dominator links and traversal intervals indexed by compact graph node.
#[derive(Debug, Clone)]
pub(super) struct DominatorTree {
    /// Tree nodes, with zero intervals for nodes unreachable from the root.
    pub(super) nodes: Vec<DominatorNode>,
}

/// Tree links and interval numbers for one graph node.
#[derive(Debug, Clone, Default)]
pub(super) struct DominatorNode {
    /// Immediate dominator, absent for the root and unreachable nodes.
    pub(super) parent: Option<u32>,
    /// First child in graph reverse postorder.
    pub(super) child: Option<u32>,
    /// Next child of the same parent.
    pub(super) sibling: Option<u32>,

    /// Entry number, or zero for an unreachable node.
    pub(super) entry: u32,
    /// Exit number, or zero for an unreachable node.
    pub(super) exit: u32,
}

/// Lengauer–Tarjan state indexed by one-based compact node indices.
#[derive(Debug)]
struct DominatorBuilder {
    /// DFS number of each node, refined to its semidominator's DFS number.
    semidominator: Vec<u32>,
    /// Node at each DFS number.
    preorder: Vec<u32>,
    /// Node with the smallest semidominator on an ancestor path.
    representative: Vec<u32>,
    /// DFS parent of each node.
    parent: Vec<u32>,

    /// Linked ancestor of each node; zero ends an ancestor path.
    ancestor: Vec<u32>,
    /// First node deferred at each semidominator; zero ends a bucket.
    bucket_head: Vec<u32>,
    /// Next node in each deferred bucket.
    bucket_next: Vec<u32>,

    /// Reusable stack for iterative path compression.
    path: Vec<u32>,
}

impl DominatorTree {
    /// Build a dominator tree with children ordered by graph reverse postorder.
    pub(super) fn build(root: Option<u32>, postorder: &[u32], graph: &ControlGraph) -> Self {
        // allocate unvisited nodes for a graph without an entry
        let mut nodes = vec![DominatorNode::default(); graph.len()];
        let Some(root) = root else {
            return Self { nodes };
        };

        // assign immediate dominators in compact graph indices
        let parents = DominatorBuilder::build(root, graph);
        for (node, parent) in nodes.iter_mut().zip(parents) {
            node.parent = parent;
        }

        // prepend children in postorder to visit them in reverse postorder
        for &block in postorder {
            if let Some(parent) = nodes[block as usize].parent {
                nodes[block as usize].sibling = nodes[parent as usize].child;
                nodes[parent as usize].child = Some(block);
            }
        }

        // number intervals with an iterative tree walk
        let mut tree = Self { nodes };
        tree.number(root);

        tree
    }

    /// Return whether one reachable node dominates another reachable node.
    pub(super) fn dominates(&self, dominator: u32, node: u32) -> bool {
        let dominator = &self.nodes[dominator as usize];
        let node = &self.nodes[node as usize];

        dominator.entry != 0 && dominator.entry <= node.entry && node.entry < dominator.exit
    }

    /// Number tree intervals by following child, sibling, and parent links.
    fn number(&mut self, root: u32) {
        // start reachable intervals above the unvisited number
        let mut block = root;
        let mut next = 1;

        // open each block before visiting its children
        loop {
            self.nodes[block as usize].entry = next;
            next += 1;

            // descend to the first child before closing this interval
            if let Some(child) = self.nodes[block as usize].child {
                block = child;
                continue;
            }

            // finish ancestors until another sibling can be visited
            loop {
                let node = &mut self.nodes[block as usize];
                node.exit = next;
                next += 1;

                // visit the next child of the same parent
                if let Some(sibling) = node.sibling {
                    block = sibling;
                    break;
                }
                // ascend after the final child
                else if let Some(parent) = node.parent {
                    block = parent;
                }
                // finish after closing the root interval
                else {
                    return;
                }
            }
        }
    }
}

impl DominatorBuilder {
    /// Compute immediate dominators with the Lengauer–Tarjan algorithm.
    fn build(root: u32, graph: &ControlGraph) -> Vec<Option<u32>> {
        // reserve index zero to terminate ancestor paths and bucket lists
        let length = graph.len() + 1;
        let mut builder = Self {
            semidominator: vec![0; length],
            preorder: vec![0; length],
            representative: vec![0; length],
            parent: vec![0; length],
            ancestor: vec![0; length],
            bucket_head: vec![0; length],
            bucket_next: vec![0; length],
            path: Vec::new(),
        };
        let mut immediate = vec![0; length];
        let count = builder.number(root, graph);

        // compute semidominators in reverse DFS order
        for number in (2..=count).rev() {
            let node = builder.preorder[number as usize];

            // ignore predecessors that the DFS did not visit
            for &predecessor in graph.predecessors(node - 1) {
                let predecessor = predecessor + 1;
                if builder.semidominator[predecessor as usize] == 0 {
                    continue;
                }

                // select the lowest semidominator on this predecessor's ancestor path
                let representative = builder.compress_path(predecessor);
                builder.semidominator[node as usize] = builder.semidominator[node as usize]
                    .min(builder.semidominator[representative as usize]);
            }

            // defer the node at its semidominator and link it to its DFS parent
            let number = builder.semidominator[node as usize] as usize;
            let semidominator = builder.preorder[number];
            builder.bucket_next[node as usize] = builder.bucket_head[semidominator as usize];
            builder.bucket_head[semidominator as usize] = node;
            let parent = builder.parent[node as usize];
            builder.ancestor[node as usize] = parent;

            // drain the parent's bucket before resolving its deferred nodes
            let mut deferred = builder.bucket_head[parent as usize];
            builder.bucket_head[parent as usize] = 0;
            while deferred != 0 {
                let representative = builder.compress_path(deferred);
                let representative_semidominator = builder.semidominator[representative as usize];
                let deferred_semidominator = builder.semidominator[deferred as usize];

                // select the representative when its semidominator precedes this node's
                immediate[deferred as usize] =
                    if representative_semidominator < deferred_semidominator {
                        representative
                    } else {
                        parent
                    };
                deferred = builder.bucket_next[deferred as usize];
            }
        }

        // correct provisional dominators in forward DFS order
        for number in 2..=count {
            let node = builder.preorder[number as usize] as usize;
            let semidominator = builder.semidominator[node] as usize;
            let parent = immediate[node] as usize;
            if immediate[node] != builder.preorder[semidominator] {
                immediate[node] = immediate[parent];
            }
        }

        // translate back to zero-based graph indices
        immediate
            .into_iter()
            .skip(1)
            .map(|parent| parent.checked_sub(1))
            .collect()
    }

    /// Number nodes and record parents in depth first order.
    fn number(&mut self, root: u32, graph: &ControlGraph) -> u32 {
        // start with the one-based root index
        let mut stack = vec![root + 1];
        let mut count = 0;

        // visit pending nodes in depth first order
        while let Some(node) = stack.pop() {
            if self.semidominator[node as usize] != 0 {
                continue;
            }

            // assign the next DFS number on the first visit
            count += 1;
            self.semidominator[node as usize] = count;
            self.preorder[count as usize] = node;
            self.representative[node as usize] = node;

            // replace a pending successor's parent until that successor is visited
            for &successor in graph.successors(node - 1) {
                let successor = successor + 1;
                if self.semidominator[successor as usize] == 0 {
                    stack.push(successor);
                    self.parent[successor as usize] = node;
                }
            }
        }

        count
    }

    /// Compress an ancestor path and return its representative with the earliest semidominator.
    fn compress_path(&mut self, node: u32) -> u32 {
        // return an unlinked node directly
        if self.ancestor[node as usize] == 0 {
            return node;
        }

        // collect ancestors until the next ancestor is unlinked
        let mut current = node;
        let mut ancestor = self.ancestor[current as usize];
        while self.ancestor[ancestor as usize] != 0 {
            self.path.push(current);
            current = ancestor;
            ancestor = self.ancestor[current as usize];
        }

        // update representatives while shortening the path from its highest ancestor
        while let Some(current) = self.path.pop() {
            let ancestor = self.ancestor[current as usize];
            let representative = self.representative[ancestor as usize];
            let current_representative = self.representative[current as usize];

            // retain the representative with the earlier semidominator
            if self.semidominator[representative as usize]
                < self.semidominator[current_representative as usize]
            {
                self.representative[current as usize] = representative;
            }

            // skip the ancestor after comparing its representative
            self.ancestor[current as usize] = self.ancestor[ancestor as usize];
        }

        self.representative[node as usize]
    }
}
