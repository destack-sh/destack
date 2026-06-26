use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, Decision, Dependency, Origin, TypeRewrite, TypeSubstitution, answer,
};
use crate::{CompilerError, CompilerResult};

/// Result of looking up one member on a receiver type.
#[derive(Debug, Clone)]
pub(in crate::check) enum MemberLookup {
    /// Lookup is waiting on unresolved dependencies.
    Pending(SmallVec<[Dependency; 2]>),
    /// No member exists.
    Missing,
    /// One structural field exists.
    Field(dir::GlobalTypeId),
    /// One or more declaration-backed members exist.
    Found(Vec<MemberCandidate>),
}

/// One active member lookup query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct MemberQuery {
    /// The module whose visibility rules apply.
    module: ModuleId,
    /// The reduced receiver type.
    receiver: dir::GlobalTypeId,
    /// The selected member namespace.
    space: dir::MemberSpace,
    /// The selected member key.
    key: dir::StaticKey,
}

/// How one declaration member behaves at a use site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum MemberRole {
    /// Field members use regular assignability and project their own type.
    Field,
    /// Method members use receiver-free method assignability.
    Method,
    /// Getter members project their return type.
    Getter,
    /// Associated members use regular assignability.
    Associated,
    /// Variant members carry a static constructor value.
    Variant,
}

/// One declaration member visible to member lookup.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct DeclaredMember {
    /// The declaring member symbol.
    pub(in crate::check) symbol: Option<dir::GlobalSymbolId>,
    /// The member space.
    pub(in crate::check) space: dir::MemberSpace,
    /// The member key.
    pub(in crate::check) key: Option<dir::StaticKey>,
    /// The member type when the declaration carries a type.
    pub(in crate::check) ty: Option<dir::GlobalTypeId>,
    /// The member static value when it carries one.
    pub(in crate::check) value: Option<dir::GlobalStaticId>,
    /// The member availability condition.
    pub(in crate::check) condition: Option<dir::GlobalTypeId>,
    /// How the member behaves at a use site.
    pub(in crate::check) role: MemberRole,
}

impl MemberLookup {
    /// Return a lookup from collected candidates.
    pub(in crate::check) fn from_candidates(candidates: Vec<MemberCandidate>) -> Self {
        if candidates.is_empty() {
            Self::Missing
        } else {
            Self::Found(candidates)
        }
    }
}

impl MemberRole {
    /// Return the use-site role of one definition member.
    pub(in crate::check) fn from_definition(member: &dir::DefinitionMember) -> Option<Self> {
        match member {
            dir::DefinitionMember::Field(_) => Some(Self::Field),
            dir::DefinitionMember::Method(method)
                if method.role == Some(dir::FunctionRole::Getter) =>
            {
                Some(Self::Getter)
            }
            dir::DefinitionMember::Method(_) => Some(Self::Method),
            dir::DefinitionMember::AssociatedType(_)
            | dir::DefinitionMember::AssociatedConst(_) => Some(Self::Associated),
            dir::DefinitionMember::Variant(_) => Some(Self::Variant),
            dir::DefinitionMember::CallSignature(_)
            | dir::DefinitionMember::ConstructSignature(_)
            | dir::DefinitionMember::IndexSignature(_) => None,
        }
    }

    /// Return whether this role uses method assignability.
    pub(in crate::check) fn uses_method_assignability(self) -> bool {
        matches!(self, Self::Method | Self::Getter)
    }
}

impl DeclaredMember {
    /// Return the lookup member represented by one definition member.
    pub(in crate::check) fn from_definition(member: &dir::DefinitionMember) -> Option<Self> {
        let role = MemberRole::from_definition(member)?;

        Some(Self {
            symbol: member.symbol(),
            space: member.space(),
            key: member.key(),
            ty: member.ty(),
            value: member.value(),
            condition: member.condition(),
            role,
        })
    }

    /// Return whether this member matches one lookup key.
    pub(in crate::check) fn matches(&self, space: dir::MemberSpace, key: dir::StaticKey) -> bool {
        self.space == space && self.key == Some(key)
    }

    /// Return the member read type after getter projection.
    pub(in crate::check) fn read_type(
        &self,
        check: &CheckState<'_>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if self.role != MemberRole::Getter {
            return Ok(ty);
        }

        match check.ty(ty)? {
            dir::Type::FunctionSignature(function) => Ok(function.return_type.unwrap_or(ty)),
            _ => Ok(ty),
        }
    }
}

/// One declaration-backed member candidate.
#[derive(Debug, Clone)]
pub(in crate::check) struct MemberCandidate {
    /// The declaring member symbol.
    pub(in crate::check) symbol: Option<dir::GlobalSymbolId>,
    /// The declaration that exposed the member.
    pub(in crate::check) owner: dir::GlobalSymbolId,
    /// How the selected member behaves at a use site.
    pub(in crate::check) role: MemberRole,
    /// The substituted member type.
    pub(in crate::check) ty: dir::GlobalTypeId,
    /// The generic arguments selected while matching the owner.
    pub(in crate::check) generic_arguments: Vec<dir::GenericArgumentBinding>,
    /// The member static value when it carries one.
    pub(in crate::check) value: Option<dir::GlobalStaticId>,
    /// The substituted static value of the member, when it has one.
    pub(in crate::check) value_type: Option<dir::GlobalTypeId>,
}

impl CheckState<'_> {
    /// Select the member meaning of one member access node.
    pub(in crate::check) fn select_member(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        name: Option<dir::StringId>,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let node = node.into_any();
        let origin = Origin::Node(node);

        let Some(name) = name else {
            return Err(CompilerError::Internal {
                message: format!("member node {node:?} has no name"),
            });
        };
        let key = dir::StaticKey::Name(name);

        // read and close the receiver
        let receiver_node = left.into_global_any(module);
        let receiver = answer!(self.node_type_answer(receiver_node)?);
        let receiver = answer!(self.split_nullish_receiver(origin, node, receiver)?);
        let receiver = answer!(self.reduce_type_root(origin, receiver)?);

        // choose static or instance member space from the receiver expression
        let space = self.member_receiver_space(receiver_node, receiver)?;
        let lookup = self.lookup_member(origin, module, receiver, space, key)?;

        match lookup {
            MemberLookup::Field(ty) => {
                let target = dir::MemberTarget::Field(key);
                let resolution = dir::MemberResolution::new(receiver, target);
                self.record_decision(node, Decision::Member(resolution))?;
                self.bind_node_type(node, ty)?;

                Ok(Answer::Ready(()))
            }
            MemberLookup::Found(candidates) => {
                let candidates = candidates
                    .into_iter()
                    .collect::<SmallVec<[MemberCandidate; 2]>>();

                match candidates.as_slice() {
                    [] => {
                        let key = self.module(module).strings.get(name).to_string();

                        self.reject_member(node, origin, receiver, key)
                    }
                    [candidate] => {
                        let target = match candidate.symbol {
                            Some(symbol) => dir::MemberTarget::Symbol(dir::MemberCandidate {
                                receiver,
                                owner: candidate.owner,
                                symbol,
                                ty: candidate.ty,
                                generic_arguments: candidate.generic_arguments.clone(),
                            }),
                            None => dir::MemberTarget::Field(key),
                        };
                        let resolution = dir::MemberResolution::new(receiver, target);
                        self.record_decision(node, Decision::Member(resolution))?;
                        self.bind_node_type(node, candidate.ty)?;

                        Ok(Answer::Ready(()))
                    }
                    many => {
                        let is_union = self
                            .reduce_type_root(origin, receiver)?
                            .ready()
                            .is_some_and(|receiver| {
                                matches!(self.ty(receiver), Ok(dir::Type::Union(_)))
                            });
                        let mut selected = Vec::new();
                        let mut types = Vec::with_capacity(many.len());
                        for candidate in many {
                            types.push(candidate.ty);
                            let Some(symbol) = candidate.symbol else {
                                continue;
                            };
                            selected.push(dir::MemberCandidate {
                                receiver,
                                owner: candidate.owner,
                                symbol,
                                ty: candidate.ty,
                                generic_arguments: candidate.generic_arguments.clone(),
                            });
                        }

                        if selected.is_empty() {
                            let key = self.module(module).strings.get(name).to_string();

                            return self.reject_member(node, origin, receiver, key);
                        }

                        let target = if is_union {
                            dir::MemberTarget::Universal(selected)
                        } else {
                            dir::MemberTarget::Existential(selected)
                        };
                        let ty = self.normalized_union_type(module, types, node.local_id)?;
                        let resolution = dir::MemberResolution::new(receiver, target);
                        self.record_decision(node, Decision::Member(resolution))?;
                        self.bind_node_type(node, ty)?;

                        Ok(Answer::Ready(()))
                    }
                }
            }
            MemberLookup::Missing => {
                let key = self.module(module).strings.get(name).to_string();

                self.reject_member(node, origin, receiver, key)
            }
            MemberLookup::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Reject one member access with a diagnostic.
    fn reject_member(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: String,
    ) -> CompilerResult<Answer<()>> {
        self.report_missing_member(origin, receiver, key)?;
        self.record_decision(node, Decision::Rejected)?;

        Ok(Answer::Ready(()))
    }

    /// Report nullish receiver arms and return the readable receiver.
    ///
    /// Member selection continues over the non-nullish elements.
    fn split_nullish_receiver(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let reduced = answer!(self.reduce_type_root(origin, receiver)?);
        let dir::Type::Union(union) = self.ty(reduced)? else {
            return Ok(Answer::Ready(receiver));
        };

        // split nullish arms from the readable arms
        let elements = union.elements.iter().copied().collect::<SmallVec<[_; 4]>>();
        let mut has_null = false;
        let mut has_undefined = false;
        let mut non_nullish = Vec::with_capacity(elements.len());
        for element in elements {
            match self.ty(element)? {
                dir::Type::Null => has_null = true,
                dir::Type::Undefined => has_undefined = true,
                _ => non_nullish.push(element),
            }
        }
        if !has_null && !has_undefined {
            return Ok(Answer::Ready(receiver));
        }

        // report the nullish arms that block the read
        let nullish = match (has_null, has_undefined) {
            (true, true) => "null or undefined",
            (true, false) => "null",
            (false, true) => "undefined",
            (false, false) => unreachable!("nullish split requires a nullish part"),
        };
        self.report_possibly_nullish(origin, nullish.to_string())?;

        // continue selection over the readable part
        let source = self.origin_source_node(origin)?;
        let value = match non_nullish.as_slice() {
            [] => self.push_type(node.module_id, dir::Type::Never, source)?,
            [single] => *single,
            _ => self.push_type(
                node.module_id,
                dir::Type::Union(dir::UnionType {
                    elements: non_nullish,
                }),
                source,
            )?,
        };

        Ok(Answer::Ready(value))
    }

    /// Return the member space selected by one receiver expression.
    fn member_receiver_space(
        &mut self,
        receiver: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::MemberSpace> {
        if matches!(self.ty(ty)?, dir::Type::Reference(_)) {
            return Ok(dir::MemberSpace::Static);
        }

        let symbol = match self.solver.decision(receiver) {
            Some(Decision::Name(resolution)) => match resolution.symbols() {
                [symbol] => Some(*symbol),
                _ => None,
            },
            Some(Decision::Instantiation(resolution)) => Some(resolution.symbol),
            _ => None,
        };
        let Some(symbol) = symbol else {
            return Ok(dir::MemberSpace::Instance);
        };

        let symbol = self.resolve_symbol_alias(symbol)?;
        let space = if self.symbol_kind(symbol).is_nominal() {
            dir::MemberSpace::Static
        } else {
            dir::MemberSpace::Instance
        };

        Ok(space)
    }

    /// Project one type-level member access through its owner.
    /// Returns ready none when the projection must stay symbolic.
    pub(in crate::check) fn project_member(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let module = origin.module();
        let lookup = self.lookup_member(
            origin,
            module,
            member.owner,
            dir::MemberSpace::Static,
            member.key,
        )?;

        match lookup {
            // single projections substitute member arguments
            MemberLookup::Field(ty) => Ok(Answer::Ready(Some(ty))),
            MemberLookup::Found(candidates) => match candidates.as_slice() {
                [candidate] => {
                    // comptime const projections yield their static values
                    if let Some(written) = candidate.value_type {
                        return Ok(Answer::Ready(Some(written)));
                    }
                    if let Some(value) = candidate.value {
                        let source = self.origin_source_node(origin)?;
                        let spelling = self.push_type(module, dir::Type::Static(value), source)?;

                        return Ok(Answer::Ready(Some(spelling)));
                    }

                    Ok(Answer::Ready(Some(candidate.ty)))
                }
                _ => Ok(Answer::Ready(None)),
            },
            MemberLookup::Missing => Ok(Answer::Ready(None)),
            MemberLookup::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Look up one member on a receiver type.
    pub(in crate::check) fn lookup_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let mut active_queries = IndexSet::new();

        self.lookup_member_query(origin, module, receiver, space, key, &mut active_queries)
    }

    /// Look up one member while tracking active recursive queries.
    fn lookup_member_query(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        active: &mut IndexSet<MemberQuery>,
    ) -> CompilerResult<MemberLookup> {
        // close the receiver root first
        let receiver = match self.reduce_type_root(origin, receiver)? {
            Answer::Ready(receiver) => receiver,
            Answer::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
        };

        // stop recursive proof paths through constraints and unions
        let query = MemberQuery {
            module,
            receiver,
            space,
            key,
        };
        if !active.insert(query) {
            return Ok(MemberLookup::Missing);
        }

        let lookup = self.lookup_member_receiver(origin, module, receiver, space, key, active);
        active.swap_remove(&query);

        lookup
    }

    /// Look up one member on an already reduced receiver type.
    fn lookup_member_receiver(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        active: &mut IndexSet<MemberQuery>,
    ) -> CompilerResult<MemberLookup> {
        match self.ty(receiver)? {
            // memory forms look through their payloads
            dir::Type::Form(role) => {
                let value = role.value;

                self.lookup_member_query(origin, module, value, space, key, active)
            }

            // declaration references search static members
            dir::Type::Reference(reference) => {
                self.lookup_declaration_member(origin, module, *reference, space, key)
            }

            // applied declarations search their definition members
            dir::Type::Instance(instance) => {
                let instance = instance.clone();

                self.lookup_symbol_member(origin, module, receiver, instance, space, key)
            }

            // enum members use the owner enum's instance members
            dir::Type::EnumMember(member) => {
                self.lookup_member_query(origin, module, member.owner, space, key, active)
            }

            // generic parameters search through their constraints
            dir::Type::Parameter(parameter) => {
                self.lookup_constraint_member(origin, module, *parameter, space, key, active)
            }

            // structural shapes expose their fields
            dir::Type::Shape(shape) => {
                let field = shape
                    .fields
                    .iter()
                    .find(|field| field.key == key)
                    .map(|field| field.ty);

                match field {
                    Some(ty) => Ok(MemberLookup::Field(ty)),
                    None => Ok(MemberLookup::Missing),
                }
            }

            // tuples expose their labeled elements
            dir::Type::Tuple(tuple) => {
                let element = tuple
                    .elements
                    .iter()
                    .find(|element| {
                        element
                            .label
                            .is_some_and(|label| key == dir::StaticKey::Name(label))
                    })
                    .map(|element| element.ty);

                match element {
                    Some(ty) => Ok(MemberLookup::Field(ty)),
                    None => Ok(MemberLookup::Missing),
                }
            }

            // unions join member lookups across their elements
            dir::Type::Union(union) => {
                let elements = union.elements.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.lookup_union_member(origin, module, &elements, space, key, active)
            }

            // intersections expose every part's members
            dir::Type::Intersection(intersection) => {
                let elements = intersection
                    .elements
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 4]>>();
                for element in elements {
                    let element = self.settled_root(element)?;
                    match self.lookup_member_query(origin, module, element, space, key, active)? {
                        MemberLookup::Missing => continue,
                        lookup => return Ok(lookup),
                    }
                }

                Ok(MemberLookup::Missing)
            }

            // scalars search their language-item owners
            dir::Type::Literal(literal) => {
                let owner = literal.owner_item();

                self.lookup_language_item_member(
                    origin,
                    module,
                    receiver,
                    owner,
                    Vec::new(),
                    space,
                    key,
                )
            }
            dir::Type::Primitive(primitive) => {
                let owner = primitive.owner_item();

                self.lookup_language_item_member(
                    origin,
                    module,
                    receiver,
                    owner,
                    Vec::new(),
                    space,
                    key,
                )
            }
            // collection views search their owner declarations
            dir::Type::Array(array) => self.lookup_language_item_member(
                origin,
                module,
                receiver,
                Some(dir::LanguageItem::Array),
                vec![array.element],
                space,
                key,
            ),
            dir::Type::Slice(slice) => self.lookup_language_item_member(
                origin,
                module,
                receiver,
                Some(dir::LanguageItem::Slice),
                vec![slice.element],
                space,
                key,
            ),
            dir::Type::FixedArray(array) => self.lookup_language_item_member(
                origin,
                module,
                receiver,
                Some(dir::LanguageItem::FixedArray),
                vec![array.element, array.count],
                space,
                key,
            ),

            _ => Ok(MemberLookup::Missing),
        }
    }

    /// Look up one member through a generic parameter's constraint.
    fn lookup_constraint_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        parameter: dir::GlobalGenericParameterId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        active: &mut IndexSet<MemberQuery>,
    ) -> CompilerResult<MemberLookup> {
        let Some(binding) = self.generic_parameter(parameter) else {
            return Ok(MemberLookup::Missing);
        };
        let Some(constraint) = binding.constraint else {
            return Ok(MemberLookup::Missing);
        };

        self.lookup_member_query(origin, module, constraint, space, key, active)
    }

    /// Look up one member through a language item owner declaration.
    fn lookup_language_item_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        owner: Option<dir::LanguageItem>,
        arguments: Vec<dir::GlobalTypeId>,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let Some(owner) = owner else {
            return Ok(MemberLookup::Missing);
        };
        let symbol = self.language_symbol(owner);
        let instance = dir::GenericInstance { symbol, arguments };

        self.lookup_symbol_member(origin, module, receiver, instance, space, key)
    }

    /// Look up one static member on a declaration reference.
    fn lookup_declaration_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        reference: dir::TypeReference,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        if space != dir::MemberSpace::Static {
            return Ok(MemberLookup::Missing);
        }

        // resolve aliases before reading declaration members
        let symbol = self.resolve_symbol_alias(reference.symbol)?;
        if !self.is_component_module(symbol.module_id) {
            self.import_external_module(symbol.module_id)?;
        }

        // search declaration members before extensions
        let inherent = self.lookup_inherent_declaration_member(origin, module, symbol, key)?;
        match inherent {
            MemberLookup::Found(_) | MemberLookup::Field(_) | MemberLookup::Pending(_) => {
                return Ok(inherent);
            }
            MemberLookup::Missing => {}
        }

        self.lookup_static_extension_member(origin, module, symbol, key)
    }

    /// Join member lookups across union elements.
    fn lookup_union_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        elements: &[dir::GlobalTypeId],
        space: dir::MemberSpace,
        key: dir::StaticKey,
        active: &mut IndexSet<MemberQuery>,
    ) -> CompilerResult<MemberLookup> {
        let mut candidates = Vec::new();
        let mut fields = SmallVec::<[dir::GlobalTypeId; 4]>::new();

        // every element must expose the member
        for element in elements {
            match self.lookup_member_query(origin, module, *element, space, key, active)? {
                MemberLookup::Field(ty) => fields.push(ty),
                MemberLookup::Found(found) => candidates.extend(found),
                MemberLookup::Missing => return Ok(MemberLookup::Missing),
                MemberLookup::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
            }
        }

        // pure field unions join into one field type
        if candidates.is_empty() {
            let joined = match fields.as_slice() {
                [single] => *single,
                _ => {
                    let source = self.origin_source_node(origin)?;
                    let union = dir::Type::Union(dir::UnionType {
                        elements: fields.into_iter().collect(),
                    });

                    self.push_type(origin.module(), union, source)?
                }
            };

            return Ok(MemberLookup::Field(joined));
        }

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Look up one member on a declaration reference.
    fn lookup_symbol_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        instance: dir::GenericInstance,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let mut instance = instance;
        instance.symbol = self.resolve_symbol_alias(instance.symbol)?;
        if !self.is_component_module(instance.symbol.module_id) {
            self.import_external_module(instance.symbol.module_id)?;
        }

        // search inherent members before extensions
        let inherent =
            self.lookup_inherent_member(origin, module, receiver, &instance, space, key)?;
        match inherent {
            MemberLookup::Found(_) | MemberLookup::Field(_) | MemberLookup::Pending(_) => {
                return Ok(inherent);
            }
            MemberLookup::Missing => {}
        }

        self.lookup_extension_member(origin, module, receiver, &instance, space, key)
    }

    /// Look up one inherent static member on a declaration reference.
    fn lookup_inherent_declaration_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let Some(definition) = self.definition(symbol) else {
            return Ok(MemberLookup::Missing);
        };
        let members = definition
            .members_with_key(dir::MemberSpace::Static, key)
            .filter_map(DeclaredMember::from_definition)
            .collect::<SmallVec<[_; 2]>>();
        let source = self.origin_source_node(origin)?;
        let substitution = TypeSubstitution::default();
        let mut candidates = Vec::new();

        // collect visible static declaration members
        for member in members {
            let Some(ty) = member.ty else {
                continue;
            };
            match self.decide_member_availability(
                origin,
                module,
                member.condition,
                &substitution,
            )? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => continue,
                Answer::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
            }
            let ty = member.read_type(self, ty)?;
            let written = member.symbol.and_then(|symbol| self.static_value(symbol));

            candidates.push(MemberCandidate {
                symbol: member.symbol,
                owner: symbol,
                role: member.role,
                ty: self.fold_type(module, source, ty, TypeRewrite::Resolve)?,
                generic_arguments: Vec::new(),
                value: member.value,
                value_type: written,
            });
        }

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Look up one inherent member on a declaration, walking its heritage.
    fn lookup_inherent_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        instance: &dir::GenericInstance,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        // collect own members and heritage applications
        let Some(definition) = self.definition(instance.symbol) else {
            return Ok(MemberLookup::Missing);
        };
        let members = definition
            .members_with_key(space, key)
            .filter_map(DeclaredMember::from_definition)
            .collect::<SmallVec<[_; 2]>>();
        let heritages = definition
            .bases()
            .iter()
            .map(|heritage| (heritage.symbol, heritage.arguments.clone()))
            .collect::<SmallVec<[_; 2]>>();

        // substitute applied arguments and the qualified receiver
        let substitution = self
            .instance_substitution(instance)?
            .with_receiver(receiver);
        let source = self.origin_source_node(origin)?;
        let mut candidates = Vec::new();
        for member in members {
            let symbol = member.symbol;
            let Some(ty) = member.ty else {
                continue;
            };

            // gate candidates on their substituted @if availability
            match self.decide_member_availability(
                origin,
                module,
                member.condition,
                &substitution,
            )? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => continue,
                Answer::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
            }
            let ty = if substitution.is_empty() {
                ty
            } else {
                self.fold_type(module, source, ty, substitution.rewrite())?
            };

            let ty = member.read_type(self, ty)?;

            // carry substituted static value types for projections
            let written = match symbol.and_then(|symbol| self.static_value(symbol)) {
                Some(written) if !substitution.is_empty() => {
                    Some(self.fold_type(module, source, written, substitution.rewrite())?)
                }
                written => written,
            };

            let generic_arguments =
                self.symbol_generic_argument_bindings(instance.symbol, &instance.arguments)?;

            candidates.push(MemberCandidate {
                symbol,
                owner: instance.symbol,
                role: member.role,
                ty,
                generic_arguments,
                value: member.value,
                value_type: written,
            });
        }
        if !candidates.is_empty() {
            return Ok(MemberLookup::Found(candidates));
        }

        // search substituted heritage applications
        for (symbol, arguments) in heritages {
            // substitute applied arguments into the heritage arguments
            let mut arguments = arguments;
            for argument in &mut arguments {
                if !substitution.is_empty() {
                    *argument =
                        self.fold_type(module, source, *argument, substitution.rewrite())?;
                }
            }

            let heritage = dir::GenericInstance { symbol, arguments };
            let lookup =
                self.lookup_inherent_member(origin, module, receiver, &heritage, space, key)?;
            match lookup {
                MemberLookup::Missing => {}
                lookup => return Ok(lookup),
            }
        }

        Ok(MemberLookup::Missing)
    }

    /// Decide one member's @if availability at a use site.
    ///
    /// Symbolic residues stay unavailable: a use of a conditionally
    /// available member must sit under a guard entailing its condition,
    /// which the active assumptions reduce to a literal.
    pub(in crate::check) fn decide_member_availability(
        &mut self,
        origin: Origin,
        module: ModuleId,
        condition: Option<dir::GlobalTypeId>,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Answer<bool>> {
        let Some(condition) = condition else {
            return Ok(Answer::Ready(true));
        };

        // substitute applied arguments into the declaration-context predicate
        let condition = if substitution.is_empty() {
            condition
        } else {
            let source = self.origin_source_node(origin)?;

            self.fold_type(module, source, condition, substitution.rewrite())?
        };

        // reduce under the use site's guard assumptions
        let reduced = answer!(self.reduce_type_root(origin, condition)?);

        match self.ty(reduced)? {
            dir::Type::Literal(dir::ScalarLiteral::Boolean(holds)) => Ok(Answer::Ready(*holds)),
            _ => Ok(Answer::Ready(false)),
        }
    }
}
