use destack_dir as dir;
use destack_repository::ProviderError;

use super::{Dir, DirModule};

impl Dir<'_> {
    /// Return the canonical language member declared by one symbol.
    pub fn language_member(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<Option<dir::LanguageMember>, ProviderError> {
        self.read_declaration_tables(symbol.module_id, |bindings, definitions| {
            // read the selected member declaration
            let declaration = bindings.get_symbol_maybe(symbol.local_id).ok_or_else(|| {
                ProviderError::internal(format!(
                    "selected member symbol {symbol:?} is absent from its binding table"
                ))
            })?;
            let Some(key) = declaration.key else {
                return Ok(None);
            };

            // select the declaration that owns the member symbol
            let owner = bindings.symbol_owner(symbol.local_id).ok_or_else(|| {
                ProviderError::internal(format!(
                    "selected member symbol {symbol:?} has no owning declaration"
                ))
            })?;
            let owner = owner.into_global(symbol.module_id);

            // resolve inherent extension members to their receiver declaration
            let owner = match definitions.extension_definition(owner) {
                Some(dir::ExtensionDefinition {
                    symbol: extension_symbol,
                    target: dir::ExtensionTarget::Rooted { root, .. },
                    ..
                }) if extension_symbol.module_id == root.module_id => *root,
                Some(_) => return Ok(None),
                None => owner,
            };
            let Some(owner) = self.environment.language.item(owner) else {
                return Ok(None);
            };

            Ok(Some(dir::LanguageMember { owner, key }))
        })
    }
}

impl DirModule<'_> {
    /// Return the canonical language member containing one node.
    pub fn enclosing_language_member(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Result<Option<dir::LanguageMember>, ProviderError> {
        // select the member that contains the node
        let Some(member) = self.view().ancestor::<dir::Member>(node) else {
            return Ok(None);
        };

        // resolve the member's canonical language identity
        let member = member.into_global_any(self.id);
        let symbol = self.bindings.declaration_symbol(member).ok_or_else(|| {
            ProviderError::internal(format!(
                "checked member {member:?} has no declaration symbol"
            ))
        })?;
        let symbol = symbol.into_global(self.id);

        self.dir.language_member(symbol)
    }

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
                let Some(resolution) = self.call_resolution(expression)? else {
                    return Ok(None);
                };
                let symbols = resolution.iter().map(|call| call.target.symbol());

                self.selected_language_member(symbols)
            }

            // property reads retain one selected member for every runtime arm
            dir::Expression::Member { .. } => {
                let Some(resolution) = self.member_resolution(expression)? else {
                    return Ok(None);
                };
                let symbols = resolution.iter().map(|access| access.target.symbol());

                self.selected_language_member(symbols)
            }
            _ => Ok(None),
        }
    }

    /// Return the canonical language member selected by one operator expression.
    pub fn operator_language_member(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::LanguageMember>, ProviderError> {
        let Some(resolution) = self.operator_resolution(expression.into_any())? else {
            return Ok(None);
        };
        let symbols = resolution
            .iter()
            .map(|application| application.call().and_then(|call| call.target.symbol()));

        self.selected_language_member(symbols)
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
