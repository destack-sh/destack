use destack_dir as dir;
use destack_repository::ProviderError;

use super::{Dir, DirModule};

impl Dir<'_> {
    /// Return the canonical language item represented by one checked type.
    pub fn representation_item(
        &self,
        type_id: dir::GlobalTypeId,
    ) -> Result<Option<dir::LanguageItem>, ProviderError> {
        // inspect the represented value beneath placement forms
        let type_id = self.strip_form(type_id)?;
        let ty = self.get_type(type_id)?;

        // inspect the erased constraint carried by dynamic representations
        if let dir::Type::Dynamic(dynamic) = ty {
            return self.representation_item(dynamic.constraint);
        }

        // recognize compiler-defined and nominal language representations
        let item = ty.representation_item().or_else(|| {
            ty.symbol()
                .and_then(|symbol| self.environment.language.item(symbol))
        });

        Ok(item)
    }

    /// Return the canonical language member selected by one symbol.
    pub fn language_member(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<Option<dir::LanguageMember>, ProviderError> {
        let (direct, requirements) =
            self.read_declaration_tables(symbol.module_id, |bindings, definitions| {
                // read the selected member declaration
                let declaration = bindings.get_symbol_maybe(symbol.local_id).ok_or_else(|| {
                    ProviderError::internal(format!(
                        "selected member symbol {symbol:?} is absent from its binding table"
                    ))
                })?;
                let Some(key) = declaration.key else {
                    return Ok((None, Vec::new()));
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
                    }) if extension_symbol.module_id == root.module_id => Some(*root),
                    Some(dir::ExtensionDefinition {
                        symbol: extension_symbol,
                        target:
                            dir::ExtensionTarget::Blanket {
                                coverage: dir::BlanketCoverage::Interface(interface),
                                ..
                            },
                        ..
                    }) if extension_symbol.module_id == interface.module_id => Some(*interface),
                    Some(_) => None,
                    None => Some(owner),
                };
                let direct = owner
                    .and_then(|owner| self.environment.language.item(owner))
                    .map(|owner| dir::LanguageMember { owner, key });
                if direct.is_some() {
                    return Ok((direct, Vec::new()));
                }

                // retain canonical requirements implemented by this member
                let requirements = definitions
                    .member_conformances()
                    .filter(|conformance| conformance.member == symbol)
                    .map(|conformance| conformance.requirement)
                    .collect();

                Ok((direct, requirements))
            })?;
        if direct.is_some() {
            return Ok(direct);
        }

        // require every canonical requirement to identify the same member
        let mut selected = None;
        for requirement in requirements {
            let Some(member) = self.language_member(requirement)? else {
                continue;
            };
            if selected.is_some_and(|selected| selected != member) {
                return Ok(None);
            }

            selected = Some(member);
        }

        Ok(selected)
    }
}

impl DirModule<'_> {
    /// Return the canonical language item represented by one checked node.
    pub fn representation_item(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Result<Option<dir::LanguageItem>, ProviderError> {
        let type_id = self.node_type_id(node)?;

        self.dir.representation_item(type_id)
    }

    /// Return whether one node is within a declaration or implementation of a language member.
    pub fn is_within_language_member(
        &self,
        node: dir::LocalNodeIdAny,
        language_member: dir::LanguageMember,
    ) -> Result<bool, ProviderError> {
        // select the member that contains the node
        let Some(declaration) = self.view().ancestor::<dir::Member>(node) else {
            return Ok(false);
        };

        // resolve the member's canonical language identity
        let declaration = declaration.into_global_any(self.id);
        let symbol = self
            .bindings
            .declaration_symbol(declaration)
            .ok_or_else(|| {
                ProviderError::internal(format!(
                    "checked member {declaration:?} has no declaration symbol"
                ))
            })?;
        let symbol = symbol.into_global(self.id);

        self.dir
            .language_member(symbol)
            .map(|member| member == Some(language_member))
    }

    /// Return the canonical language item selected directly by one expression.
    pub fn language_item(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::LanguageItem>, ProviderError> {
        let item = self
            .selected_symbol(expression)?
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
                let Some(resolution) = self.call_decision(expression)? else {
                    return Ok(None);
                };
                let symbols = resolution.arms().iter().map(|call| call.target.symbol());

                self.selected_language_member(symbols)
            }

            // property reads retain one selected member for every runtime arm
            dir::Expression::Member { .. } => {
                let Some(resolution) = self.member_decision(expression)? else {
                    return Ok(None);
                };
                let symbols = resolution
                    .arms()
                    .iter()
                    .map(|access| access.target.symbol());

                self.selected_language_member(symbols)
            }

            // operators retain one selected callable for every runtime arm
            dir::Expression::Unary { .. } | dir::Expression::Binary { .. } => {
                let Some(resolution) = self.operator_decision(expression.into_any())? else {
                    return Ok(None);
                };
                let symbols = resolution
                    .arms()
                    .iter()
                    .map(|application| application.call().and_then(|call| call.target.symbol()));

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
