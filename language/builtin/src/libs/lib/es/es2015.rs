use super::super::super::source::{BuiltinLib, BuiltinLibSource};

pub(crate) const ES2015_DECLARED_SYMBOLS: &[&str] = &[
    "Array",
    "ReadonlyArray",
    "Boolean",
    "Date",
    "Error",
    "EvalError",
    "eval",
    "Function",
    "JSON",
    "Math",
    "Number",
    "Object",
    "RangeError",
    "ReferenceError",
    "RegExp",
    "String",
    "SyntaxError",
    "TypeError",
    "ArrayBuffer",
    "DataView",
    "BuiltinIteratorReturn",
    "Generator",
    "Iterable",
    "Iterator",
    "Map",
    "Promise",
    "Proxy",
    "Reflect",
    "Set",
    "Symbol",
    "WeakMap",
    "WeakSet",
];

const ES2015_COLLECTION_DECLARED_SYMBOLS: &[&str] = &["Map", "Set", "WeakMap", "WeakSet"];
const ES2015_GENERATOR_DECLARED_SYMBOLS: &[&str] = &["Generator"];
const ES2015_ITERABLE_DECLARED_SYMBOLS: &[&str] =
    &["BuiltinIteratorReturn", "Iterable", "Iterator"];
const ES2015_PROMISE_DECLARED_SYMBOLS: &[&str] = &["Promise"];
const ES2015_PROXY_DECLARED_SYMBOLS: &[&str] = &["Proxy"];
const ES2015_REFLECT_DECLARED_SYMBOLS: &[&str] = &["Reflect"];
const ES2015_SYMBOL_DECLARED_SYMBOLS: &[&str] = &["Symbol"];
const ES2015_EMPTY_DECLARED_SYMBOLS: &[&str] = &[];

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

lib_source!(LIB_ES_ES2015_COLLECTION_D_DS, "collection.d.ts");
lib_source!(LIB_ES_ES2015_CORE_D_DS, "core.d.ts");
lib_source!(LIB_ES_ES2015_GENERATOR_D_DS, "generator.d.ts");
lib_source!(LIB_ES_ES2015_INDEX_D_DS, "index.d.ts");
lib_source!(LIB_ES_ES2015_ITERABLE_D_DS, "iterable.d.ts");
lib_source!(LIB_ES_ES2015_PROMISE_D_DS, "promise.d.ts");
lib_source!(LIB_ES_ES2015_PROXY_D_DS, "proxy.d.ts");
lib_source!(LIB_ES_ES2015_REFLECT_D_DS, "reflect.d.ts");
lib_source!(LIB_ES_ES2015_SYMBOL_D_DS, "symbol.d.ts");
lib_source!(LIB_ES_ES2015_SYMBOL_WELLKNOWN_D_DS, "symbol.wellknown.d.ts");

pub const LIB_ES2015: BuiltinLib = BuiltinLib::ambient_lib(
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
.with_declared_symbols(ES2015_DECLARED_SYMBOLS);

pub const LIB_ES2015_COLLECTION: BuiltinLib =
    BuiltinLib::ambient_lib("es2015.collection", &[LIB_ES_ES2015_COLLECTION_D_DS], &[])
        .with_declared_symbols(ES2015_COLLECTION_DECLARED_SYMBOLS);
pub const LIB_ES2015_CORE: BuiltinLib =
    BuiltinLib::ambient_lib("es2015.core", &[LIB_ES_ES2015_CORE_D_DS], &[])
        .with_declared_symbols(ES2015_EMPTY_DECLARED_SYMBOLS);
pub const LIB_ES2015_GENERATOR: BuiltinLib =
    BuiltinLib::ambient_lib("es2015.generator", &[LIB_ES_ES2015_GENERATOR_D_DS], &[])
        .with_declared_symbols(ES2015_GENERATOR_DECLARED_SYMBOLS);
pub const LIB_ES2015_ITERABLE: BuiltinLib =
    BuiltinLib::ambient_lib("es2015.iterable", &[LIB_ES_ES2015_ITERABLE_D_DS], &[])
        .with_declared_symbols(ES2015_ITERABLE_DECLARED_SYMBOLS);
pub const LIB_ES2015_PROMISE: BuiltinLib =
    BuiltinLib::ambient_lib("es2015.promise", &[LIB_ES_ES2015_PROMISE_D_DS], &[])
        .with_declared_symbols(ES2015_PROMISE_DECLARED_SYMBOLS);
pub const LIB_ES2015_PROXY: BuiltinLib =
    BuiltinLib::ambient_lib("es2015.proxy", &[LIB_ES_ES2015_PROXY_D_DS], &[])
        .with_declared_symbols(ES2015_PROXY_DECLARED_SYMBOLS);
pub const LIB_ES2015_REFLECT: BuiltinLib =
    BuiltinLib::ambient_lib("es2015.reflect", &[LIB_ES_ES2015_REFLECT_D_DS], &[])
        .with_declared_symbols(ES2015_REFLECT_DECLARED_SYMBOLS);
pub const LIB_ES2015_SYMBOL: BuiltinLib =
    BuiltinLib::ambient_lib("es2015.symbol", &[LIB_ES_ES2015_SYMBOL_D_DS], &[])
        .with_declared_symbols(ES2015_SYMBOL_DECLARED_SYMBOLS);
pub const LIB_ES2015_SYMBOL_WELLKNOWN: BuiltinLib = BuiltinLib::ambient_lib(
    "es2015.symbol.wellknown",
    &[LIB_ES_ES2015_SYMBOL_WELLKNOWN_D_DS],
    &[],
)
.with_declared_symbols(ES2015_EMPTY_DECLARED_SYMBOLS);
