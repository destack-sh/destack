use super::super::super::source::BuiltinLib;
use super::ES_CANONICAL_EXPORTS;

pub const LIB_ES6: BuiltinLib =
    BuiltinLib::ambient("es6", &[], &["es2015"]).with_canonical_exports(ES_CANONICAL_EXPORTS);
