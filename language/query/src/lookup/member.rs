use std::collections::HashSet;

use destack_dir as dir;

use crate::{ModuleQueryContext, ProgramQueryContext};

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
    /// A constructor member.
    Constructor,
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

impl From<dir::MemberKind> for MemberKind {
    /// Convert an indexed source member kind into a completion member kind.
    fn from(kind: dir::MemberKind) -> Self {
        match kind {
            dir::MemberKind::Field => Self::Field,
            dir::MemberKind::Method => Self::Method,
            dir::MemberKind::Constructor => Self::Constructor,
            dir::MemberKind::CallSignature => Self::CallSignature,
            dir::MemberKind::ConstructSignature => Self::ConstructSignature,
            dir::MemberKind::IndexSignature => Self::Field,
            dir::MemberKind::AssociatedType => Self::AssociatedType,
            dir::MemberKind::AssociatedConst => Self::AssociatedConst,
            dir::MemberKind::Variant => Self::EnumMember,
        }
    }
}

impl MemberName {
    /// Build one member name from a static DIR key.
    fn from_static_key(key: &dir::StaticKey, strings: &destack_core::StringPool) -> Self {
        match key {
            dir::StaticKey::Name(string_id) => Self::String(strings.get(*string_id).to_string()),
            dir::StaticKey::Index(index) => Self::Index(*index),
            dir::StaticKey::Symbol(_) => Self::Computed,
        }
    }

    /// Build one member name from an indexed string name.
    fn from_index_name(name: String) -> Self {
        Self::String(name)
    }

    /// Return whether this member name matches another member name.
    fn matches(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Index(left), Self::Index(right)) => left == right,
            _ => false,
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
    fn from_entry(entry: dir::MemberEntry) -> Self {
        Self {
            name: MemberName::from_index_name(entry.name),
            type_id: entry.ty,
            kind: entry.kind.into(),
            symbol_id: entry.symbol,
            is_extension: entry.origin == dir::MemberOrigin::Extension,
        }
    }
}

impl ModuleQueryContext<'_> {
    /// Return the member kind implied by one structural field type.
    fn field_member_kind(&self, type_id: dir::GlobalTypeId) -> MemberKind {
        self.read_global_type(type_id, |ty, _| {
            let is_function = matches!(
                ty,
                dir::Type::FunctionSignature(_)
                    | dir::Type::Function(_)
                    | dir::Type::FunctionPointer(_)
            );

            if is_function {
                MemberKind::Method
            } else {
                MemberKind::Field
            }
        })
    }
}

impl ModuleQueryContext<'_> {
    /// Return completion members visible on one checked type.
    pub(crate) fn resolve_type_members(
        &self,
        program: &ProgramQueryContext<'_>,
        type_id: dir::GlobalTypeId,
    ) -> Vec<MemberCandidate> {
        let mut state = MemberResolutionState::default();

        self.read_global_type(type_id, |ty, type_module| {
            type_module.resolve_type_members_inner(program, type_id, ty, &mut state)
        })
    }

    /// Resolve members from a reference type by looking up the symbol.
    pub(crate) fn resolve_reference_members(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> Vec<MemberCandidate> {
        let symbol_id = self.canonical_symbol(symbol_id);
        let mut members = program
            .owner_members(symbol_id)
            .into_iter()
            .filter(|entry| entry.origin != dir::MemberOrigin::Extension)
            .map(MemberCandidate::from_entry)
            .collect::<Vec<_>>();

        // merge extension members for this symbol
        let extension_members = self.extension_members(program, symbol_id);

        // avoid duplicate member names across direct and extension members
        for member in extension_members {
            let is_duplicate = members
                .iter()
                .any(|existing| existing.name.matches(&member.name));
            if is_duplicate {
                continue;
            }

            members.push(member);
        }

        members
    }

    /// Resolve checked type members while tracking active type ids.
    fn resolve_type_members_inner(
        &self,
        program: &ProgramQueryContext<'_>,
        type_id: dir::GlobalTypeId,
        ty: &dir::Type,
        state: &mut MemberResolutionState,
    ) -> Vec<MemberCandidate> {
        if !state.enter(type_id) {
            return Vec::new();
        }

        let strings = self.strings();

        let members = match ty {
            // resolve declaration and extension members
            dir::Type::Instance(reference) => {
                self.resolve_reference_members(program, reference.symbol)
            }

            // build structural field and signature members
            dir::Type::Shape(object) => {
                let mut members = Vec::new();

                for field in self.types().fields(object.fields) {
                    let kind = self.field_member_kind(field.ty);

                    members.push(MemberCandidate {
                        name: MemberName::from_static_key(&field.key, strings),
                        type_id: Some(field.ty),
                        kind,
                        symbol_id: None,
                        is_extension: false,
                    });
                }

                for sig_type_id in self.types().type_ids(object.call_signatures) {
                    members.push(MemberCandidate {
                        name: MemberName::Computed,
                        type_id: Some(*sig_type_id),
                        kind: MemberKind::CallSignature,
                        symbol_id: None,
                        is_extension: false,
                    });
                }

                for sig_type_id in self.types().type_ids(object.construct_signatures) {
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
                let elements = self.types().type_ids(union.elements);
                if elements.is_empty() {
                    Vec::new()
                } else {
                    let first_id = elements[0];
                    let mut common_members = self.read_global_type(first_id, |ty, type_module| {
                        type_module.resolve_type_members_inner(program, first_id, ty, state)
                    });

                    for element_id in &elements[1..] {
                        let element_members =
                            self.read_global_type(*element_id, |ty, type_module| {
                                type_module.resolve_type_members_inner(
                                    program,
                                    *element_id,
                                    ty,
                                    state,
                                )
                            });

                        common_members.retain(|member| {
                            element_members
                                .iter()
                                .any(|other| member.name.matches(&other.name))
                        });
                    }

                    common_members
                }
            }

            // combine members from every intersection element
            dir::Type::Intersection(intersection) => {
                let elements = self.types().type_ids(intersection.elements);
                let is_enum_static = elements.iter().any(|element_id| {
                    self.read_global_type(*element_id, |element, _| {
                        let dir::Type::Form(value) = element else {
                            return false;
                        };

                        self.read_global_type(value.value, |inner, _| {
                            let dir::Type::Instance(reference) = inner else {
                                return false;
                            };

                            let symbol_module = self.module_context(reference.symbol.module_id);
                            let symbols_table = symbol_module.symbols();
                            let symbol = symbols_table.get_symbol(reference.symbol.local_id);
                            symbol.kind == dir::SymbolKind::Enum
                        })
                    })
                });

                let mut all_members = Vec::new();
                let mut seen_names: Vec<MemberName> = Vec::new();

                for element_id in elements {
                    let element_members = self.read_global_type(*element_id, |ty, type_module| {
                        type_module.resolve_type_members_inner(program, *element_id, ty, state)
                    });

                    for mut member in element_members {
                        if !seen_names.iter().any(|name| name.matches(&member.name)) {
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
            dir::Type::Tuple(tuple) => self
                .types()
                .elements(tuple.elements)
                .iter()
                .enumerate()
                .map(|(element_index, element)| MemberCandidate {
                    name: MemberName::Index(element_index),
                    type_id: Some(element.ty),
                    kind: MemberKind::Field,
                    symbol_id: None,
                    is_extension: false,
                })
                .collect(),

            // use the central array type surface
            dir::Type::Slice(_) => self.array_members(program),

            dir::Type::FixedArray(_) => self.array_members(program),

            // use the primitive backing type surface
            dir::Type::Primitive(primitive) => self.primitive_members(program, *primitive),

            // use the literal backing type surface
            dir::Type::Literal(literal) => self.literal_members(program, literal),

            // follow form wrappers
            dir::Type::Form(value) => self.read_global_type(value.value, |ty, type_module| {
                type_module.resolve_type_members_inner(program, value.value, ty, state)
            }),

            // return no members for non member bearing types
            _ => Vec::new(),
        };

        state.leave(type_id);

        members
    }

    /// Resolve extension members for a target symbol across all modules.
    fn extension_members(
        &self,
        program: &ProgramQueryContext<'_>,
        target_symbol: dir::GlobalSymbolId,
    ) -> Vec<MemberCandidate> {
        let mut members = Vec::new();

        // collect members from visible extension declarations
        for extension in self.visible_extensions(program, target_symbol) {
            let extension_members = program.declaring_members(extension.declaration);
            members.extend(
                extension_members
                    .into_iter()
                    .map(MemberCandidate::from_entry),
            );
        }

        members
    }

    /// Collect visible extensions for one target symbol.
    fn visible_extensions(
        &self,
        program: &ProgramQueryContext<'_>,
        target_symbol: dir::GlobalSymbolId,
    ) -> Vec<dir::ExtensionEntry> {
        let canonical_target = self.canonical_symbol(target_symbol);
        let mut entries = Vec::new();

        // collect extensions indexed for the canonical target
        for entry in program.root_extensions(canonical_target) {
            let extension_module = self.module_context(entry.declaration.module_id);
            let extension = extension_module
                .definitions()
                .extension_definition(entry.declaration)
                .unwrap_or_else(|| panic!("missing extension definition for {entry:?}"));

            if !extension.is_visible_from(self.module_id()) {
                continue;
            }

            entries.push(entry);
        }

        entries
    }

    /// Get members for array types by resolving the language item Array symbol.
    fn array_members(&self, program: &ProgramQueryContext<'_>) -> Vec<MemberCandidate> {
        // resolve members from the Array language item symbol
        self.resolve_language_item_members(program, dir::LanguageItem::Array)
    }

    /// Get members for primitive types by resolving the appropriate language item symbol.
    fn primitive_members(
        &self,
        program: &ProgramQueryContext<'_>,
        primitive: dir::PrimitiveType,
    ) -> Vec<MemberCandidate> {
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

        // return members for primitive types with a backing language item
        if let Some(item) = language_item {
            self.resolve_language_item_members(program, item)
        } else {
            Vec::new()
        }
    }

    /// Get members for scalar literals by resolving the backing language item symbol.
    fn literal_members(
        &self,
        program: &ProgramQueryContext<'_>,
        literal: &dir::ScalarLiteral,
    ) -> Vec<MemberCandidate> {
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

        // return members for literals with a backing language item
        if let Some(item) = language_item {
            self.resolve_language_item_members(program, item)
        } else {
            Vec::new()
        }
    }

    /// Resolve members from a language item symbol (Array, String, etc.).
    fn resolve_language_item_members(
        &self,
        program: &ProgramQueryContext<'_>,
        item: dir::LanguageItem,
    ) -> Vec<MemberCandidate> {
        // resolve the exact language item symbol from the current profile
        let environment = self.global_environment();
        let Some(symbol_id) = environment.language.symbol(item) else {
            return Vec::new();
        };

        self.resolve_reference_members(program, symbol_id)
    }
}
