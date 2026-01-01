use super::super::super::source::{BuiltinLib, BuiltinLibSource};
use super::ES_CANONICAL_EXPORTS;

macro_rules! lib_source {
    ($name:ident, $file:literal) => {
        const $name: BuiltinLibSource = BuiltinLibSource::new(
            "lib",
            "es/es2019",
            $file,
            include_str!(concat!("../../../../lib/es/es2019/", $file)),
        );
    };
}

lib_source!(LIB_ES_ES2019_ARRAY_D_DS, "array.d.ts");
lib_source!(LIB_ES_ES2019_FULL_D_DS, "full.d.ts");
lib_source!(LIB_ES_ES2019_INDEX_D_DS, "index.d.ts");
lib_source!(LIB_ES_ES2019_INTL_D_DS, "intl.d.ts");
lib_source!(LIB_ES_ES2019_OBJECT_D_DS, "object.d.ts");
lib_source!(LIB_ES_ES2019_STRING_D_DS, "string.d.ts");
lib_source!(LIB_ES_ES2019_SYMBOL_D_DS, "symbol.d.ts");

pub const LIB_ES2019: BuiltinLib = BuiltinLib::ambient(
    "es2019",
    &[
        LIB_ES_ES2019_ARRAY_D_DS,
        LIB_ES_ES2019_INDEX_D_DS,
        LIB_ES_ES2019_INTL_D_DS,
        LIB_ES_ES2019_OBJECT_D_DS,
        LIB_ES_ES2019_STRING_D_DS,
        LIB_ES_ES2019_SYMBOL_D_DS,
    ],
    &["es2018"],
)
.with_canonical_exports(ES_CANONICAL_EXPORTS);

pub const LIB_ES2019_ARRAY: BuiltinLib =
    BuiltinLib::ambient("es2019.array", &[LIB_ES_ES2019_ARRAY_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2019_FULL: BuiltinLib = BuiltinLib::ambient(
    "es2019.full",
    &[LIB_ES_ES2019_FULL_D_DS],
    &[
        "es2019",
        "dom",
        "worker.importscripts",
        "scripthost",
        "dom.iterable",
        "dom.asynciterable",
    ],
)
.with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2019_INTL: BuiltinLib =
    BuiltinLib::ambient("es2019.intl", &[LIB_ES_ES2019_INTL_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2019_OBJECT: BuiltinLib =
    BuiltinLib::ambient("es2019.object", &[LIB_ES_ES2019_OBJECT_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2019_STRING: BuiltinLib =
    BuiltinLib::ambient("es2019.string", &[LIB_ES_ES2019_STRING_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2019_SYMBOL: BuiltinLib =
    BuiltinLib::ambient("es2019.symbol", &[LIB_ES_ES2019_SYMBOL_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
