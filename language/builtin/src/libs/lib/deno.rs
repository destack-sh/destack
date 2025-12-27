use super::super::source::{BuiltinLib, BuiltinLibSource};

const LIB_DENO_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno",
    "index.d.ds",
    include_str!(concat!("../../../lib/deno/index.d.ds")),
);

pub const LIB_DENO: BuiltinLib = BuiltinLib::ambient("deno", &[LIB_DENO_INDEX_D_DS], &["esnext"]);
