use destack_dir as dir;

use super::Formatter;

impl Formatter<'_, '_, '_> {
    /// Format one scalar literal.
    pub(super) fn literal(&self, literal: dir::ScalarLiteral) -> String {
        match literal {
            dir::ScalarLiteral::Null => "null".to_string(),
            dir::ScalarLiteral::Undefined => "undefined".to_string(),
            dir::ScalarLiteral::Boolean(value) => value.to_string(),
            dir::ScalarLiteral::Integer(value) => value.to_string(),
            dir::ScalarLiteral::Bigint(value) => format!("{value}n"),
            dir::ScalarLiteral::Float(value) => value.to_string(),
            dir::ScalarLiteral::Character(value) => {
                let value = value.escape_default();

                format!("'{value}'")
            }
            dir::ScalarLiteral::String(value) => quote_string(self.module.strings().get(value)),
            dir::ScalarLiteral::RegexString { content, flags } => {
                let content = self.module.strings().get(content);
                let flags = match flags {
                    Some(flags) => self.module.strings().get(flags),
                    None => "",
                };

                format!("/{content}/{flags}")
            }
        }
    }
}

/// Quote one string as source text.
pub(super) fn quote_string(value: &str) -> String {
    let escaped = value.escape_default().to_string();

    format!("\"{escaped}\"")
}
