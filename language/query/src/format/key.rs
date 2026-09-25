use tspp_dir as dir;

use crate::{QueryError, QueryResult};

use super::Formatter;
use super::literal::quote_string;

impl Formatter<'_, '_, '_> {
    /// Format one definition member name.
    pub(crate) fn member_name(&self, member: &dir::DefinitionMember) -> QueryResult<String> {
        if let Some(key) = member.key() {
            return Ok(self.property_key(key));
        }

        let name = match member {
            dir::DefinitionMember::Method(method) => match method.slot {
                dir::MemberSlot::Constructor => "constructor",
                dir::MemberSlot::New => "new",
                dir::MemberSlot::Call => "call",
                dir::MemberSlot::Key(_) => {
                    return Err(QueryError::invalid("member key"));
                }
            },
            dir::DefinitionMember::CallSignature(_) => "call",
            dir::DefinitionMember::ConstructSignature(_) => "new",
            dir::DefinitionMember::IndexSignature(_) => "[]",
            _ => return Err(QueryError::missing("member name")),
        };

        Ok(name.to_string())
    }

    /// Format one exact static key type.
    pub(super) fn key_type(&self, key: dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Name(name) => quote_string(self.module.strings().get(name)),
            dir::StaticKey::Index(index) => index.to_string(),
        }
    }

    /// Format one static key in a property declaration.
    pub(crate) fn property_key(&self, key: dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Name(name) => {
                let name = self.module.strings().get(name);
                if dir::is_identifier_compat(name) || name.parse::<dir::Keyword>().is_ok() {
                    name.to_string()
                } else {
                    quote_string(name)
                }
            }
            dir::StaticKey::Index(index) => index.to_string(),
        }
    }

    /// Format one static key as a member access suffix.
    pub(super) fn member_key(&self, key: dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Name(name) => {
                let name = self.module.strings().get(name);
                if dir::is_identifier_compat(name) || name.parse::<dir::Keyword>().is_ok() {
                    format!(".{name}")
                } else {
                    format!("[{}]", quote_string(name))
                }
            }
            dir::StaticKey::Index(index) => format!("[{index}]"),
        }
    }
}
