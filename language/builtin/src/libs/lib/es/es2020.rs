use super::super::super::source::{BuiltinLib, BuiltinLibSource};

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

lib_source!(LIB_ES_ES2020_BIGINT_D_DS, "bigint.d.ds");
lib_source!(LIB_ES_ES2020_DATE_D_DS, "date.d.ds");
lib_source!(LIB_ES_ES2020_INTL_D_DS, "intl.d.ds");
lib_source!(LIB_ES_ES2020_NUMBER_D_DS, "number.d.ds");
lib_source!(LIB_ES_ES2020_PROMISE_D_DS, "promise.d.ds");
lib_source!(LIB_ES_ES2020_SHAREDMEMORY_D_DS, "sharedmemory.d.ds");
lib_source!(LIB_ES_ES2020_STRING_D_DS, "string.d.ds");
lib_source!(LIB_ES_ES2020_SYMBOL_WELLKNOWN_D_DS, "symbol.wellknown.d.ds");

pub const LIB_ES2020: BuiltinLib = BuiltinLib::ambient(
    "es2020",
    &[
        LIB_ES_ES2020_BIGINT_D_DS,
        LIB_ES_ES2020_DATE_D_DS,
        LIB_ES_ES2020_INTL_D_DS,
        LIB_ES_ES2020_NUMBER_D_DS,
        LIB_ES_ES2020_PROMISE_D_DS,
        LIB_ES_ES2020_SHAREDMEMORY_D_DS,
        LIB_ES_ES2020_STRING_D_DS,
        LIB_ES_ES2020_SYMBOL_WELLKNOWN_D_DS,
    ],
    &["es2019"],
);

pub const LIB_ES2020_BIGINT: BuiltinLib =
    BuiltinLib::ambient("es2020.bigint", &[LIB_ES_ES2020_BIGINT_D_DS], &[]);
pub const LIB_ES2020_DATE: BuiltinLib =
    BuiltinLib::ambient("es2020.date", &[LIB_ES_ES2020_DATE_D_DS], &[]);
pub const LIB_ES2020_INTL: BuiltinLib =
    BuiltinLib::ambient("es2020.intl", &[LIB_ES_ES2020_INTL_D_DS], &[]);
pub const LIB_ES2020_NUMBER: BuiltinLib =
    BuiltinLib::ambient("es2020.number", &[LIB_ES_ES2020_NUMBER_D_DS], &[]);
pub const LIB_ES2020_PROMISE: BuiltinLib =
    BuiltinLib::ambient("es2020.promise", &[LIB_ES_ES2020_PROMISE_D_DS], &[]);
pub const LIB_ES2020_SHAREDMEMORY: BuiltinLib = BuiltinLib::ambient(
    "es2020.sharedmemory",
    &[LIB_ES_ES2020_SHAREDMEMORY_D_DS],
    &[],
);
pub const LIB_ES2020_STRING: BuiltinLib =
    BuiltinLib::ambient("es2020.string", &[LIB_ES_ES2020_STRING_D_DS], &[]);
pub const LIB_ES2020_SYMBOL_WELLKNOWN: BuiltinLib = BuiltinLib::ambient(
    "es2020.symbol.wellknown",
    &[LIB_ES_ES2020_SYMBOL_WELLKNOWN_D_DS],
    &[],
);
