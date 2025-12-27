use super::super::source::{BuiltinLib, BuiltinLibSource};

const LIB_BUN_V1_1_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun/v1.1",
    "index.d.ds",
    include_str!(concat!("../../../lib/bun/v1.1/index.d.ds")),
);
const LIB_BUN_V0_8_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun/v0.8",
    "index.d.ds",
    include_str!(concat!("../../../lib/bun/v0.8/index.d.ds")),
);
const LIB_BUN_V1_0_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun/v1.0",
    "index.d.ds",
    include_str!(concat!("../../../lib/bun/v1.0/index.d.ds")),
);

pub const LIB_BUN: BuiltinLib = BuiltinLib::ambient("bun", &[LIB_BUN_V1_1_INDEX_D_DS], &["esnext"]);
pub const LIB_BUN_V1_1: BuiltinLib =
    BuiltinLib::ambient("bun.v1.1", &[LIB_BUN_V1_1_INDEX_D_DS], &["esnext"]);
pub const LIB_BUN_V0_8: BuiltinLib =
    BuiltinLib::ambient("bun.v0.8", &[LIB_BUN_V0_8_INDEX_D_DS], &["esnext"]);
pub const LIB_BUN_V1_0: BuiltinLib =
    BuiltinLib::ambient("bun.v1.0", &[LIB_BUN_V1_0_INDEX_D_DS], &["esnext"]);
