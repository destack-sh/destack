use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Bytecode style dispatch loop with mixed arithmetic updates.
    pub const BYTECODE_VM,
    name: "bytecode_vm",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/dispatch/bytecode_vm.mir")),
    entry: "bytecode_vm",
    expected: || bytecode_vm(10_000),
    default_args: |_interp| vec![Value::int64(10_000)],
    tags: &["dispatch", "bytecode"],
    scales: &[
        scale_axis("iter", 0, 10000, 100000, 1000000, true),
    ],
}

/// Compute the expected value for the bytecode vm benchmark.
fn bytecode_vm(iterations: i64) -> Value {
    // init state
    let mut pc = 0i64;
    let mut acc = 1i64;
    let mut reg = 3i64;

    // run dispatch loop
    while pc < iterations {
        // build biased opcode selection
        let mixed = pc.wrapping_mul(1103515245).wrapping_add(12345);
        let byte = mixed & 255;
        let opcode = if byte < 192 {
            byte.wrapping_rem(4)
        } else {
            byte.wrapping_rem(3).wrapping_add(4)
        };

        // execute opcode
        match opcode {
            0 => {
                acc = acc.wrapping_add(reg);
            }
            1 => {
                let shifted = reg.wrapping_shl(1);
                acc ^= shifted;
                reg = reg.wrapping_add(3);
            }
            2 => {
                let mulled = acc.wrapping_mul(3);
                acc = mulled.wrapping_add(reg);
            }
            3 => {
                acc = acc.wrapping_sub(reg);
            }
            4 => {
                reg = reg.wrapping_add(5);
                acc ^= reg;
            }
            5 => {
                let is_odd = (reg & 1) == 1;
                if is_odd {
                    acc = acc.wrapping_add(reg);
                } else {
                    acc = acc.wrapping_sub(reg);
                }
            }
            6 => {
                acc = acc.wrapping_add(7);
            }
            _ => {
                acc = acc.wrapping_add(2);
            }
        }

        // advance counter
        pc = pc.wrapping_add(1);
    }

    // return value
    Value::int64(acc)
}
