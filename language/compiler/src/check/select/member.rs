use std::slice;

use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, CallableArgument, CandidateOutcome, CandidateVerdict, CheckState, Decision,
    FlowSite, NullishPart, Origin, PlaceUse, ReceiverSteps, Relation, SignatureMatch,
    TypeSubstitution, Value, ValueUse, answer,
};
use crate::{CompilerError, CompilerResult};

/// One structural property found by member lookup.
#[derive(Debug, Clone)]
pub(in crate::check) struct FieldLookup {
    /// The receiver that exposes the field.
    pub(in crate::check) receiver: dir::MemberReceiver,
    /// The structural aggregate that declares the field.
    pub(in crate::check) owner: dir::GlobalTypeId,
    /// The projected property operations.
    pub(in crate::check) access: dir::PropertyAccess,
    /// Whether the property may be absent.
    pub(in crate::check) is_optional: bool,
}

/// Result of looking up one member on a receiver type.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub(in crate::check) enum MemberLookup {
    /// No member exists.
    Missing,
    /// One structural field exists.
    Field(FieldLookup),
    /// One or more declaration-backed members exist.
    Found(Vec<MemberCandidate>),
    /// Every union receiver arm exposes one member lookup.
    Union(Vec<MemberArmLookup>),
    /// One intersection receiver imposes every constituent lookup.
    Intersection(Vec<MemberLookup>),
}

/// One member lookup selected for a runtime union arm.
#[derive(Debug, Clone)]
pub(in crate::check) struct MemberArmLookup {
    /// The runtime receiver arm.
    pub(in crate::check) receiver: dir::GlobalTypeId,
    /// The member selected on that arm.
    pub(in crate::check) lookup: MemberLookup,
}

impl FieldLookup {
    /// Return the type produced by reading this property.
    pub(in crate::check) fn read_type(
        &self,
        _module: ModuleId,
        body: &mut BodyState<'_, '_>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(read) = self.access.read() else {
            return Ok(None);
        };
        if !self.is_optional {
            return Ok(Some(read));
        }
        let undefined = body.intern_type(dir::Type::Undefined)?;
        let read = body.normalized_union_type([read, undefined])?;

        Ok(Some(read))
    }

    /// Return the type accepted by writing this property.
    pub(in crate::check) fn write_type(&self) -> Option<dir::GlobalTypeId> {
        self.access.write()
    }
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
    /// Setter members accept their first parameter type.
    Setter,
    /// Associated members use regular assignability.
    Associated,
    /// Variant values select one unit case.
    VariantValue,
    /// Variant constructors accept one payload value.
    VariantConstructor,
}

/// One declaration member visible to member lookup.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct DeclaredMember {
    /// The declaring member symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The member space.
    pub(in crate::check) space: dir::MemberSpace,
    /// The member key.
    pub(in crate::check) key: Option<dir::StaticKey>,
    /// The member type when the declaration has one.
    pub(in crate::check) ty: Option<dir::GlobalTypeId>,
    /// The member static value when it carries one.
    pub(in crate::check) value: Option<dir::GlobalStaticId>,
    /// How the member behaves at a use site.
    pub(in crate::check) role: MemberRole,
    /// The member kind.
    pub(in crate::check) kind: dir::MemberKind,
    /// Whether the member accepts writes.
    pub(in crate::check) is_writable: bool,
    /// Whether the member may be absent.
    pub(in crate::check) is_optional: bool,
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

    /// Consume declaration candidates from a lookup without runtime alternatives.
    pub(in crate::check) fn into_candidates(self) -> Option<Vec<MemberCandidate>> {
        match self {
            Self::Found(candidates) => Some(candidates),
            Self::Intersection(lookups) => {
                let mut candidates = Vec::new();
                for lookup in lookups {
                    candidates.extend(lookup.into_candidates()?);
                }

                Some(candidates)
            }
            Self::Missing | Self::Field { .. } | Self::Union(_) => None,
        }
    }

    /// Return whether lookup found at least one member.
    pub(in crate::check) fn is_found(&self) -> bool {
        !matches!(self, Self::Missing)
    }

    /// Return the declaration candidates selected by lookup precedence.
    fn declaration_candidates(&self) -> Vec<&MemberCandidate> {
        match self {
            Self::Missing | Self::Field(_) => Vec::new(),
            Self::Found(candidates) => Self::selected_candidates(candidates),
            Self::Union(arms) => arms
                .iter()
                .flat_map(|arm| arm.lookup.declaration_candidates())
                .collect(),
            Self::Intersection(lookups) => lookups
                .iter()
                .flat_map(Self::declaration_candidates)
                .collect(),
        }
    }

    /// Return the declaration candidates selected from one lookup result.
    fn selected_candidates(candidates: &[MemberCandidate]) -> Vec<&MemberCandidate> {
        let best = candidates.iter().map(MemberCandidate::precedence).min();

        candidates
            .iter()
            .filter(|candidate| Some(candidate.precedence()) == best)
            .collect()
    }

    /// Return whether this lookup may produce an absent member.
    fn is_optional(&self) -> bool {
        match self {
            Self::Missing => false,
            Self::Field(field) => field.is_optional,
            Self::Found(candidates) => Self::selected_candidates(candidates)
                .into_iter()
                .any(|candidate| candidate.is_optional),
            Self::Union(arms) => arms.iter().any(|arm| arm.lookup.is_optional()),
            Self::Intersection(lookups) => {
                !lookups.is_empty() && lookups.iter().all(Self::is_optional)
            }
        }
    }

    /// Return the member kind represented by this lookup.
    fn kind(&self) -> dir::MemberKind {
        let mut kinds = Vec::new();
        self.collect_kinds(&mut kinds);
        kinds.sort();
        kinds.dedup();

        match kinds.as_slice() {
            [] => dir::MemberKind::Field,
            [kind] => *kind,
            _ => dir::MemberKind::Property,
        }
    }

    /// Collect member kinds selected by this lookup.
    fn collect_kinds(&self, kinds: &mut Vec<dir::MemberKind>) {
        match self {
            Self::Missing => {}
            Self::Field(_) => kinds.push(dir::MemberKind::Field),
            Self::Found(candidates) => kinds.extend(
                Self::selected_candidates(candidates)
                    .into_iter()
                    .map(|candidate| candidate.kind),
            ),
            Self::Union(arms) => {
                for arm in arms {
                    arm.lookup.collect_kinds(kinds);
                }
            }
            Self::Intersection(lookups) => {
                for lookup in lookups {
                    lookup.collect_kinds(kinds);
                }
            }
        }
    }

    /// Return whether any selected declaration is write-only.
    fn has_setter(&self) -> bool {
        match self {
            Self::Missing | Self::Field { .. } => false,
            Self::Found(candidates) => candidates
                .iter()
                .any(|candidate| candidate.role == MemberRole::Setter),
            Self::Union(lookups) => lookups.iter().any(|arm| arm.lookup.has_setter()),
            Self::Intersection(lookups) => lookups.iter().any(Self::has_setter),
        }
    }

    /// Prepend one implicit adjustment to every selected receiver.
    pub(in crate::check) fn prepend_adjustment(&mut self, adjustment: dir::ReceiverAdjustment) {
        match self {
            Self::Missing => {}
            Self::Field(field) => field.receiver.prepend(adjustment),
            Self::Found(candidates) => {
                for candidate in candidates {
                    candidate.receiver.prepend(adjustment.clone());
                }
            }
            Self::Union(lookups) => {
                for arm in lookups {
                    arm.lookup.prepend_adjustment(adjustment.clone());
                }
            }
            Self::Intersection(lookups) => {
                for lookup in lookups {
                    lookup.prepend_adjustment(adjustment.clone());
                }
            }
        }
    }

    /// Select every found member through one erased receiver.
    pub(in crate::check) fn select_dynamic(&mut self, dispatch: dir::DynamicDispatch) {
        match self {
            Self::Missing => {}
            Self::Field(field) => {
                field.receiver = dir::MemberReceiver::Dynamic(dispatch);
            }
            Self::Found(candidates) => {
                for candidate in candidates {
                    candidate.receiver = LookupReceiver::Dynamic(dispatch.clone());
                }
            }
            Self::Union(lookups) => {
                for arm in lookups {
                    arm.lookup.select_dynamic(dispatch.clone());
                }
            }
            Self::Intersection(lookups) => {
                for lookup in lookups {
                    lookup.select_dynamic(dispatch.clone());
                }
            }
        }
    }
}

impl MemberRole {
    /// Return whether this role selects a callable declaration.
    pub(in crate::check) fn is_callable(self) -> bool {
        matches!(
            self,
            Self::Method | Self::Getter | Self::Setter | Self::VariantConstructor
        )
    }

    /// Return the use-site role of one definition member.
    pub(in crate::check) fn from_definition(member: &dir::DefinitionMember) -> Option<Self> {
        match member {
            dir::DefinitionMember::Field(_) => Some(Self::Field),
            dir::DefinitionMember::Method(method)
                if method.role == Some(dir::FunctionRole::Getter) =>
            {
                Some(Self::Getter)
            }
            dir::DefinitionMember::Method(method)
                if method.role == Some(dir::FunctionRole::Setter) =>
            {
                Some(Self::Setter)
            }
            dir::DefinitionMember::Method(_) => Some(Self::Method),
            dir::DefinitionMember::AssociatedType(_)
            | dir::DefinitionMember::AssociatedConst(_) => Some(Self::Associated),
            dir::DefinitionMember::EnumVariant(_) => Some(Self::VariantValue),
            dir::DefinitionMember::TaggedKey(_) => Some(Self::VariantValue),
            dir::DefinitionMember::TaggedVariant(variant) if variant.argument.is_some() => {
                Some(Self::VariantConstructor)
            }
            dir::DefinitionMember::TaggedVariant(_) => Some(Self::VariantValue),
            dir::DefinitionMember::CallSignature(_)
            | dir::DefinitionMember::ConstructSignature(_)
            | dir::DefinitionMember::IndexSignature(_) => None,
        }
    }

    /// Return whether this role can be read by member access.
    pub(in crate::check) fn is_readable(self) -> bool {
        !matches!(self, Self::Setter)
    }
}

impl DeclaredMember {
    /// Return whether this member matches one lookup key.
    pub(in crate::check) fn matches(&self, space: dir::MemberSpace, key: dir::StaticKey) -> bool {
        self.space == space && self.key == Some(key)
    }

    /// Return the value type exposed by this member at a use site.
    pub(in crate::check) fn access_type(
        &self,
        check: &CheckState<'_>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if !matches!(self.role, MemberRole::Getter | MemberRole::Setter) {
            return Ok(ty);
        }
        let dir::Type::FunctionSignature(function) = check.ty(ty)? else {
            return Err(CompilerError::Internal {
                message: format!(
                    "accessor member {:?} has non-signature type {ty:?}",
                    self.symbol
                ),
            });
        };
        let function = check.type_signature(ty.module_id, function)?;

        if self.role == MemberRole::Getter {
            return function.return_type.ok_or_else(|| CompilerError::Internal {
                message: format!("getter member {:?} has no result type", self.symbol),
            });
        }

        // setters expose their one written parameter as the property type
        let parameters = check.signature_parameters(ty.module_id, function.parameters)?;
        let [parameter] = parameters else {
            return Err(CompilerError::Internal {
                message: format!(
                    "setter member {:?} has {} value parameters",
                    self.symbol,
                    parameters.len(),
                ),
            });
        };

        Ok(parameter.ty)
    }

    /// Return the callable type exposed by this declaration member.
    pub(in crate::check) fn callable_type(
        &self,
        _module: ModuleId,
        owner: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
        body: &mut BodyState<'_, '_>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        if self.role.is_callable() {
            return Ok(Some(ty));
        }
        if self.role != MemberRole::VariantValue {
            return Ok(None);
        }

        // tagged unit variants also admit the explicit zero argument form
        let is_unit_variant = match body.definition(owner)? {
            Some(dir::Definition::Newtype(definition)) => definition
                .tagged_variant_by_symbol(self.symbol)
                .is_some_and(|variant| variant.argument.is_none()),
            _ => false,
        };
        if !is_unit_variant {
            return Ok(None);
        }

        let parameters = body.intern_parameters(&[])?;
        let callable = body.intern_signature(dir::FunctionSignatureType {
            asynchrony: dir::Asynchrony::Sync,
            template: None,
            this_parameter: None,
            parameters,
            return_type: Some(ty),
            is_generator: false,
        })?;

        Ok(Some(callable))
    }
}

/// One declaration-backed member candidate.
#[derive(Debug, Clone)]
pub(in crate::check) struct MemberCandidate {
    /// The selected member key.
    pub(in crate::check) key: dir::StaticKey,
    /// The declaring member symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The declaration that exposed the member.
    pub(in crate::check) owner: dir::GlobalSymbolId,
    /// The declaration family the member came from.
    pub(in crate::check) origin: dir::MemberOrigin,
    /// The member space that selected this candidate.
    pub(in crate::check) space: dir::MemberSpace,
    /// How the member behaves at a use site.
    pub(in crate::check) role: MemberRole,
    /// The member kind.
    pub(in crate::check) kind: dir::MemberKind,
    /// Whether the member accepts writes.
    pub(in crate::check) is_writable: bool,
    /// The substituted type before optional read widening.
    pub(in crate::check) access_type: dir::GlobalTypeId,
    /// The substituted callable type for methods and accessors.
    pub(in crate::check) callable: Option<dir::GlobalTypeId>,
    /// Whether the member may be absent.
    pub(in crate::check) is_optional: bool,
    /// The generic arguments matched through the owner.
    pub(in crate::check) generic_arguments: Vec<dir::GenericArgumentBinding>,
    /// The member static value when it carries one.
    pub(in crate::check) value: Option<dir::GlobalStaticId>,
    /// The substituted static value of the member, when it has one.
    pub(in crate::check) value_type: Option<dir::GlobalTypeId>,
    /// The receiver adjustments selected during lookup.
    pub(in crate::check) receiver: LookupReceiver,
}

/// Receiver state retained by one member lookup candidate.
#[derive(Debug, Clone)]
pub(in crate::check) enum LookupReceiver {
    /// Direct adjustments attached to the use-site receiver on resolution.
    Direct(ReceiverSteps),
    /// Erased receiver already selected for dynamic dispatch.
    Dynamic(dir::DynamicDispatch),
}

impl MemberCandidate {
    /// Return this candidate's selection precedence.
    pub(in crate::check) fn precedence(&self) -> (dir::MemberOrigin, bool) {
        let is_adjusted = match &self.receiver {
            LookupReceiver::Direct(steps) => !steps.is_empty(),
            LookupReceiver::Dynamic(_) => true,
        };

        (self.origin, is_adjusted)
    }
}

impl LookupReceiver {
    /// Prepend one adjustment performed before the existing adjustments.
    fn prepend(&mut self, adjustment: dir::ReceiverAdjustment) {
        match self {
            Self::Direct(steps) => steps.insert(0, adjustment),
            Self::Dynamic(dispatch) => dispatch.receiver.prepend(adjustment),
        }
    }

    /// Attach this lookup state to its use-site receiver.
    pub(in crate::check) fn resolve(&self, source: dir::GlobalTypeId) -> dir::MemberReceiver {
        match self {
            Self::Direct(steps) => dir::MemberReceiver::Direct(dir::AdjustedReceiver {
                source,
                adjustments: steps.clone(),
            }),
            Self::Dynamic(dispatch) => dir::MemberReceiver::Dynamic(dispatch.clone()),
        }
    }
}

impl MemberCandidate {
    /// Return the type produced by reading this candidate.
    pub(in crate::check) fn read_type(
        &self,
        _module: ModuleId,
        body: &mut BodyState<'_, '_>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        if !self.role.is_readable() {
            return Ok(None);
        }
        if !self.is_optional {
            return Ok(Some(self.access_type));
        }
        let undefined = body.intern_type(dir::Type::Undefined)?;
        let ty = body.normalized_union_type([self.access_type, undefined])?;

        Ok(Some(ty))
    }

    /// Return the durable member candidate for this lookup candidate.
    pub(in crate::check) fn resolution_candidate(
        &self,
        receiver: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
    ) -> dir::MemberCandidate {
        dir::MemberCandidate {
            receiver: self.receiver.resolve(receiver),
            space: self.space,
            owner: self.owner,
            symbol: self.symbol,
            access_type: ty,
            callable_type: self.callable,
            generic_arguments: self.generic_arguments.clone(),
        }
    }

    /// Return the durable member access for this lookup candidate.
    pub(in crate::check) fn access(
        &self,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        ty: dir::GlobalTypeId,
    ) -> dir::MemberAccess {
        let target = match self.field(key) {
            Some(target) => dir::MemberTarget::Field(dir::FieldResolution {
                receiver: self.receiver.resolve(receiver),
                target,
                ty,
            }),
            None => dir::MemberTarget::Symbol(self.resolution_candidate(receiver, ty)),
        };

        dir::MemberAccess::new(receiver, target, ty)
    }

    /// Return the stored field selected by this candidate.
    pub(in crate::check) fn field(&self, key: dir::StaticKey) -> Option<dir::FieldTarget> {
        if self.role != MemberRole::Field {
            return None;
        }

        Some(dir::FieldTarget::Member {
            symbol: self.symbol,
            key,
        })
    }
}

impl BodyState<'_, '_> {
    /// Resolve the receiver and member space used by member lookup.
    pub(in crate::check) fn resolve_member_subject(
        &mut self,
        origin: Origin,
        receiver_node: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<(dir::MemberSubject, Option<NullishPart>)>> {
        let split = answer!(self.split_nullish_type(origin, receiver)?);
        let rejected = split.map(|split| split.rejected);
        let receiver = split.map_or(receiver, |split| split.value);
        let receiver = answer!(self.reduce_type_head(origin, receiver)?);
        let mut space = self.member_receiver_space(receiver_node, receiver)?;
        let mut subject = receiver;

        // static type aliases look up through their declaration reference
        let resolution = self
            .resolutions(receiver_node.module_id)
            .name_resolution(receiver_node)
            .cloned();
        if let Some(resolution) = resolution
            && let [symbol] = resolution.symbols()
        {
            let symbol = self.resolve_symbol_alias(*symbol)?;
            if self.symbol_kind(symbol)?.is_type_alias() {
                space = dir::MemberSpace::Static;
                subject = self.intern_type(dir::Type::Reference(dir::TypeReference { symbol }))?;
            }
        }

        let subject = dir::MemberSubject::new(receiver, subject, space)
            .with_scope(self.origin_scope(origin)?);

        Ok(Answer::Ready((subject, rejected)))
    }

    /// Return the readable type exposed by one member lookup.
    pub(in crate::check) fn member_read_type(
        &mut self,
        origin: Origin,
        lookup: &MemberLookup,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        match lookup {
            MemberLookup::Missing => Ok(None),
            MemberLookup::Field(field) => field.read_type(origin.module(), self),
            MemberLookup::Found(candidates) => {
                let candidates = MemberLookup::selected_candidates(candidates);
                let single_slot = candidates
                    .iter()
                    .position(|candidate| candidate.role != MemberRole::Method);
                let candidates = match single_slot {
                    Some(first) => vec![candidates[first]],
                    None => candidates,
                };
                let mut types = Vec::with_capacity(candidates.len());
                for candidate in candidates {
                    types.extend(candidate.read_type(origin.module(), self)?);
                }
                if types.is_empty() {
                    return Ok(None);
                }

                self.normalized_intersection_type(types).map(Some)
            }
            MemberLookup::Union(arms) => {
                let mut types = Vec::with_capacity(arms.len());
                for arm in arms {
                    let Some(ty) = self.member_read_type(origin, &arm.lookup)? else {
                        return Ok(None);
                    };
                    types.push(ty);
                }

                self.normalized_union_type(types).map(Some)
            }
            MemberLookup::Intersection(lookups) => {
                let mut types = Vec::with_capacity(lookups.len());
                for lookup in lookups {
                    types.extend(self.member_read_type(origin, lookup)?);
                }
                if types.is_empty() {
                    return Ok(None);
                }

                self.normalized_intersection_type(types).map(Some)
            }
        }
    }

    /// Return the writable type accepted by one member lookup.
    pub(in crate::check) fn member_write_type(
        &mut self,
        lookup: &MemberLookup,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        match lookup {
            MemberLookup::Missing => Ok(None),
            MemberLookup::Field(field) => Ok(field.write_type()),
            MemberLookup::Found(candidates) => {
                let writable = MemberLookup::selected_candidates(candidates)
                    .into_iter()
                    .filter(|candidate| candidate.is_writable)
                    .collect::<Vec<_>>();
                let [candidate] = writable.as_slice() else {
                    return Ok(None);
                };

                Ok(Some(candidate.access_type))
            }
            MemberLookup::Union(arms) => {
                let mut types = Vec::with_capacity(arms.len());
                for arm in arms {
                    let Some(type_id) = self.member_write_type(&arm.lookup)? else {
                        return Ok(None);
                    };
                    types.push(type_id);
                }

                self.normalized_intersection_type(types).map(Some)
            }
            MemberLookup::Intersection(lookups) => {
                let mut types = Vec::with_capacity(lookups.len());
                for lookup in lookups {
                    let Some(type_id) = self.member_write_type(lookup)? else {
                        return Ok(None);
                    };
                    types.push(type_id);
                }

                self.normalized_intersection_type(types).map(Some)
            }
        }
    }

    /// Return the durable binding represented by one completed member lookup.
    pub(in crate::check) fn member_binding(
        &mut self,
        origin: Origin,
        key: dir::StaticKey,
        lookup: &MemberLookup,
    ) -> CompilerResult<Option<dir::MemberBinding>> {
        let read = self.member_read_type(origin, lookup)?;
        let write = self.member_write_type(lookup)?;
        let access = match (read, write) {
            (Some(read), Some(write)) => dir::PropertyAccess::ReadWrite { read, write },
            (Some(read), None) => dir::PropertyAccess::Read(read),
            (None, Some(write)) => dir::PropertyAccess::Write(write),
            (None, None) => return Ok(None),
        };

        // retain each selected declaration once
        let mut declarations = Vec::new();
        for candidate in lookup.declaration_candidates() {
            if declarations
                .iter()
                .any(|declaration: &dir::MemberDeclaration| declaration.symbol == candidate.symbol)
            {
                continue;
            }
            declarations.push(dir::MemberDeclaration {
                symbol: candidate.symbol,
                origin: candidate.origin,
                callable_type: candidate.callable,
            });
        }

        Ok(Some(dir::MemberBinding::new(
            key,
            lookup.kind(),
            access,
            lookup.is_optional(),
            declarations,
        )))
    }

    /// Return the lookup member represented by one definition member.
    pub(in crate::check) fn declared_member(
        &mut self,
        member: &dir::DefinitionMember,
    ) -> CompilerResult<Answer<Option<DeclaredMember>>> {
        let Some(role) = MemberRole::from_definition(member) else {
            return Ok(Answer::Ready(None));
        };
        let Some(symbol) = member.symbol() else {
            return Err(CompilerError::Internal {
                message: format!("keyed definition member {member:?} has no symbol"),
            });
        };
        let ty = answer!(self.definition_member_type(member)?);

        Ok(Answer::Ready(Some(DeclaredMember {
            symbol,
            space: member.space(),
            key: member.key(),
            ty,
            value: member.static_value(),
            role,
            kind: member.kind(),
            is_writable: member.is_writable(),
            is_optional: member.is_optional(),
        })))
    }

    /// Select the readable resolution exposed by one member lookup.
    pub(in crate::check) fn select_member_read(
        &mut self,
        origin: Origin,
        receiver: Value,
        key: dir::StaticKey,
        lookup: &MemberLookup,
    ) -> CompilerResult<Answer<Option<dir::MemberResolution>>> {
        match lookup {
            MemberLookup::Missing => Ok(Answer::Ready(None)),
            MemberLookup::Field(field) => {
                // write-only properties expose nothing to a read selection
                let Some(ty) = field.read_type(origin.module(), self)? else {
                    return Ok(Answer::Ready(None));
                };
                let target = dir::MemberTarget::Field(dir::FieldResolution {
                    receiver: field.receiver.clone(),
                    target: dir::FieldTarget::Structural {
                        owner: field.owner,
                        key,
                    },
                    ty,
                });

                let access = dir::MemberAccess::new(receiver.ty, target, ty);

                Ok(Answer::Ready(Some(dir::OperationResolution::One(access))))
            }
            MemberLookup::Found(candidates) => {
                // keep the candidates the selection precedence ranks first
                let best = candidates.iter().map(MemberCandidate::precedence).min();
                let candidates = candidates
                    .iter()
                    .filter(|candidate| Some(candidate.precedence()) == best)
                    .collect::<Vec<_>>();

                // keep the first single-slot declaration among the survivors
                let single_slot = candidates
                    .iter()
                    .position(|candidate| candidate.role != MemberRole::Method);
                let candidates = match single_slot {
                    Some(first) => vec![candidates[first]],
                    None => candidates,
                };

                let mut targets = Vec::new();
                let mut types = Vec::new();
                for candidate in candidates {
                    let Some(ty) = candidate.read_type(origin.module(), self)? else {
                        continue;
                    };
                    let access = if candidate.role == MemberRole::Getter {
                        // rejecting receivers skip to the next declared candidate
                        let Some(call) =
                            answer!(self.select_getter_call(origin, receiver, candidate)?)
                        else {
                            continue;
                        };

                        dir::MemberAccess::new(
                            receiver.ty,
                            dir::MemberTarget::Call(Box::new(call)),
                            ty,
                        )
                    } else {
                        candidate.access(receiver.ty, key, ty)
                    };

                    targets.push(access.target);
                    types.push(access.ty);
                }
                let target = match targets.as_slice() {
                    [] => return Ok(Answer::Ready(None)),
                    [target] => target.clone(),
                    _ => dir::MemberTarget::Existential(targets),
                };
                let ty = self.normalized_intersection_type(types)?;

                let access = dir::MemberAccess::new(receiver.ty, target, ty);

                Ok(Answer::Ready(Some(dir::OperationResolution::One(access))))
            }
            MemberLookup::Union(lookups) => {
                let mut accesses = Vec::with_capacity(lookups.len());
                let mut types = Vec::with_capacity(lookups.len());
                for arm in lookups {
                    let arm_receiver = Value {
                        ty: arm.receiver,
                        ..receiver
                    };
                    let Some(resolution) =
                        answer!(self.select_member_read(origin, arm_receiver, key, &arm.lookup,)?)
                    else {
                        return Ok(Answer::Ready(None));
                    };

                    let dir::OperationResolution::One(access) = resolution else {
                        return Err(CompilerError::Internal {
                            message: "union member lookup contains a nested union".to_string(),
                        });
                    };

                    types.push(access.ty);
                    accesses.push(access);
                }
                let ty = self.normalized_union_type(types)?;

                Ok(Answer::Ready(Some(dir::OperationResolution::Union {
                    arms: accesses,
                    ty,
                })))
            }
            MemberLookup::Intersection(lookups) => {
                let mut resolutions = Vec::with_capacity(lookups.len());
                for lookup in lookups {
                    let Some(resolution) =
                        answer!(self.select_member_read(origin, receiver, key, lookup)?)
                    else {
                        return Ok(Answer::Ready(None));
                    };
                    resolutions.push(resolution);
                }
                let resolution = self.intersect_member_resolutions(origin, resolutions)?;

                Ok(Answer::Ready(Some(resolution)))
            }
        }
    }

    /// Intersect simultaneous member resolutions, distributing runtime union arms.
    pub(in crate::check) fn intersect_member_resolutions(
        &mut self,
        origin: Origin,
        resolutions: Vec<dir::MemberResolution>,
    ) -> CompilerResult<dir::MemberResolution> {
        let has_union = resolutions
            .iter()
            .any(|resolution| matches!(resolution, dir::OperationResolution::Union { .. }));
        let mut combinations = vec![Vec::new()];
        for resolution in resolutions {
            let accesses = match &resolution {
                dir::OperationResolution::One(access) => slice::from_ref(access),
                dir::OperationResolution::Union { arms, .. } => arms.as_slice(),
            };
            let mut next = Vec::with_capacity(combinations.len() * accesses.len());
            for combination in combinations {
                for access in accesses {
                    let mut combined = combination.clone();
                    combined.push(access.clone());
                    next.push(combined);
                }
            }
            combinations = next;
        }

        let mut arms = Vec::with_capacity(combinations.len());
        for accesses in combinations {
            arms.push(self.intersect_member_accesses(origin, accesses)?);
        }
        if !has_union && arms.len() == 1 {
            return Ok(dir::OperationResolution::One(arms.remove(0)));
        }
        let types = arms.iter().map(|access| access.ty).collect::<Vec<_>>();
        let ty = self.normalized_union_type(types)?;

        Ok(dir::OperationResolution::Union { arms, ty })
    }

    /// Intersect simultaneous member accesses into one runtime access.
    fn intersect_member_accesses(
        &mut self,
        _origin: Origin,
        accesses: Vec<dir::MemberAccess>,
    ) -> CompilerResult<dir::MemberAccess> {
        if accesses.len() == 1 {
            let mut accesses = accesses;

            return Ok(accesses.remove(0));
        }

        let mut receivers = Vec::with_capacity(accesses.len());
        let mut targets = Vec::with_capacity(accesses.len());
        let mut types = Vec::with_capacity(accesses.len());
        for access in accesses {
            receivers.push(access.receiver);
            types.push(access.ty);
            match access.target {
                dir::MemberTarget::Intersection(nested) => targets.extend(nested),
                target => targets.push(target),
            }
        }
        let receiver = self.normalized_intersection_type(receivers)?;
        let ty = self.normalized_intersection_type(types)?;
        let target = dir::MemberTarget::Intersection(targets);

        Ok(dir::MemberAccess::new(receiver, target, ty))
    }

    /// Select one getter invocation from a readable member candidate.
    pub(in crate::check) fn select_getter_call(
        &mut self,
        origin: Origin,
        receiver: Value,
        candidate: &MemberCandidate,
    ) -> CompilerResult<Answer<Option<dir::Call>>> {
        let symbol = candidate.symbol;
        let resolution = candidate.receiver.resolve(receiver.ty);
        let selection_type = match &resolution {
            dir::MemberReceiver::Direct(receiver) => receiver.ty(),
            dir::MemberReceiver::Dynamic(dispatch) => dispatch.constraint,
        };
        let selection_receiver = Value {
            ty: selection_type,
            ..receiver
        };
        let arguments = SmallVec::<[CallableArgument; 4]>::new();
        let callable = candidate.callable.ok_or_else(|| CompilerError::Internal {
            message: format!("getter {symbol:?} has no callable type"),
        })?;
        let selected = answer!(self.attempt_callable(
            origin,
            callable,
            Some(candidate.owner),
            Some(selection_receiver),
            &candidate.generic_arguments,
            &[],
            &arguments,
            None,
        )?);

        // a rejecting receiver skips to the next declared candidate
        let SignatureMatch::Selected(signature) = selected else {
            return Ok(Answer::Ready(None));
        };
        let arguments = Self::source_argument_bindings(&[], &signature.parameters);
        let resolution = signature.member_call(resolution, candidate.owner, symbol, arguments);

        Ok(Answer::Ready(Some(resolution)))
    }

    /// Select one setter invocation from a writable member candidate.
    pub(in crate::check) fn select_setter_call(
        &mut self,
        origin: Origin,
        receiver: Value,
        candidate: &MemberCandidate,
    ) -> CompilerResult<Answer<dir::Call>> {
        let symbol = candidate.symbol;
        let resolution = candidate.receiver.resolve(receiver.ty);
        let selection_type = match &resolution {
            dir::MemberReceiver::Direct(receiver) => receiver.ty(),
            dir::MemberReceiver::Dynamic(dispatch) => dispatch.constraint,
        };
        let selection_receiver = Value {
            ty: selection_type,
            ..receiver
        };
        let sources = [dir::ArgumentSource::Write];
        let source = self
            .origin_source_node(origin)?
            .into_global(origin.module());
        let arguments = [CallableArgument {
            source,
            ty: Some(candidate.access_type),
            relation: Relation::Assignable,
            use_: ValueUse::Argument,
        }];
        let selected = answer!(self.attempt_callable(
            origin,
            candidate.callable.ok_or_else(|| CompilerError::Internal {
                message: format!("setter {symbol:?} has no callable type"),
            })?,
            Some(candidate.owner),
            Some(selection_receiver),
            &candidate.generic_arguments,
            &[],
            &arguments,
            None,
        )?);
        let SignatureMatch::Selected(signature) = selected else {
            return Err(CompilerError::Internal {
                message: format!("selected setter {symbol:?} rejects its declared value type"),
            });
        };
        let arguments = Self::source_argument_bindings(&sources, &signature.parameters);
        let resolution = signature.member_call(resolution, candidate.owner, symbol, arguments);

        Ok(Answer::Ready(resolution))
    }

    /// Select the member meaning of one member access node.
    pub(in crate::check) fn select_member(
        &mut self,
        site: FlowSite,
        left: dir::LocalNodeId<dir::Expression>,
        name: Option<dir::StringId>,
        is_optional: bool,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();

        // reduce the receiver before member lookup
        let receiver_node = left.into_global_any(module);
        let receiver_site = self.node_site(receiver_node)?;
        let written_receiver = answer!(self.infer_node_type(receiver_site, PlaceUse::Read)?);
        let (subject, rejected) =
            answer!(self.resolve_member_subject(origin, receiver_node, written_receiver)?);
        if let Some(rejected) = rejected
            && !is_optional
        {
            self.report_possibly_nullish(origin, rejected.label().to_string())?;
        }

        // retain the exact lookup subject at this authored site
        self.module_mut(module)
            .members
            .record_subject(node, subject);

        // commit an error for an omitted member name
        let Some(name) = name else {
            self.commit_decision(node, Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(Answer::Ready(()));
        };
        let key = dir::StaticKey::Name(name);

        // select compiler-defined member projections before declaration lookup
        if subject.space == dir::MemberSpace::Instance
            && let Some(projection) =
                self.tagged_discriminator_projection(module, subject.receiver, key)?
        {
            return self.commit_projection_member(
                node,
                receiver_node,
                subject.receiver,
                key,
                projection,
            );
        }
        let lookup = answer!(self.lookup_member(origin, module, subject, key,)?);

        match lookup {
            MemberLookup::Missing => {
                let key = self.strings().get(name).to_string();

                self.reject_member(node, origin, written_receiver, key)
            }
            lookup => self.commit_member_lookup(
                node,
                receiver_node,
                origin,
                subject.receiver,
                key,
                name,
                lookup,
            ),
        }
    }

    /// Commit one compiler-defined member projection.
    fn commit_projection_member(
        &mut self,
        node: dir::GlobalNodeIdAny,
        receiver_node: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        projection: dir::Projection,
    ) -> CompilerResult<Answer<()>> {
        let ty = projection.ty();
        let target = dir::MemberTarget::Projection { key, projection };
        let access = dir::MemberAccess::new(receiver, target, ty);
        let resolution = dir::OperationResolution::One(access);
        self.commit_decision(node, Decision::Member(resolution))?;
        self.commit_projected_access(node, receiver_node, key)?;
        let site = self.node_site(node)?;
        let ty = answer!(self.flow_type_at(site, ty)?);
        self.commit_node_type(node, ty)?;

        Ok(Answer::Ready(()))
    }

    /// Commit one member lookup.
    fn commit_member_lookup(
        &mut self,
        node: dir::GlobalNodeIdAny,
        receiver_node: dir::GlobalNodeIdAny,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        name: dir::StringId,
        lookup: MemberLookup,
    ) -> CompilerResult<Answer<()>> {
        let receiver_site = self.node_site(receiver_node)?;
        let receiver = answer!(self.expression_value(receiver_site, receiver)?);
        let Some(resolution) = answer!(self.select_member_read(origin, receiver, key, &lookup)?)
        else {
            if lookup.has_setter() {
                let key = self.strings().get(name).to_string();
                self.report_write_only_member(origin, key)?;
                self.commit_decision(node, Decision::Rejected)?;
                self.commit_error_node(node)?;

                return Ok(Answer::Ready(()));
            }

            let key = self.strings().get(name).to_string();

            return self.reject_member(node, origin, receiver.ty, key);
        };

        // commit the exact runtime target tree and joined value type
        let ty = resolution.ty();
        let stored_key = resolution.stored_key();
        self.commit_decision(node, Decision::Member(resolution))?;
        if let Some(key) = stored_key {
            self.commit_projected_access(node, receiver_node, key)?;
        }
        let site = self.node_site(node)?;
        let ty = answer!(self.flow_type_at(site, ty)?);
        self.commit_node_type(node, ty)?;

        Ok(Answer::Ready(()))
    }

    /// Reject one member access with a diagnostic.
    fn reject_member(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: String,
    ) -> CompilerResult<Answer<()>> {
        // poison instead of reporting again when the receiver already reported an error
        if self.any_error_operand(&[receiver])? {
            self.poison_node(node)?;

            return Ok(Answer::Ready(()));
        }
        let key_span = self.module(node.module_id).diagnostic_span(node.local_id);
        self.report_missing_member(origin, receiver, key, key_span)?;
        self.commit_decision(node, Decision::Rejected)?;
        self.commit_error_node(node)?;

        Ok(Answer::Ready(()))
    }
}

impl BodyState<'_, '_> {
    /// Return the use-site type projected by one declaration member.
    pub(in crate::check) fn projected_member_type(
        &mut self,
        origin: Origin,
        receiver: Option<dir::GlobalTypeId>,
        role: MemberRole,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        if role != MemberRole::Field {
            return Ok(Answer::Ready(ty));
        }
        let Some(receiver) = receiver else {
            return Ok(Answer::Ready(ty));
        };
        let ty = answer!(self.project_member_place(origin, receiver, ty)?);

        // readonly receivers project deep readonly views onto stored fields
        if !answer!(self.receiver_projects_readonly(origin, receiver)?) {
            return Ok(Answer::Ready(ty));
        }
        let ty = answer!(self.reduce_type_head(origin, ty)?);
        if matches!(self.ty(ty)?, dir::Type::Form(form) if form.form == dir::Form::Readonly) {
            return Ok(Answer::Ready(ty));
        }
        if !answer!(self.type_projects_readonly(origin, ty)?) {
            return Ok(Answer::Ready(ty));
        }

        let projected = self.intern_type(dir::Type::Form(dir::FormType {
            form: dir::Form::Readonly,
            value: ty,
        }))?;

        self.reduce_type_head(origin, projected)
    }

    /// Return whether one receiver projects stored fields as readonly.
    pub(in crate::check) fn receiver_projects_readonly(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let mut current = receiver;

        loop {
            current = answer!(self.reduce_type_head(origin, current)?);
            let form = match self.ty(current)? {
                dir::Type::Form(form) => form,
                _ => return Ok(Answer::Ready(false)),
            };

            // explicit readonly views make every stored field readonly
            if form.form == dir::Form::Readonly {
                return Ok(Answer::Ready(true));
            }

            // readonly borrows expose only readonly stored fields
            if let dir::Form::Borrowed(borrow) = form.form
                && let access = self.type_borrow(current.module_id, borrow)?.access
                && answer!(self.access_is_readonly(origin, access)?)
            {
                return Ok(Answer::Ready(true));
            }

            current = form.value;
        }
    }

    /// Project one type-level member access through its owner.
    pub(in crate::check) fn project_member(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // bind the projection from written refinements
        if let Some(qualifier) = member.qualifier {
            let (_, bindings) = self.refinement_bindings(qualifier)?;
            if let Some((_, value)) = bindings.iter().find(|(key, _)| *key == member.key) {
                return Ok(Answer::Ready(Some(*value)));
            }
        }

        // select the implementation for applied interface scopes
        if let Some(qualifier) = member.qualifier
            && !self.is_rigid_projection_owner(member.owner)?
        {
            let (base, _) = self.refinement_bindings(qualifier)?;
            return match self.ty(base)? {
                dir::Type::Application(_) => self.project_selected_member(origin, member, base),
                // project the lexical extension scope's own associated member
                dir::Type::Reference(reference) => {
                    self.project_scope_member(origin, member, reference.symbol)
                }
                _ => Ok(Answer::Ready(None)),
            };
        }

        // resolve remaining projections through member lookup
        let module = origin.module();
        let subject = dir::MemberSubject::new(member.owner, member.owner, dir::MemberSpace::Static);
        let lookup = answer!(self.lookup_member(origin, module, subject, member.key)?);
        let projected = match lookup {
            MemberLookup::Missing => answer!(self.project_default_member(origin, member)?),
            lookup => self.project_member_lookup(module, member, lookup)?,
        };

        Ok(Answer::Ready(projected))
    }

    /// Project one associated member through its selected interface implementation.
    fn project_selected_member(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
        qualifier: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // read the qualifying interface application
        let module = origin.module();
        let (interface_module, interface) = self.nominal_application(qualifier)?;
        let owner = member.owner;

        // project a declaring class scope's own associated member
        let scope = self.resolve_symbol_alias(interface.symbol)?;
        if let Some(definition) = self.definition(scope)?
            && !matches!(definition, dir::Definition::Interface(_))
        {
            let members = definition.members().to_vec();
            let substitution =
                self.qualified_instance_substitution(interface_module, &interface, owner)?;

            return self.project_implemented_member(
                origin,
                member,
                &members,
                &substitution,
                qualifier,
            );
        }

        // enumerate candidate extensions by receiver family
        let apparent = self.intern_apparent_type(module, owner)?;
        let extensions = answer!(self.visible_implementation_extensions(
            origin,
            module,
            apparent,
            interface.symbol,
        )?);
        for extension_symbol in extensions {
            let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol)?
            else {
                continue;
            };
            if !extension.is_visible_from(module) {
                continue;
            }

            // read the extension's target, implements, and members
            let target_type = extension.target.r#type();
            let implements = extension.implements.clone();
            let members = extension.members.clone();
            let template = self.symbol_template(extension_symbol)?;

            // classify the candidate's bounds in a probe before committing
            let verdict = self.probe_candidate(|state| {
                let matched = state.match_extension_implementation(
                    origin,
                    Relation::Assignable,
                    interface_module,
                    owner,
                    owner,
                    &interface,
                    template,
                    target_type,
                    &implements,
                )?;

                Ok(matched.map(|matched| match matched {
                    Some(_) => CandidateOutcome::Accepted(()),
                    None => CandidateOutcome::Rejected(()),
                }))
            })?;
            if !matches!(verdict, Answer::Ready(CandidateVerdict::Viable)) {
                continue;
            }

            // rerun the match to commit its substitution
            let matched = answer!(self.match_extension_implementation(
                origin,
                Relation::Assignable,
                interface_module,
                owner,
                owner,
                &interface,
                template,
                target_type,
                &implements,
            )?);
            let Some((substitution, implementation)) = matched else {
                continue;
            };

            return self.project_implemented_member(
                origin,
                member,
                &members,
                &substitution,
                implementation,
            );
        }

        // match the owner's own declared implementations
        if let Some((application_module, application)) = self.nominal_application_maybe(owner)?
            && let Some(definition) = self.definition(application.symbol)?
        {
            let implements = definition.implementations().to_vec();
            let members = definition.members().to_vec();
            if !implements.is_empty() {
                let mut substitution =
                    self.instance_substitution(application_module, &application)?;
                let matched = answer!(self.match_implemented_interface(
                    origin,
                    Relation::Assignable,
                    interface_module,
                    &[],
                    &mut substitution,
                    &implements,
                    &interface,
                )?);
                if let Some(implementation) = matched {
                    return self.project_implemented_member(
                        origin,
                        member,
                        &members,
                        &substitution,
                        implementation,
                    );
                }
            }
        }

        Ok(Answer::Ready(None))
    }

    /// Project one associated member declared by a lexical extension scope.
    fn project_scope_member(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
        scope: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(definition) = self.definition(scope)? else {
            return Ok(Answer::Ready(None));
        };

        // project the scope's own members against the written owner
        let members = definition.members().to_vec();
        let substitution = TypeSubstitution::default().with_receiver(member.owner);

        self.project_implemented_member(origin, member, &members, &substitution, member.owner)
    }

    /// Project one associated member of a matched implementation.
    fn project_implemented_member(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
        members: &[dir::DefinitionMember],
        substitution: &TypeSubstitution,
        implementation: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // prefer the implementer's declared associated type
        let declared = members.iter().find_map(|declared| match declared {
            dir::DefinitionMember::AssociatedType(associated) if associated.key == member.key => {
                associated.value
            }
            _ => None,
        });
        if let Some(value) = declared {
            let value = self.substitute_type(value, substitution)?;

            return Ok(Answer::Ready(Some(value)));
        }

        // read written refinements on the matched header
        let (_, bindings) = self.refinement_bindings(implementation)?;
        if let Some((_, value)) = bindings.iter().find(|(key, _)| *key == member.key) {
            return Ok(Answer::Ready(Some(*value)));
        }

        self.project_default_member(origin, member)
    }

    /// Return the type projected by one selected member lookup.
    fn project_member_lookup(
        &mut self,
        module: ModuleId,
        member: &dir::MemberType,
        lookup: MemberLookup,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        match lookup {
            MemberLookup::Field(field) => field.read_type(module, self),
            MemberLookup::Found(candidates) => match candidates.as_slice() {
                [candidate] => {
                    if let Some(written) = candidate.value_type {
                        return Ok(Some(written));
                    }
                    if let Some(value) = candidate.value {
                        let ty = self.intern_type(dir::Type::Static(value))?;

                        return Ok(Some(ty));
                    }

                    if let Some(projected) = self.member_head(candidate.access_type)?
                        && projected.owner == member.owner
                        && projected.key == member.key
                    {
                        return Ok(None);
                    }

                    candidate.read_type(module, self)
                }
                _ => Ok(None),
            },
            MemberLookup::Union(lookups) => {
                let mut types = Vec::with_capacity(lookups.len());
                for arm in lookups {
                    let Some(ty) = self.project_member_lookup(module, member, arm.lookup)? else {
                        return Ok(None);
                    };
                    types.push(ty);
                }

                self.normalized_union_type(types).map(Some)
            }
            MemberLookup::Intersection(lookups) => {
                let mut types = Vec::with_capacity(lookups.len());
                for lookup in lookups {
                    let Some(ty) = self.project_member_lookup(module, member, lookup)? else {
                        return Ok(None);
                    };
                    types.push(ty);
                }

                self.normalized_intersection_type(types).map(Some)
            }
            MemberLookup::Missing => Ok(None),
        }
    }

    /// Project one interface default through a qualified owner.
    fn project_default_member(
        &mut self,
        _origin: Origin,
        member: &dir::MemberType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        if self.is_rigid_projection_owner(member.owner)? {
            return Ok(Answer::Ready(None));
        }
        let Some(qualifier) = member.qualifier else {
            return Ok(Answer::Ready(None));
        };
        let Some((interface_module, interface)) = self.nominal_application_maybe(qualifier)? else {
            return Ok(Answer::Ready(None));
        };
        // select the declared interface default
        let value = match self.definition(interface.symbol)? {
            Some(dir::Definition::Interface(definition)) => {
                definition
                    .members
                    .iter()
                    .find_map(|declared| match declared {
                        dir::DefinitionMember::AssociatedType(associated)
                            if associated.key == member.key =>
                        {
                            associated.value
                        }
                        _ => None,
                    })
            }
            _ => None,
        };
        let Some(value) = value else {
            return Ok(Answer::Ready(None));
        };
        let substitution =
            self.qualified_instance_substitution(interface_module, &interface, member.owner)?;
        let value = self.substitute_type(value, &substitution)?;

        Ok(Answer::Ready(Some(value)))
    }

    /// Return whether associated defaults remain overridable beneath one owner.
    pub(in crate::check) fn is_rigid_projection_owner(
        &self,
        owner: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let owner = self.settled_root(owner)?;
        let is_rigid = matches!(self.ty(owner)?, dir::Type::Parameter(_) | dir::Type::This);

        Ok(is_rigid)
    }

    /// Return the unique interface application declaring one projected member.
    pub(in crate::check) fn projection_qualifier(
        &mut self,
        origin: Origin,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let mut interfaces = SmallVec::<[dir::GlobalTypeId; 4]>::new();

        // parameter projections select from their declared bounds
        if let dir::Type::Parameter(parameter) = self.ty(owner)? {
            interfaces.extend(self.parameter_bounds(origin, parameter)?);
        }
        // nominal projections select from their checked heritage
        else if self.nominal_application_maybe(owner)?.is_some() {
            let closure = answer!(self.heritage_closure(origin, owner)?);
            interfaces.extend(
                closure
                    .applications
                    .into_iter()
                    .map(|application| application.ty),
            );
        }
        // other owners cannot expose associated declarations
        else {
            return Ok(Answer::Ready(None));
        }

        // retain only interfaces declaring this associated member
        let mut qualifier = None;
        for interface in interfaces {
            let interface = answer!(self.reduce_type_head(origin, interface)?);
            let Some((_, instance)) = self.nominal_application_maybe(interface)? else {
                continue;
            };
            let Some(dir::Definition::Interface(definition)) = self.definition(instance.symbol)?
            else {
                continue;
            };
            let declares = definition.members.iter().any(|member| {
                matches!(
                    member,
                    dir::DefinitionMember::AssociatedType(associated)
                        if associated.key == key
                )
            });
            if !declares || qualifier == Some(interface) {
                continue;
            }
            if qualifier.is_some() {
                let key = self.format_static_key(&key);
                self.report_ambiguous_member(origin, key)?;

                return Ok(Answer::Ready(None));
            }
            qualifier = Some(interface);
        }

        Ok(Answer::Ready(qualifier))
    }

    /// Return the substituted constraint declared for one rigid projection.
    pub(in crate::check) fn projection_constraint(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let owner = answer!(self.strip_form(origin, member.owner)?);
        let dir::Type::Parameter(parameter) = self.ty(owner)? else {
            return Ok(Answer::Ready(None));
        };

        for bound in self.parameter_bounds(origin, parameter)? {
            // only interface bounds can declare projected members
            let bound = answer!(self.reduce_type_head(origin, bound)?);
            let dir::Type::Application(instance) = self.ty(bound)? else {
                continue;
            };
            let members = match self.definition(instance.symbol)? {
                Some(dir::Definition::Interface(definition)) => definition.members.clone(),
                _ => continue,
            };

            // read the declared constraint under the bound's application
            for declared in &members {
                let dir::DefinitionMember::AssociatedType(associated) = declared else {
                    continue;
                };
                if associated.key != member.key {
                    continue;
                }
                let Some(constraint) = associated.constraint else {
                    continue;
                };
                let substitution = self
                    .instance_substitution(bound.module_id, &instance)?
                    .with_receiver(owner);
                let constraint = self.substitute_type(constraint, &substitution)?;

                return Ok(Answer::Ready(Some(constraint)));
            }
        }

        Ok(Answer::Ready(None))
    }

    /// Return one field type projected through the receiver placement.
    fn project_member_place(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let Some(place) = answer!(self.receiver_projected_place(origin, receiver)?) else {
            return Ok(Answer::Ready(ty));
        };
        let ty = answer!(self.place_relative_type(origin, place, ty)?);

        self.reduce_type_head(origin, ty)
    }

    /// Resolve one relative member type in a projected receiver place.
    pub(in crate::check) fn place_relative_type(
        &mut self,
        origin: Origin,
        place: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        // bare members of local receivers stay bare
        let root = answer!(self.reduce_type_head(origin, place)?);
        if self.check.place_space(root)? == Some(dir::Space::Local) {
            return Ok(Answer::Ready(ty));
        }

        self.resolve_relative_place(origin, ty, place)
    }

    /// Return the place projected by one receiver type.
    pub(in crate::check) fn receiver_projected_place(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let mut current = receiver;
        loop {
            current = answer!(self.reduce_type_head(origin, current)?);
            let dir::Type::Form(form) = self.ty(current)? else {
                // bare nominal instances live in their declared or inherited space
                if let dir::Type::Application(instance) = self.ty(current)?
                    && let Some(space) = self.check.nominal_space(instance.symbol)?
                {
                    let place = self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Place(
                        dir::Place::Space(space),
                    )))?;

                    return Ok(Answer::Ready(Some(place)));
                }

                return Ok(Answer::Ready(None));
            };

            match form.form {
                dir::Form::Placed { place } => return Ok(Answer::Ready(Some(place))),
                _ => current = form.value,
            }
        }
    }

    /// Return whether one projected value should retain a readonly view.
    fn type_projects_readonly(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let ty = answer!(self.reduce_type_head(origin, ty)?);
        if answer!(self.satisfies_auto_interface(origin, ty, dir::AutoInterface::Copy)?) {
            return Ok(Answer::Ready(false));
        }

        // unions project a view only when one element requires it
        if let dir::Type::Union(union) = self.ty(ty)? {
            let elements: SmallVec<[_; 4]> =
                SmallVec::from_slice(self.type_ids(ty.module_id, union.elements)?);
            for element in elements {
                if answer!(self.type_projects_readonly(origin, element)?) {
                    return Ok(Answer::Ready(true));
                }
            }

            return Ok(Answer::Ready(false));
        }

        let result = match self.ty(ty)? {
            dir::Type::Error
            | dir::Type::Never
            | dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Key(_)
            | dir::Type::Intrinsic
            | dir::Type::This
            | dir::Type::Memory(_)
            | dir::Type::Static(_) => false,
            dir::Type::Union(_)
            | dir::Type::Refined(_)
            | dir::Type::Object(_)
            | dir::Type::Reference(_)
            | dir::Type::Application(_)
            | dir::Type::Member(_)
            | dir::Type::Variant(_)
            | dir::Type::Form(_)
            | dir::Type::Dynamic(_)
            | dir::Type::Operation(_)
            | dir::Type::Array(_)
            | dir::Type::FixedArray(_)
            | dir::Type::Range(_)
            | dir::Type::Slice(_)
            | dir::Type::Tuple(_)
            | dir::Type::Shape(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::Function(_)
            | dir::Type::FunctionPointer(_)
            | dir::Type::Variable(_)
            | dir::Type::Parameter(_)
            | dir::Type::Erased(_)
            | dir::Type::Intersection(_) => true,
        };

        Ok(Answer::Ready(result))
    }

    /// Return whether one memory access component is readonly.
    pub(in crate::check) fn access_is_readonly(
        &mut self,
        origin: Origin,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let access = answer!(self.reduce_type_head(origin, access)?);
        let is_readonly = matches!(
            self.ty(access)?,
            dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Readonly))
        );

        Ok(Answer::Ready(is_readonly))
    }
}
