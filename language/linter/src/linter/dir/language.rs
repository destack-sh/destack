use tspp_dir as dir;
use tspp_repository::ProviderError;

use super::{Dir, DirModule};

impl Dir<'_> {
    /// Return the canonical language item represented by one symbol.
    pub(crate) fn language_item(&self, symbol: dir::GlobalSymbolId) -> Option<dir::LanguageItem> {
        self.environment.language.item(symbol)
    }

    /// Return the written derive selection for one nominal declaration.
    pub(crate) fn written_derives(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<Option<Vec<dir::AutoInterface>>, ProviderError> {
        self.read_declaration_tables(symbol.module_id, |tables| {
            let definition = tables.definitions.definition(symbol).ok_or_else(|| {
                ProviderError::internal(format!("nominal declaration {symbol:?} has no definition"))
            })?;

            Ok(definition.derives().map(<[_]>::to_vec))
        })
    }

    /// Return the canonical language item represented by one type.
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

    /// Return whether one type represents the given language item, alone or intersected.
    pub fn represents_item(
        &self,
        type_id: dir::GlobalTypeId,
        item: dir::LanguageItem,
    ) -> Result<bool, ProviderError> {
        // narrowing keeps the representation beside the surviving arm, so inspect every element
        let type_id = self.strip_form(type_id)?;
        for element in self.intersection_elements(type_id)? {
            if self.representation_item(element)? == Some(item) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return the canonical language member represented by one symbol.
    pub fn language_member(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<Option<dir::LanguageMember>, ProviderError> {
        self.canonical_language_member(symbol)
    }

    /// Return the canonical language member one symbol implements.
    pub fn implemented_language_member(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<Option<dir::LanguageMember>, ProviderError> {
        let implemented = self.implemented_language_members(symbol)?;
        let implemented = Self::shared_member(implemented);

        Ok(implemented.or(self.declared_language_member(symbol)?))
    }

    /// Return the canonical language member represented by one symbol.
    fn canonical_language_member(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<Option<dir::LanguageMember>, ProviderError> {
        if let Some(member) = self.declared_language_member(symbol)? {
            return Ok(Some(member));
        }

        let implemented = self.implemented_language_members(symbol)?;

        Ok(Self::shared_member(implemented))
    }

    /// Return the canonical language member declared by one member symbol.
    fn declared_language_member(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<Option<dir::LanguageMember>, ProviderError> {
        self.read_declaration_tables(symbol.module_id, |tables| {
            // locate the member in its definition
            let Some((declaring, definition, member)) = tables.definitions.member(symbol) else {
                return Ok(None);
            };
            let Some(key) = member.key() else {
                return Ok(None);
            };

            // map package-owned members through their canonical owner
            let Some(owner) = definition.language_member_owner(declaring) else {
                return Ok(None);
            };
            let Some(owner) = self.environment.language.item(owner) else {
                return Ok(None);
            };

            Ok(Some(dir::LanguageMember { owner, key }))
        })
    }

    /// Return canonical language members implemented by one member symbol.
    fn implemented_language_members(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Result<Vec<dir::LanguageMember>, ProviderError> {
        // collect the declarations this symbol satisfies, a default member standing for its own
        let declarations = self.read_declaration_tables(symbol.module_id, |tables| {
            Ok(tables
                .members
                .member_declarations(symbol)
                .filter(|declaration| *declaration != symbol)
                .collect::<Vec<_>>())
        })?;

        // resolve each declaration to its canonical identity
        let mut members = Vec::new();
        for declaration in declarations {
            if let Some(member) = self.canonical_language_member(declaration)? {
                members.push(member);
            }
        }

        Ok(members)
    }

    /// Return the language member shared by every resolved declaration.
    fn shared_member(members: Vec<dir::LanguageMember>) -> Option<dir::LanguageMember> {
        let mut members = members.into_iter();
        let selected = members.next()?;

        members.all(|member| member == selected).then_some(selected)
    }
}

impl DirModule<'_> {
    /// Return the receiver of one canonical iterator call.
    pub(crate) fn iterator_receiver(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
        // require a direct zero-argument member call
        let Some(call) = self.member_call(expression) else {
            return Ok(None);
        };
        if call.is_optional() || !call.arguments.is_empty() {
            return Ok(None);
        }

        // require the canonical iterator operation for its selected owner
        let Some(member) = self.language_member(expression)? else {
            return Ok(None);
        };
        if member != member.owner.member("iterator") {
            return Ok(None);
        }

        Ok(Some(call.receiver))
    }

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

    /// Return the canonical language item represented by one node.
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
        let symbol = self.declaration_symbol(declaration)?;

        self.dir
            .language_member(symbol)
            .map(|member| member == Some(language_member))
    }

    /// Return whether one node is within a canonical language item declaration.
    pub fn is_within_language_item(
        &self,
        node: dir::LocalNodeIdAny,
        language_item: dir::LanguageItem,
    ) -> Result<bool, ProviderError> {
        // select the declaration that contains the node
        let Some(declaration) = self.view().ancestor::<dir::Declaration>(node) else {
            return Ok(false);
        };

        // resolve the declaration's canonical language identity
        let symbol = self.declaration_symbol(declaration)?;
        let item = self.dir.environment.language.item(symbol);

        Ok(item == Some(language_item))
    }

    /// Return the canonical language item targeted by one expression.
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
    fn selected_member<Resolve>(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        resolve: &Resolve,
    ) -> Result<Option<dir::LanguageMember>, ProviderError>
    where
        Resolve:
            Fn(&Dir<'_>, dir::GlobalSymbolId) -> Result<Option<dir::LanguageMember>, ProviderError>,
    {
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
    fn shared_language_member<Resolve>(
        &self,
        symbols: impl Iterator<Item = Option<dir::GlobalSymbolId>>,
        resolve: &Resolve,
    ) -> Result<Option<dir::LanguageMember>, ProviderError>
    where
        Resolve:
            Fn(&Dir<'_>, dir::GlobalSymbolId) -> Result<Option<dir::LanguageMember>, ProviderError>,
    {
        let mut symbols = symbols;
        let Some(symbol) = symbols.next() else {
            return Err(ProviderError::internal(
                "operation resolution has no runtime alternatives",
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
