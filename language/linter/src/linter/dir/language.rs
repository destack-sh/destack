use destack_dir as dir;
use destack_repository::ProviderError;

use super::{Dir, DirModule};

impl Dir {
    /// Return the canonical language member declared by one symbol.
    pub fn language_member(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<Option<dir::LanguageMember>, ProviderError> {
        let module = self.module(symbol.module_id)?;
        let declaration = module
            .bindings
            .get_symbol_maybe(symbol.local_id)
            .ok_or_else(|| {
                ProviderError::internal(format!(
                    "selected member symbol {symbol:?} is absent from its binding table"
                ))
            })?;
        let Some(key) = declaration.key else {
            return Ok(None);
        };

        // select the declaration that owns the member symbol
        let Some(owner) = module.bindings.symbol_owner(symbol.local_id) else {
            return Ok(None);
        };
        let owner = owner.into_global(symbol.module_id);

        // resolve inherent extension members to their receiver declaration
        let owner = match module.definitions.extension_definition(owner) {
            Some(extension) if extension.is_inherent() => {
                let Some(target) = extension.target.root() else {
                    return Ok(None);
                };

                target
            }
            Some(_) => return Ok(None),
            None => owner,
        };
        let Some(owner) = self.environment.language.item(owner) else {
            return Ok(None);
        };

        Ok(Some(dir::LanguageMember { owner, key }))
    }
}

impl DirModule<'_> {
    /// Return the canonical language item selected directly by one expression.
    pub fn language_item(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::LanguageItem>, ProviderError> {
        let item = self
            .symbol(expression)?
            .and_then(|symbol| self.dir.environment.language.item(symbol));

        Ok(item)
    }

    /// Return the canonical language member selected by one expression.
    pub fn language_member(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::LanguageMember>, ProviderError> {
        let view = self.view();
        match view.get(expression) {
            // calls retain one selected callable for every runtime arm
            dir::Expression::Call { .. } => {
                let resolution = self.call_resolution(expression)?;
                let symbols = resolution.iter().map(|call| call.target.symbol());

                self.selected_language_member(symbols)
            }

            // property reads retain one selected member for every runtime arm
            dir::Expression::Member { .. } => {
                let resolution = self.member_resolution(expression)?;
                let symbols = resolution.iter().map(|access| access.target.symbol());

                self.selected_language_member(symbols)
            }
            _ => Ok(None),
        }
    }

    /// Return the language member shared by every selected declaration.
    fn selected_language_member(
        &self,
        symbols: impl Iterator<Item = Option<dir::GlobalSymbolId>>,
    ) -> Result<Option<dir::LanguageMember>, ProviderError> {
        let mut symbols = symbols;
        let Some(symbol) = symbols.next() else {
            return Err(ProviderError::internal(
                "checked operation resolution has no runtime alternatives",
            ));
        };
        let Some(symbol) = symbol else {
            return Ok(None);
        };
        let Some(selected) = self.dir.language_member(symbol)? else {
            return Ok(None);
        };

        // require every runtime alternative to select the same language member
        for symbol in symbols {
            let Some(symbol) = symbol else {
                return Ok(None);
            };
            let Some(member) = self.dir.language_member(symbol)? else {
                return Ok(None);
            };
            if selected != member {
                return Ok(None);
            }
        }

        Ok(Some(selected))
    }
}
