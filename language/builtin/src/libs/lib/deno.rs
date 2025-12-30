use super::super::source::{BuiltinLib, BuiltinLibSource};

const LIB_DENO_V2_6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno/v2.6",
    "index.d.ds",
    include_str!(concat!("../../../lib/deno/v2.6/index.d.ds")),
);

pub const LIB_DENO: BuiltinLib =
    BuiltinLib::ambient("deno", &[LIB_DENO_V2_6_INDEX_D_DS], &["esnext"]);
pub const LIB_DENO_V2_6: BuiltinLib =
    BuiltinLib::ambient("deno.v2.6", &[LIB_DENO_V2_6_INDEX_D_DS], &["esnext"]);
