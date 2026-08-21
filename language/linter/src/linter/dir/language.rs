use destack_dir as dir;
use destack_repository::ProviderError;

use super::{Dir, DirModule};

/// Resolution of one selected member symbol to its canonical language member.
type MemberResolve =
    dyn Fn(&Dir<'_>, dir::GlobalSymbolId) -> Result<Option<dir::LanguageMember>, ProviderError>;

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

    /// Return whether one checked type represents the given language item, alone or intersected.
    pub fn represents_item(
        &self,
        type_id: dir::GlobalTypeId,
        item: dir::LanguageItem,
    ) -> Result<bool, ProviderError> {
        // narrowing keeps the carrier beside the surviving arm, so inspect every element
        let type_id = self.strip_form(type_id)?;
        for element in self.intersection_elements(type_id)? {
            if self.representation_item(element)? == Some(item) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return the canonical language member one symbol declares.
    pub fn language_member(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<Option<dir::LanguageMember>, ProviderError> {
        let (declared, requirements) = self.member_conformance(symbol)?;
        if declared.is_some() {
            return Ok(declared);
        }

        self.requirement_member(requirements)
    }

    /// Return the canonical language member one symbol implements.
    pub fn implemented_language_member(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<Option<dir::LanguageMember>, ProviderError> {
        let (declared, requirements) = self.member_conformance(symbol)?;
        let implemented = self.requirement_member(requirements)?;

        Ok(implemented.or(declared))
    }

    /// Return the language member one symbol declares and the canonical requirements it implements.
    fn member_conformance(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<(Option<dir::LanguageMember>, Vec<dir::GlobalSymbolId>), ProviderError> {
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

            // resolve extension members declared in the item's package to the item
            let owner = match definitions.extension_definition(owner) {
                Some(dir::ExtensionDefinition {
                    symbol: extension_symbol,
                    target:
                        dir::ExtensionTarget::Rooted {
                            root: dir::TypeRoot::Declaration(root),
                            ..
                        },
                    ..
                }) if extension_symbol.module_id.package_id == root.module_id.package_id => {
                    Some(*root)
                }
                Some(dir::ExtensionDefinition {
                    symbol: extension_symbol,
                    target:
                        dir::ExtensionTarget::Blanket {
                            coverage: dir::BlanketCoverage::Interface(interface),
                            ..
                        },
                    ..
                }) if extension_symbol.module_id.package_id == interface.module_id.package_id => {
                    Some(*interface)
                }
                Some(_) => None,
                None => Some(owner),
            };
            let declared = owner
                .and_then(|owner| self.environment.language.item(owner))
                .map(|owner| dir::LanguageMember { owner, key });

            // retain canonical requirements implemented by this member
            let requirements = definitions
                .member_conformances()
                .filter(|conformance| conformance.member == symbol)
                .map(|conformance| conformance.requirement)
                .collect();

            Ok((declared, requirements))
        })
    }

    /// Return the language member shared by every implemented canonical requirement.
    fn requirement_member(
        &self,
        requirements: Vec<dir::GlobalSymbolId>,
    ) -> Result<Option<dir::LanguageMember>, ProviderError> {
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
    /// Return the receiver of one canonical length access.
    pub(crate) fn length_receiver(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
        let dir::Expression::Member {
            left: collection,
            is_optional: false,
            ..
        } = self.view().get(expression)
        else {
            return Ok(None);
        };
        if !matches!(
            self.language_member(expression)?,
            Some(member)
                if member == dir::LanguageItem::Sequence.member("length")
                    || member == dir::LanguageItem::Array.member("length")
                    || member == dir::LanguageItem::Slice.member("length")
                    || member == dir::LanguageItem::String.member("length")
        ) {
            return Ok(None);
        }

        Ok(Some(*collection))
    }

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

    /// Return the canonical language item targeted by one checked expression.
    pub fn language_item(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::LanguageItem>, ProviderError> {
        let symbol = match self.view().get(expression) {
            dir::Expression::Call { .. } => self.call_symbol(expression)?,
            dir::Expression::New { .. } => self.construct_symbol(expression)?,
            _ => self.selected_symbol(expression)?,
        };
        let item = symbol.and_then(|symbol| self.dir.environment.language.item(symbol));

        Ok(item)
    }

    /// Return the canonical language member declared by one expression's selection.
    pub fn language_member(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::LanguageMember>, ProviderError> {
        self.selected_member(expression, &|dir, symbol| dir.language_member(symbol))
    }

    /// Return the canonical language member one expression's selection implements.
    pub fn implemented_language_member(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::LanguageMember>, ProviderError> {
        self.selected_member(expression, &|dir, symbol| {
            dir.implemented_language_member(symbol)
        })
    }

    /// Return the language member every runtime arm of one expression resolves to.
    fn selected_member(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        resolve: &MemberResolve,
    ) -> Result<Option<dir::LanguageMember>, ProviderError> {
        let view = self.view();
        match view.get(expression) {
            // calls retain one selected callable for every runtime arm
            dir::Expression::Call { .. } => {
                let Some(resolution) = self.call_decision(expression)? else {
                    return Ok(None);
                };
                let symbols = resolution.arms().iter().map(|call| call.target.symbol());

                self.shared_language_member(symbols, resolve)
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

                self.shared_language_member(symbols, resolve)
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

                self.shared_language_member(symbols, resolve)
            }
            _ => Ok(None),
        }
    }

    /// Return the language member shared by every selected declaration.
    fn shared_language_member(
        &self,
        symbols: impl Iterator<Item = Option<dir::GlobalSymbolId>>,
        resolve: &MemberResolve,
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
        let Some(selected) = resolve(self.dir, symbol)? else {
            return Ok(None);
        };

        // require every runtime alternative to select the same language member
        for symbol in symbols {
            let Some(symbol) = symbol else {
                return Ok(None);
            };
            let Some(member) = resolve(self.dir, symbol)? else {
                return Ok(None);
            };
            if selected != member {
                return Ok(None);
            }
        }

        Ok(Some(selected))
    }
}
