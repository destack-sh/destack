use super::super::super::source::{BuiltinLib, BuiltinLibSource};

pub(crate) const ES2021_DECLARED_SYMBOLS: &[&str] = &[
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
    "AggregateError",
];

const ES2021_PROMISE_DECLARED_SYMBOLS: &[&str] = &["AggregateError"];
const ES2021_WEAKREF_DECLARED_SYMBOLS: &[&str] = &["WeakRef", "FinalizationRegistry"];
const ES2021_EMPTY_DECLARED_SYMBOLS: &[&str] = &[];

macro_rules! lib_source {
    ($name:ident, $file:literal) => {
        const $name: BuiltinLibSource = BuiltinLibSource::new(
            "lib",
            "es/es2021",
            $file,
            include_str!(concat!("../../../../lib/es/es2021/", $file)),
        );
    };
}

lib_source!(LIB_ES_ES2021_INTL_D_DS, "intl.d.ts");
lib_source!(LIB_ES_ES2021_FULL_D_DS, "full.d.ts");
lib_source!(LIB_ES_ES2021_INDEX_D_DS, "index.d.ts");
lib_source!(LIB_ES_ES2021_PROMISE_D_DS, "promise.d.ts");
lib_source!(LIB_ES_ES2021_STRING_D_DS, "string.d.ts");
lib_source!(LIB_ES_ES2021_WEAKREF_D_DS, "weakref.d.ts");

pub const LIB_ES2021: BuiltinLib = BuiltinLib::ambient_lib(
    "es2021",
    &[
        LIB_ES_ES2021_INDEX_D_DS,
        LIB_ES_ES2021_INTL_D_DS,
        LIB_ES_ES2021_PROMISE_D_DS,
        LIB_ES_ES2021_STRING_D_DS,
        LIB_ES_ES2021_WEAKREF_D_DS,
    ],
    &[],
)
.with_declared_symbols(ES2021_DECLARED_SYMBOLS);

pub const LIB_ES2021_INTL: BuiltinLib =
    BuiltinLib::ambient_lib("es2021.intl", &[LIB_ES_ES2021_INTL_D_DS], &[])
        .with_declared_symbols(ES2021_EMPTY_DECLARED_SYMBOLS);
pub const LIB_ES2021_FULL: BuiltinLib =
    BuiltinLib::ambient_lib("es2021.full", &[LIB_ES_ES2021_FULL_D_DS], &[])
        .with_declared_symbols(ES2021_DECLARED_SYMBOLS);
pub const LIB_ES2021_PROMISE: BuiltinLib =
    BuiltinLib::ambient_lib("es2021.promise", &[LIB_ES_ES2021_PROMISE_D_DS], &[])
        .with_declared_symbols(ES2021_PROMISE_DECLARED_SYMBOLS);
pub const LIB_ES2021_STRING: BuiltinLib =
    BuiltinLib::ambient_lib("es2021.string", &[LIB_ES_ES2021_STRING_D_DS], &[])
        .with_declared_symbols(ES2021_EMPTY_DECLARED_SYMBOLS);
pub const LIB_ES2021_WEAKREF: BuiltinLib =
    BuiltinLib::ambient_lib("es2021.weakref", &[LIB_ES_ES2021_WEAKREF_D_DS], &[])
        .with_declared_symbols(ES2021_WEAKREF_DECLARED_SYMBOLS);
