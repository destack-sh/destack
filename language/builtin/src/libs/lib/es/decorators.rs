use super::super::super::source::{BuiltinLib, BuiltinLibSource};
use super::es5::ES5_DECLARED_SYMBOLS;

const LIB_ES_DECORATORS_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "es",
    "decorators.d.ts",
    include_str!(concat!("../../../../lib/es/decorators.d.ts")),
);
const LIB_ES_DECORATORS_LEGACY_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "es",
    "decorators.legacy.d.ts",
    include_str!(concat!("../../../../lib/es/decorators.legacy.d.ts")),
);

pub const LIB_DECORATORS: BuiltinLib =
    BuiltinLib::ambient_lib("decorators", &[LIB_ES_DECORATORS_D_DS], &[])
        .with_declared_symbols(ES5_DECLARED_SYMBOLS);
pub const LIB_DECORATORS_LEGACY: BuiltinLib = BuiltinLib::ambient_lib(
    "decorators.legacy",
    &[LIB_ES_DECORATORS_LEGACY_D_DS],
    &[],
)
.with_declared_symbols(ES5_DECLARED_SYMBOLS);
