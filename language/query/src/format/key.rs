use destack_dir as dir;

use crate::QueryResult;

use super::literal::quote_string;
use super::{Formatter, formatted};

impl Formatter<'_, '_, '_> {
    /// Format one exact static key type.
    pub(super) fn key_type(&self, key: dir::StaticKey) -> QueryResult<Option<String>> {
        let text = match key {
            dir::StaticKey::Name(name) => quote_string(self.module.strings().get(name)),
            dir::StaticKey::Index(index) => index.to_string(),
            dir::StaticKey::Symbol(symbol) => formatted!(self.symbol_key(symbol)),
        };

        Ok(Some(text))
    }

    /// Format one static key in a property declaration.
    pub(super) fn property_key(&self, key: dir::StaticKey) -> QueryResult<Option<String>> {
        let text = match key {
            dir::StaticKey::Name(name) => {
                let name = self.module.strings().get(name);
                if dir::is_identifier_compat(name) || name.parse::<dir::Keyword>().is_ok() {
                    name.to_string()
                } else {
                    quote_string(name)
                }
            }
            dir::StaticKey::Index(index) => index.to_string(),
            dir::StaticKey::Symbol(symbol) => {
                let symbol = formatted!(self.symbol_key(symbol));

                format!("[{symbol}]")
            }
        };

        Ok(Some(text))
    }

    /// Format one static key as a member access suffix.
    pub(super) fn member_key(&self, key: dir::StaticKey) -> QueryResult<Option<String>> {
        let text = match key {
            dir::StaticKey::Name(name) => {
                let name = self.module.strings().get(name);
                if dir::is_identifier_compat(name) || name.parse::<dir::Keyword>().is_ok() {
                    format!(".{name}")
                } else {
                    format!("[{}]", quote_string(name))
                }
            }
            dir::StaticKey::Index(index) => format!("[{index}]"),
            dir::StaticKey::Symbol(symbol) => {
                let symbol = formatted!(self.symbol_key(symbol));

                format!("[{symbol}]")
            }
        };

        Ok(Some(text))
    }

    /// Format one static symbol key expression.
    fn symbol_key(&self, key: dir::SymbolKey) -> QueryResult<Option<String>> {
        let text = match key {
            dir::SymbolKey::Unique(symbol) => formatted!(self.unique_symbol(symbol)),
            dir::SymbolKey::Registry(name) => {
                let name = quote_string(self.module.strings().get(name));

                format!("Symbol.for({name})")
            }
        };

        Ok(Some(text))
    }
}
