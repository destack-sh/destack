use super::super::super::source::{BuiltinLib, BuiltinLibSource};
use super::ES_CANONICAL_EXPORTS;

const LIB_ES_DECORATORS_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "es",
    "decorators.d.ds",
    include_str!(concat!("../../../../lib/es/decorators.d.ds")),
);
const LIB_ES_DECORATORS_LEGACY_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "es",
    "decorators.legacy.d.ds",
    include_str!(concat!("../../../../lib/es/decorators.legacy.d.ds")),
);

pub const LIB_DECORATORS: BuiltinLib =
    BuiltinLib::ambient("decorators", &[LIB_ES_DECORATORS_D_DS], &["es5"])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_DECORATORS_LEGACY: BuiltinLib = BuiltinLib::ambient(
    "decorators.legacy",
    &[LIB_ES_DECORATORS_LEGACY_D_DS],
    &["es5"],
)
.with_canonical_exports(ES_CANONICAL_EXPORTS);
