use super::super::super::source::{BuiltinLib, BuiltinLibSource};
use super::ES_CANONICAL_EXPORTS;

macro_rules! lib_source {
    ($name:ident, $file:literal) => {
        const $name: BuiltinLibSource = BuiltinLibSource::new(
            "lib",
            "es/es2020",
            $file,
            include_str!(concat!("../../../../lib/es/es2020/", $file)),
        );
    };
}

lib_source!(LIB_ES_ES2020_BIGINT_D_DS, "bigint.d.ts");
lib_source!(LIB_ES_ES2020_DATE_D_DS, "date.d.ts");
lib_source!(LIB_ES_ES2020_FULL_D_DS, "full.d.ts");
lib_source!(LIB_ES_ES2020_INDEX_D_DS, "index.d.ts");
lib_source!(LIB_ES_ES2020_INTL_D_DS, "intl.d.ts");
lib_source!(LIB_ES_ES2020_NUMBER_D_DS, "number.d.ts");
lib_source!(LIB_ES_ES2020_PROMISE_D_DS, "promise.d.ts");
lib_source!(LIB_ES_ES2020_SHAREDMEMORY_D_DS, "sharedmemory.d.ts");
lib_source!(LIB_ES_ES2020_STRING_D_DS, "string.d.ts");
lib_source!(LIB_ES_ES2020_SYMBOL_WELLKNOWN_D_DS, "symbol.wellknown.d.ts");

pub const LIB_ES2020: BuiltinLib = BuiltinLib::ambient(
    "es2020",
    &[
        LIB_ES_ES2020_BIGINT_D_DS,
        LIB_ES_ES2020_DATE_D_DS,
        LIB_ES_ES2020_INDEX_D_DS,
        LIB_ES_ES2020_INTL_D_DS,
        LIB_ES_ES2020_NUMBER_D_DS,
        LIB_ES_ES2020_PROMISE_D_DS,
        LIB_ES_ES2020_SHAREDMEMORY_D_DS,
        LIB_ES_ES2020_STRING_D_DS,
        LIB_ES_ES2020_SYMBOL_WELLKNOWN_D_DS,
    ],
    &["es2019"],
)
.with_canonical_exports(ES_CANONICAL_EXPORTS);

pub const LIB_ES2020_BIGINT: BuiltinLib =
    BuiltinLib::ambient("es2020.bigint", &[LIB_ES_ES2020_BIGINT_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2020_DATE: BuiltinLib =
    BuiltinLib::ambient("es2020.date", &[LIB_ES_ES2020_DATE_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2020_FULL: BuiltinLib = BuiltinLib::ambient(
    "es2020.full",
    &[LIB_ES_ES2020_FULL_D_DS],
    &[
        "es2020",
        "dom",
        "worker.importscripts",
        "scripthost",
        "dom.iterable",
        "dom.asynciterable",
    ],
)
.with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2020_INTL: BuiltinLib =
    BuiltinLib::ambient("es2020.intl", &[LIB_ES_ES2020_INTL_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2020_NUMBER: BuiltinLib =
    BuiltinLib::ambient("es2020.number", &[LIB_ES_ES2020_NUMBER_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2020_PROMISE: BuiltinLib =
    BuiltinLib::ambient("es2020.promise", &[LIB_ES_ES2020_PROMISE_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2020_SHAREDMEMORY: BuiltinLib = BuiltinLib::ambient(
    "es2020.sharedmemory",
    &[LIB_ES_ES2020_SHAREDMEMORY_D_DS],
    &[],
)
.with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2020_STRING: BuiltinLib =
    BuiltinLib::ambient("es2020.string", &[LIB_ES_ES2020_STRING_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2020_SYMBOL_WELLKNOWN: BuiltinLib = BuiltinLib::ambient(
    "es2020.symbol.wellknown",
    &[LIB_ES_ES2020_SYMBOL_WELLKNOWN_D_DS],
    &[],
)
.with_canonical_exports(ES_CANONICAL_EXPORTS);
