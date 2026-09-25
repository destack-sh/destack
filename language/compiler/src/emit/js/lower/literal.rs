use tspp_dir as dir;
use tspp_js as js;

use crate::emit::js::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower a scalar literal from DIR into JavaScript.
    pub(crate) fn lower_scalar_literal(&mut self, literal: &dir::Literal) -> js::Literal {
        match literal {
            dir::Literal::Null => js::Literal::Null,
            dir::Literal::Undefined => js::Literal::Undefined,
            dir::Literal::Boolean(boolean) => js::Literal::Boolean(*boolean),
            dir::Literal::Integer(integer) => js::Literal::Number(*integer as f64),
            dir::Literal::Bigint(bigint) => js::Literal::Number(*bigint as f64),
            dir::Literal::Float(float) => js::Literal::Number(*float),
            dir::Literal::Character(character) => {
                let character_text = character.to_string();
                let string = self.strings.intern(&character_text);
                js::Literal::String(string)
            }
            dir::Literal::String(string) => {
                let string = *string;
                js::Literal::String(string)
            }
            dir::Literal::RegexString { content, flags } => {
                let content = *content;
                let flags = flags.map(|flag| flag);
                js::Literal::RegexString { content, flags }
            }
        }
    }
}
