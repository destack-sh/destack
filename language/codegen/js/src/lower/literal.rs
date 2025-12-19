use crate::{ModuleLowerer, ScalarLiteral};
use destack_dir as dir;

impl ModuleLowerer<'_> {
    /// Lower a scalar literal from DIR into JS AST.
    pub fn lower_scalar_literal(&mut self, literal: &dir::ScalarLiteral) -> ScalarLiteral {
        match literal {
            dir::ScalarLiteral::Boolean(boolean) => ScalarLiteral::Boolean(*boolean),
            dir::ScalarLiteral::Integer(integer) => ScalarLiteral::Number(*integer as f64),
            dir::ScalarLiteral::Bigint(bigint) => ScalarLiteral::Number(*bigint as f64),
            dir::ScalarLiteral::Float(float) => ScalarLiteral::Number(*float),
            dir::ScalarLiteral::Character(character) => {
                let string = self.strings.intern(character.to_string());
                ScalarLiteral::String(string)
            }
            dir::ScalarLiteral::String(string) => {
                let string = self.strings.intern_from(&self.ast.strings, *string);
                ScalarLiteral::String(string)
            }
            dir::ScalarLiteral::RegexString { content, flags } => {
                let content = self.strings.intern_from(&self.ast.strings, *content);
                let flags = flags.map(|flag| self.strings.intern_from(&self.ast.strings, flag));
                ScalarLiteral::RegexString { content, flags }
            }
        }
    }
}
