use super::super::super::source::{BuiltinLib, BuiltinLibSource};

pub(crate) const ES5_DECLARED_SYMBOLS: &[&str] = &[
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
    "Record",
    "RangeError",
    "ReferenceError",
    "RegExp",
    "String",
    "SyntaxError",
    "TypeError",
    "TemplateStringsArray",
];

const LIB_ES_ES5_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "es/es5",
    "index.d.ts",
    include_str!(concat!("../../../../lib/es/es5/index.d.ts")),
);

pub const LIB_ES5: BuiltinLib = BuiltinLib::ambient_lib("es5", &[LIB_ES_ES5_INDEX_D_DS], &[])
    .with_declared_symbols(ES5_DECLARED_SYMBOLS);
