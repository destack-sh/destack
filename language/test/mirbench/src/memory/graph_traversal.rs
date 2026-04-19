use super::super::common::clamp_min;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Graph traversal over a small adjacency queue.
    pub const GRAPH_TRAVERSAL,
    name: "graph_traversal",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/graph_traversal.mir")),
    entry: "graph_traversal",
    expected: || graph_traversal(1000, 5),
    default_args: |_interp| vec![Value::int64(1000), Value::int64(5)],
    tags: &["memory", "graph"],
    scales: &[
        scale_axis("nodes", 0, 1000, 10000, 100000, true),
    ],
}

/// Compute the expected value for the graph traversal benchmark.
fn graph_traversal(count: i64, stride: i64) -> Value {
    // handle empty graph
    if count == 0 {
        return Value::int64(0);
    }

    // init queue and visited flags
    let mut visited = vec![0i64; count as usize];
    let mut queue = vec![0i64; count as usize];
    let mut head = 0i64;
    let mut tail = 1i64;
    let mut sum = 0i64;
    let stride = clamp_min(stride, 1);

    // seed with node zero
    queue[0] = 0;
    visited[0] = 1;

    // walk breadth first
    while head < tail {
        // pop next node
        let node = queue[head as usize];
        head += 1;
        sum = sum.wrapping_add(node);

        // enqueue first neighbor
        let mut neighbor1 = node.wrapping_add(1);
        if neighbor1 >= count {
            neighbor1 = neighbor1.wrapping_sub(count);
        }

        if visited[neighbor1 as usize] == 0 {
            visited[neighbor1 as usize] = 1;
            queue[tail as usize] = neighbor1;
            tail += 1;
        }

        // enqueue second neighbor
        let mut neighbor2 = node.wrapping_add(2);
        if neighbor2 >= count {
            neighbor2 = neighbor2.wrapping_sub(count);
        }

        if visited[neighbor2 as usize] == 0 {
            visited[neighbor2 as usize] = 1;
            queue[tail as usize] = neighbor2;
            tail += 1;
        }

        // enqueue third neighbor
        let mut neighbor3 = node.wrapping_add(stride);
        if neighbor3 >= count {
            neighbor3 = neighbor3.wrapping_sub(count);
        }

        if visited[neighbor3 as usize] == 0 {
            visited[neighbor3 as usize] = 1;
            queue[tail as usize] = neighbor3;
            tail += 1;
        }
    }

    // return value
    Value::int64(sum)
}
