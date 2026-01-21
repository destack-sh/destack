use super::super::super::source::{BuiltinLib, BuiltinLibSource};

pub(crate) const ES2017_DECLARED_SYMBOLS: &[&str] = &[
    "Array",
    "ReadonlyArray",
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
];

const ES2017_SHAREDMEMORY_DECLARED_SYMBOLS: &[&str] = &["SharedArrayBuffer"];
const ES2017_EMPTY_DECLARED_SYMBOLS: &[&str] = &[];

macro_rules! lib_source {
    ($name:ident, $file:literal) => {
        const $name: BuiltinLibSource = BuiltinLibSource::new(
            "lib",
            "es/es2017",
            $file,
            include_str!(concat!("../../../../lib/es/es2017/", $file)),
        );
    };
}

lib_source!(LIB_ES_ES2017_DATE_D_DS, "date.d.ts");
lib_source!(LIB_ES_ES2017_ARRAYBUFFER_D_DS, "arraybuffer.d.ts");
lib_source!(LIB_ES_ES2017_FULL_D_DS, "full.d.ts");
lib_source!(LIB_ES_ES2017_INDEX_D_DS, "index.d.ts");
lib_source!(LIB_ES_ES2017_INTL_D_DS, "intl.d.ts");
lib_source!(LIB_ES_ES2017_OBJECT_D_DS, "object.d.ts");
lib_source!(LIB_ES_ES2017_SHAREDMEMORY_D_DS, "sharedmemory.d.ts");
lib_source!(LIB_ES_ES2017_STRING_D_DS, "string.d.ts");
lib_source!(LIB_ES_ES2017_TYPEDARRAYS_D_DS, "typedarrays.d.ts");

pub const LIB_ES2017: BuiltinLib = BuiltinLib::ambient_lib(
    "es2017",
    &[
        LIB_ES_ES2017_ARRAYBUFFER_D_DS,
        LIB_ES_ES2017_DATE_D_DS,
        LIB_ES_ES2017_INDEX_D_DS,
        LIB_ES_ES2017_INTL_D_DS,
        LIB_ES_ES2017_OBJECT_D_DS,
        LIB_ES_ES2017_SHAREDMEMORY_D_DS,
        LIB_ES_ES2017_STRING_D_DS,
        LIB_ES_ES2017_TYPEDARRAYS_D_DS,
    ],
    &["es2016"],
)
.with_declared_symbols(ES2017_DECLARED_SYMBOLS);

pub const LIB_ES2017_DATE: BuiltinLib =
    BuiltinLib::ambient_lib("es2017.date", &[LIB_ES_ES2017_DATE_D_DS], &[])
        .with_declared_symbols(ES2017_EMPTY_DECLARED_SYMBOLS);
pub const LIB_ES2017_ARRAYBUFFER: BuiltinLib =
    BuiltinLib::ambient_lib("es2017.arraybuffer", &[LIB_ES_ES2017_ARRAYBUFFER_D_DS], &[])
        .with_declared_symbols(ES2017_EMPTY_DECLARED_SYMBOLS);
pub const LIB_ES2017_FULL: BuiltinLib = BuiltinLib::ambient_lib(
    "es2017.full",
    &[LIB_ES_ES2017_FULL_D_DS],
    &[
        "es2017",
        "dom",
        "worker.importscripts",
        "scripthost",
        "dom.iterable",
    ],
)
.with_declared_symbols(ES2017_DECLARED_SYMBOLS);
pub const LIB_ES2017_INTL: BuiltinLib =
    BuiltinLib::ambient_lib("es2017.intl", &[LIB_ES_ES2017_INTL_D_DS], &[])
        .with_declared_symbols(ES2017_EMPTY_DECLARED_SYMBOLS);
pub const LIB_ES2017_OBJECT: BuiltinLib =
    BuiltinLib::ambient_lib("es2017.object", &[LIB_ES_ES2017_OBJECT_D_DS], &[])
        .with_declared_symbols(ES2017_EMPTY_DECLARED_SYMBOLS);
pub const LIB_ES2017_SHAREDMEMORY: BuiltinLib = BuiltinLib::ambient_lib(
    "es2017.sharedmemory",
    &[LIB_ES_ES2017_SHAREDMEMORY_D_DS],
    &[],
)
.with_declared_symbols(ES2017_SHAREDMEMORY_DECLARED_SYMBOLS);
pub const LIB_ES2017_STRING: BuiltinLib =
    BuiltinLib::ambient_lib("es2017.string", &[LIB_ES_ES2017_STRING_D_DS], &[])
        .with_declared_symbols(ES2017_EMPTY_DECLARED_SYMBOLS);
pub const LIB_ES2017_TYPEDARRAYS: BuiltinLib =
    BuiltinLib::ambient_lib("es2017.typedarrays", &[LIB_ES_ES2017_TYPEDARRAYS_D_DS], &[])
        .with_declared_symbols(ES2017_EMPTY_DECLARED_SYMBOLS);
