use tspp_dir as dir;

use crate::{DocError, DocResult};

use super::{Generator, Module};

impl Module<'_> {
    /// Return the display name of one declaration.
    pub(crate) fn declaration_display_name(
        &self,
        declaration: &dir::Declaration,
    ) -> Option<String> {
        declaration
            .name()
            .map(|name| self.strings().get(name.string()).to_string())
    }

    /// Return the exact definition member carried by one indexed symbol.
    pub(crate) fn definition_member(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> DocResult<
        Option<(
            dir::GlobalSymbolId,
            &dir::Definition,
            &dir::DefinitionMember,
        )>,
    > {
        if symbol.module_id != self.module_id() {
            return Err(DocError::invalid(format!(
                "local member symbol: {symbol:?}, {:?}",
                self.module_id()
            )));
        }

        // select the declaring definition through the compiler member index
        let Some(entry) = self.members().symbol_entry(symbol) else {
            return Ok(None);
        };
        let declaring = entry.declaring;
        let definition = self.definitions().definition(declaring).ok_or_else(|| {
            DocError::missing(format!(
                "member declaring definition: {symbol:?}, {declaring:?}"
            ))
        })?;
        let member = definition.member(symbol).ok_or_else(|| {
            DocError::missing(format!(
                "indexed definition member: {symbol:?}, {declaring:?}"
            ))
        })?;

        Ok(Some((declaring, definition, member)))
    }
}

impl Generator<'_> {
    /// Return the name of one symbol when it has one.
    pub(crate) fn symbol_name(&self, symbol_id: dir::GlobalSymbolId) -> DocResult<Option<String>> {
        let module = self.module(symbol_id.module_id)?;
        let symbol = module.bindings().get_symbol(symbol_id.local_id);

        Ok(symbol
            .name()
            .map(|name| module.strings().get(name).to_string()))
    }
}
