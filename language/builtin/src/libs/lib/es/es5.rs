use super::super::super::source::{BuiltinLib, BuiltinLibSource};
use super::ES_CANONICAL_EXPORTS;

const LIB_ES_ES5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "es/es5",
    "index.d.ds",
    include_str!(concat!("../../../../lib/es/es5/index.d.ds")),
);

pub const LIB_ES5: BuiltinLib = BuiltinLib::ambient("es5", &[LIB_ES_ES5_INDEX_D_DS], &[])
    .with_canonical_exports(ES_CANONICAL_EXPORTS);
