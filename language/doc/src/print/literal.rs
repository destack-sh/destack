use tspp_dir as dir;

use super::Printer;

impl Printer<'_, '_, '_> {
    /// Format one scalar literal.
    pub(super) fn literal(&self, literal: dir::Literal) -> String {
        match literal {
            dir::Literal::Null => "null".to_string(),
            dir::Literal::Undefined => "undefined".to_string(),
            dir::Literal::Boolean(value) => value.to_string(),
            dir::Literal::Integer(value) => value.to_string(),
            dir::Literal::Bigint(value) => format!("{value}n"),
            dir::Literal::Float(value) => value.to_string(),
            dir::Literal::Character(value) => {
                let value = value.escape_default();

                format!("'{value}'")
            }
            dir::Literal::String(value) => quote_string(self.module.strings().get(value)),
            dir::Literal::RegexString { content, flags } => {
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
