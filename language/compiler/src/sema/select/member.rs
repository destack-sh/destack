use std::slice;
use std::sync::Arc;

use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    BodyState, CallableArgument, CandidateOutcome, CandidateVerdict, Check, CheckState, FlowSite,
    InferMode, NullishPart, Origin, PlaceUse, ReceiverSteps, Relation, SelectionCheck,
    SignatureMatch, TypeSubstitution, Value, ValueUse, VariableRole, Widening,
};
use crate::{CompilerError, CompilerResult};

pub(in crate::sema) use destack_dir::MemberRole;

/// One field found by member lookup.
#[derive(Debug, Clone)]
pub(in crate::sema) enum FieldLookup {
    /// One field declared by a structural aggregate.
    Structural {
        /// The receiver selection retained during lookup.
        receiver: LookupReceiver,
        /// The structural aggregate that declares the field.
        owner: dir::GlobalTypeId,
        /// The projected property operations.
        access: dir::PropertyAccess,
        /// Whether the property may be absent.
        is_optional: bool,
    },
    /// One compiler-defined field projection.
    Projection {
        /// The receiver adjustments applied before projection.
        adjustments: ReceiverSteps,
        /// The selected projection.
        projection: dir::Projection,
    },
}

/// Result of looking up one member on a receiver type.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub(in crate::sema) enum MemberLookup {
    /// The receiver has no such member.
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
pub(in crate::sema) struct MemberArmLookup {
    /// The runtime receiver arm.
    pub(in crate::sema) receiver: dir::GlobalTypeId,
    /// The member selected on that arm.
    pub(in crate::sema) lookup: MemberLookup,
}

impl FieldLookup {
    /// Return the type produced by reading this property.
    pub(in crate::sema) fn read_type(
        &self,
        body: &mut BodyState<'_, '_>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let (read, is_optional) = match self {
            Self::Structural {
                access,
                is_optional,
                ..
            } => (access.read(), *is_optional),
            Self::Projection { projection, .. } => (Some(projection.ty()), false),
        };

        let Some(read) = read else {
            return Ok(None);
        };

        if !is_optional {
            return Ok(Some(read));
        }

        // read an optional property as its type or undefined
        let undefined = body.intern_type(dir::Type::Undefined)?;
        let read = body.normalized_union_type([read, undefined])?;

        Ok(Some(read))
    }

    /// Return the type accepted by writing this property.
    pub(in crate::sema) fn write_type(&self) -> Option<dir::GlobalTypeId> {
        match self {
            Self::Structural { access, .. } => access.write(),
            Self::Projection { .. } => None,
        }
    }

    /// Return whether this field may be absent.
    fn is_optional(&self) -> bool {
        match self {
            Self::Structural { is_optional, .. } => *is_optional,
            Self::Projection { .. } => false,
        }
    }

    /// Return the member access selected by reading this field.
    pub(in crate::sema) fn read_access(
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
                receiver: selection,
                owner,
                ..
            } => dir::MemberTarget::Field(dir::FieldResolution {
                receiver: selection.resolve(receiver),
                target: dir::FieldTarget::Structural { owner: *owner, key },
                ty,
            }),
            Self::Projection {
                adjustments,
                projection,
            } => dir::MemberTarget::Projection {
                key,
                receiver: dir::AdjustedReceiver {
                    source: receiver,
                    adjustments: adjustments.clone(),
                },
                projection: projection.clone(),
            },
        };

        Ok(Some(dir::MemberAccess::new(receiver, target, ty)))
    }

    /// Return the value projection selected by reading this field.
    pub(in crate::sema) fn projection(
        &self,
        source: dir::GlobalTypeId,
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
                receiver: receiver.resolve(source),
                target: dir::FieldTarget::Structural { owner: *owner, key },
                ty,
            }),
            Self::Projection {
                adjustments,
                projection,
            } => {
                let target = dir::MemberTarget::Projection {
                    key,
                    receiver: dir::AdjustedReceiver {
                        source,
                        adjustments: adjustments.clone(),
                    },
                    projection: projection.clone(),
                };
                let access = dir::MemberAccess::new(source, target, ty);

                dir::Projection::Member(Box::new(access))
            }
        };

        Ok(Some(projection))
    }

    /// Return the member access selected by writing this field.
    pub(in crate::sema) fn write_access(
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
            Self::Projection { .. } => return None,
        };

        let target = dir::MemberTarget::Field(dir::FieldResolution {
            receiver: field_receiver.resolve(receiver),
            target: dir::FieldTarget::Structural { owner: *owner, key },
            ty,
        });

        Some(dir::MemberAccess::new(receiver, target, ty))
    }

    /// Prepend one implicit receiver adjustment.
    fn prepend_adjustment(&mut self, adjustment: dir::ReceiverAdjustment) {
        match self {
            Self::Structural { receiver, .. } => receiver.prepend(adjustment),
            Self::Projection { adjustments, .. } => adjustments.insert(0, adjustment),
        }
    }

    /// Select this field through one erased receiver.
    fn select_dynamic(&mut self, constraint: dir::GlobalTypeId) -> CompilerResult<()> {
        match self {
            Self::Structural { receiver, .. } => {
                *receiver = LookupReceiver::Dynamic {
                    adjustments: ReceiverSteps::new(),
                    constraint,
                };
            }
            Self::Projection { .. } => {
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
pub(in crate::sema) struct DeclaredMember {
    /// The declaring member symbol.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
    /// The symbol whose checked type carries this member's value.
    pub(in crate::sema) type_symbol: Option<dir::GlobalSymbolId>,
    /// The static key selecting this member.
    pub(in crate::sema) key: dir::StaticKey,
    /// The member space.
    pub(in crate::sema) space: dir::MemberSpace,
    /// The member type when the declaration has one.
    pub(in crate::sema) ty: Option<dir::GlobalTypeId>,
    /// The member static value when it carries one.
    pub(in crate::sema) value: Option<dir::GlobalStaticId>,
    /// How the member behaves at a use site.
    pub(in crate::sema) role: MemberRole,
    /// The member kind.
    pub(in crate::sema) kind: dir::MemberKind,
    /// Whether the member accepts writes.
    pub(in crate::sema) is_writable: bool,
    /// Whether the member may be absent.
    pub(in crate::sema) is_optional: bool,
}

impl MemberLookup {
    /// Return a lookup from collected candidates.
    pub(in crate::sema) fn from_candidates(candidates: Vec<MemberCandidate>) -> Self {
        if candidates.is_empty() {
            Self::Missing
        } else {
            Self::Found(candidates)
        }
    }

    /// Consume declaration candidates from a lookup without runtime alternatives.
    pub(in crate::sema) fn into_candidates(self) -> Option<Vec<MemberCandidate>> {
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
    pub(in crate::sema) fn is_found(&self) -> bool {
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

    /// Return the required field type this lookup reads directly.
    pub(in crate::sema) fn direct_field_type(&self) -> Option<dir::GlobalTypeId> {
        match self {
            // a refined structural field reads its stored value
            Self::Field(FieldLookup::Structural {
                receiver: LookupReceiver::Direct(_),
                access,
                is_optional: false,
                ..
            }) => access.read(),
            // one selected declared field reads its stored value
            Self::Found(candidates) => {
                let candidates = Self::selected_candidates(candidates);
                let [candidate] = candidates.as_slice() else {
                    return None;
                };
                if candidate.role != MemberRole::Field
                    || candidate.is_optional
                    || !matches!(candidate.receiver, LookupReceiver::Direct(_))
                {
                    return None;
                }

                Some(candidate.access_type)
            }
            // every remaining lookup reads through a property, a projection, or many arms
            Self::Missing
            | Self::Field(FieldLookup::Structural { .. })
            | Self::Field(FieldLookup::Projection { .. })
            | Self::Union(_)
            | Self::Intersection(_) => None,
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
    pub(in crate::sema) fn prepend_adjustment(&mut self, adjustment: dir::ReceiverAdjustment) {
        match self {
            Self::Missing => {}
            Self::Field(field) => field.prepend_adjustment(adjustment),
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
    pub(in crate::sema) fn select_dynamic(
        &mut self,
        constraint: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        match self {
            Self::Missing => {}
            Self::Field(field) => field.select_dynamic(constraint)?,
            Self::Found(candidates) => {
                for candidate in candidates {
                    candidate.receiver = LookupReceiver::Dynamic {
                        adjustments: ReceiverSteps::new(),
                        constraint,
                    };
                }
            }
            Self::Union(lookups) => {
                for arm in lookups {
                    arm.lookup.select_dynamic(constraint)?;
                }
            }
            Self::Intersection(lookups) => {
                for lookup in lookups {
                    lookup.select_dynamic(constraint)?;
                }
            }
        }

        Ok(())
    }
}

impl DeclaredMember {
    /// Read one definition member, carrying the type its declaration writes.
    ///
    /// The written type stays unresolved until the member's target matches.
    pub(in crate::sema) fn from_definition(
        member: &dir::DefinitionMember,
    ) -> CompilerResult<Option<Self>> {
        let Some(role) = MemberRole::from_definition(member) else {
            return Ok(None);
        };
        let Some(key) = member.key() else {
            return Ok(None);
        };
        let Some(symbol) = member.symbol() else {
            return Err(CompilerError::Internal {
                message: format!("keyed definition member {member:?} has no symbol"),
            });
        };

        Ok(Some(Self {
            symbol,
            type_symbol: member.type_symbol(),
            key,
            space: member.space(),
            ty: member.value_type(),
            value: member.static_value(),
            role,
            kind: member.kind(),
            is_writable: member.is_writable(),
            is_optional: member.is_optional(),
        }))
    }

    /// Return the value type exposed by this member at a use site.
    pub(in crate::sema) fn access_type(
        &self,
        check: &CheckState<'_>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(access) = check.property_access(self.role, ty, false)? else {
            return Ok(ty);
        };

        Ok(access.store())
    }

    /// Return the callable type exposed by this declaration member.
    pub(in crate::sema) fn callable_type(
        &self,
        ty: dir::GlobalTypeId,
    ) -> Option<dir::GlobalTypeId> {
        self.role.is_callable().then_some(ty)
    }
}

/// One closed subject whose extension member table is decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) struct MemberSubject {
    /// The module whose scope selects the visible extensions.
    pub(in crate::sema) module: ModuleId,
    /// The receiver type carrying its memory form.
    pub(in crate::sema) receiver: dir::GlobalTypeId,
    /// The canonical extension subject.
    pub(in crate::sema) subject: dir::GlobalTypeId,
    /// The subject's declaration symbol.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
    /// The member space looked up.
    pub(in crate::sema) space: dir::MemberSpace,
}

/// Extension members of one subject, grouped by static key.
pub(in crate::sema) type MemberTable = Arc<FxIndexMap<dir::StaticKey, Vec<MemberCandidate>>>;

/// One declaration-backed member candidate.
#[derive(Debug, Clone)]
pub(in crate::sema) struct MemberCandidate {
    /// The declaring member symbol.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
    /// The declaration that exposed the member.
    pub(in crate::sema) owner: dir::GlobalSymbolId,
    /// The declaration family the member came from.
    pub(in crate::sema) origin: dir::MemberOrigin,
    /// The member space that selected this candidate.
    pub(in crate::sema) space: dir::MemberSpace,
    /// How the member behaves at a use site.
    pub(in crate::sema) role: MemberRole,
    /// The member kind.
    pub(in crate::sema) kind: dir::MemberKind,
    /// Whether the member accepts writes.
    pub(in crate::sema) is_writable: bool,
    /// The substituted type before optional read widening.
    pub(in crate::sema) access_type: dir::GlobalTypeId,
    /// The substituted callable type for methods and accessors.
    pub(in crate::sema) callable: Option<dir::GlobalTypeId>,
    /// Whether the member may be absent.
    pub(in crate::sema) is_optional: bool,
    /// The generic arguments matched through the owner.
    pub(in crate::sema) generic_arguments: Vec<dir::GenericArgumentBinding>,
    /// The member static value when it carries one.
    pub(in crate::sema) value: Option<dir::GlobalStaticId>,
    /// The substituted static value of the member, when it has one.
    pub(in crate::sema) value_type: Option<dir::GlobalTypeId>,
    /// The receiver adjustments selected during lookup.
    pub(in crate::sema) receiver: LookupReceiver,
}

/// Receiver state retained by one member lookup candidate.
#[derive(Debug, Clone)]
pub(in crate::sema) enum LookupReceiver {
    /// Direct adjustments attached to the use-site receiver on resolution.
    Direct(ReceiverSteps),
    /// Erased receiver selected for dynamic dispatch.
    Dynamic {
        /// The receiver adjustments applied before dispatch.
        adjustments: ReceiverSteps,
        /// The interface constraint declaring the dispatch member.
        constraint: dir::GlobalTypeId,
    },
}

impl MemberCandidate {
    /// Return this candidate's selection precedence.
    pub(in crate::sema) fn precedence(&self) -> (dir::MemberOrigin, bool) {
        let is_adjusted = match &self.receiver {
            LookupReceiver::Direct(steps) => !steps.is_empty(),
            LookupReceiver::Dynamic { .. } => true,
        };

        (self.origin, is_adjusted)
    }
}

impl LookupReceiver {
    /// Prepend one adjustment performed before the existing adjustments.
    fn prepend(&mut self, adjustment: dir::ReceiverAdjustment) {
        match self {
            Self::Direct(steps) => steps.insert(0, adjustment),
            Self::Dynamic { adjustments, .. } => adjustments.insert(0, adjustment),
        }
    }

    /// Attach this lookup state to its use-site receiver.
    pub(in crate::sema) fn resolve(&self, source: dir::GlobalTypeId) -> dir::MemberReceiver {
        match self {
            Self::Direct(steps) => dir::MemberReceiver::Direct(dir::AdjustedReceiver {
                source,
                adjustments: steps.clone(),
            }),
            Self::Dynamic {
                adjustments,
                constraint,
            } => dir::MemberReceiver::Dynamic(dir::DynamicDispatch {
                receiver: dir::AdjustedReceiver {
                    source,
                    adjustments: adjustments.clone(),
                },
                constraint: *constraint,
            }),
        }
    }
}

impl MemberCandidate {
    /// Return the type produced by reading this candidate.
    pub(in crate::sema) fn read_type(
        &self,
        body: &mut BodyState<'_, '_>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        if !self.role.is_readable() {
            return Ok(None);
        }

        if !self.is_optional {
            return Ok(Some(self.access_type));
        }

        // read an optional member as its type or undefined
        let undefined = body.intern_type(dir::Type::Undefined)?;
        let ty = body.normalized_union_type([self.access_type, undefined])?;

        Ok(Some(ty))
    }

    /// Return the durable member candidate for this lookup candidate.
    pub(in crate::sema) fn resolution_candidate(
        &self,
        receiver: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
    ) -> dir::MemberCandidate {
        dir::MemberCandidate {
            receiver: self.receiver.resolve(receiver),
            space: self.space,
            owner: self.owner,
            access_type: ty,
            callable_type: self.callable,
            selection: dir::Selection::new(self.symbol, self.generic_arguments.clone()),
        }
    }

    /// Return the durable member access for this lookup candidate.
    pub(in crate::sema) fn access(
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
    pub(in crate::sema) fn field(&self, key: dir::StaticKey) -> Option<dir::FieldTarget> {
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
    pub(in crate::sema) fn resolve_member_subject(
        &mut self,
        origin: Origin,
        receiver_node: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<(dir::MemberSubject, Option<NullishPart>)> {
        // split the nullish part off the lookup target
        let split = self.split_nullish_type(origin, target)?;
        let rejected = split.map(|split| split.rejected);
        let target = split.map_or(target, |split| split.value);

        // settle an open target's value variables before member lookup
        if self.type_flags(target)?.has_variable() {
            let mut variables = self.type_variables(target)?;
            variables.retain(|variable| {
                !matches!(
                    self.infer.variable_role(*variable),
                    Ok(VariableRole::Memory { .. })
                )
            });
            self.resolve_variables(&variables)?;
        }

        // look the member up in the receiver's own space by default
        let mut space = self.member_receiver_space(receiver_node, target)?;
        let mut subject = target;

        // static type aliases look up through their declaration reference
        let resolution = self
            .resolutions(receiver_node.module_id)
            .name_resolution(receiver_node)
            .cloned();
        if let Some(resolution) = resolution
            && let [symbol] = resolution.symbols()
            && self.symbol_kind(*symbol)?.is_type_alias()
        {
            space = dir::MemberSpace::Static;
            subject =
                self.intern_type(dir::Type::Reference(dir::TypeReference { symbol: *symbol }))?;
        }

        let subject = dir::MemberSubject::new(receiver, subject, space)
            .with_scope(self.assuming_scope(origin)?);

        Ok((subject, rejected))
    }

    /// Return the readable type exposed by one member lookup.
    pub(in crate::sema) fn member_read_type(
        &mut self,
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
                    types.extend(candidate.read_type(self)?);
                }

                if types.is_empty() {
                    return Ok(None);
                }

                self.normalized_intersection_type(types).map(Some)
            }
            MemberLookup::Union(arms) => {
                let mut types = Vec::with_capacity(arms.len());
                for arm in arms {
                    let Some(ty) = self.member_read_type(&arm.lookup)? else {
                        return Ok(None);
                    };
                    types.push(ty);
                }

                self.normalized_union_type(types).map(Some)
            }
            MemberLookup::Intersection(lookups) => {
                let mut types = Vec::with_capacity(lookups.len());
                for lookup in lookups {
                    types.extend(self.member_read_type(lookup)?);
                }

                if types.is_empty() {
                    return Ok(None);
                }

                self.normalized_intersection_type(types).map(Some)
            }
        }
    }

    /// Return the writable type accepted by one member lookup.
    pub(in crate::sema) fn member_write_type(
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
    pub(in crate::sema) fn member_binding(
        &mut self,
        key: dir::StaticKey,
        lookup: &MemberLookup,
    ) -> CompilerResult<Option<dir::MemberBinding>> {
        // compose the operations this lookup exposes
        let read = self.member_read_type(lookup)?;
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
    pub(in crate::sema) fn subject_member_bindings(
        &mut self,
        origin: Origin,
        module: ModuleId,
        subject: dir::MemberSubject,
    ) -> CompilerResult<Vec<dir::MemberBinding>> {
        // resolve each reachable key through the same lookup used at source sites
        let keys = self.subject_member_keys(origin, module, &subject)?;
        let mut bindings = Vec::with_capacity(keys.len());
        for key in keys {
            let lookup = self.lookup_member(origin, module, subject, key)?;
            if let Some(binding) = self.member_binding(key, &lookup)? {
                bindings.push(binding);
            }
        }

        Ok(bindings)
    }

    /// Return the lookup member represented by one definition member.
    pub(in crate::sema) fn declared_member(
        &mut self,
        member: &dir::DefinitionMember,
    ) -> CompilerResult<Option<DeclaredMember>> {
        let Some(mut declared) = DeclaredMember::from_definition(member)? else {
            return Ok(None);
        };
        declared.ty = self.definition_member_type(member)?;

        Ok(Some(declared))
    }

    /// Resolve the checked type of each member declared through its own symbol.
    pub(in crate::sema) fn resolve_declared_types(
        &mut self,
        members: &mut [DeclaredMember],
    ) -> CompilerResult<()> {
        for member in members {
            let Some(type_symbol) = member.type_symbol else {
                continue;
            };

            member.ty = Some(self.symbol_type(type_symbol)?);
        }

        Ok(())
    }

    /// Select the readable resolution exposed by one member lookup.
    pub(in crate::sema) fn select_member_read(
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

                // build one access per surviving candidate
                let mut targets = Vec::new();
                let mut types = Vec::new();
                for candidate in candidates {
                    let Some(ty) = candidate.read_type(self)? else {
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

                // several accesses on one key read as their intersection
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
    pub(in crate::sema) fn intersect_member_decisions(
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
        // take a lone access without intersecting
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
    pub(in crate::sema) fn select_getter_call(
        &mut self,
        origin: Origin,
        receiver: Value,
        candidate: &MemberCandidate,
    ) -> CompilerResult<Option<dir::Call>> {
        // select against the adjusted receiver the lookup settled on
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

        // call a getter without arguments
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

        // skip a rejecting receiver to the next declared candidate
        let SignatureMatch::Selected(signature) = selected else {
            return Ok(None);
        };

        let arguments = self.bind_argument_sources(origin, &signature, &[])?;
        let resolution = signature.member_call(resolution, candidate.owner, symbol, arguments);

        Ok(Some(resolution))
    }

    /// Select one setter invocation from a writable member candidate.
    pub(in crate::sema) fn select_setter_call(
        &mut self,
        origin: Origin,
        receiver: Value,
        candidate: &MemberCandidate,
    ) -> CompilerResult<dir::Call> {
        // select against the adjusted receiver the lookup settled on
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

        // pass the written value as a setter's sole argument
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

        let arguments = self.bind_argument_sources(origin, &signature, &sources)?;
        let resolution = signature.member_call(resolution, candidate.owner, symbol, arguments);

        Ok(resolution)
    }

    /// Select the member meaning of one member access node.
    pub(in crate::sema) fn select_member(
        &mut self,
        site: FlowSite,
        left: dir::LocalNodeId<dir::Expression>,
        name: Option<dir::StringId>,
        is_optional: bool,
    ) -> CompilerResult<()> {
        // read the access node and its typing origin
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();

        // infer the receiver before member lookup
        let receiver_node = left.into_global_any(module);
        let receiver_site = self.visit_site(receiver_node)?;
        let receiver = self.infer_node(receiver_site, PlaceUse::Read, InferMode::Exact)?;
        let written_receiver = self.flow_type_at(receiver_site, receiver)?;
        self.commit_expression_place(receiver_site, written_receiver)?;

        // unknown receivers defer selection until their value settles
        if let Some(stalled_on) = self.check.root_variable(written_receiver)? {
            return self.defer_member_selection(site, stalled_on);
        }

        // strip the nullish arms the access reads through
        let (subject, rejected) =
            self.resolve_member_subject(origin, receiver_node, receiver, written_receiver)?;
        if let Some(rejected) = rejected
            && !is_optional
        {
            self.report_possibly_nullish(origin, rejected.label().to_string())?;
        }

        // retain the lookup subject at this source site
        self.module_mut(module)
            .members_tail
            .record_subject(dir::MemberSite::Node(node), subject);

        // commit an error for an omitted member name
        let Some(name) = name else {
            self.commit_decision(node, dir::Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(());
        };

        // look the written key up and commit whatever it finds
        let key = dir::StaticKey::Name(name);
        let mut lookup = self.lookup_member(origin, module, subject, key)?;
        self.adjust_narrowed_lookup(origin, receiver, subject.target, &mut lookup)?;

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
        // select the readable resolution, or report why the read fails
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

        // narrow the read through the flow state at this site
        let site = self.visit_site(node)?;
        let ty = self.flow_type_at(site, ty)?;
        self.commit_node_type(node, ty)?;

        Ok(())
    }

    /// Defer one member access until its receiver value settles, committing an open hole.
    fn defer_member_selection(
        &mut self,
        site: FlowSite,
        stalled_on: dir::TypeVariableId,
    ) -> CompilerResult<()> {
        // commit an open hole so enclosing checks proceed
        let node = site.node;
        if self.committed_node_type(node).is_none() {
            let variable =
                self.allocate_variable(site.origin(), Widening::Never, VariableRole::Regular);
            let hole = self.variable_type(variable)?;
            self.commit_node_type(node, hole)?;
        }

        // re-select once the receiver's variable solves
        self.check.register_check(Check::Selection(SelectionCheck {
            site,
            use_: PlaceUse::Read,
            stalled_on: Some(stalled_on),
        }));

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
        // poison when the receiver already reported an error
        if self.any_error_operand(&[receiver])? {
            self.poison_node(node)?;

            return Ok(());
        }

        // report the missing member at the written key
        let key_span = self.module(node.module_id).diagnostic_span(node.local_id);
        self.report_missing_member(origin, receiver, key, key_span)?;
        self.commit_decision(node, dir::Decision::Rejected)?;
        self.commit_error_node(node)?;

        Ok(())
    }
}

impl BodyState<'_, '_> {
    /// Return the use-site type projected by one declaration member.
    pub(in crate::sema) fn projected_member_type(
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

        // resolve the field in the receiver's place
        let ty = self.project_member_place(origin, receiver, ty)?;

        // readonly receivers project deep readonly views onto stored fields
        if !self.receiver_projects_readonly(receiver)? {
            return Ok(ty);
        }
        if matches!(self.ty(ty)?, dir::Type::Form(form) if form.form == dir::Form::Readonly) {
            return Ok(ty);
        }
        if !self.type_projects_readonly(origin, ty)? {
            return Ok(ty);
        }

        // wrap the projected field in the readonly view
        let projected = self.intern_type(dir::Type::Form(dir::FormType {
            form: dir::Form::Readonly,
            value: ty,
        }))?;

        self.normalize(origin, projected)
    }

    /// Return whether one receiver projects stored fields as readonly.
    pub(in crate::sema) fn receiver_projects_readonly(
        &mut self,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let mut current = receiver;

        loop {
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
                && self.access_is_readonly(access)?
            {
                return Ok(true);
            }

            current = form.value;
        }
    }

    /// Project one type-level member access through its owner.
    pub(in crate::sema) fn project_member(
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
        let module = self.module_id;
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
        let module = self.module_id;
        let (interface_module, interface) = self.nominal_application(qualifier)?;
        let owner = member.owner;

        // project a declaring class scope's own associated member
        let scope = interface.symbol;
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
        let apparent = self.intern_apparent_type(owner)?;
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

            // read the extension target, interfaces, and members
            let target_type = extension.target.r#type();
            let interfaces = extension
                .implements
                .iter()
                .map(|conformance| conformance.interface)
                .collect::<SmallVec<[_; 2]>>();
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
                    &interfaces,
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
                &interfaces,
            )?;
            let Some((substitution, _)) = matched else {
                continue;
            };

            // project the extension's own declared value, falling back to the interface default
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
            let interfaces = definition
                .implementations()
                .iter()
                .map(|conformance| conformance.interface)
                .collect::<SmallVec<[_; 2]>>();
            let members = definition.members().to_vec();

            if !interfaces.is_empty() {
                let mut substitution =
                    self.instance_substitution(application_module, &application)?;
                let matched = self.match_implemented_interface(
                    origin,
                    Relation::Assignable,
                    interface_module,
                    &[],
                    &mut substitution,
                    &interfaces,
                    &interface,
                )?;
                if matched.is_some() {
                    // project the owner's own declared value, falling back to the interface default
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

        // bind an extension scope's parameters through its matched target
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

        // apply the implementation's arguments to the declared value
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
                        return candidate.read_type(self);
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

                    // static values project as their own singleton
                    if let Some(value) = candidate.value {
                        let ty = self.intern_type(dir::Type::Static(value))?;

                        return Ok(Some(ty));
                    }

                    // keep a member projecting back onto itself symbolic
                    if let Some(projected) = self.member_head(candidate.access_type)?
                        && projected.owner == member.owner
                        && projected.key == member.key
                    {
                        return Ok(None);
                    }

                    candidate.read_type(self)
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

        // read the interface holding the default from a qualified projection only
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

        // apply the interface's arguments to the default
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
    pub(in crate::sema) fn is_rigid_projection_owner(
        &self,
        owner: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let owner = self.shallow_resolve(owner)?;
        let is_rigid = matches!(self.ty(owner)?, dir::Type::Parameter(_) | dir::Type::This);

        Ok(is_rigid)
    }

    /// Select the unique interface application declaring one associated member.
    pub(in crate::sema) fn select_associated_qualifier(
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
        // other owners select no qualifier
        else {
            return Ok(None);
        }

        // retain only interfaces declaring this associated member
        let mut qualifier = None;
        for interface in interfaces {
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

            // two declaring interfaces leave the projection ambiguous
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
    pub(in crate::sema) fn projection_constraint(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let owner = self.strip_form(origin, member.owner)?;
        let dir::Type::Parameter(parameter) = self.ty(owner)? else {
            return Ok(None);
        };

        for bound in self.parameter_bounds(origin, parameter)? {
            // read projected members from interface bounds only
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
        let Some(place) = self.receiver_projected_place(receiver)? else {
            return Ok(ty);
        };

        let ty = self.place_relative_type(origin, place, ty)?;

        self.normalize(origin, ty)
    }

    /// Resolve one relative member type in a projected receiver place.
    pub(in crate::sema) fn place_relative_type(
        &mut self,
        origin: Origin,
        place: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // bare members of local receivers stay bare
        if self.check.place_space(place)? == Some(dir::Space::Local) {
            return Ok(ty);
        }

        self.resolve_relative_place(origin, ty, place)
    }

    /// Return the place projected by one receiver type.
    pub(in crate::sema) fn receiver_projected_place(
        &mut self,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut current = receiver;
        loop {
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
        // retain readonly access over every safe reference carrier
        if self.type_is_reference(origin, ty)? {
            return Ok(true);
        }

        // project no view from a copied value, keeping the view while the copy is undecided
        if self
            .satisfies_auto_interface(origin, ty, dir::AutoInterface::Copy)?
            .holds()
        {
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
    pub(in crate::sema) fn access_is_readonly(
        &mut self,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let is_readonly = matches!(
            self.ty(access)?,
            dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Readonly))
        );

        Ok(is_readonly)
    }
}
