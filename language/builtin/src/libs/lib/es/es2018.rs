use super::super::super::source::{BuiltinLib, BuiltinLibSource};

pub(crate) const ES2018_DECLARED_SYMBOLS: &[&str] = &[
    "Array",
    "Boolean",
    "Date",
    "Error",
    "EvalError",
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
];

const ES2018_ASYNCGENERATOR_DECLARED_SYMBOLS: &[&str] = &["AsyncGenerator"];
const ES2018_ASYNCITERABLE_DECLARED_SYMBOLS: &[&str] = &["AsyncIterable", "AsyncIterator"];
const ES2018_EMPTY_DECLARED_SYMBOLS: &[&str] = &[];

macro_rules! lib_source {
    ($name:ident, $file:literal) => {
        const $name: BuiltinLibSource = BuiltinLibSource::new(
            "lib",
            "es/es2018",
            $file,
            include_str!(concat!("../../../../lib/es/es2018/", $file)),
        );
    };
}

lib_source!(LIB_ES_ES2018_ASYNCGENERATOR_D_DS, "asyncgenerator.d.ts");
lib_source!(LIB_ES_ES2018_ASYNCITERABLE_D_DS, "asynciterable.d.ts");
lib_source!(LIB_ES_ES2018_FULL_D_DS, "full.d.ts");
lib_source!(LIB_ES_ES2018_INDEX_D_DS, "index.d.ts");
lib_source!(LIB_ES_ES2018_INTL_D_DS, "intl.d.ts");
lib_source!(LIB_ES_ES2018_PROMISE_D_DS, "promise.d.ts");
lib_source!(LIB_ES_ES2018_REGEXP_D_DS, "regexp.d.ts");

pub const LIB_ES2018: BuiltinLib = BuiltinLib::ambient_lib(
    "es2018",
    &[
        LIB_ES_ES2018_ASYNCGENERATOR_D_DS,
        LIB_ES_ES2018_ASYNCITERABLE_D_DS,
        LIB_ES_ES2018_INDEX_D_DS,
        LIB_ES_ES2018_INTL_D_DS,
        LIB_ES_ES2018_PROMISE_D_DS,
        LIB_ES_ES2018_REGEXP_D_DS,
    ],
    &["es2017"],
)
.with_declared_symbols(ES2018_DECLARED_SYMBOLS);

pub const LIB_ES2018_ASYNCGENERATOR: BuiltinLib = BuiltinLib::ambient_lib(
    "es2018.asyncgenerator",
    &[LIB_ES_ES2018_ASYNCGENERATOR_D_DS],
    &[],
)
    .with_declared_symbols(ES2018_ASYNCGENERATOR_DECLARED_SYMBOLS);
pub const LIB_ES2018_ASYNCITERABLE: BuiltinLib = BuiltinLib::ambient_lib(
    "es2018.asynciterable",
    &[LIB_ES_ES2018_ASYNCITERABLE_D_DS],
    &[],
)
    .with_declared_symbols(ES2018_ASYNCITERABLE_DECLARED_SYMBOLS);
pub const LIB_ES2018_FULL: BuiltinLib = BuiltinLib::ambient_lib(
    "es2018.full",
    &[LIB_ES_ES2018_FULL_D_DS],
    &[
        "es2018",
        "dom",
        "worker.importscripts",
        "scripthost",
        "dom.iterable",
        "dom.asynciterable",
    ],
)
.with_declared_symbols(ES2018_DECLARED_SYMBOLS);
pub const LIB_ES2018_INTL: BuiltinLib =
    BuiltinLib::ambient_lib("es2018.intl", &[LIB_ES_ES2018_INTL_D_DS], &[])
        .with_declared_symbols(ES2018_EMPTY_DECLARED_SYMBOLS);
pub const LIB_ES2018_PROMISE: BuiltinLib =
    BuiltinLib::ambient_lib("es2018.promise", &[LIB_ES_ES2018_PROMISE_D_DS], &[])
        .with_declared_symbols(ES2018_EMPTY_DECLARED_SYMBOLS);
pub const LIB_ES2018_REGEXP: BuiltinLib =
    BuiltinLib::ambient_lib("es2018.regexp", &[LIB_ES_ES2018_REGEXP_D_DS], &[])
        .with_declared_symbols(ES2018_EMPTY_DECLARED_SYMBOLS);
