use super::super::super::source::{BuiltinLib, BuiltinLibSource};
use super::ES_CANONICAL_EXPORTS;

macro_rules! lib_source {
    ($name:ident, $file:literal) => {
        const $name: BuiltinLibSource = BuiltinLibSource::new(
            "lib",
            "es/es2015",
            $file,
            include_str!(concat!("../../../../lib/es/es2015/", $file)),
        );
    };
}

lib_source!(LIB_ES_ES2015_COLLECTION_D_DS, "collection.d.ds");
lib_source!(LIB_ES_ES2015_CORE_D_DS, "core.d.ds");
lib_source!(LIB_ES_ES2015_GENERATOR_D_DS, "generator.d.ds");
lib_source!(LIB_ES_ES2015_INDEX_D_DS, "index.d.ds");
lib_source!(LIB_ES_ES2015_ITERABLE_D_DS, "iterable.d.ds");
lib_source!(LIB_ES_ES2015_PROMISE_D_DS, "promise.d.ds");
lib_source!(LIB_ES_ES2015_PROXY_D_DS, "proxy.d.ds");
lib_source!(LIB_ES_ES2015_REFLECT_D_DS, "reflect.d.ds");
lib_source!(LIB_ES_ES2015_SYMBOL_D_DS, "symbol.d.ds");
lib_source!(LIB_ES_ES2015_SYMBOL_WELLKNOWN_D_DS, "symbol.wellknown.d.ds");

pub const LIB_ES2015: BuiltinLib = BuiltinLib::ambient(
    "es2015",
    &[
        LIB_ES_ES2015_COLLECTION_D_DS,
        LIB_ES_ES2015_CORE_D_DS,
        LIB_ES_ES2015_GENERATOR_D_DS,
        LIB_ES_ES2015_INDEX_D_DS,
        LIB_ES_ES2015_ITERABLE_D_DS,
        LIB_ES_ES2015_PROMISE_D_DS,
        LIB_ES_ES2015_PROXY_D_DS,
        LIB_ES_ES2015_REFLECT_D_DS,
        LIB_ES_ES2015_SYMBOL_D_DS,
        LIB_ES_ES2015_SYMBOL_WELLKNOWN_D_DS,
    ],
    &["es5"],
)
.with_canonical_exports(ES_CANONICAL_EXPORTS);

pub const LIB_ES2015_COLLECTION: BuiltinLib =
    BuiltinLib::ambient("es2015.collection", &[LIB_ES_ES2015_COLLECTION_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2015_CORE: BuiltinLib =
    BuiltinLib::ambient("es2015.core", &[LIB_ES_ES2015_CORE_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2015_GENERATOR: BuiltinLib =
    BuiltinLib::ambient("es2015.generator", &[LIB_ES_ES2015_GENERATOR_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2015_ITERABLE: BuiltinLib =
    BuiltinLib::ambient("es2015.iterable", &[LIB_ES_ES2015_ITERABLE_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2015_PROMISE: BuiltinLib =
    BuiltinLib::ambient("es2015.promise", &[LIB_ES_ES2015_PROMISE_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2015_PROXY: BuiltinLib =
    BuiltinLib::ambient("es2015.proxy", &[LIB_ES_ES2015_PROXY_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2015_REFLECT: BuiltinLib =
    BuiltinLib::ambient("es2015.reflect", &[LIB_ES_ES2015_REFLECT_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2015_SYMBOL: BuiltinLib =
    BuiltinLib::ambient("es2015.symbol", &[LIB_ES_ES2015_SYMBOL_D_DS], &[])
        .with_canonical_exports(ES_CANONICAL_EXPORTS);
pub const LIB_ES2015_SYMBOL_WELLKNOWN: BuiltinLib = BuiltinLib::ambient(
    "es2015.symbol.wellknown",
    &[LIB_ES_ES2015_SYMBOL_WELLKNOWN_D_DS],
    &[],
)
.with_canonical_exports(ES_CANONICAL_EXPORTS);
