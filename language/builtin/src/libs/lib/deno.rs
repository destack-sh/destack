use super::super::source::{BuiltinLib, BuiltinLibSource};

const LIB_DENO_V1_45_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno/v1.45",
    "index.d.ds",
    include_str!(concat!("../../../lib/deno/v1.45/index.d.ds")),
);
const LIB_DENO_V1_42_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno/v1.42",
    "index.d.ds",
    include_str!(concat!("../../../lib/deno/v1.42/index.d.ds")),
);
const LIB_DENO_V1_41_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno/v1.41",
    "index.d.ds",
    include_str!(concat!("../../../lib/deno/v1.41/index.d.ds")),
);
const LIB_DENO_V1_43_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno/v1.43",
    "index.d.ds",
    include_str!(concat!("../../../lib/deno/v1.43/index.d.ds")),
);
const LIB_DENO_V1_44_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "deno/v1.44",
    "index.d.ds",
    include_str!(concat!("../../../lib/deno/v1.44/index.d.ds")),
);

pub const LIB_DENO: BuiltinLib =
    BuiltinLib::ambient("deno", &[LIB_DENO_V1_45_INDEX_D_DS], &["esnext"]);
pub const LIB_DENO_V1_45: BuiltinLib =
    BuiltinLib::ambient("deno.v1.45", &[LIB_DENO_V1_45_INDEX_D_DS], &["esnext"]);
pub const LIB_DENO_V1_42: BuiltinLib =
    BuiltinLib::ambient("deno.v1.42", &[LIB_DENO_V1_42_INDEX_D_DS], &["esnext"]);
pub const LIB_DENO_V1_41: BuiltinLib =
    BuiltinLib::ambient("deno.v1.41", &[LIB_DENO_V1_41_INDEX_D_DS], &["esnext"]);
pub const LIB_DENO_V1_43: BuiltinLib =
    BuiltinLib::ambient("deno.v1.43", &[LIB_DENO_V1_43_INDEX_D_DS], &["esnext"]);
pub const LIB_DENO_V1_44: BuiltinLib =
    BuiltinLib::ambient("deno.v1.44", &[LIB_DENO_V1_44_INDEX_D_DS], &["esnext"]);
