use super::super::super::source::BuiltinLib;
use super::es2015::ES2015_DECLARED_SYMBOLS;

pub const LIB_ES6: BuiltinLib =
    BuiltinLib::ambient_lib("es6", &[], &["es2015"]).with_declared_symbols(ES2015_DECLARED_SYMBOLS);
