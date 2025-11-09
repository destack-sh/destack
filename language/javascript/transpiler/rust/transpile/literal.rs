use crate::{Transpiler, TranspilerUnit};

use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::ScalarLiteral;

impl<'a> Transpiler<'a> {
    /// Transpile a scalar literal from DIR into JS AST.
    pub fn transpile_scalar_literal(
        &self,
        _module: &Module,
        literal: &dir::ScalarLiteral,
        _unit: &mut TranspilerUnit,
    ) -> ScalarLiteral {
        match literal {
            dir::ScalarLiteral::Boolean(boolean) => ScalarLiteral::Boolean(*boolean),
            dir::ScalarLiteral::Byte(byte) => ScalarLiteral::Number(*byte as f64),
            dir::ScalarLiteral::Integer(integer) => ScalarLiteral::Number(*integer as f64),
            dir::ScalarLiteral::Bigint(bigint) => ScalarLiteral::Number(*bigint as f64),
            dir::ScalarLiteral::Float(float) => ScalarLiteral::Number(*float),
            dir::ScalarLiteral::String(string) => ScalarLiteral::String(*string),
            dir::ScalarLiteral::RegexString { content, flags } => ScalarLiteral::RegexString {
                content: *content,
                flags: *flags,
            },
            _ => panic!("unsupposed literal {literal:?}"),
        }
    }
}
