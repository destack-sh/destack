use crate::BitSet;

const UNVISITED_NODE: u32 = u32::MAX;
const UNASSIGNED_COMPONENT: u32 = u32::MAX;

/// A dense directed graph in compressed sparse row form.
#[derive(Debug, Clone, Copy)]
pub struct DenseGraph<'a> {
    /// Per-source start offset into `edge_targets`.
    edge_offsets: &'a [u32],
    /// Edge heads as dense node ids.
    edge_targets: &'a [u32],
}

impl<'a> DenseGraph<'a> {
    /// Create one dense graph from CSR edge storage.
    pub fn new(edge_offsets: &'a [u32], edge_targets: &'a [u32]) -> Self {
        assert!(!edge_offsets.is_empty(), "dense graph offsets are empty");

        let node_count = edge_offsets.len() - 1;
        let edge_count = edge_offsets[edge_offsets.len() - 1];
        assert_eq!(
            edge_count as usize,
            edge_targets.len(),
            "dense graph edge count mismatch",
        );
        assert!(
            edge_offsets.windows(2).all(|window| window[0] <= window[1]),
            "dense graph offsets are not sorted",
        );
        assert!(
            edge_targets
                .iter()
                .all(|target| (*target as usize) < node_count),
            "dense graph edge target out of bounds",
        );

        Self {
            edge_offsets,
            edge_targets,
        }
    }

    /// Return the number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.edge_offsets.len() - 1
    }

    /// Return the strongly connected component partition.
    pub fn strongly_connected_components(&self) -> SccPartition {
        let node_count = self.node_count();
        let mut search = SccSearch::new(self, node_count);

        search.run()
    }

    /// Return the successor slice for one dense node.
    fn successors(&self, node: usize) -> &[u32] {
        let start = self.edge_offsets[node] as usize;
        let end = self.edge_offsets[node + 1] as usize;

        &self.edge_targets[start..end]
    }
}

/// Strongly connected components of one dense graph.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SccPartition {
    /// The component id of each node.
    component: Vec<u32>,
    /// The number of components in the partition.
    component_count: u32,
}

impl SccPartition {
    /// Return the component id of one node.
    pub fn component(&self, node: usize) -> u32 {
        self.component[node]
    }

    /// Return every node's component id.
    pub fn components(&self) -> &[u32] {
        &self.component
    }

    /// Return the number of components.
    pub fn component_count(&self) -> u32 {
        self.component_count
    }
}

/// Iterative Tarjan state for dense graph SCC discovery.
struct SccSearch<'a> {
    /// The graph being partitioned.
    graph: &'a DenseGraph<'a>,
    /// DFS order for each node.
    order: Vec<u32>,
    /// Tarjan lowlink for each node.
    lowlink: Vec<u32>,
    /// Component id for each node.
    component: Vec<u32>,
    /// Membership set for nodes currently on the stack.
    on_stack: BitSet,
    /// Open Tarjan stack.
    stack: Vec<u32>,
    /// Explicit DFS frames.
    frames: Vec<SccFrame>,
    /// The next DFS order id.
    next_order: u32,
    /// The next component id.
    next_component: u32,
}

impl<'a> SccSearch<'a> {
    /// Create one SCC search.
    fn new(graph: &'a DenseGraph<'a>, node_count: usize) -> Self {
        Self {
            graph,
            order: vec![UNVISITED_NODE; node_count],
            lowlink: vec![0; node_count],
            component: vec![UNASSIGNED_COMPONENT; node_count],
            on_stack: BitSet::new(node_count),
            stack: Vec::new(),
            frames: Vec::new(),
            next_order: 0,
            next_component: 0,
        }
    }

    /// Run SCC discovery for every node.
    fn run(&mut self) -> SccPartition {
        for root in 0..self.graph.node_count() {
            if self.order[root] != UNVISITED_NODE {
                continue;
            }

            self.open_frame(root as u32);
            self.drain_frames();
        }

        SccPartition {
            component: std::mem::take(&mut self.component),
            component_count: self.next_component,
        }
    }

    /// Drain the explicit DFS stack.
    fn drain_frames(&mut self) {
        while let Some(frame) = self.frames.last().copied() {
            let successors = self.graph.successors(frame.node as usize);

            // descend into the next successor
            if frame.cursor < successors.len() as u32 {
                let target = successors[frame.cursor as usize];
                let frame_index = self.frames.len() - 1;
                self.frames[frame_index].cursor += 1;
                self.visit_edge(frame.node, target);

                continue;
            }

            // close the node once every successor has been seen
            self.close_frame(frame.node);
        }
    }

    /// Visit one graph edge during SCC discovery.
    fn visit_edge(&mut self, node: u32, target: u32) {
        // descend into undiscovered targets
        if self.order[target as usize] == UNVISITED_NODE {
            self.open_frame(target);

            return;
        }

        // fold back edges into the current lowlink
        if self.on_stack.contains(target as usize) {
            let node_lowlink = &mut self.lowlink[node as usize];
            *node_lowlink = (*node_lowlink).min(self.order[target as usize]);
        }
    }

    /// Push one newly discovered node.
    fn open_frame(&mut self, node: u32) {
        self.order[node as usize] = self.next_order;
        self.lowlink[node as usize] = self.next_order;
        self.next_order += 1;
        self.stack.push(node);
        self.on_stack.insert(node as usize);
        self.frames.push(SccFrame { node, cursor: 0 });
    }

    /// Close one fully visited node.
    fn close_frame(&mut self, node: u32) {
        self.frames.pop();

        // propagate lowlink into the parent frame
        if let Some(parent) = self.frames.last().copied() {
            let child_lowlink = self.lowlink[node as usize];
            let parent_lowlink = &mut self.lowlink[parent.node as usize];
            *parent_lowlink = (*parent_lowlink).min(child_lowlink);
        }

        // emit a component when this node roots one
        if self.lowlink[node as usize] == self.order[node as usize] {
            self.close_component(node);
        }
    }

    /// Pop one completed component from the Tarjan stack.
    fn close_component(&mut self, root: u32) {
        let component = self.next_component;
        self.next_component += 1;

        while let Some(member) = self.stack.pop() {
            self.on_stack.remove(member as usize);
            self.component[member as usize] = component;

            if member == root {
                break;
            }
        }
    }
}

/// One explicit DFS frame.
#[derive(Debug, Clone, Copy)]
struct SccFrame {
    /// The dense node id.
    node: u32,
    /// The next successor index to visit.
    cursor: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partitions_dense_graph() {
        let edge_offsets = [0, 1, 2, 4, 5, 6, 6];
        let edge_targets = [1, 2, 0, 3, 4, 3];
        let graph = DenseGraph::new(&edge_offsets, &edge_targets);

        let partition = graph.strongly_connected_components();

        assert_eq!(partition.component_count(), 3);
        assert_eq!(partition.component(0), partition.component(1));
        assert_eq!(partition.component(1), partition.component(2));
        assert_eq!(partition.component(3), partition.component(4));
        assert_ne!(partition.component(2), partition.component(3));
        assert_ne!(partition.component(4), partition.component(5));
    }

    #[test]
    fn test_handles_self_loop() {
        let edge_offsets = [0, 1];
        let edge_targets = [0];
        let graph = DenseGraph::new(&edge_offsets, &edge_targets);

        let partition = graph.strongly_connected_components();

        assert_eq!(partition.component_count(), 1);
        assert_eq!(partition.component(0), 0);
    }
}
