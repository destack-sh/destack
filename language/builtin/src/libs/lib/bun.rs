use super::super::source::{BuiltinLib, BuiltinLibSource};

const LIB_BUN_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun",
    "index.d.ds",
    include_str!(concat!("../../../lib/bun/index.d.ds")),
);

pub const LIB_BUN: BuiltinLib = BuiltinLib::ambient("bun", &[LIB_BUN_INDEX_D_DS], &["esnext"]);
