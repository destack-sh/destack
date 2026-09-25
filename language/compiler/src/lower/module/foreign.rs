use tspp_dir as dir;

use crate::CompilerResult;
use crate::lower::{DeclaredMethod, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Return the canonical name of one callable, a member's after its owner.
    pub(in crate::lower) fn callable_path(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<String> {
        match self.imported_member(symbol)? {
            Some(member) => {
                let name = self.member_extern_name(symbol, member.owner, member.role)?;

                self.qualified_name(symbol.module_id, &name)
            }
            None => self.symbol_path(symbol),
        }
    }

    /// Return the interface declaring one member symbol as a requirement.
    pub(in crate::lower) fn requirement_owner(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let Some(member) = self.imported_member(symbol)? else {
            return Ok(None);
        };
        let is_interface = matches!(
            self.definition(member.owner)?,
            Some(dir::Definition::Interface(_))
        );

        Ok(is_interface.then_some(member.owner))
    }

    /// Return whether one member symbol is declared static.
    pub(in crate::lower) fn is_static_member(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        Ok(self
            .imported_member(symbol)?
            .is_some_and(|member| member.is_static))
    }

    /// Return the definition member one symbol declares, absent for a free callable.
    pub(in crate::lower) fn imported_member(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<DeclaredMethod>> {
        Ok(self.state(symbol.module_id)?.declared_method(symbol))
    }
}
