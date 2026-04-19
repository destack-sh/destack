use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Router table dispatch using grouped switches.
    pub const ROUTER_TABLE,
    name: "router_table",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/router_table.mir")),
    entry: "router_table",
    expected: || router_table(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["dispatch", "router", "table"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the router table benchmark.
fn router_table(iterations: i64) -> Value {
    // init tables
    let mut route_table = [0i64; 16];
    let mut host_table = [0i64; 8];
    let mut method_table = [0i64; 4];

    let mut table_index = 0usize;
    while table_index < route_table.len() {
        let value = (table_index as i64).wrapping_mul(9).wrapping_add(5) & 31;
        route_table[table_index] = value;
        table_index += 1;
    }

    table_index = 0;
    while table_index < host_table.len() {
        let value = (table_index as i64).wrapping_mul(7).wrapping_add(3) & 15;
        host_table[table_index] = value;
        table_index += 1;
    }

    table_index = 0;
    while table_index < method_table.len() {
        let value = (table_index as i64).wrapping_mul(11).wrapping_add(1) & 7;
        method_table[table_index] = value;
        table_index += 1;
    }

    // run router loop
    let mut index = 0i64;
    let mut acc = 0i64;
    while index < iterations {
        let seed = index.wrapping_mul(1103515245).wrapping_add(12345);
        let method = (seed & 3) as usize;
        let host = ((seed >> 2) & 7) as usize;
        let path_len = (seed >> 5) & 7;
        let path_len = path_len.wrapping_add(2);
        let query_len = (seed >> 9) & 3;
        let query_len = query_len.wrapping_add(1);

        // compute path hash
        let mut seg = 0i64;
        let mut path_hash = 0i64;
        while seg < path_len {
            let seg_seed = seed.wrapping_add(seg.wrapping_mul(131));
            let mix = seg_seed ^ (seg_seed >> 3);
            let seg_val = mix & 255;
            path_hash = path_hash.wrapping_mul(33).wrapping_add(seg_val) & 1023;
            seg = seg.wrapping_add(1);
        }

        // compute query score
        let mut query_index = 0i64;
        let mut query_score = 0i64;
        let mut flag = 0i64;
        while query_index < query_len {
            let shift = query_index & 7;
            let qval = (seed >> shift).wrapping_add(query_index.wrapping_mul(17));
            let qbyte = qval & 255;
            query_score = query_score.wrapping_add(qbyte);
            if (qbyte & 15) == 0 {
                flag ^= 1;
            }
            query_index = query_index.wrapping_add(1);
        }

        // route lookup
        let method_weight = method_table[method];
        let host_weight = host_table[host];
        let bucket = path_hash
            .wrapping_add(method_weight)
            .wrapping_add(host_weight)
            .wrapping_add(query_score & 31)
            & 15;
        let mut route = route_table[bucket as usize];
        match method {
            0 => {
                route = route.wrapping_add(3);
            }
            1 => {
                route = route.wrapping_mul(2).wrapping_add(host as i64);
            }
            2 => {
                route ^= path_hash & 7;
            }
            _ => {
                route = route.wrapping_sub(host_weight);
            }
        }
        if flag == 1 {
            route = route.wrapping_add(7);
        }

        acc = acc
            .wrapping_add(route)
            .wrapping_add(path_hash)
            .wrapping_add(query_score)
            .wrapping_add(index);

        index = index.wrapping_add(1);
    }

    Value::int64(acc)
}
