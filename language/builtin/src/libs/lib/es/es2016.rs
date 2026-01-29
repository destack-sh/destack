use super::super::super::source::{BuiltinLib, BuiltinLibSource};
use super::es2015::ES2015_DECLARED_SYMBOLS;

pub(crate) const ES2016_DECLARED_SYMBOLS: &[&str] = ES2015_DECLARED_SYMBOLS;
const ES2016_EMPTY_DECLARED_SYMBOLS: &[&str] = &[];

macro_rules! lib_source {
    ($name:ident, $file:literal) => {
        const $name: BuiltinLibSource = BuiltinLibSource::new(
            "lib",
            "es/es2016",
            $file,
            include_str!(concat!("../../../../lib/es/es2016/", $file)),
        );
    };
}

lib_source!(LIB_ES_ES2016_ARRAY_INCLUDE_D_DS, "array.include.d.ts");
lib_source!(LIB_ES_ES2016_FULL_D_DS, "full.d.ts");
lib_source!(LIB_ES_ES2016_INDEX_D_DS, "index.d.ts");
lib_source!(LIB_ES_ES2016_INTL_D_DS, "intl.d.ts");

pub const LIB_ES2016: BuiltinLib = BuiltinLib::ambient_lib(
    "es2016",
    &[
        LIB_ES_ES2016_ARRAY_INCLUDE_D_DS,
        LIB_ES_ES2016_INDEX_D_DS,
        LIB_ES_ES2016_INTL_D_DS,
    ],
    &[],
)
.with_declared_symbols(ES2016_DECLARED_SYMBOLS);

pub const LIB_ES2016_ARRAY_INCLUDE: BuiltinLib = BuiltinLib::ambient_lib(
    "es2016.array.include",
    &[LIB_ES_ES2016_ARRAY_INCLUDE_D_DS],
    &[],
)
.with_declared_symbols(ES2016_EMPTY_DECLARED_SYMBOLS);
pub const LIB_ES2016_FULL: BuiltinLib =
    BuiltinLib::ambient_lib("es2016.full", &[LIB_ES_ES2016_FULL_D_DS], &[])
        .with_declared_symbols(ES2016_DECLARED_SYMBOLS);
pub const LIB_ES2016_INTL: BuiltinLib =
    BuiltinLib::ambient_lib("es2016.intl", &[LIB_ES_ES2016_INTL_D_DS], &[])
        .with_declared_symbols(ES2016_EMPTY_DECLARED_SYMBOLS);
