use super::super::source::{BuiltinLib, BuiltinLibSource};

const LIB_BUN_V1_3_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun/v1.3",
    "index.d.ts",
    include_str!(concat!("../../../lib/bun/v1.3/index.d.ts")),
);
const LIB_BUN_V1_2_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun/v1.2",
    "index.d.ts",
    include_str!(concat!("../../../lib/bun/v1.2/index.d.ts")),
);

pub const LIB_BUN: BuiltinLib =
    BuiltinLib::ambient("bun", &[LIB_BUN_V1_3_INDEX_D_DS], &["esnext", "node"]);
pub const LIB_BUN_V1_3: BuiltinLib =
    BuiltinLib::ambient("bun.v1.3", &[LIB_BUN_V1_3_INDEX_D_DS], &["esnext", "node"]);
pub const LIB_BUN_V1_2: BuiltinLib =
    BuiltinLib::ambient("bun.v1.2", &[LIB_BUN_V1_2_INDEX_D_DS], &["esnext", "node"]);
