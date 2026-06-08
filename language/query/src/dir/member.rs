use std::collections::HashSet;

use destack_dir as dir;
use destack_source::Span;

use crate::core::{
    MemberEntry, MemberEntryKind, MemberSource, ModuleQueryContext, Name, WorkspaceQueryContext,
};

/// Resolved member candidate.
#[derive(Debug, Clone)]
pub(crate) struct MemberCandidate {
    /// The name of the member.
    pub name: MemberName,
    /// The type of the member.
    pub type_id: Option<dir::GlobalTypeId>,
    /// The kind of member (field, method, etc.).
    pub kind: MemberKind,
    /// The symbol id if this member comes from a symbol declaration.
    pub symbol_id: Option<dir::GlobalSymbolId>,
    /// Whether this member comes from an extension declaration.
    pub is_extension: bool,
}

/// The name of a member.
#[derive(Debug, Clone)]
pub(crate) enum MemberName {
    /// A resolved string name.
    String(String),
    /// A numeric index.
    Index(usize),
    /// A computed key (cannot be displayed directly).
    Computed,
}

/// The kind of member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemberKind {
    /// A field or property.
    Field,
    /// A method (function typed member).
    Method,
    /// A call signature.
    CallSignature,
    /// A construct signature.
    ConstructSignature,
    /// An enum member/variant.
    EnumMember,
    /// An associated type member.
    AssociatedType,
    /// An associated constant member.
    AssociatedConst,
}

impl From<MemberEntryKind> for MemberKind {
    /// Convert an indexed source member kind into a completion member kind.
    fn from(kind: MemberEntryKind) -> Self {
        match kind {
            MemberEntryKind::Field => Self::Field,
            MemberEntryKind::Method => Self::Method,
            MemberEntryKind::AssociatedType => Self::AssociatedType,
            MemberEntryKind::AssociatedConst => Self::AssociatedConst,
            MemberEntryKind::Variant => Self::EnumMember,
        }
    }
}

/// Active type ids during member resolution.
#[derive(Debug, Default)]
struct MemberResolutionState {
    /// The global type ids already on the recursion stack.
    active_type_ids: HashSet<dir::GlobalTypeId>,
}

impl MemberResolutionState {
    /// Enter one type id.
    fn enter(&mut self, type_id: dir::GlobalTypeId) -> bool {
        self.active_type_ids.insert(type_id)
    }

    /// Leave one type id.
    fn leave(&mut self, type_id: dir::GlobalTypeId) {
        self.active_type_ids.remove(&type_id);
    }
}

impl MemberCandidate {
    /// Build a completion member candidate from one indexed member entry.
    fn from_entry(entry: MemberEntry) -> Self {
        Self {
            name: member_name_from_query_name(entry.name),
            type_id: entry.declared_type,
            kind: entry.kind.into(),
            symbol_id: Some(entry.member_symbol),
            is_extension: entry.source == MemberSource::Extension,
        }
    }
}

impl ModuleQueryContext<'_> {
    /// Build source-backed member index entries for this module.
    pub(crate) fn build_member_candidates(&self) -> Vec<MemberEntry> {
        let tree = self.dir().view();
        let mut entries = Vec::new();

        // collect members declared below each nominal declaration
        for (declaration_id, declaration) in tree.iter_nodes_of_type::<dir::Declaration>() {
            let Some(declaring_symbol) = self.dir().global_symbol_for_node(declaration_id.into())
            else {
                continue;
            };
            let owner_symbol = if declaration_is_extension(declaration) {
                self.extension_target_symbol(declaring_symbol)
                    .unwrap_or(declaring_symbol)
            } else {
                declaring_symbol
            };
            let owner_name = super::declaration_display_name(self.dir().strings(), declaration);

            if let Some(member_ids) = declaration.member_ids() {
                for member_id in member_ids {
                    if let Some(entry) = self.member_entry_for_declaration_member(
                        tree,
                        owner_symbol,
                        declaring_symbol,
                        &owner_name,
                        *member_id,
                        declaration_is_extension(declaration),
                    ) {
                        entries.push(entry);
                    }
                }
            }

            if let Some(member_ids) = declaration.type_member_ids() {
                for member_id in member_ids {
                    if let Some(entry) = self.member_entry_for_type_member(
                        tree,
                        owner_symbol,
                        &owner_name,
                        *member_id,
                    ) {
                        entries.push(entry);
                    }
                }
            }

            if let dir::Declaration::Enum(declaration) = declaration {
                for field_id in &declaration.fields {
                    let Some(entry) = self.member_entry_for_enum_variant(
                        tree,
                        owner_symbol,
                        &owner_name,
                        *field_id,
                    ) else {
                        continue;
                    };

                    entries.push(entry);
                }
            }
        }

        entries
    }

    /// Return the canonical target symbol declared by one extension symbol.
    pub(crate) fn extension_target_symbol(
        &self,
        extension_symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalSymbolId> {
        let (_, extension) = self
            .dir()
            .extensions()
            .iter_extensions()
            .find(|(_, extension)| extension.symbol == extension_symbol)?;
        extension
            .target
            .nominal_root()
            .map(|target_symbol| self.canonical_symbol(target_symbol))
    }

    /// Return the searchable name introduced by one declaration member.
    fn name_for_member(&self, member: &dir::Member) -> Option<Name> {
        if let Some(name) = member.name() {
            return Some(Name::String(self.dir().strings().get(name).to_string()));
        }

        let key = member.key()?;

        self.name_for_key(key)
    }

    /// Return the searchable name introduced by one type member.
    fn name_for_type_member(&self, member: &dir::TypeMember) -> Option<Name> {
        if let Some(name) = member.name() {
            return Some(Name::String(self.dir().strings().get(name).to_string()));
        }

        let key = member.key()?;

        self.name_for_key(key)
    }

    /// Return a searchable name for one direct key.
    fn name_for_key(&self, key: &dir::Key) -> Option<Name> {
        let key = key.direct_static_key()?;

        self.name_for_static_key(&key)
    }

    /// Return a searchable name for one direct static key.
    fn name_for_static_key(&self, key: &dir::StaticKey) -> Option<Name> {
        match key {
            dir::StaticKey::Name(name) => {
                Some(Name::String(self.dir().strings().get(*name).to_string()))
            }
            dir::StaticKey::Index(index) => Some(Name::Index(*index as u64)),
            dir::StaticKey::Symbol(_) => None,
        }
    }
}

/// Return the query member kind for one declaration member.
fn member_entry_kind_for_member(member: &dir::Member) -> Option<MemberEntryKind> {
    match member {
        dir::Member::AssociatedType { .. } => Some(MemberEntryKind::AssociatedType),
        dir::Member::AssociatedConst { .. } => Some(MemberEntryKind::AssociatedConst),
        dir::Member::Field { .. } => Some(MemberEntryKind::Field),
        dir::Member::Method { .. } => Some(MemberEntryKind::Method),
        dir::Member::StaticBlock { .. }
        | dir::Member::ComptimeBlock { .. }
        | dir::Member::Error => None,
    }
}

/// Return the query member kind for one type member.
fn member_entry_kind_for_type_member(member: &dir::TypeMember) -> Option<MemberEntryKind> {
    match member {
        dir::TypeMember::AssociatedType { .. } => Some(MemberEntryKind::AssociatedType),
        dir::TypeMember::AssociatedConst { .. } => Some(MemberEntryKind::AssociatedConst),
        dir::TypeMember::Field { .. } => Some(MemberEntryKind::Field),
        dir::TypeMember::Method { .. } => Some(MemberEntryKind::Method),
        dir::TypeMember::CallSignature { .. }
        | dir::TypeMember::ConstructSignature { .. }
        | dir::TypeMember::IndexSignature { .. }
        | dir::TypeMember::Error => None,
    }
}

/// Return whether one declaration is an extension declaration.
fn declaration_is_extension(declaration: &dir::Declaration) -> bool {
    matches!(declaration, dir::Declaration::Extension { .. })
}

/// Convert a static key to a member name, resolving the string ID to an actual string.
fn static_key_to_member_name(
    key: &dir::StaticKey,
    strings: &destack_core::StringPool,
) -> MemberName {
    // convert the key into a displayable member name
    match key {
        dir::StaticKey::Name(string_id) => MemberName::String(strings.get(*string_id).to_string()),
        dir::StaticKey::Index(index) => MemberName::Index(*index),
        dir::StaticKey::Symbol(_) => MemberName::Computed,
    }
}

/// Convert one query name to one completion member name.
fn member_name_from_query_name(name: Name) -> MemberName {
    match name {
        Name::String(name) => MemberName::String(name),
        Name::Index(index) => MemberName::Index(index as usize),
    }
}

impl ModuleQueryContext<'_> {
    /// Check if a type is a function type.
    fn is_function_type(&self, type_id: dir::GlobalTypeId) -> bool {
        self.with_global_type(type_id, |ty, _| matches!(ty, dir::Type::Function(_)))
            .unwrap_or(false)
    }
}

/// Check if two member names match.
fn member_names_match(a: &MemberName, b: &MemberName) -> bool {
    match (a, b) {
        (MemberName::String(a), MemberName::String(b)) => a == b,
        (MemberName::Index(a), MemberName::Index(b)) => a == b,
        _ => false,
    }
}

impl ModuleQueryContext<'_> {
    /// Return completion members visible on one checked type.
    pub(crate) fn resolve_type_members(
        &self,
        workspace: &WorkspaceQueryContext<'_>,
        type_id: dir::GlobalTypeId,
    ) -> Vec<MemberCandidate> {
        let mut state = MemberResolutionState::default();

        self.with_global_type(type_id, |ty, type_ctx| {
            type_ctx.resolve_type_members_inner(workspace, type_id, ty, &mut state)
        })
        .unwrap_or_default()
    }

    /// Resolve members from a reference type by looking up the symbol.
    pub(crate) fn resolve_reference_members(
        &self,
        workspace: &WorkspaceQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> Vec<MemberCandidate> {
        let symbol_id = self.canonical_symbol(symbol_id);
        let mut members = workspace
            .members_for_owner(symbol_id)
            .into_iter()
            .filter(|entry| entry.source != MemberSource::Extension)
            .map(MemberCandidate::from_entry)
            .collect::<Vec<_>>();

        // merge extension members for this symbol
        let extension_members = self.resolve_extension_members_for_symbol(workspace, symbol_id);

        // avoid duplicate member names across direct and extension members
        for member in extension_members {
            let is_duplicate = members
                .iter()
                .any(|existing| member_names_match(&existing.name, &member.name));
            if is_duplicate {
                continue;
            }

            members.push(member);
        }

        members
    }

    /// Build one member entry for a nominal declaration member.
    fn member_entry_for_declaration_member(
        &self,
        tree: dir::View<'_>,
        owner_symbol: dir::GlobalSymbolId,
        declaring_symbol: dir::GlobalSymbolId,
        owner_name: &str,
        member_id: dir::LocalNodeId<dir::Member>,
        is_extension: bool,
    ) -> Option<MemberEntry> {
        let ctx = self;
        let member = tree.get::<dir::Member>(member_id);
        let source_node: dir::LocalNodeIdAny = member_id.into();
        let range = ctx.member_entry_range(tree, source_node)?;

        // ignore synthetic parser scaffolding for method declarations
        if let Some(key) = member.key()
            && let Some(Name::String(name)) = ctx.name_for_key(key)
            && super::is_synthetic_function_keyword_field(member, &name, range)
        {
            return None;
        }

        let name = ctx.name_for_member(member)?;
        let kind = member_entry_kind_for_member(member)?;
        let source = if is_extension {
            MemberSource::Extension
        } else {
            MemberSource::Declaration
        };

        Some(ctx.member_entry(
            owner_symbol,
            declaring_symbol,
            owner_name,
            source_node,
            name,
            kind,
            source,
            member.has_static_modifier(),
            range,
        ))
    }

    /// Build one member entry for a type-surface member.
    fn member_entry_for_type_member(
        &self,
        tree: dir::View<'_>,
        owner_symbol: dir::GlobalSymbolId,
        owner_name: &str,
        member_id: dir::LocalNodeId<dir::TypeMember>,
    ) -> Option<MemberEntry> {
        let ctx = self;
        let member = tree.get::<dir::TypeMember>(member_id);
        let source_node: dir::LocalNodeIdAny = member_id.into();
        let range = ctx.member_entry_range(tree, source_node)?;
        let name = ctx.name_for_type_member(member)?;
        let kind = member_entry_kind_for_type_member(member)?;

        Some(ctx.member_entry(
            owner_symbol,
            owner_symbol,
            owner_name,
            source_node,
            name,
            kind,
            MemberSource::TypeMember,
            member.has_static_modifier(),
            range,
        ))
    }

    /// Build one member entry for an enum variant.
    fn member_entry_for_enum_variant(
        &self,
        tree: dir::View<'_>,
        owner_symbol: dir::GlobalSymbolId,
        owner_name: &str,
        field_id: dir::LocalNodeId<dir::EnumField>,
    ) -> Option<MemberEntry> {
        let ctx = self;
        let field = tree.get::<dir::EnumField>(field_id);
        let source_node: dir::LocalNodeIdAny = field_id.into();
        let range = ctx.member_entry_range(tree, source_node)?;
        let name = Name::String(ctx.dir().strings().get(field.name.string()).to_string());

        Some(ctx.member_entry(
            owner_symbol,
            owner_symbol,
            owner_name,
            source_node,
            name,
            MemberEntryKind::Variant,
            MemberSource::EnumVariant,
            true,
            range,
        ))
    }

    /// Build one completed member entry.
    #[allow(clippy::too_many_arguments)]
    fn member_entry(
        &self,
        owner_symbol: dir::GlobalSymbolId,
        declaring_symbol: dir::GlobalSymbolId,
        owner_name: &str,
        source_node: dir::LocalNodeIdAny,
        name: Name,
        kind: MemberEntryKind,
        source: MemberSource,
        is_static: bool,
        range: Span,
    ) -> MemberEntry {
        let ctx = self;
        let module_id = ctx.module_id();
        let source_node = source_node.into_global(module_id);
        let member_symbol = ctx
            .dir()
            .global_symbol_for_node(source_node.local_id)
            .unwrap_or_else(|| panic!("searchable member node {source_node:?} has no symbol"));
        let declared_type = ctx.member_declared_type(source_node, member_symbol);

        MemberEntry {
            name,
            kind,
            owner_symbol,
            declaring_symbol,
            member_symbol,
            source_node,
            module_id,
            file_id: ctx.file_id(),
            range,
            container_name: Some(owner_name.to_string()),
            declared_type,
            source,
            is_static,
        }
    }

    /// Return the checked type attached to one member.
    fn member_declared_type(
        &self,
        source_node: dir::GlobalNodeIdAny,
        member_symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        let ctx = self;
        let types = ctx.dir().types();

        types
            .get_symbol_type_id(member_symbol)
            .or_else(|| types.get_node_type_id(source_node))
    }

    /// Resolve one member source range without failing the whole index.
    fn member_entry_range(
        &self,
        tree: dir::View<'_>,
        node_id: dir::LocalNodeIdAny,
    ) -> Option<Span> {
        let ctx = self;
        ctx.dir().try_span_for_dir_node(tree, node_id)
    }

    /// Resolve checked type members while tracking active type ids.
    fn resolve_type_members_inner(
        &self,
        workspace: &WorkspaceQueryContext<'_>,
        type_id: dir::GlobalTypeId,
        ty: &dir::Type,
        state: &mut MemberResolutionState,
    ) -> Vec<MemberCandidate> {
        let ctx = self;
        if !state.enter(type_id) {
            return Vec::new();
        }

        let strings = ctx.dir().strings();

        let members = match ty {
            // resolve declaration and extension members
            dir::Type::Reference(reference) => {
                ctx.resolve_reference_members(workspace, reference.symbol)
            }

            // build structural field and signature members
            dir::Type::Shape(object) => {
                let mut members = Vec::new();

                for field in &object.fields {
                    let kind = if ctx.is_function_type(field.ty) {
                        MemberKind::Method
                    } else {
                        MemberKind::Field
                    };

                    members.push(MemberCandidate {
                        name: static_key_to_member_name(&field.key, strings),
                        type_id: Some(field.ty),
                        kind,
                        symbol_id: None,
                        is_extension: false,
                    });
                }

                for sig_type_id in &object.call_signatures {
                    members.push(MemberCandidate {
                        name: MemberName::Computed,
                        type_id: Some(*sig_type_id),
                        kind: MemberKind::CallSignature,
                        symbol_id: None,
                        is_extension: false,
                    });
                }

                for sig_type_id in &object.construct_signatures {
                    members.push(MemberCandidate {
                        name: MemberName::Computed,
                        type_id: Some(*sig_type_id),
                        kind: MemberKind::ConstructSignature,
                        symbol_id: None,
                        is_extension: false,
                    });
                }

                members
            }

            // keep members common to every union element
            dir::Type::Union(union) => {
                if union.elements.is_empty() {
                    Vec::new()
                } else {
                    let mut common_members = ctx
                        .with_global_type(union.elements[0], |ty, type_ctx| {
                            type_ctx.resolve_type_members_inner(
                                workspace,
                                union.elements[0],
                                ty,
                                state,
                            )
                        })
                        .unwrap_or_default();

                    for element_id in &union.elements[1..] {
                        let Some(element_members) =
                            ctx.with_global_type(*element_id, |ty, type_ctx| {
                                type_ctx.resolve_type_members_inner(
                                    workspace,
                                    *element_id,
                                    ty,
                                    state,
                                )
                            })
                        else {
                            continue;
                        };

                        common_members.retain(|member| {
                            element_members
                                .iter()
                                .any(|other| member_names_match(&member.name, &other.name))
                        });
                    }

                    common_members
                }
            }

            // combine members from every intersection element
            dir::Type::Intersection(intersection) => {
                let is_enum_static = intersection.elements.iter().any(|element_id| {
                    ctx.with_global_type(*element_id, |element, _| {
                        let dir::Type::Form(value) = element else {
                            return false;
                        };

                        ctx.with_global_type(value.value, |inner, _| {
                            let dir::Type::Reference(reference) = inner else {
                                return false;
                            };

                            if let Some(ctx) = ctx.module_context(reference.symbol.module_id) {
                                let symbols_table = ctx.dir().symbols();
                                let sym = symbols_table.get_symbol(reference.symbol.local_id);
                                return sym.kind == dir::SymbolKind::Enum;
                            }
                            false
                        })
                        .unwrap_or(false)
                    })
                    .unwrap_or(false)
                });

                let mut all_members = Vec::new();
                let mut seen_names = Vec::new();

                for element_id in &intersection.elements {
                    let Some(element_members) =
                        ctx.with_global_type(*element_id, |ty, type_ctx| {
                            type_ctx.resolve_type_members_inner(workspace, *element_id, ty, state)
                        })
                    else {
                        continue;
                    };

                    for mut member in element_members {
                        if !seen_names
                            .iter()
                            .any(|name| member_names_match(name, &member.name))
                        {
                            if is_enum_static && member.kind == MemberKind::Field {
                                member.kind = MemberKind::EnumMember;
                            }
                            seen_names.push(member.name.clone());
                            all_members.push(member);
                        }
                    }
                }

                all_members
            }

            // expose tuple element indexes
            dir::Type::Tuple(tuple) => tuple
                .elements
                .iter()
                .enumerate()
                .map(|(i, element)| MemberCandidate {
                    name: MemberName::Index(i),
                    type_id: Some(element.ty),
                    kind: MemberKind::Field,
                    symbol_id: None,
                    is_extension: false,
                })
                .collect(),

            // use the central array type surface
            dir::Type::Slice(_) => ctx.array_members(workspace),

            dir::Type::FixedArray(_) => ctx.array_members(workspace),

            // use the primitive backing type surface
            dir::Type::Primitive(primitive) => ctx.primitive_members(workspace, *primitive),

            // use the literal backing type surface
            dir::Type::Literal(literal) => ctx.literal_members(workspace, literal),

            // follow form wrappers
            dir::Type::Form(value) => ctx
                .with_global_type(value.value, |ty, type_ctx| {
                    type_ctx.resolve_type_members_inner(workspace, value.value, ty, state)
                })
                .unwrap_or_default(),

            // return no members for non member bearing types
            _ => Vec::new(),
        };

        state.leave(type_id);

        members
    }

    /// Resolve extension members for a target symbol across all modules.
    fn resolve_extension_members_for_symbol(
        &self,
        workspace: &WorkspaceQueryContext<'_>,
        target_symbol: dir::GlobalSymbolId,
    ) -> Vec<MemberCandidate> {
        let ctx = self;
        let mut members = Vec::new();

        // collect members from visible extension declarations
        ctx.for_each_visible_extension(workspace, target_symbol, |_, extension| {
            let extension_members = workspace.members_for_declaring(extension.symbol);
            members.extend(
                extension_members
                    .into_iter()
                    .map(MemberCandidate::from_entry),
            );

            false
        });

        members
    }

    /// Get members for array types by resolving the language item Array symbol.
    fn array_members(&self, workspace: &WorkspaceQueryContext<'_>) -> Vec<MemberCandidate> {
        let ctx = self;
        // resolve members from the Array language item symbol
        ctx.resolve_language_item_members(workspace, dir::LanguageItem::Array)
    }

    /// Get members for primitive types by resolving the appropriate language item symbol.
    fn primitive_members(
        &self,
        workspace: &WorkspaceQueryContext<'_>,
        primitive: dir::PrimitiveType,
    ) -> Vec<MemberCandidate> {
        let ctx = self;
        // map primitive types to their backing language item types
        let language_item = match primitive {
            dir::PrimitiveType::String => Some(dir::LanguageItem::String),
            dir::PrimitiveType::Symbol | dir::PrimitiveType::UniqueSymbol => {
                Some(dir::LanguageItem::Symbol)
            }
            dir::PrimitiveType::Integer(_)
            | dir::PrimitiveType::Float(_)
            | dir::PrimitiveType::Boolean
            | dir::PrimitiveType::Bigint
            | dir::PrimitiveType::Character => None,
        };

        // return members for the resolved language item symbol
        language_item
            .map(|item| ctx.resolve_language_item_members(workspace, item))
            .unwrap_or_default()
    }

    /// Get members for scalar literals by resolving the backing language item symbol.
    fn literal_members(
        &self,
        workspace: &WorkspaceQueryContext<'_>,
        literal: &dir::ScalarLiteral,
    ) -> Vec<MemberCandidate> {
        let ctx = self;
        // map scalar literals to their backing language item types
        let language_item = match literal {
            dir::ScalarLiteral::String(_) => Some(dir::LanguageItem::String),
            dir::ScalarLiteral::Null
            | dir::ScalarLiteral::Undefined
            | dir::ScalarLiteral::Integer(_)
            | dir::ScalarLiteral::Float(_)
            | dir::ScalarLiteral::Boolean(_)
            | dir::ScalarLiteral::Bigint(_)
            | dir::ScalarLiteral::Character(_)
            | dir::ScalarLiteral::RegexString { .. } => None,
        };

        // return members for the resolved language item symbol
        language_item
            .map(|item| ctx.resolve_language_item_members(workspace, item))
            .unwrap_or_default()
    }

    /// Resolve members from a language item symbol (Array, String, etc.).
    fn resolve_language_item_members(
        &self,
        workspace: &WorkspaceQueryContext<'_>,
        item: dir::LanguageItem,
    ) -> Vec<MemberCandidate> {
        let ctx = self;
        // resolve the exact language item symbol from the current profile
        let environment = ctx.global_environment();
        let Some(symbol_id) = environment.language.symbol(item) else {
            return Vec::new();
        };

        ctx.resolve_reference_members(workspace, symbol_id)
    }
}
