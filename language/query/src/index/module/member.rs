use destack_dir as dir;
use destack_repository::{ProviderError, ProviderResult};

use super::context::ModuleIndexContext;

/// Builder for one member index from checked DIR.
pub(in crate::index) struct MemberIndexer<'context, 'index> {
    /// The indexed module context.
    module: &'context ModuleIndexContext<'index>,
    /// The collected index entries.
    entries: Vec<dir::MemberEntry>,
}

impl<'context, 'index> MemberIndexer<'context, 'index> {
    /// Build the member index.
    pub(in crate::index) fn build(
        module: &'context ModuleIndexContext<'index>,
    ) -> ProviderResult<dir::MemberIndex> {
        let mut indexer = Self {
            module,
            entries: Vec::new(),
        };

        // collect checked definition members
        indexer.collect_members()?;

        Ok(dir::MemberIndex::new(indexer.entries))
    }

    /// Collect checked member index entries.
    fn collect_members(&mut self) -> ProviderResult<()> {
        for (declaring_symbol, definition) in self.module.definitions().iter_definitions() {
            self.collect_definition(declaring_symbol, definition)?;
        }

        Ok(())
    }

    /// Collect members declared by one definition.
    fn collect_definition(
        &mut self,
        declaring_symbol: dir::GlobalSymbolId,
        definition: &dir::Definition,
    ) -> ProviderResult<()> {
        // resolve owner metadata shared by each member row
        let owner_symbol = definition.member_owner(declaring_symbol);
        let origin = match definition {
            dir::Definition::Extension(extension) if extension.target.is_blanket() => {
                dir::MemberOrigin::BlanketExtension
            }
            dir::Definition::Extension(_) => dir::MemberOrigin::RootedExtension,
            _ => dir::MemberOrigin::Declaration,
        };
        let owner_name = self.local_symbol_name(declaring_symbol);

        // collect each checked definition member
        for member in definition.members() {
            let Some(entry) = self.member_entry(
                owner_symbol,
                declaring_symbol,
                owner_name.as_deref(),
                origin,
                member,
            )?
            else {
                continue;
            };

            self.entries.push(entry);
        }

        Ok(())
    }

    /// Return the local symbol name for one declaring symbol.
    fn local_symbol_name(&self, symbol_id: dir::GlobalSymbolId) -> Option<String> {
        if symbol_id.module_id != self.module.module_id() {
            return None;
        }

        let symbol = self.module.bindings().get_symbol(symbol_id.local_id);
        let name = symbol.name()?;

        Some(self.module.strings().get(name).to_string())
    }

    /// Build one member entry from a checked definition member.
    fn member_entry(
        &self,
        owner_symbol: Option<dir::GlobalSymbolId>,
        declaring_symbol: dir::GlobalSymbolId,
        owner_name: Option<&str>,
        origin: dir::MemberOrigin,
        member: &dir::DefinitionMember,
    ) -> ProviderResult<Option<dir::MemberEntry>> {
        // keep only member declarations owned by this module
        let source = member.source();
        if source.module_id != self.module.module_id() {
            return Ok(None);
        }

        // resolve member metadata
        let span = self
            .module
            .view()
            .get_span_by_id(source.local_id.id)
            .ok_or_else(|| {
                ProviderError::internal(format!("definition member has no source span: {source:?}"))
            })?;
        let selection = self
            .module
            .node_selection_span(self.module.view(), source.local_id);
        // index symbol-keyed members under their authored key expression
        let name = member
            .name(self.module.strings())
            .or_else(|| self.member_key_label(source.local_id))
            .ok_or_else(|| {
                ProviderError::internal(format!("definition member has no indexed key: {source:?}"))
            })?;
        let kind = Self::member_kind(member);
        let symbol = member.symbol();
        let type_id = self.member_type(member)?;

        // emit member declaration row
        Ok(Some(dir::MemberEntry {
            name,
            kind,
            owner: owner_symbol,
            declaring: declaring_symbol,
            symbol,
            source,
            file: span.file,
            span,
            selection,
            container: owner_name.map(str::to_string),
            ty: type_id,
            origin,
            space: member.space(),
        }))
    }

    /// Return the checked type attached to one definition member.
    fn member_type(
        &self,
        member: &dir::DefinitionMember,
    ) -> ProviderResult<Option<dir::GlobalTypeId>> {
        // require the checked type carried by symbol backed members
        if let Some(symbol) = member.type_symbol() {
            let type_id = self
                .module
                .types()
                .get_symbol_type_id(symbol)
                .ok_or_else(|| {
                    ProviderError::internal(format!(
                        "definition member has no checked type: {symbol:?}"
                    ))
                })?;

            return Ok(Some(type_id));
        }

        Ok(member.value_type())
    }

    /// Render one member's computed key expression as its indexed name.
    fn member_key_label(&self, source: dir::LocalNodeIdAny) -> Option<String> {
        let view = self.module.view();

        // read the authored key expression from the member node
        let key = match source.ty {
            dir::NodeType::Member => {
                let member_id = dir::LocalNodeId::<dir::Member>::new(source.id);

                *view.get(member_id).key()?
            }
            dir::NodeType::TypeMember => {
                let member_id = dir::LocalNodeId::<dir::TypeMember>::new(source.id);

                *view.get(member_id).key()?
            }
            _ => return None,
        };
        let dir::Key::Expression(expression) = key else {
            return None;
        };

        // join the referenced path into a bracketed label
        let mut segments = Vec::new();
        let mut current = expression;
        loop {
            match view.get(current) {
                dir::Expression::Identifier { name } => {
                    segments.push(self.module.strings().get(*name).to_string());

                    break;
                }
                dir::Expression::Member {
                    left,
                    name: Some(name),
                    ..
                } => {
                    segments.push(self.module.strings().get(*name).to_string());
                    current = *left;
                }
                _ => return None,
            }
        }
        segments.reverse();

        Some(format!("[{}]", segments.join(".")))
    }

    /// Return the index kind for one checked definition member.
    fn member_kind(member: &dir::DefinitionMember) -> dir::MemberKind {
        match member {
            dir::DefinitionMember::Field(_) => dir::MemberKind::Field,
            dir::DefinitionMember::Method(method) => {
                if matches!(
                    method.role,
                    Some(dir::FunctionRole::Getter | dir::FunctionRole::Setter)
                ) {
                    dir::MemberKind::Property
                } else {
                    match method.slot {
                        dir::MemberSlot::Constructor | dir::MemberSlot::New => {
                            dir::MemberKind::Constructor
                        }
                        dir::MemberSlot::Call => dir::MemberKind::CallSignature,
                        dir::MemberSlot::Key(_) => dir::MemberKind::Method,
                    }
                }
            }
            dir::DefinitionMember::AssociatedType(_) => dir::MemberKind::AssociatedType,
            dir::DefinitionMember::AssociatedConst(_) => dir::MemberKind::AssociatedConst,
            dir::DefinitionMember::EnumVariant(_)
            | dir::DefinitionMember::TaggedKey(_)
            | dir::DefinitionMember::TaggedVariant(_) => dir::MemberKind::Variant,
            dir::DefinitionMember::CallSignature(_) => dir::MemberKind::CallSignature,
            dir::DefinitionMember::ConstructSignature(_) => dir::MemberKind::ConstructSignature,
            dir::DefinitionMember::IndexSignature(_) => dir::MemberKind::IndexSignature,
        }
    }
}
