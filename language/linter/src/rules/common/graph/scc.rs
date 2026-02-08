use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// Compute strongly connected components for a directed graph.
pub fn strongly_connected_components<NodeId>(
    adjacency: &HashMap<NodeId, Vec<NodeId>>,
) -> Vec<Vec<NodeId>>
where
    NodeId: Copy + Eq + Hash + Ord,
{
    let mut tarjan = TarjanScc::new(adjacency);
    tarjan.run()
}

/// Return one cycle path contained in the component.
pub fn find_cycle_path<NodeId>(
    adjacency: &HashMap<NodeId, Vec<NodeId>>,
    component: &[NodeId],
) -> Option<Vec<NodeId>>
where
    NodeId: Copy + Eq + Hash + Ord,
{
    if component.is_empty() {
        return None;
    }

    let component_set = component.iter().copied().collect::<HashSet<_>>();

    // self cycle
    if component.len() == 1 {
        let node = component[0];
        let has_self_edge = adjacency
            .get(&node)
            .is_some_and(|neighbors| neighbors.contains(&node));
        if has_self_edge {
            return Some(vec![node, node]);
        }

        return None;
    }

    // find one cycle with dfs stack tracking
    let mut visited = HashSet::new();
    let mut stack = Vec::new();
    let mut index_in_stack = HashMap::new();

    let mut nodes = component.to_vec();
    nodes.sort_unstable();

    for node in nodes {
        let cycle = find_cycle_path_from(
            node,
            adjacency,
            &component_set,
            &mut visited,
            &mut stack,
            &mut index_in_stack,
        );
        if cycle.is_some() {
            return cycle;
        }
    }

    None
}

/// DFS helper for finding one cycle path.
fn find_cycle_path_from<NodeId>(
    node: NodeId,
    adjacency: &HashMap<NodeId, Vec<NodeId>>,
    component_set: &HashSet<NodeId>,
    visited: &mut HashSet<NodeId>,
    stack: &mut Vec<NodeId>,
    index_in_stack: &mut HashMap<NodeId, usize>,
) -> Option<Vec<NodeId>>
where
    NodeId: Copy + Eq + Hash + Ord,
{
    if let Some(start_index) = index_in_stack.get(&node).copied() {
        let mut cycle = stack[start_index..].to_vec();
        cycle.push(node);
        return Some(cycle);
    }

    if visited.contains(&node) {
        return None;
    }
    visited.insert(node);

    index_in_stack.insert(node, stack.len());
    stack.push(node);

    if let Some(neighbors) = adjacency.get(&node) {
        for neighbor in neighbors.iter().copied() {
            if !component_set.contains(&neighbor) {
                continue;
            }

            let cycle = find_cycle_path_from(
                neighbor,
                adjacency,
                component_set,
                visited,
                stack,
                index_in_stack,
            );
            if cycle.is_some() {
                return cycle;
            }
        }
    }

    stack.pop();
    index_in_stack.remove(&node);

    None
}

/// Tarjan SCC finder for directed graphs.
struct TarjanScc<'a, NodeId>
where
    NodeId: Copy + Eq + Hash + Ord,
{
    /// The dependency adjacency.
    adjacency: &'a HashMap<NodeId, Vec<NodeId>>,
    /// The next DFS index.
    next_index: usize,
    /// DFS index per node.
    node_index: HashMap<NodeId, usize>,
    /// Lowlink per node.
    lowlink: HashMap<NodeId, usize>,
    /// DFS stack.
    stack: Vec<NodeId>,
    /// Membership set for nodes currently on the stack.
    on_stack: HashSet<NodeId>,
    /// Collected strongly connected components.
    components: Vec<Vec<NodeId>>,
}

impl<'a, NodeId> TarjanScc<'a, NodeId>
where
    NodeId: Copy + Eq + Hash + Ord,
{
    /// Build a new SCC detector.
    fn new(adjacency: &'a HashMap<NodeId, Vec<NodeId>>) -> Self {
        Self {
            adjacency,
            next_index: 0,
            node_index: HashMap::new(),
            lowlink: HashMap::new(),
            stack: Vec::new(),
            on_stack: HashSet::new(),
            components: Vec::new(),
        }
    }

    /// Run SCC detection for all nodes.
    fn run(&mut self) -> Vec<Vec<NodeId>> {
        let mut nodes = self.adjacency.keys().copied().collect::<Vec<_>>();
        nodes.sort_unstable();

        for node in nodes {
            if self.node_index.contains_key(&node) {
                continue;
            }

            self.strong_connect(node);
        }

        std::mem::take(&mut self.components)
    }

    /// Perform a DFS step and collect SCCs.
    fn strong_connect(&mut self, node: NodeId) {
        let current_index = self.next_index;
        self.next_index += 1;

        self.node_index.insert(node, current_index);
        self.lowlink.insert(node, current_index);
        self.stack.push(node);
        self.on_stack.insert(node);

        let neighbors = self
            .adjacency
            .get(&node)
            .map(Vec::as_slice)
            .unwrap_or_default();
        for neighbor in neighbors.iter().copied() {
            if !self.node_index.contains_key(&neighbor) {
                self.strong_connect(neighbor);
                let neighbor_lowlink = self.lowlink[&neighbor];
                let node_lowlink = self.lowlink[&node];
                if neighbor_lowlink < node_lowlink {
                    self.lowlink.insert(node, neighbor_lowlink);
                }
            } else if self.on_stack.contains(&neighbor) {
                let neighbor_index = self.node_index[&neighbor];
                let node_lowlink = self.lowlink[&node];
                if neighbor_index < node_lowlink {
                    self.lowlink.insert(node, neighbor_index);
                }
            }
        }

        if self.lowlink[&node] != self.node_index[&node] {
            return;
        }

        let mut component = Vec::new();
        while let Some(stack_node) = self.stack.pop() {
            self.on_stack.remove(&stack_node);
            component.push(stack_node);
            if stack_node == node {
                break;
            }
        }

        self.components.push(component);
    }
}
