use super::super::common::mix_result;
use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Opcode dispatch loop over a generated opcode stream.
    pub const OPCODE_ROUTER,
    name: "opcode_router",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/opcode_router.mir")),
    entry: "opcode_router",
    expected: || opcode_router(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["dispatch", "opcode", "router"],
    scales: &[
        scale_axis("len", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the opcode router benchmark.
fn opcode_router(length: i64) -> Value {
    // init state
    let mut opcodes = vec![0i64; length as usize];
    let mut index = 0i64;

    // fill opcodes
    while index < length {
        let mixed = index
            .wrapping_mul(1103515245)
            .wrapping_add(12345)
            .wrapping_rem(6);
        opcodes[index as usize] = mixed;
        index = index.wrapping_add(1);
    }

    // run router loop
    let mut acc = 0i64;
    index = 0;
    while index < length {
        let opcode = opcodes[index as usize];
        acc = match opcode {
            0 => acc.wrapping_add(index),
            1 => acc ^ opcode,
            2 => acc.wrapping_mul(2),
            3 => acc.wrapping_sub(index),
            4 => acc.wrapping_add(3),
            5 => acc.wrapping_add(1),
            _ => acc.wrapping_add(5),
        };
        index = index.wrapping_add(1);
    }

    // return value
    let mixed = mix_result(acc, length);
    Value::int64(mixed)
}
