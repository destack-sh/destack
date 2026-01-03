use super::super::source::{BuiltinLib, BuiltinLibSource};

include!("bun_sources.rs");

const BUN_SPECIFIER_ALIASES: &[(&str, &str)] = &[("undici-types", "undici-types.v7")];

pub const LIB_BUN: BuiltinLib = BuiltinLib::ambient(
    "bun",
    BUN_V1_3_SOURCES,
    &["esnext", "node", "undici-types.v7"],
)
.with_specifier_aliases(BUN_SPECIFIER_ALIASES);
pub const LIB_BUN_V1_3: BuiltinLib = BuiltinLib::ambient(
    "bun.v1.3",
    BUN_V1_3_SOURCES,
    &["esnext", "node", "undici-types.v7"],
)
.with_specifier_aliases(BUN_SPECIFIER_ALIASES);
pub const LIB_BUN_V1_2: BuiltinLib = BuiltinLib::ambient(
    "bun.v1.2",
    BUN_V1_2_SOURCES,
    &["esnext", "node", "undici-types.v7"],
)
.with_specifier_aliases(BUN_SPECIFIER_ALIASES);
