use crate::{BuiltinLib, BuiltinLibSource};

pub(crate) const ES2020_DECLARED_SYMBOLS: &[&str] = &[
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
    "SharedArrayBuffer",
    "AsyncGenerator",
    "AsyncIterable",
    "AsyncIterator",
    "BigInt",
    "BigInt64Array",
    "BigUint64Array",
    "globalThis",
];

const ES2020_BIGINT_DECLARED_SYMBOLS: &[&str] = &["BigInt", "BigInt64Array", "BigUint64Array"];
const ES2020_EMPTY_DECLARED_SYMBOLS: &[&str] = &[];

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

pub const LIB_ES2020: BuiltinLib = BuiltinLib::ambient_lib(
    "es2020",
    &[
        LIB_ES_ES2020_INTL_D_DS,
        LIB_ES_ES2020_BIGINT_D_DS,
        LIB_ES_ES2020_DATE_D_DS,
        LIB_ES_ES2020_INDEX_D_DS,
        LIB_ES_ES2020_NUMBER_D_DS,
        LIB_ES_ES2020_PROMISE_D_DS,
        LIB_ES_ES2020_SHAREDMEMORY_D_DS,
        LIB_ES_ES2020_STRING_D_DS,
        LIB_ES_ES2020_SYMBOL_WELLKNOWN_D_DS,
    ],
    &["es2019", "es2018.intl"],
)
.with_declared_symbols(ES2020_DECLARED_SYMBOLS);

pub const LIB_ES2020_BIGINT: BuiltinLib = BuiltinLib::ambient_lib(
    "es2020.bigint",
    &[LIB_ES_ES2020_BIGINT_D_DS],
    &["es2020.intl"],
)
.with_declared_symbols(ES2020_BIGINT_DECLARED_SYMBOLS);
pub const LIB_ES2020_DATE: BuiltinLib =
    BuiltinLib::ambient_lib("es2020.date", &[LIB_ES_ES2020_DATE_D_DS], &["es2020.intl"])
        .with_declared_symbols(ES2020_EMPTY_DECLARED_SYMBOLS);
pub const LIB_ES2020_FULL: BuiltinLib = BuiltinLib::ambient_lib(
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
.with_declared_symbols(ES2020_DECLARED_SYMBOLS);
pub const LIB_ES2020_INTL: BuiltinLib =
    BuiltinLib::ambient_lib("es2020.intl", &[LIB_ES_ES2020_INTL_D_DS], &["es2018.intl"])
        .with_declared_symbols(ES2020_EMPTY_DECLARED_SYMBOLS);
pub const LIB_ES2020_NUMBER: BuiltinLib = BuiltinLib::ambient_lib(
    "es2020.number",
    &[LIB_ES_ES2020_NUMBER_D_DS],
    &["es2020.intl"],
)
.with_declared_symbols(ES2020_EMPTY_DECLARED_SYMBOLS);
pub const LIB_ES2020_PROMISE: BuiltinLib =
    BuiltinLib::ambient_lib("es2020.promise", &[LIB_ES_ES2020_PROMISE_D_DS], &[])
        .with_declared_symbols(ES2020_EMPTY_DECLARED_SYMBOLS);
pub const LIB_ES2020_SHAREDMEMORY: BuiltinLib = BuiltinLib::ambient_lib(
    "es2020.sharedmemory",
    &[LIB_ES_ES2020_SHAREDMEMORY_D_DS],
    &[],
)
.with_declared_symbols(ES2020_EMPTY_DECLARED_SYMBOLS);
pub const LIB_ES2020_STRING: BuiltinLib = BuiltinLib::ambient_lib(
    "es2020.string",
    &[LIB_ES_ES2020_STRING_D_DS],
    &["es2020.intl"],
)
.with_declared_symbols(ES2020_EMPTY_DECLARED_SYMBOLS);
pub const LIB_ES2020_SYMBOL_WELLKNOWN: BuiltinLib = BuiltinLib::ambient_lib(
    "es2020.symbol.wellknown",
    &[LIB_ES_ES2020_SYMBOL_WELLKNOWN_D_DS],
    &[],
)
.with_declared_symbols(ES2020_EMPTY_DECLARED_SYMBOLS);
