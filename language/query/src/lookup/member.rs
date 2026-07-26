use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use rustc_hash::FxHashSet;

use crate::{ModuleQueryContext, ProgramQueryContext, QueryError, QueryResult};

/// Resolved member candidate.
#[derive(Debug, Clone)]
pub(crate) struct MemberCandidate {
    /// The name of the member.
    pub name: MemberName,
    /// The type of the member.
    pub type_id: Option<dir::GlobalTypeId>,
    /// The member kind.
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
    /// A field.
    Field,
    /// A property without one callable insertion form.
    Property,
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
            dir::MemberKind::Property => Self::Property,
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

/// Type ids on the current member lookup path.
#[derive(Debug, Default)]
struct ActiveTypes {
    /// The type ids on the current path.
    ids: FxHashSet<dir::GlobalTypeId>,
}

impl ActiveTypes {
    /// Enter one type id.
    fn enter(&mut self, type_id: dir::GlobalTypeId) -> bool {
        self.ids.insert(type_id)
    }

    /// Leave one type id.
    fn leave(&mut self, type_id: dir::GlobalTypeId) {
        self.ids.remove(&type_id);
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

    /// Retain only metadata shared with another member of the same name.
    fn merge(&mut self, other: &Self) {
        if self.type_id != other.type_id {
            self.type_id = None;
        }
        if self.kind != other.kind {
            self.kind = MemberKind::Property;
        }
        if self.symbol_id != other.symbol_id {
            self.symbol_id = None;
        }
        self.is_extension &= other.is_extension;
    }
}

impl ModuleQueryContext<'_> {
    /// Return completion members visible on one checked type.
    pub(crate) fn resolve_type_members(
        &self,
        program: &ProgramQueryContext<'_>,
        environment: &GlobalEnvironment,
        type_id: dir::GlobalTypeId,
        is_optional: bool,
    ) -> QueryResult<Vec<MemberCandidate>> {
        let mut active_types = ActiveTypes::default();

        self.read_global_type(program, type_id, |checked_type, type_module| {
            type_module.resolve_type_members_inner(
                program,
                environment,
                type_id,
                checked_type,
                &mut active_types,
                is_optional,
            )
        })?
    }

    /// Resolve members from a reference type by looking up the symbol.
    pub(crate) fn resolve_reference_members(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
        space: dir::MemberSpace,
    ) -> QueryResult<Vec<MemberCandidate>> {
        let Some(symbol_id) = program.canonical_symbol(symbol_id)? else {
            return Ok(Vec::new());
        };
        let mut members = program
            .owner_members(symbol_id)?
            .into_iter()
            .filter(|entry| entry.origin != dir::MemberOrigin::Extension && entry.space == space)
            .map(MemberCandidate::from_entry)
            .collect::<Vec<_>>();

        // merge extension members for this symbol
        let extension_members = self.extension_members(program, symbol_id, space)?;

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

        Ok(members)
    }

    /// Resolve checked type members while tracking active type ids.
    fn resolve_type_members_inner(
        &self,
        program: &ProgramQueryContext<'_>,
        environment: &GlobalEnvironment,
        type_id: dir::GlobalTypeId,
        checked_type: &dir::Type,
        active_types: &mut ActiveTypes,
        is_optional: bool,
    ) -> QueryResult<Vec<MemberCandidate>> {
        if !active_types.enter(type_id) {
            return Ok(Vec::new());
        }

        let strings = self.strings();

        let members = match checked_type {
            // resolve declaration and extension members
            dir::Type::Reference(reference) => {
                self.resolve_reference_members(program, reference.symbol, dir::MemberSpace::Static)?
            }
            dir::Type::Application(reference) => self.resolve_reference_members(
                program,
                reference.symbol,
                dir::MemberSpace::Instance,
            )?,

            // build structural field and signature members
            dir::Type::Shape(object) => {
                let mut members = Vec::new();

                for field in self.types().fields(object.fields) {
                    members.push(MemberCandidate {
                        name: MemberName::from_static_key(&field.key, strings),
                        type_id: Some(field.ty),
                        kind: MemberKind::Property,
                        symbol_id: None,
                        is_extension: false,
                    });
                }

                for signature_type_id in self.types().type_ids(object.call_signatures) {
                    members.push(MemberCandidate {
                        name: MemberName::Computed,
                        type_id: Some(*signature_type_id),
                        kind: MemberKind::CallSignature,
                        symbol_id: None,
                        is_extension: false,
                    });
                }

                for signature_type_id in self.types().type_ids(object.construct_signatures) {
                    members.push(MemberCandidate {
                        name: MemberName::Computed,
                        type_id: Some(*signature_type_id),
                        kind: MemberKind::ConstructSignature,
                        symbol_id: None,
                        is_extension: false,
                    });
                }

                members
            }

            // keep members common to every union element
            dir::Type::Union(union) => {
                let mut elements = Vec::new();
                for element_id in self.types().type_ids(union.elements) {
                    if is_optional && self.type_is_nullish(program, *element_id)? {
                        continue;
                    }

                    elements.push(*element_id);
                }

                if elements.is_empty() {
                    Vec::new()
                } else {
                    let first_id = elements[0];
                    let mut common_members =
                        self.read_global_type(program, first_id, |checked_type, type_module| {
                            type_module.resolve_type_members_inner(
                                program,
                                environment,
                                first_id,
                                checked_type,
                                active_types,
                                is_optional,
                            )
                        })??;

                    for element_id in &elements[1..] {
                        let element_members = self.read_global_type(
                            program,
                            *element_id,
                            |checked_type, type_module| {
                                type_module.resolve_type_members_inner(
                                    program,
                                    environment,
                                    *element_id,
                                    checked_type,
                                    active_types,
                                    is_optional,
                                )
                            },
                        )??;

                        common_members.retain_mut(|member| {
                            let Some(other) = element_members
                                .iter()
                                .find(|other| member.name.matches(&other.name))
                            else {
                                return false;
                            };

                            member.merge(other);

                            true
                        });
                    }

                    common_members
                }
            }

            // combine members from every intersection element
            dir::Type::Intersection(intersection) => {
                let elements = self.types().type_ids(intersection.elements);
                let mut all_members = Vec::new();

                for element_id in elements {
                    let element_members = self.read_global_type(
                        program,
                        *element_id,
                        |checked_type, type_module| {
                            type_module.resolve_type_members_inner(
                                program,
                                environment,
                                *element_id,
                                checked_type,
                                active_types,
                                is_optional,
                            )
                        },
                    )??;

                    for member in element_members {
                        if let Some(existing) =
                            all_members
                                .iter_mut()
                                .find(|existing: &&mut MemberCandidate| {
                                    existing.name.matches(&member.name)
                                })
                        {
                            existing.merge(&member);
                        } else {
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

            // use the central Array language item
            dir::Type::Slice(_) => self.array_members(program, environment)?,

            dir::Type::FixedArray(_) => self.array_members(program, environment)?,

            // use primitive language items
            dir::Type::Primitive(primitive) => {
                self.primitive_members(program, environment, *primitive)?
            }

            // use literal language items
            dir::Type::Literal(literal) => self.literal_members(program, environment, literal)?,

            // follow form wrappers
            dir::Type::Form(value) => {
                self.read_global_type(program, value.value, |checked_type, type_module| {
                    type_module.resolve_type_members_inner(
                        program,
                        environment,
                        value.value,
                        checked_type,
                        active_types,
                        is_optional,
                    )
                })??
            }

            // return no members for non member bearing types
            _ => Vec::new(),
        };

        active_types.leave(type_id);

        Ok(members)
    }

    /// Return whether one checked type is `null` or `undefined`.
    fn type_is_nullish(
        &self,
        program: &ProgramQueryContext<'_>,
        type_id: dir::GlobalTypeId,
    ) -> QueryResult<bool> {
        self.read_global_type(program, type_id, |checked_type, _| {
            matches!(checked_type, dir::Type::Null | dir::Type::Undefined)
        })
    }

    /// Resolve extension members for a target symbol across all modules.
    fn extension_members(
        &self,
        program: &ProgramQueryContext<'_>,
        target_symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
    ) -> QueryResult<Vec<MemberCandidate>> {
        let mut members = Vec::new();

        // collect members from visible extension declarations
        for extension in self.visible_extensions(program, target_symbol)? {
            let extension_members = program.declaring_members(extension.declaration)?;
            members.extend(
                extension_members
                    .into_iter()
                    .filter(|member| member.space == space)
                    .map(MemberCandidate::from_entry),
            );
        }

        Ok(members)
    }

    /// Collect visible extensions for one target symbol.
    fn visible_extensions(
        &self,
        program: &ProgramQueryContext<'_>,
        target_symbol: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<dir::ExtensionEntry>> {
        // FUGU #Incomplete: select blanket extensions through checked type relations
        let Some(canonical_target) = program.canonical_symbol(target_symbol)? else {
            return Ok(Vec::new());
        };
        let mut entries = Vec::new();

        // collect extensions indexed for the canonical target
        for entry in program.root_extensions(canonical_target)? {
            let extension_module = program.module(entry.declaration.module_id)?;
            let extension = extension_module
                .definitions()
                .extension_definition(entry.declaration)
                .ok_or(QueryError::missing(format!(
                    "extension definition: {:?}",
                    entry.declaration
                )))?;

            if !extension.is_visible_from(self.module_id()) {
                continue;
            }

            entries.push(entry);
        }

        Ok(entries)
    }

    /// Get members for array types by resolving the language item Array symbol.
    fn array_members(
        &self,
        program: &ProgramQueryContext<'_>,
        environment: &GlobalEnvironment,
    ) -> QueryResult<Vec<MemberCandidate>> {
        // resolve members from the Array language item symbol
        self.resolve_language_item_members(program, environment, dir::LanguageItem::Array)
    }

    /// Get members for primitive types by resolving the appropriate language item symbol.
    fn primitive_members(
        &self,
        program: &ProgramQueryContext<'_>,
        environment: &GlobalEnvironment,
        primitive: dir::PrimitiveType,
    ) -> QueryResult<Vec<MemberCandidate>> {
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
            self.resolve_language_item_members(program, environment, item)
        } else {
            Ok(Vec::new())
        }
    }

    /// Get members for scalar literals by resolving the backing language item symbol.
    fn literal_members(
        &self,
        program: &ProgramQueryContext<'_>,
        environment: &GlobalEnvironment,
        literal: &dir::ScalarLiteral,
    ) -> QueryResult<Vec<MemberCandidate>> {
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
            self.resolve_language_item_members(program, environment, item)
        } else {
            Ok(Vec::new())
        }
    }

    /// Resolve members from a language intrinsic symbol.
    fn resolve_language_item_members(
        &self,
        program: &ProgramQueryContext<'_>,
        environment: &GlobalEnvironment,
        item: dir::LanguageItem,
    ) -> QueryResult<Vec<MemberCandidate>> {
        // resolve the exact language item symbol from the current profile
        let Some(symbol_id) = environment.language.symbol(item) else {
            return Ok(Vec::new());
        };

        self.resolve_reference_members(program, symbol_id, dir::MemberSpace::Instance)
    }
}
