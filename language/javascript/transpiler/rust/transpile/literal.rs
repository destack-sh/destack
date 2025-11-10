use crate::{Transpiler, TranspilerUnit};

use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::ScalarLiteral;

impl<'a> Transpiler<'a> {
    /// Transpile a scalar literal from DIR into JS AST.
    pub fn transpile_scalar_literal(
        &self,
        _module: &Module,
        literal: &dir::ScalarLiteral,
        unit: &mut TranspilerUnit,
    ) -> ScalarLiteral {
        match literal {
            dir::ScalarLiteral::Boolean(boolean) => ScalarLiteral::Boolean(*boolean),
            dir::ScalarLiteral::Byte(byte) => ScalarLiteral::Number(*byte as f64),
            dir::ScalarLiteral::Integer(integer) => ScalarLiteral::Number(*integer as f64),
            dir::ScalarLiteral::Bigint(bigint) => ScalarLiteral::Number(*bigint as f64),
            dir::ScalarLiteral::Float(float) => ScalarLiteral::Number(*float),
            dir::ScalarLiteral::Character(character) => {
                let string = unit.strings.intern(character.to_string());
                ScalarLiteral::String(string)
            }
            dir::ScalarLiteral::String(string) => {
                let string = unit.strings.intern_from(self.strings, *string);
                ScalarLiteral::String(string)
            }
            dir::ScalarLiteral::RegexString { content, flags } => {
                let content = unit.strings.intern_from(self.strings, *content);
                let flags = flags.map(|flag| unit.strings.intern_from(self.strings, flag));
                ScalarLiteral::RegexString { content, flags }
            }
            dir::ScalarLiteral::ByteString(byte_string) => {
                let string = unit.strings.intern(String::from_utf8_lossy(byte_string));
                ScalarLiteral::String(string)
            }
        }
    }
}
