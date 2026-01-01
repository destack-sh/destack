use super::super::source::{BuiltinLib, BuiltinLibSource};

const LIB_SCRIPTHOST_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "scripthost",
    "index.d.ts",
    include_str!(concat!("../../../lib/scripthost/index.d.ts")),
);

pub const LIB_SCRIPTHOST: BuiltinLib =
    BuiltinLib::ambient("scripthost", &[LIB_SCRIPTHOST_D_DS], &["es5"]);
