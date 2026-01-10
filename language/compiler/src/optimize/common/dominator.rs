/// Compute immediate dominators with the Lengauer Tarjan algorithm.
#[derive(Debug)]
pub struct DominatorResult {
    /// Immediate dominator index for each node in the graph.
    pub immediate_dominators: Vec<Option<usize>>,
}

/// Compute immediate dominators for a graph rooted at `root`.
pub fn compute_immediate_dominators(
    successors: &[Vec<usize>],
    predecessors: &[Vec<usize>],
    root: usize,
) -> DominatorResult {
    // validate graph shape
    if successors.len() != predecessors.len() {
        return DominatorResult {
            immediate_dominators: vec![None; successors.len()],
        };
    }

    let node_count = successors.len();
    if root >= node_count {
        return DominatorResult {
            immediate_dominators: vec![None; node_count],
        };
    }

    // dfs numbering
    let mut dfs_index = vec![0usize; node_count];
    let mut vertex = vec![0usize; node_count + 1];
    let mut parent = vec![0usize; node_count + 1];
    let mut counter = 1usize;

    dfs_index[root] = counter;
    vertex[counter] = root;
    parent[counter] = 0;

    // dfs stack
    struct Frame {
        node: usize,
        next_successor: usize,
    }

    let mut stack = vec![Frame {
        node: root,
        next_successor: 0,
    }];

    while let Some(frame) = stack.last_mut() {
        if frame.next_successor >= successors[frame.node].len() {
            stack.pop();
            continue;
        }

        let succ = successors[frame.node][frame.next_successor];
        frame.next_successor += 1;

        if dfs_index[succ] != 0 {
            continue;
        }

        counter += 1;
        dfs_index[succ] = counter;
        vertex[counter] = succ;
        parent[counter] = dfs_index[frame.node];

        stack.push(Frame {
            node: succ,
            next_successor: 0,
        });
    }

    // dominator arrays (1-based by dfs index)
    let mut semi = vec![0usize; node_count + 1];
    let mut label = vec![0usize; node_count + 1];
    let mut ancestor = vec![0usize; node_count + 1];
    let mut idom = vec![0usize; node_count + 1];
    let mut bucket: Vec<Vec<usize>> = vec![Vec::new(); node_count + 1];

    for i in 1..=counter {
        semi[i] = i;
        label[i] = i;
    }

    // path compression for eval
    fn compress(v: usize, ancestor: &mut [usize], label: &mut [usize], semi: &[usize]) {
        let ancestor_v = ancestor[v];
        let ancestor_parent = ancestor[ancestor_v];
        if ancestor_parent != 0 {
            compress(ancestor_v, ancestor, label, semi);
            if semi[label[ancestor_v]] < semi[label[v]] {
                label[v] = label[ancestor_v];
            }
            ancestor[v] = ancestor_parent;
        }
    }

    fn eval(v: usize, ancestor: &mut [usize], label: &mut [usize], semi: &[usize]) -> usize {
        if ancestor[v] == 0 {
            return label[v];
        }

        compress(v, ancestor, label, semi);

        let ancestor_label = label[ancestor[v]];
        if semi[ancestor_label] < semi[label[v]] {
            ancestor_label
        } else {
            label[v]
        }
    }

    // compute semi dominators
    for i in (2..=counter).rev() {
        let w = i;
        let w_node = vertex[w];

        // evaluate predecessors
        for &pred in &predecessors[w_node] {
            let v = dfs_index[pred];
            if v == 0 {
                continue;
            }

            let u = eval(v, &mut ancestor, &mut label, &semi);
            if semi[u] < semi[w] {
                semi[w] = semi[u];
            }
        }

        bucket[semi[w]].push(w);

        // link in the spanning tree
        ancestor[w] = parent[w];

        // process bucket for parent
        let parent_w = parent[w];
        if parent_w != 0 {
            for v in bucket[parent_w].drain(..) {
                let u = eval(v, &mut ancestor, &mut label, &semi);
                if semi[u] < semi[v] {
                    idom[v] = u;
                } else {
                    idom[v] = parent_w;
                }
            }
        }
    }

    // finalize idoms
    for i in 2..=counter {
        if idom[i] != semi[i] {
            idom[i] = idom[idom[i]];
        }
    }

    // map back to node indices
    let mut immediate_dominators = vec![None; node_count];
    for i in 2..=counter {
        let node = vertex[i];
        let idom_index = idom[i];
        if idom_index != 0 {
            immediate_dominators[node] = Some(vertex[idom_index]);
        }
    }

    DominatorResult {
        immediate_dominators,
    }
}
