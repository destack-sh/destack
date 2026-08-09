use std::slice;
use std::sync::Arc;

use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    BodyState, CallableArgument, CandidateOutcome, CandidateVerdict, CheckState, DeferredCheck,
    FlowSite, NullishPart, Origin, PlaceUse, ReceiverSteps, Relation, SignatureMatch,
    TypeSubstitution, Value, ValueUse, VariableRole, Widening,
};
use crate::{CompilerError, CompilerResult};

pub(in crate::check) use destack_dir::MemberRole;

/// One field found by member lookup.
#[derive(Debug, Clone)]
pub(in crate::check) enum FieldLookup {
    /// One field declared by a structural aggregate.
    Structural {
        /// The receiver that exposes the field.
        receiver: dir::MemberReceiver,
        /// The structural aggregate that declares the field.
        owner: dir::GlobalTypeId,
        /// The projected property operations.
        access: dir::PropertyAccess,
        /// Whether the property may be absent.
        is_optional: bool,
    },
    /// One compiler-defined field projection.
    Projection(dir::Projection),
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
        body: &mut BodyState<'_, '_>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let (read, is_optional) = match self {
            Self::Structural {
                access,
                is_optional,
                ..
            } => (access.read(), *is_optional),
            Self::Projection(projection) => (Some(projection.ty()), false),
        };

        let Some(read) = read else {
            return Ok(None);
        };

        if !is_optional {
            return Ok(Some(read));
        }

        let undefined = body.intern_type(dir::Type::Undefined)?;
        let read = body.normalized_union_type([read, undefined])?;

        Ok(Some(read))
    }

    /// Return the type accepted by writing this property.
    pub(in crate::check) fn write_type(&self) -> Option<dir::GlobalTypeId> {
        match self {
            Self::Structural { access, .. } => access.write(),
            Self::Projection(_) => None,
        }
    }

    /// Return whether this field may be absent.
    fn is_optional(&self) -> bool {
        match self {
            Self::Structural { is_optional, .. } => *is_optional,
            Self::Projection(_) => false,
        }
    }

    /// Return the member access selected by reading this field.
    pub(in crate::check) fn read_access(
        &self,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        body: &mut BodyState<'_, '_>,
    ) -> CompilerResult<Option<dir::MemberAccess>> {
        let Some(ty) = self.read_type(body)? else {
            return Ok(None);
        };
        let target = match self {
            Self::Structural {
                receiver, owner, ..
            } => dir::MemberTarget::Field(dir::FieldResolution {
                receiver: receiver.clone(),
                target: dir::FieldTarget::Structural { owner: *owner, key },
                ty,
            }),
            Self::Projection(projection) => dir::MemberTarget::Projection {
                key,
                projection: projection.clone(),
            },
        };

        Ok(Some(dir::MemberAccess::new(receiver, target, ty)))
    }

    /// Return the value projection selected by reading this field.
    pub(in crate::check) fn projection(
        &self,
        key: dir::StaticKey,
        body: &mut BodyState<'_, '_>,
    ) -> CompilerResult<Option<dir::Projection>> {
        let Some(ty) = self.read_type(body)? else {
            return Ok(None);
        };
        let projection = match self {
            Self::Structural {
                receiver, owner, ..
            } => dir::Projection::Field(dir::FieldResolution {
                receiver: receiver.clone(),
                target: dir::FieldTarget::Structural { owner: *owner, key },
                ty,
            }),
            Self::Projection(projection) => projection.clone(),
        };

        Ok(Some(projection))
    }

    /// Return the member access selected by writing this field.
    pub(in crate::check) fn write_access(
        &self,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> Option<dir::MemberAccess> {
        let (field_receiver, owner, ty) = match self {
            Self::Structural {
                receiver,
                owner,
                access,
                ..
            } => (receiver, owner, access.write()?),
            Self::Projection(_) => return None,
        };
        let target = dir::MemberTarget::Field(dir::FieldResolution {
            receiver: field_receiver.clone(),
            target: dir::FieldTarget::Structural { owner: *owner, key },
            ty,
        });

        Some(dir::MemberAccess::new(receiver, target, ty))
    }

    /// Prepend one implicit receiver adjustment.
    fn prepend_adjustment(&mut self, adjustment: dir::ReceiverAdjustment) -> CompilerResult<()> {
        match self {
            Self::Structural { receiver, .. } => receiver.prepend(adjustment),
            Self::Projection(_) => {
                return Err(CompilerError::Internal {
                    message: "compiler-defined field projection received an implicit adjustment"
                        .to_string(),
                });
            }
        }

        Ok(())
    }

    /// Select this field through one erased receiver.
    fn select_dynamic(&mut self, dispatch: dir::DynamicDispatch) -> CompilerResult<()> {
        match self {
            Self::Structural { receiver, .. } => {
                *receiver = dir::MemberReceiver::Dynamic(dispatch);
            }
            Self::Projection(_) => {
                return Err(CompilerError::Internal {
                    message: "compiler-defined field projection selected dynamic dispatch"
                        .to_string(),
                });
            }
        }

        Ok(())
    }
}

/// One declaration member visible to member lookup.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct DeclaredMember {
    /// The declaring member symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The member space.
    pub(in crate::check) space: dir::MemberSpace,
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
            Self::Missing | Self::Field(_) | Self::Union(_) => None,
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
            Self::Field(field) => field.is_optional(),
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

    /// Return whether any selected declaration is a setter.
    fn has_setter(&self) -> bool {
        match self {
            Self::Missing | Self::Field(_) => false,
            Self::Found(candidates) => candidates
                .iter()
                .any(|candidate| candidate.role == MemberRole::Setter),
            Self::Union(lookups) => lookups.iter().any(|arm| arm.lookup.has_setter()),
            Self::Intersection(lookups) => lookups.iter().any(Self::has_setter),
        }
    }

    /// Prepend one implicit adjustment to every selected receiver.
    pub(in crate::check) fn prepend_adjustment(
        &mut self,
        adjustment: dir::ReceiverAdjustment,
    ) -> CompilerResult<()> {
        match self {
            Self::Missing => {}
            Self::Field(field) => field.prepend_adjustment(adjustment)?,
            Self::Found(candidates) => {
                for candidate in candidates {
                    candidate.receiver.prepend(adjustment.clone());
                }
            }
            Self::Union(lookups) => {
                for arm in lookups {
                    arm.lookup.prepend_adjustment(adjustment.clone())?;
                }
            }
            Self::Intersection(lookups) => {
                for lookup in lookups {
                    lookup.prepend_adjustment(adjustment.clone())?;
                }
            }
        }

        Ok(())
    }

    /// Select every found member through one erased receiver.
    pub(in crate::check) fn select_dynamic(
        &mut self,
        dispatch: dir::DynamicDispatch,
    ) -> CompilerResult<()> {
        match self {
            Self::Missing => {}
            Self::Field(field) => field.select_dynamic(dispatch)?,
            Self::Found(candidates) => {
                for candidate in candidates {
                    candidate.receiver = LookupReceiver::Dynamic(dispatch.clone());
                }
            }
            Self::Union(lookups) => {
                for arm in lookups {
                    arm.lookup.select_dynamic(dispatch.clone())?;
                }
            }
            Self::Intersection(lookups) => {
                for lookup in lookups {
                    lookup.select_dynamic(dispatch.clone())?;
                }
            }
        }

        Ok(())
    }
}

impl DeclaredMember {
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

/// One closed subject whose extension member table is decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct MemberSubject {
    /// The module whose scope selects the visible extensions.
    pub(in crate::check) module: ModuleId,
    /// The receiver type carrying its memory form.
    pub(in crate::check) receiver: dir::GlobalTypeId,
    /// The canonical extension subject.
    pub(in crate::check) subject: dir::GlobalTypeId,
    /// The subject's declaration symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The member space looked up.
    pub(in crate::check) space: dir::MemberSpace,
}

/// Extension members of one subject, grouped by static key.
pub(in crate::check) type MemberTable = Arc<FxIndexMap<dir::StaticKey, Vec<MemberCandidate>>>;

/// One declaration-backed member candidate.
#[derive(Debug, Clone)]
pub(in crate::check) struct MemberCandidate {
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
    ) -> CompilerResult<(dir::MemberSubject, Option<NullishPart>)> {
        // split the nullish part off before reducing the receiver
        let split = self.split_nullish_type(origin, receiver)?;
        let rejected = split.map(|split| split.rejected);
        let receiver = split.map_or(receiver, |split| split.value);
        let receiver = self.reduce_type_head(origin, receiver)?;

        // an open receiver settles its value variables before member lookup,
        //  leaving memory inference open for its own settle points
        let receiver = if self.type_flags(receiver)?.has_variable() {
            let mut variables = self.type_variables(receiver)?;
            variables.retain(|variable| {
                !matches!(
                    self.infer.variable_role(*variable),
                    Ok(VariableRole::Memory { .. })
                )
            });
            self.resolve_variables(&variables)?;

            self.reduce_type_head(origin, receiver)?
        } else {
            receiver
        };
        // look the member up in the receiver's own space by default
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
            .with_scope(self.assuming_scope(origin)?);

        Ok((subject, rejected))
    }

    /// Return the readable type exposed by one member lookup.
    pub(in crate::check) fn member_read_type(
        &mut self,
        origin: Origin,
        lookup: &MemberLookup,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        match lookup {
            MemberLookup::Missing => Ok(None),
            MemberLookup::Field(field) => field.read_type(self),
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
                owner: candidate.owner,
                origin: candidate.origin,
                role: candidate.role,
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

    /// Resolve every member binding exposed by one subject.
    pub(in crate::check) fn resolve_member_bindings(
        &mut self,
        origin: Origin,
        module: ModuleId,
        subject: dir::MemberSubject,
    ) -> CompilerResult<Vec<dir::MemberBinding>> {
        // nominal subjects resolve their members from one derived table
        let reduced = self.reduce_type_head(origin, subject.target)?;
        if let Some(instance) = self.apparent_instance(reduced)? {
            let inherent =
                self.inherent_member_table(origin, subject.receiver, &instance, subject.space)?;
            let extensions = self.subject_extension_members(
                origin,
                module,
                subject.receiver,
                reduced,
                instance.symbol,
                subject.space,
            )?;
            let mut bindings = Vec::with_capacity(inherent.len() + extensions.len());
            for (key, lookup) in inherent.iter() {
                if let Some(binding) = self.member_binding(origin, *key, lookup)? {
                    bindings.push(binding);
                }
            }

            // extensions serve the keys the inherent members left open
            for (key, candidates) in extensions.iter() {
                if inherent.contains_key(key) {
                    continue;
                }
                let lookup = MemberLookup::Found(candidates.clone());
                if let Some(binding) = self.member_binding(origin, *key, &lookup)? {
                    bindings.push(binding);
                }
            }

            return Ok(bindings);
        }

        // other subjects resolve each key through the source member lookup
        let keys = self.subject_member_keys(origin, module, &subject)?;
        let mut bindings = Vec::with_capacity(keys.len());
        for key in keys {
            let lookup = self.lookup_member(origin, module, subject, key)?;
            if let Some(binding) = self.member_binding(origin, key, &lookup)? {
                bindings.push(binding);
            }
        }

        Ok(bindings)
    }

    /// Return the lookup member represented by one definition member.
    pub(in crate::check) fn declared_member(
        &mut self,
        member: &dir::DefinitionMember,
    ) -> CompilerResult<Option<DeclaredMember>> {
        let Some(role) = MemberRole::from_definition(member) else {
            return Ok(None);
        };
        let Some(symbol) = member.symbol() else {
            return Err(CompilerError::Internal {
                message: format!("keyed definition member {member:?} has no symbol"),
            });
        };
        let ty = self.definition_member_type(member)?;

        Ok(Some(DeclaredMember {
            symbol,
            space: member.space(),
            ty,
            value: member.static_value(),
            role,
            kind: member.kind(),
            is_writable: member.is_writable(),
            is_optional: member.is_optional(),
        }))
    }

    /// Select the readable resolution exposed by one member lookup.
    pub(in crate::check) fn select_member_read(
        &mut self,
        origin: Origin,
        receiver: Value,
        key: dir::StaticKey,
        lookup: &MemberLookup,
    ) -> CompilerResult<Option<dir::MemberDecision>> {
        match lookup {
            MemberLookup::Missing => Ok(None),
            MemberLookup::Field(field) => Ok(field
                .read_access(receiver.ty, key, self)?
                .map(dir::OperationResolution::One)),
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
                        let Some(call) = self.select_getter_call(origin, receiver, candidate)?
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
                    [] => return Ok(None),
                    [target] => target.clone(),
                    _ => dir::MemberTarget::Existential(targets),
                };
                let ty = self.normalized_intersection_type(types)?;

                let access = dir::MemberAccess::new(receiver.ty, target, ty);

                Ok(Some(dir::OperationResolution::One(access)))
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
                        self.select_member_read(origin, arm_receiver, key, &arm.lookup)?
                    else {
                        return Ok(None);
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

                Ok(Some(dir::OperationResolution::Union { arms: accesses, ty }))
            }
            MemberLookup::Intersection(lookups) => {
                let mut resolutions = Vec::with_capacity(lookups.len());
                for lookup in lookups {
                    let Some(resolution) =
                        self.select_member_read(origin, receiver, key, lookup)?
                    else {
                        return Ok(None);
                    };
                    resolutions.push(resolution);
                }
                let resolution = self.intersect_member_decisions(origin, resolutions)?;

                Ok(Some(resolution))
            }
        }
    }

    /// Intersect simultaneous member resolutions, distributing runtime union arms.
    pub(in crate::check) fn intersect_member_decisions(
        &mut self,
        origin: Origin,
        resolutions: Vec<dir::MemberDecision>,
    ) -> CompilerResult<dir::MemberDecision> {
        // expand each resolution into the runtime combinations it contributes
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

        // intersect the accesses of every combination into one arm
        let mut arms = Vec::with_capacity(combinations.len());
        for accesses in combinations {
            arms.push(self.intersect_member_accesses(origin, accesses)?);
        }
        if !has_union && arms.len() == 1 {
            return Ok(dir::OperationResolution::One(arms.remove(0)));
        }

        // join the surviving arms into one runtime union
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
        // a lone access needs no intersection
        if accesses.len() == 1 {
            let mut accesses = accesses;

            return Ok(accesses.remove(0));
        }

        // flatten every access into one intersected receiver, target, and type
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
    ) -> CompilerResult<Option<dir::Call>> {
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
        let selected = self.attempt_callable(
            origin,
            callable,
            Some(candidate.owner),
            Some(selection_receiver),
            &candidate.generic_arguments,
            &[],
            &arguments,
            None,
        )?;

        // a rejecting receiver skips to the next declared candidate
        let SignatureMatch::Selected(signature) = selected else {
            return Ok(None);
        };
        let arguments = Self::source_argument_bindings(&[], &signature.parameters);
        let resolution = signature.member_call(resolution, candidate.owner, symbol, arguments);

        Ok(Some(resolution))
    }

    /// Select one setter invocation from a writable member candidate.
    pub(in crate::check) fn select_setter_call(
        &mut self,
        origin: Origin,
        receiver: Value,
        candidate: &MemberCandidate,
    ) -> CompilerResult<dir::Call> {
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
        let selected = self.attempt_callable(
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
        )?;
        let SignatureMatch::Selected(signature) = selected else {
            return Err(CompilerError::Internal {
                message: format!("selected setter {symbol:?} rejects its declared value type"),
            });
        };
        let arguments = Self::source_argument_bindings(&sources, &signature.parameters);
        let resolution = signature.member_call(resolution, candidate.owner, symbol, arguments);

        Ok(resolution)
    }

    /// Select the member meaning of one member access node.
    pub(in crate::check) fn select_member(
        &mut self,
        site: FlowSite,
        left: dir::LocalNodeId<dir::Expression>,
        name: Option<dir::StringId>,
        is_optional: bool,
    ) -> CompilerResult<()> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();

        // reduce the receiver before member lookup
        let receiver_node = left.into_global_any(module);
        let receiver_site = self.visit_site(receiver_node)?;
        let written_receiver = self.infer_node_type(receiver_site, PlaceUse::Read)?;

        // unknown receivers defer selection until their value settles
        if !self.check.infer.forcing
            && let Some(stalled_on) = self.check.root_variable(written_receiver)?
        {
            return self.defer_member_selection(site, stalled_on);
        }

        let (subject, rejected) =
            self.resolve_member_subject(origin, receiver_node, written_receiver)?;
        if let Some(rejected) = rejected
            && !is_optional
        {
            self.report_possibly_nullish(origin, rejected.label().to_string())?;
        }

        // retain the lookup subject at this source site
        self.module_mut(module)
            .members
            .record_subject(dir::MemberSite::Node(node), subject);

        // commit an error for an omitted member name
        let Some(name) = name else {
            self.commit_decision(node, dir::Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(());
        };
        let key = dir::StaticKey::Name(name);

        let lookup = self.lookup_member(origin, module, subject, key)?;

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
    ) -> CompilerResult<()> {
        let receiver_site = self.visit_site(receiver_node)?;
        let receiver = self.expression_value(receiver_site, receiver)?;
        let Some(resolution) = self.select_member_read(origin, receiver, key, &lookup)? else {
            if lookup.has_setter() {
                let key = self.strings().get(name).to_string();
                self.report_write_only_member(origin, key)?;
                self.commit_decision(node, dir::Decision::Rejected)?;
                self.commit_error_node(node)?;

                return Ok(());
            }

            let key = self.strings().get(name).to_string();

            return self.reject_member(node, origin, receiver.ty, key);
        };

        // commit the exact runtime target tree and joined value type
        let ty = resolution.ty();
        let stored_key = resolution.stored_key();
        self.commit_decision(node, dir::Decision::Member(resolution))?;
        if let Some(key) = stored_key {
            self.commit_projected_access(node, receiver_node, key)?;
        }
        let site = self.visit_site(node)?;
        let ty = self.flow_type_at(site, ty)?;
        self.commit_node_type(node, ty)?;

        Ok(())
    }

    /// Defer one member access whose receiver value has not settled yet.
    ///
    /// The node commits an open hole so enclosing checks proceed; the
    /// deferred re-selection solves the hole once the receiver closes.
    fn defer_member_selection(
        &mut self,
        site: FlowSite,
        stalled_on: dir::TypeVariableId,
    ) -> CompilerResult<()> {
        let node = site.node;
        if self.committed_node_type(node).is_none() {
            let variable =
                self.allocate_variable(site.origin(), Widening::Never, VariableRole::Regular);
            let hole = self.variable_type(variable)?;
            self.commit_node_type(node, hole)?;
        }
        self.check.register_check(DeferredCheck::Infer {
            site,
            use_: PlaceUse::Read,
            stalled_on: Some(stalled_on),
        });

        Ok(())
    }

    /// Reject one member access with a diagnostic.
    fn reject_member(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: String,
    ) -> CompilerResult<()> {
        // poison instead of reporting again when the receiver already reported an error
        if self.any_error_operand(&[receiver])? {
            self.poison_node(node)?;

            return Ok(());
        }

        let key_span = self.module(node.module_id).diagnostic_span(node.local_id);
        self.report_missing_member(origin, receiver, key, key_span)?;
        self.commit_decision(node, dir::Decision::Rejected)?;
        self.commit_error_node(node)?;

        Ok(())
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
    ) -> CompilerResult<dir::GlobalTypeId> {
        if role != MemberRole::Field {
            return Ok(ty);
        }
        let Some(receiver) = receiver else {
            return Ok(ty);
        };
        let ty = self.project_member_place(origin, receiver, ty)?;

        // readonly receivers project deep readonly views onto stored fields
        if !self.receiver_projects_readonly(origin, receiver)? {
            return Ok(ty);
        }
        let ty = self.reduce_type_head(origin, ty)?;
        if matches!(self.ty(ty)?, dir::Type::Form(form) if form.form == dir::Form::Readonly) {
            return Ok(ty);
        }
        if !self.type_projects_readonly(origin, ty)? {
            return Ok(ty);
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
    ) -> CompilerResult<bool> {
        let mut current = receiver;

        loop {
            current = self.reduce_type_head(origin, current)?;
            let form = match self.ty(current)? {
                dir::Type::Form(form) => form,
                _ => return Ok(false),
            };

            // explicit readonly views make every stored field readonly
            if form.form == dir::Form::Readonly {
                return Ok(true);
            }

            // readonly borrows expose only readonly stored fields
            if let dir::Form::Borrowed(borrow) = form.form
                && let access = self.type_borrow(current.module_id, borrow)?.access
                && self.access_is_readonly(origin, access)?
            {
                return Ok(true);
            }

            current = form.value;
        }
    }

    /// Project one type-level member access through its owner.
    pub(in crate::check) fn project_member(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // bind the projection from written refinements
        if let Some(qualifier) = member.qualifier {
            let (_, bindings) = self.refinement_bindings(qualifier)?;
            if let Some((_, value)) = bindings.iter().find(|(key, _)| *key == member.key) {
                return Ok(Some(*value));
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
                _ => Ok(None),
            };
        }

        // resolve remaining projections through member lookup
        let module = origin.module();
        let subject = dir::MemberSubject::new(member.owner, member.owner, dir::MemberSpace::Static);
        let lookup = self.lookup_member(origin, module, subject, member.key)?;
        let projected = match lookup {
            MemberLookup::Missing => self.project_default_member(origin, member)?,
            lookup => self.project_member_lookup(module, member, lookup)?,
        };

        Ok(projected)
    }

    /// Project one associated member through its selected interface implementation.
    fn project_selected_member(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
        qualifier: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
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

            return self.project_declared_associated_member(
                origin,
                member,
                &members,
                &substitution,
            );
        }

        // enumerate candidate extensions by receiver family
        let apparent = self.intern_apparent_type(module, owner)?;
        let extensions =
            self.visible_implementation_extensions(origin, module, apparent, interface.symbol)?;
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

                Ok(match matched {
                    Some(_) => CandidateOutcome::Accepted(()),
                    None => CandidateOutcome::Rejected(()),
                })
            })?;
            if !matches!(verdict, CandidateVerdict::Viable) {
                continue;
            }

            // rerun the match to commit its substitution
            let matched = self.match_extension_implementation(
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
            let Some((substitution, _)) = matched else {
                continue;
            };

            // project the extension's own declared value with the member's
            //  arguments bound, falling back to the interface default
            let projected =
                self.project_declared_associated_member(origin, member, &members, &substitution)?;
            if projected.is_some() {
                return Ok(projected);
            }

            return self.project_qualified_default(origin, member, qualifier);
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
                let matched = self.match_implemented_interface(
                    origin,
                    Relation::Assignable,
                    interface_module,
                    &[],
                    &mut substitution,
                    &implements,
                    &interface,
                )?;
                if matched.is_some() {
                    // project the declaring owner's own value with the member's
                    //  arguments bound, falling back to the interface default
                    let projected = self.project_declared_associated_member(
                        origin,
                        member,
                        &members,
                        &substitution,
                    )?;
                    if projected.is_some() {
                        return Ok(projected);
                    }

                    return self.project_qualified_default(origin, member, qualifier);
                }
            }
        }

        Ok(None)
    }

    /// Project one associated member declared by a lexical extension scope.
    fn project_scope_member(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
        scope: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(definition) = self.definition(scope)?.cloned() else {
            return Err(CompilerError::Internal {
                message: format!("associated type scope {scope:?} has no definition"),
            });
        };

        // bind an extension scope's parameters through its matched target;
        //  the projection reads the owner value beneath any receiver form
        let substitution = match &definition {
            dir::Definition::Extension(extension) => {
                let template = self.symbol_template(scope)?;
                let target = extension.target.r#type();
                let owner = self.strip_form(origin, member.owner)?;
                let matched =
                    self.match_extension_subject(origin, owner, owner, template, target)?;
                let Some(matched) = matched else {
                    return Ok(None);
                };

                matched.with_receiver(owner)
            }
            _ => TypeSubstitution::default().with_receiver(member.owner),
        };

        // project the scope's own members against the written owner
        let members = definition.members().to_vec();

        self.project_declared_associated_member(origin, member, &members, &substitution)
    }

    /// Project one associated type declared by a selected scope.
    fn project_declared_associated_member(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
        members: &[dir::DefinitionMember],
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // select and substitute the scope's declared value
        let declared = members.iter().find_map(|declared| match declared {
            dir::DefinitionMember::AssociatedType(associated) if associated.key == member.key => {
                associated.value.map(|value| (associated, value))
            }
            _ => None,
        });
        if let Some((declared, value)) = declared {
            let value = self.project_associated_value(
                origin.module(),
                member,
                declared.symbol,
                value,
                substitution,
            )?;

            return Ok(Some(value));
        }

        Ok(None)
    }

    /// Project one associated type default declared by the qualifying interface.
    fn project_qualified_default(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
        qualifier: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // read the interface's own members under its applied arguments
        let (interface_module, interface) = self.nominal_application(qualifier)?;
        let scope = self.resolve_symbol_alias(interface.symbol)?;
        let Some(definition) = self.definition(scope)? else {
            return Ok(None);
        };
        let members = definition.members().to_vec();
        let substitution =
            self.qualified_instance_substitution(interface_module, &interface, member.owner)?;

        self.project_declared_associated_member(origin, member, &members, &substitution)
    }

    /// Return the type projected by one selected member lookup.
    fn project_member_lookup(
        &mut self,
        module: ModuleId,
        member: &dir::MemberType,
        lookup: MemberLookup,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        match lookup {
            MemberLookup::Field(field) => field.read_type(self),
            MemberLookup::Found(candidates) => match candidates.as_slice() {
                [candidate] => {
                    // retain nominal singleton identity for variant values
                    if candidate.role == MemberRole::VariantValue {
                        return candidate.read_type(module, self);
                    }

                    // project associated values through their applied arguments
                    if candidate.value_type.is_some() {
                        let written = self.static_value(candidate.symbol).ok_or_else(|| {
                            CompilerError::Internal {
                                message: format!(
                                    "associated member {:?} lost its declared value",
                                    candidate.symbol,
                                ),
                            }
                        })?;
                        let substitution = TypeSubstitution {
                            bindings: candidate.generic_arguments.iter().copied().collect(),
                            receiver: Some(member.owner),
                        };
                        let written = self.project_associated_value(
                            module,
                            member,
                            candidate.symbol,
                            written,
                            &substitution,
                        )?;

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
        origin: Origin,
        member: &dir::MemberType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        if self.is_rigid_projection_owner(member.owner)? {
            return Ok(None);
        }
        let Some(qualifier) = member.qualifier else {
            return Ok(None);
        };
        let Some((interface_module, interface)) = self.nominal_application_maybe(qualifier)? else {
            return Ok(None);
        };

        // select the declared interface default
        let associated = match self.definition(interface.symbol)? {
            Some(dir::Definition::Interface(definition)) => {
                definition
                    .members
                    .iter()
                    .find_map(|declared| match declared {
                        dir::DefinitionMember::AssociatedType(associated)
                            if associated.key == member.key =>
                        {
                            associated.value.map(|value| (associated.symbol, value))
                        }
                        _ => None,
                    })
            }
            _ => None,
        };
        let Some((symbol, value)) = associated else {
            return Ok(None);
        };
        let substitution =
            self.qualified_instance_substitution(interface_module, &interface, member.owner)?;
        let value =
            self.project_associated_value(origin.module(), member, symbol, value, &substitution)?;

        Ok(Some(value))
    }

    /// Project one associated value through its owner and applied arguments.
    fn project_associated_value(
        &mut self,
        module: ModuleId,
        member: &dir::MemberType,
        symbol: dir::GlobalSymbolId,
        value: dir::GlobalTypeId,
        owner_substitution: &TypeSubstitution,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // substitute owner parameters inside applied member arguments
        let arguments = self.type_ids(module, member.arguments)?;
        let mut arguments = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(arguments);
        for argument in &mut arguments {
            *argument = self.substitute_type(*argument, owner_substitution)?;
        }

        // extend the owner substitution with the member's own parameters
        let bindings = self.symbol_generic_argument_bindings(symbol, &arguments)?;
        let mut substitution = owner_substitution.clone();
        for binding in bindings {
            substitution.bind(binding.parameter, binding.argument)?;
        }

        self.substitute_type(value, &substitution)
    }

    /// Return whether associated defaults remain overridable beneath one owner.
    pub(in crate::check) fn is_rigid_projection_owner(
        &self,
        owner: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let owner = self.shallow_resolve(owner)?;
        let is_rigid = matches!(self.ty(owner)?, dir::Type::Parameter(_) | dir::Type::This);

        Ok(is_rigid)
    }

    /// Select the unique interface application declaring one associated member.
    pub(in crate::check) fn select_associated_qualifier(
        &mut self,
        origin: Origin,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut interfaces = SmallVec::<[dir::GlobalTypeId; 4]>::new();

        // parameter projections select from their declared bounds
        if let dir::Type::Parameter(parameter) = self.ty(owner)? {
            interfaces.extend(self.parameter_bounds(origin, parameter)?);
        }
        // nominal projections select from their checked heritage
        else if self.nominal_application_maybe(owner)?.is_some() {
            let closure = self.heritage_closure(origin, owner)?;
            interfaces.extend(
                closure
                    .applications
                    .into_iter()
                    .map(|application| application.ty),
            );
        }
        // other owners cannot expose associated declarations
        else {
            return Ok(None);
        }

        // retain only interfaces declaring this associated member
        let mut qualifier = None;
        for interface in interfaces {
            let interface = self.reduce_type_head(origin, interface)?;
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

                return Ok(None);
            }
            qualifier = Some(interface);
        }

        Ok(qualifier)
    }

    /// Return the substituted constraint declared for one rigid projection.
    pub(in crate::check) fn projection_constraint(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let owner = self.strip_form(origin, member.owner)?;
        let dir::Type::Parameter(parameter) = self.ty(owner)? else {
            return Ok(None);
        };

        for bound in self.parameter_bounds(origin, parameter)? {
            // only interface bounds can declare projected members
            let bound = self.reduce_type_head(origin, bound)?;
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

                return Ok(Some(constraint));
            }
        }

        Ok(None)
    }

    /// Return one field type projected through the receiver placement.
    fn project_member_place(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(place) = self.receiver_projected_place(origin, receiver)? else {
            return Ok(ty);
        };
        let ty = self.place_relative_type(origin, place, ty)?;

        self.reduce_type_head(origin, ty)
    }

    /// Resolve one relative member type in a projected receiver place.
    pub(in crate::check) fn place_relative_type(
        &mut self,
        origin: Origin,
        place: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // bare members of local receivers stay bare
        let root = self.reduce_type_head(origin, place)?;
        if self.check.place_space(root)? == Some(dir::Space::Local) {
            return Ok(ty);
        }

        self.resolve_relative_place(origin, ty, place)
    }

    /// Return the place projected by one receiver type.
    pub(in crate::check) fn receiver_projected_place(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut current = receiver;
        loop {
            current = self.reduce_type_head(origin, current)?;
            let dir::Type::Form(form) = self.ty(current)? else {
                // bare nominal instances live in their declared or inherited space
                if let dir::Type::Application(instance) = self.ty(current)?
                    && let Some(space) = self.check.nominal_space(instance.symbol)?
                {
                    let place = self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Place(
                        dir::Place::Space(space),
                    )))?;

                    return Ok(Some(place));
                }

                return Ok(None);
            };

            match form.form {
                dir::Form::Placed { place } => return Ok(Some(place)),
                _ => current = form.value,
            }
        }
    }

    /// Return whether one projected value should retain a readonly view.
    fn type_projects_readonly(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let ty = self.reduce_type_head(origin, ty)?;

        // retain readonly access over every safe reference carrier
        if self.type_is_reference(origin, ty)? {
            return Ok(true);
        }

        // copied values carry no view to project
        if self.satisfies_auto_interface(origin, ty, dir::AutoInterface::Copy)? {
            return Ok(false);
        }

        // unions project a view only when one element requires it
        if let dir::Type::Union(union) = self.ty(ty)? {
            let elements: SmallVec<[_; 4]> =
                SmallVec::from_slice(self.type_ids(ty.module_id, union.elements)?);
            for element in elements {
                if self.type_projects_readonly(origin, element)? {
                    return Ok(true);
                }
            }

            return Ok(false);
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

        Ok(result)
    }

    /// Return whether one memory access component is readonly.
    pub(in crate::check) fn access_is_readonly(
        &mut self,
        origin: Origin,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let access = self.reduce_type_head(origin, access)?;
        let is_readonly = matches!(
            self.ty(access)?,
            dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Readonly))
        );

        Ok(is_readonly)
    }
}
