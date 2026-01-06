use super::super::source::{BuiltinLib, BuiltinLibSource};

const LIB_BUN_V1_3_INDEX_D_TS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun/v1.3",
    "index.d.ts",
    include_str!(concat!("../../../lib/bun/v1.3/index.d.ts")),
);
const LIB_BUN_V1_2_INDEX_D_TS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun/v1.2",
    "index.d.ts",
    include_str!(concat!("../../../lib/bun/v1.2/index.d.ts")),
);

const BUN_SPECIFIER_ALIASES: &[(&str, &str)] = &[("undici-types", "undici-types.v7")];

pub const LIB_BUN: BuiltinLib = BuiltinLib::ambient_lib(
    "bun",
    &[LIB_BUN_V1_3_INDEX_D_TS],
    &["esnext", "node.v24", "undici-types.v7"],
)
.with_specifier_aliases(BUN_SPECIFIER_ALIASES);
pub const LIB_BUN_V1_3: BuiltinLib = BuiltinLib::ambient_lib(
    "bun.v1.3",
    &[LIB_BUN_V1_3_INDEX_D_TS],
    &["esnext", "node.v24", "undici-types.v7"],
)
.with_specifier_aliases(BUN_SPECIFIER_ALIASES);
pub const LIB_BUN_V1_2: BuiltinLib = BuiltinLib::ambient_lib(
    "bun.v1.2",
    &[LIB_BUN_V1_2_INDEX_D_TS],
    &["esnext", "node.v24", "undici-types.v7"],
)
.with_specifier_aliases(BUN_SPECIFIER_ALIASES);
