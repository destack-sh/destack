use destack_dir as dir;

use crate::ModuleQueryContext;

/// Builder for one member index from checked DIR.
pub(super) struct MemberIndexer<'context, 'query> {
    /// The indexed module context.
    module: &'context ModuleQueryContext<'query>,
    /// The collected index entries.
    entries: Vec<dir::MemberEntry>,
}

impl<'context, 'query> MemberIndexer<'context, 'query> {
    /// Build the member index.
    pub(super) fn build(module: &'context ModuleQueryContext<'query>) -> dir::MemberIndex {
        let mut indexer = Self {
            module,
            entries: Vec::new(),
        };

        // collect checked definition members
        indexer.collect_members();

        dir::MemberIndex::new(indexer.entries)
    }

    /// Collect checked member index entries.
    fn collect_members(&mut self) {
        for (declaring_symbol, definition) in self.module.definitions().iter_definitions() {
            self.collect_definition(declaring_symbol, definition);
        }
    }

    /// Collect members declared by one definition.
    fn collect_definition(
        &mut self,
        declaring_symbol: dir::GlobalSymbolId,
        definition: &dir::Definition,
    ) {
        // resolve owner metadata shared by each member row
        let owner_symbol = self.definition_member_owner(declaring_symbol, definition);
        let origin = if matches!(definition, dir::Definition::Extension(_)) {
            dir::MemberOrigin::Extension
        } else {
            dir::MemberOrigin::Definition
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
            ) else {
                continue;
            };

            self.entries.push(entry);
        }
    }

    /// Return the member owner symbol for one checked definition.
    fn definition_member_owner(
        &self,
        declaring_symbol: dir::GlobalSymbolId,
        definition: &dir::Definition,
    ) -> Option<dir::GlobalSymbolId> {
        match definition {
            dir::Definition::Extension(extension) => extension.target.root(),
            dir::Definition::Struct(_)
            | dir::Definition::Class(_)
            | dir::Definition::Interface(_)
            | dir::Definition::Enum(_)
            | dir::Definition::TypeAlias(_)
            | dir::Definition::Newtype(_) => Some(declaring_symbol),
        }
    }

    /// Return the local symbol name for one declaring symbol.
    fn local_symbol_name(&self, symbol_id: dir::GlobalSymbolId) -> Option<String> {
        if symbol_id.module_id != self.module.module_id() {
            return None;
        }

        let symbol = self.module.symbols().get_symbol(symbol_id.local_id);
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
    ) -> Option<dir::MemberEntry> {
        // keep only member declarations owned by this module
        let source = member.source();
        if source.module_id != self.module.module_id() {
            return None;
        }

        // resolve member metadata
        let span = self.module.get_span(self.module.view(), source.local_id);
        let name = self.member_name(member)?;
        let kind = Self::member_kind(member);
        let symbol = member.symbol();
        let ty = self.member_type(member, symbol);

        // emit member declaration row
        Some(dir::MemberEntry {
            name,
            kind,
            owner: owner_symbol,
            declaring: declaring_symbol,
            symbol,
            source,
            file: span.file,
            span,
            container: owner_name.map(str::to_string),
            ty,
            origin,
            space: member.space(),
            is_abstract: Self::is_abstract(member),
            is_override: Self::is_override(member),
            is_default: member.is_default(),
            is_static: member.space() == dir::MemberSpace::Static,
        })
    }

    /// Return the searchable name for one checked member.
    fn member_name(&self, member: &dir::DefinitionMember) -> Option<String> {
        // prefer explicit static keys
        if let Some(key) = member.key() {
            return self.static_name(&key);
        }

        // name structural member signatures by their call form
        match member {
            dir::DefinitionMember::Method(method) => match method.slot {
                dir::MemberSlot::Constructor => Some("constructor".to_string()),
                dir::MemberSlot::New => Some("new".to_string()),
                dir::MemberSlot::Call => Some("call".to_string()),
                dir::MemberSlot::Key(_) => None,
            },
            dir::DefinitionMember::CallSignature(_) => Some("call".to_string()),
            dir::DefinitionMember::ConstructSignature(_) => Some("new".to_string()),
            dir::DefinitionMember::IndexSignature(_) => Some("[]".to_string()),
            _ => None,
        }
    }

    /// Return the searchable name for one static member key.
    fn static_name(&self, key: &dir::StaticKey) -> Option<String> {
        match key {
            dir::StaticKey::Name(name) => Some(self.module.strings().get(*name).to_string()),
            dir::StaticKey::Index(index) => Some(index.to_string()),
            dir::StaticKey::Symbol(_) => None,
        }
    }

    /// Return the checked type attached to one definition member.
    fn member_type(
        &self,
        member: &dir::DefinitionMember,
        symbol: Option<dir::GlobalSymbolId>,
    ) -> Option<dir::GlobalTypeId> {
        // prefer the checked type attached to the member symbol
        if let Some(symbol) = symbol {
            if let Some(ty) = self.module.types().get_symbol_type_id(symbol) {
                return Some(ty);
            }
        }

        // read structural signature types without a symbol payload
        match member {
            dir::DefinitionMember::AssociatedType(member) => member.value.or(member.constraint),
            dir::DefinitionMember::CallSignature(member)
            | dir::DefinitionMember::ConstructSignature(member) => Some(member.ty),
            dir::DefinitionMember::IndexSignature(member) => Some(member.value_type),
            dir::DefinitionMember::Field(_)
            | dir::DefinitionMember::Method(_)
            | dir::DefinitionMember::AssociatedConst(_)
            | dir::DefinitionMember::Variant(_) => None,
        }
    }

    /// Return the index kind for one checked definition member.
    fn member_kind(member: &dir::DefinitionMember) -> dir::MemberKind {
        match member {
            dir::DefinitionMember::Field(_) => dir::MemberKind::Field,
            dir::DefinitionMember::Method(method) => match method.slot {
                dir::MemberSlot::Constructor | dir::MemberSlot::New => dir::MemberKind::Constructor,
                dir::MemberSlot::Call => dir::MemberKind::CallSignature,
                dir::MemberSlot::Key(_) => dir::MemberKind::Method,
            },
            dir::DefinitionMember::AssociatedType(_) => dir::MemberKind::AssociatedType,
            dir::DefinitionMember::AssociatedConst(_) => dir::MemberKind::AssociatedConst,
            dir::DefinitionMember::Variant(_) => dir::MemberKind::Variant,
            dir::DefinitionMember::CallSignature(_) => dir::MemberKind::CallSignature,
            dir::DefinitionMember::ConstructSignature(_) => dir::MemberKind::ConstructSignature,
            dir::DefinitionMember::IndexSignature(_) => dir::MemberKind::IndexSignature,
        }
    }

    /// Return whether one checked member is abstract.
    fn is_abstract(member: &dir::DefinitionMember) -> bool {
        match member {
            dir::DefinitionMember::Field(field) => field.is_abstract,
            dir::DefinitionMember::Method(method) => method.abstraction.is_abstract(),
            dir::DefinitionMember::AssociatedType(_)
            | dir::DefinitionMember::AssociatedConst(_)
            | dir::DefinitionMember::Variant(_)
            | dir::DefinitionMember::CallSignature(_)
            | dir::DefinitionMember::ConstructSignature(_)
            | dir::DefinitionMember::IndexSignature(_) => false,
        }
    }

    /// Return whether one checked member overrides an inherited member.
    fn is_override(member: &dir::DefinitionMember) -> bool {
        match member {
            dir::DefinitionMember::Field(field) => field.is_override,
            dir::DefinitionMember::Method(method) => method.is_override,
            dir::DefinitionMember::AssociatedType(_)
            | dir::DefinitionMember::AssociatedConst(_)
            | dir::DefinitionMember::Variant(_)
            | dir::DefinitionMember::CallSignature(_)
            | dir::DefinitionMember::ConstructSignature(_)
            | dir::DefinitionMember::IndexSignature(_) => false,
        }
    }
}
