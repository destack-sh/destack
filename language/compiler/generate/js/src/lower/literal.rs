use destack_dir as dir;
use destack_js as js;

use crate::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower a scalar literal from DIR into JS AST.
    pub fn lower_scalar_literal(&mut self, literal: &dir::ScalarLiteral) -> js::ScalarLiteral {
        match literal {
            dir::ScalarLiteral::Null => js::ScalarLiteral::Null,
            dir::ScalarLiteral::Undefined => js::ScalarLiteral::Undefined,
            dir::ScalarLiteral::Boolean(boolean) => js::ScalarLiteral::Boolean(*boolean),
            dir::ScalarLiteral::Integer(integer) => js::ScalarLiteral::Number(*integer as f64),
            dir::ScalarLiteral::Bigint(bigint) => js::ScalarLiteral::Number(*bigint as f64),
            dir::ScalarLiteral::Float(float) => js::ScalarLiteral::Number(*float),
            dir::ScalarLiteral::Character(character) => {
                let character_text = character.to_string();
                let string = self.strings.intern(&character_text);
                js::ScalarLiteral::String(string)
            }
            dir::ScalarLiteral::String(string) => {
                let string = *string;
                js::ScalarLiteral::String(string)
            }
            dir::ScalarLiteral::RegexString { content, flags } => {
                let content = *content;
                let flags = flags.map(|flag| flag);
                js::ScalarLiteral::RegexString { content, flags }
            }
        }
    }
}
