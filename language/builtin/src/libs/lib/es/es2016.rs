use super::super::super::source::{BuiltinLib, BuiltinLibSource};
use super::ES_CANONICAL_EXPORTS;

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

lib_source!(LIB_ES_ES2016_ARRAY_INCLUDE_D_DS, "array.include.d.ds");
lib_source!(LIB_ES_ES2016_FULL_D_DS, "full.d.ds");
lib_source!(LIB_ES_ES2016_INDEX_D_DS, "index.d.ds");
lib_source!(LIB_ES_ES2016_INTL_D_DS, "intl.d.ds");

pub const LIB_ES2016: BuiltinLib = BuiltinLib::ambient(
    "es2016",
    &[
        LIB_ES_ES2016_ARRAY_INCLUDE_D_DS,
        LIB_ES_ES2016_INDEX_D_DS,
        LIB_ES_ES2016_INTL_D_DS,
    ],
    &["es2015"],
)
.with_canonical_exports(ES_CANONICAL_EXPORTS);

pub const LIB_ES2016_ARRAY_INCLUDE: BuiltinLib = BuiltinLib::ambient(
    "es2016.array.include",
    &[LIB_ES_ES2016_ARRAY_INCLUDE_D_DS],
    &[],
)
.with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2016_FULL: BuiltinLib = BuiltinLib::ambient(
    "es2016.full",
    &[LIB_ES_ES2016_FULL_D_DS],
    &[
        "es2016",
        "dom",
        "worker.importscripts",
        "scripthost",
        "dom.iterable",
    ],
)
.with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2016_INTL: BuiltinLib =
    BuiltinLib::ambient("es2016.intl", &[LIB_ES_ES2016_INTL_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
