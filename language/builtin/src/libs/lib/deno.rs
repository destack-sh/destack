use super::super::source::{BuiltinLib, BuiltinLibSource};

const LIB_DENO_V2_6_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno/v2.6",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno/v2.6/index.d.ts")),
);
const LIB_DENO_V2_5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno/v2.5",
    "index.d.ts",
    include_str!(concat!("../../../lib/deno/v2.5/index.d.ts")),
);

pub const LIB_DENO: BuiltinLib =
    BuiltinLib::ambient("deno", &[LIB_DENO_V2_6_INDEX_D_DS], &["esnext"]);
pub const LIB_DENO_V2_6: BuiltinLib =
    BuiltinLib::ambient("deno.v2.6", &[LIB_DENO_V2_6_INDEX_D_DS], &["esnext"]);
pub const LIB_DENO_V2_5: BuiltinLib =
    BuiltinLib::ambient("deno.v2.5", &[LIB_DENO_V2_5_INDEX_D_DS], &["esnext"]);
