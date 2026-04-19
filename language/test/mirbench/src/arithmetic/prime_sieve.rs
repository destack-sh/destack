use super::super::{Program, scale_axis};
use destack_vm::Value;

declare_program! {
    /// Prime counting via trial division.
    pub const PRIME_SIEVE,
    name: "prime_sieve",
    source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/mirbench/arithmetic/prime_sieve.mir")),
    entry: "count_primes",
    expected: || Value::int64(25),
    default_args: |_interp| vec![Value::int64(100)],
    tags: &["arithmetic", "prime", "sieve"],
    scales: &[
        scale_axis("limit", 0, 100, 1000, 10000, true),
    ],
}
