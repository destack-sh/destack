use super::super::common::clamp_min;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Object graph build and mark traversal with stride-based retention.
    pub const OBJECT_GRAPH_MARK,
    name: "object_graph_mark",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/memory/object_graph_mark.mir")),
    entry: "object_graph_mark",
    expected: || object_graph_mark(512, 5, 9),
    default_args: |_interp| vec![Value::int64(512), Value::int64(5), Value::int64(9)],
    tags: &["memory", "graph"],
    scales: &[
        scale_axis("nodes", 0, 512, 5120, 51200, true),
    ],
}

/// Compute the expected value for the object graph mark benchmark.
fn object_graph_mark(nodes: i64, root_stride: i64, seed: i64) -> Value {
    // normalize parameters
    let node_count = clamp_min(nodes, 1);
    let root_stride = clamp_min(root_stride, 1);
    let node_count_usize = node_count as usize;

    // build node payloads
    let mut values = vec![0i64; node_count_usize];
    let mut edge1 = vec![0i64; node_count_usize];
    let mut edge2 = vec![0i64; node_count_usize];

    // initialize node contents
    let mut index = 0i64;
    while index < node_count {
        // compute node payload
        let value = index ^ seed;

        // compute edge targets
        let edge1_raw = index.wrapping_mul(3).wrapping_add(1);
        let edge2_raw = index.wrapping_mul(7).wrapping_add(2);
        let neighbor1 = edge1_raw % node_count;
        let neighbor2 = edge2_raw % node_count;

        // store node data
        let slot = index as usize;
        values[slot] = value;
        edge1[slot] = neighbor1;
        edge2[slot] = neighbor2;

        // advance cursor
        index = index.wrapping_add(1);
    }

    // initialize traversal state
    let mut visited = vec![0i64; node_count_usize];
    let mut acc = 0i64;

    // scan roots at the given stride
    let mut root = 0i64;
    while root < node_count {
        // mark the root node if needed
        let root_slot = root as usize;
        let root_mark = visited[root_slot];
        if root_mark == 0 {
            visited[root_slot] = 1;
            let root_value = values[root_slot];
            acc = acc.wrapping_add(root_value);
        }

        // scan nodes relative to the root
        let mut iter = 0i64;
        while iter < node_count {
            // resolve scan slot
            let offset = root.wrapping_add(iter);
            let slot = offset % node_count;
            let slot_usize = slot as usize;

            // visit neighbors for marked nodes
            let seen = visited[slot_usize];
            if seen != 0 {
                let neighbor1 = edge1[slot_usize];
                let neighbor1_usize = neighbor1 as usize;
                let neighbor1_mark = visited[neighbor1_usize];
                if neighbor1_mark == 0 {
                    visited[neighbor1_usize] = 1;
                    let neighbor1_value = values[neighbor1_usize];
                    acc = acc.wrapping_add(neighbor1_value);
                }

                let neighbor2 = edge2[slot_usize];
                let neighbor2_usize = neighbor2 as usize;
                let neighbor2_mark = visited[neighbor2_usize];
                if neighbor2_mark == 0 {
                    visited[neighbor2_usize] = 1;
                    let neighbor2_value = values[neighbor2_usize];
                    acc = acc.wrapping_add(neighbor2_value);
                }
            }

            // advance scan cursor
            iter = iter.wrapping_add(1);
        }

        // advance root cursor
        root = root.wrapping_add(root_stride);
    }

    // return value
    Value::int64(acc)
}
