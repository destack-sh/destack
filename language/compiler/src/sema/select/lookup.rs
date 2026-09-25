use smallvec::SmallVec;
use tspp_core::{FxIndexSet, NameMatch, find_best_match};
use tspp_dir as dir;
use tspp_dir::{MemberRole, TypeFold};
use tspp_source::ModuleId;

use crate::sema::{
    ApparentInstance, CheckState, ExtensionHead, GenericParameterId, Origin, Relation,
    RelationCheck, TypeSubstitution, Verdict,
};
use crate::{CompilerError, CompilerResult, diagnostic_suggestion_distance};

/// Every member one lookup found on a receiver, empty when the receiver has none.
#[derive(Debug, Clone, Default)]
pub(in crate::sema) struct MemberLookup {
    /// The candidates in lookup order.
    pub(in crate::sema) candidates: Vec<MemberCandidate>,
}

/// The candidates one runtime arm of a lookup reads through.
pub(in crate::sema) struct MemberArmGroup<'lookup> {
    /// The runtime arm, absent for the receiver itself.
    pub(in crate::sema) arm: Option<MemberArm>,
    /// The candidates found on that arm, arm-less ones included.
    pub(in crate::sema) candidates: Vec<&'lookup MemberCandidate>,
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
    /// How the member behaves at a use site.
    pub(in crate::sema) role: MemberRole,
    /// The member kind.
    pub(in crate::sema) kind: dir::MemberKind,
    /// Whether the member accepts writes.
    pub(in crate::sema) is_writable: bool,
    /// Whether the member may be absent.
    pub(in crate::sema) is_optional: bool,
}

impl DeclaredMember {
    /// Read one definition member, carrying the type its declaration writes.
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
                message: format!("a keyed definition member {member:?} without a symbol"),
            });
        };

        Ok(Some(Self {
            symbol,
            type_symbol: member.type_symbol(),
            key,
            space: member.space(),
            ty: member.value_type(),
            role,
            kind: member.kind(),
            is_writable: member.is_writable(),
            is_optional: member.is_optional(),
        }))
    }

    /// Return the operations this member exposes over its substituted type.
    pub(in crate::sema) fn access(
        &self,
        check: &mut CheckState<'_>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::PropertyAccess> {
        let access = check.property_access(self.role, ty, !self.is_writable)?;

        Ok(access.unwrap_or(dir::PropertyAccess::Read(ty)))
    }

    /// Return the callable type exposed by this declaration member.
    pub(in crate::sema) fn callable_type(
        &self,
        ty: dir::GlobalTypeId,
    ) -> Option<dir::GlobalTypeId> {
        self.role.is_callable().then_some(ty)
    }
}

/// One member found on a receiver, with the runtime arm it reads through.
#[derive(Debug, Clone)]
pub(in crate::sema) struct MemberCandidate {
    /// What the candidate reads.
    pub(in crate::sema) source: CandidateSource,
    /// The member space that selected this candidate.
    pub(in crate::sema) space: dir::MemberSpace,
    /// How the member behaves at a use site.
    pub(in crate::sema) role: MemberRole,
    /// The member kind.
    pub(in crate::sema) kind: dir::MemberKind,
    /// The projected operations, optional widening excluded.
    pub(in crate::sema) access: dir::PropertyAccess,
    /// The substituted callable type for methods and accessors.
    pub(in crate::sema) callable: Option<dir::GlobalTypeId>,
    /// Whether the member may be absent.
    pub(in crate::sema) is_optional: bool,
    /// The receiver adjustments selected during lookup.
    pub(in crate::sema) receiver: LookupReceiver,
    /// The runtime union arm the candidate reads through.
    pub(in crate::sema) arm: Option<MemberArm>,
}

/// What one member candidate reads.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub(in crate::sema) enum CandidateSource {
    /// One declaration member.
    Declared(DeclaredSource),
    /// One field a structural aggregate declares.
    Structural {
        /// The aggregate that declares the field.
        owner: dir::GlobalTypeId,
    },
    /// One compiler-defined field projection.
    Projection(dir::Projection),
}

/// One declaration member selected by lookup.
#[derive(Debug, Clone)]
pub(in crate::sema) struct DeclaredSource {
    /// The declaring member symbol.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
    /// The declaration that exposed the member.
    pub(in crate::sema) owner: dir::GlobalSymbolId,
    /// The declaration family the member came from.
    pub(in crate::sema) origin: dir::MemberOrigin,
    /// The interface whose requirement the member implements, when it does.
    pub(in crate::sema) requirement: Option<dir::GlobalSymbolId>,
    /// The generic arguments matched through the owner.
    pub(in crate::sema) generic_arguments: Vec<dir::GenericArgumentBinding>,
    /// The region arguments matched through the owner, erased from the instance key.
    pub(in crate::sema) region_arguments: Vec<dir::GenericArgumentBinding>,
    /// The member static value when it carries one.
    pub(in crate::sema) value: Option<dir::GlobalStaticId>,
    /// The substituted static value of the member, when it has one.
    pub(in crate::sema) value_type: Option<dir::GlobalTypeId>,
    /// The declared bounds the winning candidate writes at its site.
    pub(in crate::sema) bounds: Vec<RelationCheck>,
    /// The declared target the winning candidate's receiver satisfies at its site.
    pub(in crate::sema) target: Option<RelationCheck>,
    /// The erased owner parameters selecting sites reopen, each with its site default.
    pub(in crate::sema) site_parameters:
        SmallVec<[(GenericParameterId, Option<dir::GlobalTypeId>); 2]>,
}

impl DeclaredSource {
    /// Create a declared source over one owner with the arguments matched through it.
    pub(in crate::sema) fn new(
        symbol: dir::GlobalSymbolId,
        owner: dir::GlobalSymbolId,
        origin: dir::MemberOrigin,
        generic_arguments: Vec<dir::GenericArgumentBinding>,
    ) -> Self {
        Self {
            symbol,
            owner,
            origin,
            requirement: None,
            generic_arguments,
            region_arguments: Vec::new(),
            value: None,
            value_type: None,
            bounds: Vec::new(),
            target: None,
            site_parameters: SmallVec::new(),
        }
    }

    /// Return the declaration block this source answers for: its requirement or its owner.
    pub(in crate::sema) fn declaring_block(&self) -> dir::GlobalSymbolId {
        self.requirement.unwrap_or(self.owner)
    }
}

/// One runtime union arm a member candidate reads through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) struct MemberArm {
    /// The union element the arm selects.
    pub(in crate::sema) element: dir::GlobalTypeId,
    /// The receiver narrowed to that element.
    pub(in crate::sema) receiver: dir::GlobalTypeId,
}

/// Receiver state kept by one member lookup candidate.
#[derive(Debug, Clone)]
pub(in crate::sema) enum LookupReceiver {
    /// Direct adjustments attached to the use-site receiver on resolution.
    Direct(Vec<dir::ReceiverAdjustment>),
    /// Erased receiver selected for dynamic dispatch.
    Dynamic {
        /// The receiver adjustments applied before dispatch.
        adjustments: Vec<dir::ReceiverAdjustment>,
        /// The interface constraint declaring the dispatch member.
        constraint: dir::GlobalTypeId,
    },
}

impl LookupReceiver {
    /// Return the adjustments applied before reaching the member.
    fn adjustments(&self) -> &[dir::ReceiverAdjustment] {
        match self {
            Self::Direct(steps)
            | Self::Dynamic {
                adjustments: steps, ..
            } => steps,
        }
    }

    /// Prepend one adjustment performed before the existing adjustments.
    fn prepend(&mut self, adjustment: dir::ReceiverAdjustment) {
        match self {
            Self::Direct(steps) => steps.insert(0, adjustment),
            Self::Dynamic { adjustments, .. } => adjustments.insert(0, adjustment),
        }
    }

    /// Drop the leading adjustments.
    fn strip(&mut self, count: usize) {
        match self {
            Self::Direct(steps)
            | Self::Dynamic {
                adjustments: steps, ..
            } => {
                steps.drain(..count);
            }
        }
    }

    /// Attach this lookup state to its use-site receiver.
    pub(in crate::sema) fn resolve(&self, source: dir::GlobalTypeId) -> dir::MemberReceiver {
        // attach the steps each lookup receiver carries
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
    /// Create a declared candidate reading directly through the use-site receiver.
    pub(in crate::sema) fn declared(
        declared: DeclaredSource,
        member: &DeclaredMember,
        access: dir::PropertyAccess,
        callable: Option<dir::GlobalTypeId>,
    ) -> Self {
        Self {
            source: CandidateSource::Declared(declared),
            space: member.space,
            role: member.role,
            kind: member.kind,
            access,
            callable,
            is_optional: member.is_optional,
            receiver: LookupReceiver::Direct(Vec::new()),
            arm: None,
        }
    }

    /// Create a structural field candidate over one aggregate.
    pub(in crate::sema) fn structural(
        owner: dir::GlobalTypeId,
        access: dir::PropertyAccess,
        is_optional: bool,
    ) -> Self {
        Self {
            source: CandidateSource::Structural { owner },
            space: dir::MemberSpace::Instance,
            role: MemberRole::Field,
            kind: dir::MemberKind::Field,
            access,
            callable: None,
            is_optional,
            receiver: LookupReceiver::Direct(Vec::new()),
            arm: None,
        }
    }

    /// Create a candidate reading one compiler-defined projection.
    pub(in crate::sema) fn projection(projection: dir::Projection) -> Self {
        Self {
            access: dir::PropertyAccess::Read(projection.ty()),
            source: CandidateSource::Projection(projection),
            space: dir::MemberSpace::Instance,
            role: MemberRole::Field,
            kind: dir::MemberKind::Field,
            callable: None,
            is_optional: false,
            receiver: LookupReceiver::Direct(Vec::new()),
            arm: None,
        }
    }

    /// Return the declaration this candidate reads, when it reads one.
    pub(in crate::sema) fn declaration(&self) -> Option<&DeclaredSource> {
        match &self.source {
            CandidateSource::Declared(declared) => Some(declared),
            CandidateSource::Structural { .. } | CandidateSource::Projection(_) => None,
        }
    }

    /// Return the declaration this candidate reads, mutably.
    pub(in crate::sema) fn declaration_mut(&mut self) -> Option<&mut DeclaredSource> {
        match &mut self.source {
            CandidateSource::Declared(declared) => Some(declared),
            CandidateSource::Structural { .. } | CandidateSource::Projection(_) => None,
        }
    }

    /// Return the declaring symbol, when the candidate reads a declaration.
    pub(in crate::sema) fn symbol(&self) -> Option<dir::GlobalSymbolId> {
        self.declaration().map(|declared| declared.symbol)
    }

    /// Return whether two candidates read one member.
    pub(in crate::sema) fn reads_same(&self, other: &Self) -> bool {
        // compare the two candidates by their sources
        match (&self.source, &other.source) {
            (CandidateSource::Declared(left), CandidateSource::Declared(right)) => {
                left.symbol == right.symbol
            }
            (
                CandidateSource::Structural { owner: left },
                CandidateSource::Structural { owner: right },
            ) => left == right,
            _ => false,
        }
    }

    /// Reopen this candidate's erased owner parameters at one use site.
    pub(in crate::sema) fn instantiate(
        &self,
        origin: Origin,
        body: &mut CheckState<'_>,
    ) -> CompilerResult<MemberCandidate> {
        // hand candidates without erased owner parameters through unchanged
        let mut candidate = self.clone();
        let Some(declared) = candidate.declaration_mut() else {
            return Ok(candidate);
        };
        let parameters = std::mem::take(&mut declared.site_parameters);

        // open one site variable per erased parameter
        let mut opened =
            SmallVec::<[(dir::TypeVariableId, dir::GlobalTypeId, dir::GlobalTypeId); 2]>::new();
        for (parameter, _) in &parameters {
            let variable = body.open_omitted_parameter(origin, *parameter)?;
            let fresh = body.variable_type(variable)?;
            let from = body.intern_type(dir::Type::Erased(*parameter))?;
            candidate.map_types(&mut |ty| body.replace_type(ty, from, fresh))?;
            opened.push((variable, from, fresh));
        }

        // default each reopened parameter as its site declared, over the fresh siblings
        for ((_, default), (variable, _, _)) in parameters.iter().zip(&opened) {
            let Some(mut default) = *default else {
                continue;
            };
            for (_, from, fresh) in &opened {
                default = body.replace_type(default, *from, *fresh)?;
            }
            body.set_variable_default(*variable, default)?;
        }

        Ok(candidate)
    }

    /// Apply this candidate's declared bounds and receiver requirement.
    pub(in crate::sema) fn constrain(&self, check: &mut CheckState<'_>) -> CompilerResult<()> {
        let Some(declared) = self.declaration() else {
            return Ok(());
        };

        // schedule the selected declaration's bounds
        for constraint in &declared.bounds {
            check.push_relation(*constraint)?;
        }

        // constrain the receiver against its declared target
        if let Some(target) = declared.target {
            check.constrain_type(
                target.origin,
                target.cause,
                target.relation,
                target.source,
                target.target,
            )?;
        }

        Ok(())
    }

    /// Return this candidate's selection precedence.
    pub(in crate::sema) fn precedence(&self) -> (bool, dir::MemberOrigin, bool) {
        let is_adjusted = match &self.receiver {
            LookupReceiver::Direct(steps) => !steps.is_empty(),
            LookupReceiver::Dynamic { .. } => true,
        };
        let (is_requirement, origin) = match self.declaration() {
            Some(declared) => (declared.requirement.is_some(), declared.origin),
            None => (false, dir::MemberOrigin::Declaration),
        };

        // rank inherent members before interface requirements
        (is_requirement, origin, is_adjusted)
    }

    /// Return the type produced by reading this candidate.
    pub(in crate::sema) fn read_type(
        &self,
        body: &mut CheckState<'_>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(read) = self.access.read() else {
            return Ok(None);
        };
        if !self.is_optional {
            return Ok(Some(read));
        }

        // read an optional member as its type or undefined
        let undefined = body.intern_type(dir::Type::Undefined)?;
        let ty = body.normalized_union_type([read, undefined])?;

        Ok(Some(ty))
    }

    /// Return the durable member candidate for this lookup candidate.
    pub(in crate::sema) fn resolution_candidate(
        &self,
        declared: &DeclaredSource,
        receiver: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
    ) -> dir::MemberCandidate {
        dir::MemberCandidate {
            receiver: self.receiver.resolve(receiver),
            space: self.space,
            owner: declared.owner,
            access_type: ty,
            callable_type: self.callable,
            key: dir::InstanceKey::new(declared.symbol, declared.generic_arguments.clone()),
            regions: declared.region_arguments.clone(),
        }
    }

    /// Return the durable member access reading this candidate.
    pub(in crate::sema) fn access(
        &self,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        ty: dir::GlobalTypeId,
    ) -> dir::MemberAccess {
        // name the target each candidate source resolves to
        let target = match &self.source {
            CandidateSource::Declared(declared) => match self.field(key) {
                Some(target) => dir::MemberTarget::Field(dir::FieldResolution {
                    receiver: self.receiver.resolve(receiver),
                    target,
                    ty,
                }),
                None => {
                    dir::MemberTarget::Symbol(self.resolution_candidate(declared, receiver, ty))
                }
            },
            CandidateSource::Structural { owner } => {
                dir::MemberTarget::Field(dir::FieldResolution {
                    receiver: self.receiver.resolve(receiver),
                    target: dir::FieldTarget::Structural { owner: *owner, key },
                    ty,
                })
            }
            CandidateSource::Projection(projection) => dir::MemberTarget::Projection {
                key,
                receiver: dir::AdjustedReceiver {
                    source: receiver,
                    adjustments: self.receiver.adjustments().to_vec(),
                },
                projection: projection.clone(),
            },
        };

        dir::MemberAccess::new(receiver, target, ty)
    }

    /// Return the value projection reading this candidate.
    pub(in crate::sema) fn projected(
        &self,
        source: dir::GlobalTypeId,
        key: dir::StaticKey,
        ty: dir::GlobalTypeId,
    ) -> dir::Projection {
        match self.access(source, key, ty).target {
            dir::MemberTarget::Field(field) => dir::Projection::Field(field),
            target => dir::Projection::Member(Box::new(dir::MemberAccess::new(source, target, ty))),
        }
    }

    /// Return the stored field selected by this candidate.
    pub(in crate::sema) fn field(&self, key: dir::StaticKey) -> Option<dir::FieldTarget> {
        if self.role != MemberRole::Field {
            return None;
        }

        Some(match &self.source {
            CandidateSource::Declared(declared) => dir::FieldTarget::Member {
                symbol: declared.symbol,
                key,
            },
            CandidateSource::Structural { owner } => {
                dir::FieldTarget::Structural { owner: *owner, key }
            }
            CandidateSource::Projection(_) => return None,
        })
    }

    /// Prepend one implicit adjustment to the selected receiver.
    pub(in crate::sema) fn prepend_adjustment(&mut self, adjustment: dir::ReceiverAdjustment) {
        self.receiver.prepend(adjustment);
    }

    /// Drop the leading adjustments performed before member selection.
    pub(in crate::sema) fn strip_adjustments(&mut self, count: usize) {
        self.receiver.strip(count);
    }
}

impl MemberLookup {
    /// Return whether the lookup found no member.
    pub(in crate::sema) fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    /// Group the candidates by the runtime arm they read through, arm-less ones joining every arm.
    pub(in crate::sema) fn arms(&self) -> Vec<MemberArmGroup<'_>> {
        let mut arms = Vec::<MemberArm>::new();
        for candidate in &self.candidates {
            if let Some(arm) = candidate.arm
                && !arms.contains(&arm)
            {
                arms.push(arm);
            }
        }
        if arms.is_empty() {
            return vec![MemberArmGroup {
                arm: None,
                candidates: self.candidates.iter().collect(),
            }];
        }

        // group the candidates by the runtime arm they read
        arms.into_iter()
            .map(|arm| MemberArmGroup {
                arm: Some(arm),
                candidates: self
                    .candidates
                    .iter()
                    .filter(|candidate| candidate.arm.is_none_or(|own| own == arm))
                    .collect(),
            })
            .collect()
    }

    /// Return the member kind the selected candidates share.
    pub(in crate::sema) fn kind(&self) -> CompilerResult<dir::MemberKind> {
        let mut kinds = Vec::new();
        for group in self.arms() {
            kinds.extend(group.selected().into_iter().map(|candidate| candidate.kind));
        }
        kinds.sort();
        kinds.dedup();

        // join the collected kinds
        match kinds.as_slice() {
            [] => Err(CompilerError::Internal {
                message: "a member kind over a lookup without selected candidates".to_string(),
            }),
            [kind] => Ok(*kind),
            _ => Ok(dir::MemberKind::Property),
        }
    }

    /// Return whether some runtime arm may lack the member.
    pub(in crate::sema) fn is_optional(&self) -> bool {
        self.arms().iter().any(MemberArmGroup::is_optional)
    }

    /// Prepend one implicit adjustment to every candidate's receiver.
    pub(in crate::sema) fn prepend_adjustment(&mut self, adjustment: &dir::ReceiverAdjustment) {
        for candidate in &mut self.candidates {
            candidate.prepend_adjustment(adjustment.clone());
        }
    }

    /// Select every candidate through one erased receiver.
    pub(in crate::sema) fn select_dynamic(
        &mut self,
        constraint: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        for candidate in &mut self.candidates {
            // read a compiler-defined field projection's union representation directly
            if matches!(candidate.source, CandidateSource::Projection(_)) {
                return Err(CompilerError::Internal {
                    message: "compiler-defined field projection selected dynamic dispatch"
                        .to_string(),
                });
            }
            candidate.receiver = LookupReceiver::Dynamic {
                adjustments: Vec::new(),
                constraint,
            };
        }

        Ok(())
    }
}

impl From<Vec<MemberCandidate>> for MemberLookup {
    fn from(candidates: Vec<MemberCandidate>) -> Self {
        Self { candidates }
    }
}

impl<'lookup> MemberArmGroup<'lookup> {
    /// Return the candidates lookup precedence ranks first.
    pub(in crate::sema) fn selected(&self) -> Vec<&'lookup MemberCandidate> {
        let best = self
            .candidates
            .iter()
            .map(|candidate| candidate.precedence())
            .min();

        // keep the candidates at the best precedence
        self.candidates
            .iter()
            .copied()
            .filter(|candidate| Some(candidate.precedence()) == best)
            .collect()
    }

    /// Return the candidates one read joins, the first field or accessor else every method.
    pub(in crate::sema) fn reads(&self) -> Vec<&'lookup MemberCandidate> {
        let candidates = self.selected();
        let first_field = candidates
            .iter()
            .position(|candidate| candidate.role != MemberRole::Method);

        // keep the first field or accessor
        match first_field {
            Some(first) => vec![candidates[first]],
            None => candidates,
        }
    }

    /// Return whether every selected candidate of this arm is optional.
    pub(in crate::sema) fn is_optional(&self) -> bool {
        let selected = self.selected();

        !selected.is_empty() && selected.iter().all(|candidate| candidate.is_optional)
    }

    /// Return the required field type this arm reads directly.
    pub(in crate::sema) fn direct_field_type(&self) -> Option<dir::GlobalTypeId> {
        let selected = self.selected();
        let [candidate] = selected.as_slice() else {
            return None;
        };
        if candidate.role != MemberRole::Field
            || candidate.is_optional
            || matches!(candidate.source, CandidateSource::Projection(_))
            || !matches!(candidate.receiver, LookupReceiver::Direct(_))
        {
            return None;
        }

        candidate.access.read()
    }
}

/// One member lookup on the active recursion path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct MemberLookupKey {
    /// The module whose visibility rules apply.
    module: ModuleId,
    /// The receiver type kept in selected resolutions.
    receiver: dir::GlobalTypeId,
    /// The type currently queried for matching members.
    subject: dir::GlobalTypeId,
    /// The member namespace.
    space: dir::MemberSpace,
    /// The member key.
    key: dir::StaticKey,
    /// Whether extension members are admitted.
    extensions: ExtensionFilter,
}

/// Which member sources one lookup admits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum ExtensionFilter {
    /// Exclude extension members.
    Exclude,
    /// Include extension members.
    Include,
}

impl CheckState<'_> {
    /// Collect the declared member keys one lookup subject exposes.
    pub(in crate::sema) fn subject_member_keys(
        &mut self,
        origin: Origin,
        module: ModuleId,
        subject: &dir::MemberSubject,
    ) -> CompilerResult<Vec<dir::StaticKey>> {
        let mut keys = FxIndexSet::default();
        let mut visited = FxIndexSet::default();
        self.collect_subject_keys(
            origin,
            module,
            subject.key_source,
            subject.space,
            &mut keys,
            &mut visited,
        )?;

        Ok(keys.into_iter().collect())
    }

    /// Look one member up on a resolved subject.
    pub(in crate::sema) fn lookup_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        subject: dir::MemberSubject,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        self.counters.member_derivations += 1;

        // derive the lookup, guarding the recursion path it walks
        let mut active_queries = FxIndexSet::default();
        self.lookup_subject_member(
            origin,
            module,
            subject.receiver,
            subject.target,
            subject.space,
            key,
            ExtensionFilter::Include,
            &mut active_queries,
        )
    }

    /// Look up one inherent member on a receiver type.
    pub(in crate::sema) fn lookup_inherent_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let mut active_queries = FxIndexSet::default();

        // look the key up from the receiver itself
        self.lookup_subject_member(
            origin,
            module,
            receiver,
            receiver,
            space,
            key,
            ExtensionFilter::Exclude,
            &mut active_queries,
        )
    }

    /// Look up one member on a receiver type through every visible extension.
    pub(in crate::sema) fn lookup_visible_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let mut active_queries = FxIndexSet::default();
        self.lookup_subject_member(
            origin,
            module,
            receiver,
            receiver,
            space,
            key,
            ExtensionFilter::Include,
            &mut active_queries,
        )
    }

    /// Look one member up in `subject` while keeping `receiver` for projection.
    fn lookup_subject_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionFilter,
        active: &mut FxIndexSet<MemberLookupKey>,
    ) -> CompilerResult<MemberLookup> {
        // look the key up in the bounds a where clause grants this subject
        let root = self.shallow_resolve(subject)?;
        let bounds = self.subject_predicate_bounds(origin, root)?;
        if !bounds.is_empty() {
            let lookup = self.lookup_bound_member(
                origin, module, receiver, &bounds, space, key, extensions, active,
            )?;
            if !lookup.is_empty() {
                return Ok(lookup);
            }
        }

        // read the declaration for static names through reference and application heads
        if space == dir::MemberSpace::Static {
            let named = match self.ty(root)? {
                dir::Type::Reference(reference) => Some((
                    reference.symbol,
                    SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(
                        self.type_ids(root.module_id, reference.arguments)?,
                    ),
                )),
                dir::Type::Application(instance) => Some((
                    instance.symbol,
                    SmallVec::from_slice(self.type_ids(root.module_id, instance.arguments)?),
                )),
                _ => None,
            };
            if let Some((symbol, arguments)) = named {
                return self.lookup_declaration_member(
                    origin,
                    module,
                    receiver,
                    dir::TypeReference::new(symbol),
                    &arguments,
                    space,
                    key,
                    extensions,
                    active,
                );
            }
        }

        // normalize assumed heads under the scope before resolving members
        let subject = self.normalize(origin, subject)?;

        // stop cyclic paths through constraints, unions, and heritage
        let query = MemberLookupKey {
            module,
            receiver,
            subject,
            space,
            key,
            extensions,
        };
        if !active.insert(query) {
            return Ok(MemberLookup::default());
        }

        // look the key up in the resolved subject, then leave the path
        let lookup = self.lookup_normalized_member(
            origin, module, receiver, subject, space, key, extensions, active,
        );
        active.swap_remove(&query);
        lookup
    }

    /// Return the bounds the assumed where clauses grant one composite subject, like `&'a T`.
    fn subject_predicate_bounds(
        &mut self,
        origin: Origin,
        subject: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        let mut bounds = SmallVec::new();
        if !self.type_flags(subject)?.has_parameter()
            || matches!(self.ty(subject)?, dir::Type::Parameter(_))
        {
            return Ok(bounds);
        }

        // take the right side of each satisfies clause whose left side equals the subject
        for predicate in self.assumed_predicates(origin)? {
            if predicate.relation != dir::WhereRelation::Satisfies {
                continue;
            }
            let left = self.shallow_resolve(predicate.left)?;
            let is_subject = left == subject
                || self.decide_relation(origin, Relation::Equal, left, subject)? == Verdict::Holds;
            if is_subject {
                bounds.push(predicate.right);
            }
        }

        Ok(bounds)
    }

    /// Look one member up by the shape of one normalized subject type.
    fn lookup_normalized_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionFilter,
        active: &mut FxIndexSet<MemberLookupKey>,
    ) -> CompilerResult<MemberLookup> {
        match self.ty(subject)? {
            // memory forms look through their payloads
            dir::Type::Form(form) => self.lookup_subject_member(
                origin, module, receiver, form.value, space, key, extensions, active,
            ),

            // refinements answer their refined member, then their base
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(subject.module_id, refined)?;
                if refined.key == key {
                    return Ok(vec![MemberCandidate::structural(
                        subject,
                        dir::PropertyAccess::Read(refined.value),
                        false,
                    )]
                    .into());
                }

                self.lookup_subject_member(
                    origin,
                    module,
                    receiver,
                    refined.base,
                    space,
                    key,
                    extensions,
                    active,
                )
            }

            // declaration references read static members
            dir::Type::Reference(reference) => {
                let arguments: SmallVec<[_; 4]> = self
                    .type_ids(subject.module_id, reference.arguments)?
                    .into();

                self.lookup_declaration_member(
                    origin, module, receiver, reference, &arguments, space, key, extensions, active,
                )
            }

            // precise enum variants expose their owner's apparent members
            dir::Type::Variant(_) => self.lookup_apparent_instance_member(
                origin, module, receiver, subject, space, key, extensions,
            ),

            // read the definition members of an applied declaration
            dir::Type::Application(_)
            | dir::Type::Literal(_)
            | dir::Type::Primitive(_)
            | dir::Type::Slice(_)
            | dir::Type::FixedArray(_)
            | dir::Type::Function(_)
            | dir::Type::FunctionSignature(_) => {
                // widen literal subjects to their representation before lookup
                let subject = match self.ty(subject)? {
                    dir::Type::Literal(literal) => self.intern_type(literal.widen())?,
                    _ => subject,
                };

                let mut lookup = self.lookup_apparent_instance_member(
                    origin, module, receiver, subject, space, key, extensions,
                )?;

                // dispatch erased value receivers through their dynamic payload
                if space == dir::MemberSpace::Instance
                    && self.is_erased_value(subject)?
                    && self.is_subject_receiver(origin, receiver, subject)?
                {
                    lookup.select_dynamic(subject)?;
                }

                // newtypes dereference to their backing for missing members
                if lookup.is_empty()
                    && let Some(instance) = self.decompose_newtype(origin, subject)?
                {
                    let value = instance.backing;
                    let receiver = self.replace_form_value(origin, receiver, value)?;
                    let adjustment = instance.into_receiver_adjustment(receiver);
                    let mut lookup = self.lookup_subject_member(
                        origin, module, receiver, value, space, key, extensions, active,
                    )?;

                    // keep the payload adjustment before deeper receiver steps
                    lookup.prepend_adjustment(&adjustment);

                    return Ok(lookup);
                }

                Ok(lookup)
            }

            // expose the static space of a type held in a static term
            dir::Type::Static(value) => {
                let term = self.r#static(value)?.clone();
                match term {
                    dir::StaticTerm::Type { ty } => self.lookup_subject_member(
                        origin,
                        module,
                        receiver,
                        ty,
                        dir::MemberSpace::Static,
                        key,
                        extensions,
                        active,
                    ),
                    _ => Ok(MemberLookup::default()),
                }
            }

            // generic parameters look through their bounds
            dir::Type::Parameter(parameter) => {
                let bounds = self.parameter_bounds(origin, parameter)?;

                self.lookup_bound_member(
                    origin, module, receiver, &bounds, space, key, extensions, active,
                )
            }

            // rigid projections look through their declared constraint
            dir::Type::Member(member) => {
                let member = self.type_member(subject.module_id, member)?;
                let bounds = self.projection_constraint(origin, &member)?;
                let bounds = bounds.into_iter().collect::<SmallVec<[_; 1]>>();

                self.lookup_bound_member(
                    origin, module, receiver, &bounds, space, key, extensions, active,
                )
            }

            // contextual this looks through its assumed bounds
            dir::Type::This => {
                let bounds = self.assumed_bounds(origin, |ty| matches!(ty, dir::Type::This))?;

                self.lookup_bound_member(
                    origin, module, receiver, &bounds, space, key, extensions, active,
                )
            }

            // erased values expose members through their dynamic payload
            dir::Type::Dynamic(dynamic) => {
                let constraint = dynamic.constraint;

                // look the key up in the erased constraint, then dispatch dynamically
                let constraint_receiver = self.replace_form_value(origin, receiver, constraint)?;
                let mut lookup = self.lookup_bound_member(
                    origin,
                    module,
                    constraint_receiver,
                    &[constraint],
                    space,
                    key,
                    ExtensionFilter::Exclude,
                    active,
                )?;
                lookup.select_dynamic(constraint)?;

                // extensions remain direct calls over the erased receiver
                if lookup.is_empty()
                    && extensions == ExtensionFilter::Include
                    && let Some(instance) = self.apparent_instance(constraint)?
                {
                    return self.lookup_extension_member(
                        origin,
                        module,
                        receiver,
                        constraint,
                        dir::TypeRoot::Declaration(instance.symbol),
                        space,
                        key,
                    );
                }

                Ok(lookup)
            }

            // structural shapes expose every operation for the selected key
            dir::Type::Object(shape) => {
                let properties = self.object_properties(subject.module_id, shape.properties)?;
                let properties = properties
                    .iter()
                    .filter(|property| property.key == key)
                    .copied()
                    .collect::<SmallVec<[_; 2]>>();
                if properties.is_empty() {
                    return Ok(MemberLookup::default());
                }

                // project each declared operation through the receiver
                let mut reads = SmallVec::<[dir::GlobalTypeId; 2]>::new();
                let mut writes = SmallVec::<[dir::GlobalTypeId; 2]>::new();
                let mut is_optional = true;
                for property in properties {
                    is_optional &= property.is_optional;

                    if let Some(read) = property.access.read() {
                        let read = self.projected_member_type(
                            origin,
                            Some(receiver),
                            MemberRole::Field,
                            read,
                        )?;
                        reads.push(read);
                    }

                    if let Some(write) = property.access.write() {
                        let write = self.projected_member_type(
                            origin,
                            Some(receiver),
                            MemberRole::Field,
                            write,
                        )?;
                        writes.push(write);
                    }
                }

                // several declarations of one key intersect into one operation
                let read = match reads.as_slice() {
                    [] => None,
                    [read] => Some(*read),
                    reads => Some(self.normalized_intersection_type(reads.iter().copied())?),
                };
                let write = match writes.as_slice() {
                    [] => None,
                    [write] => Some(*write),
                    writes => Some(self.normalized_intersection_type(writes.iter().copied())?),
                };

                let access = match (read, write) {
                    (Some(read), Some(write)) => dir::PropertyAccess::ReadWrite { read, write },
                    (Some(read), None) => dir::PropertyAccess::Read(read),
                    (None, Some(write)) => dir::PropertyAccess::Write(write),
                    (None, None) => {
                        return Err(CompilerError::Internal {
                            message: format!("a structural property {key:?} without an operation"),
                        });
                    }
                };

                Ok(vec![MemberCandidate::structural(subject, access, is_optional)].into())
            }

            // tuples expose their labeled elements, then their extension members
            dir::Type::Tuple(tuple) => {
                let element = self
                    .tuple_elements(subject.module_id, tuple.elements)?
                    .iter()
                    .find(|element| {
                        element
                            .label
                            .is_some_and(|label| key == dir::StaticKey::Name(label))
                    })
                    .copied();

                let Some(element) = element else {
                    return self.lookup_apparent_instance_member(
                        origin, module, receiver, subject, space, key, extensions,
                    );
                };

                // readonly elements expose the read operation alone
                let access_type = self.projected_member_type(
                    origin,
                    Some(receiver),
                    MemberRole::Field,
                    element.ty,
                )?;
                let access = if element.is_readonly {
                    dir::PropertyAccess::Read(access_type)
                } else {
                    dir::PropertyAccess::ReadWrite {
                        read: access_type,
                        write: access_type,
                    }
                };

                Ok(vec![MemberCandidate::structural(
                    subject,
                    access,
                    element.is_optional,
                )]
                .into())
            }

            // unions join member lookups across their elements, else derive over the whole union
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(subject.module_id, union.elements)?.into();
                let lookup = self.lookup_union_member(
                    origin, module, receiver, subject, &elements, space, key, extensions, active,
                )?;
                if !lookup.is_empty() {
                    return Ok(lookup);
                }

                self.lookup_derived_member(origin, subject, space, key)
            }

            // intersections expose each element's members
            dir::Type::Intersection(intersection) => {
                let elements: SmallVec<[_; 8]> = self
                    .type_ids(subject.module_id, intersection.elements)?
                    .into();
                let mut candidates = Vec::new();
                for element in elements.iter().copied() {
                    let element = self.shallow_resolve(element)?;
                    let mut lookup = self.lookup_subject_member(
                        origin, module, receiver, element, space, key, extensions, active,
                    )?;
                    if lookup.is_empty() {
                        continue;
                    }

                    // read a member of a kept union arm through the representation beside it
                    let arm = self.replace_form_value(origin, receiver, element)?;
                    for representation in elements.iter().copied() {
                        if representation == element {
                            continue;
                        }
                        let representation =
                            self.replace_form_value(origin, receiver, representation)?;
                        if let Some(steps) =
                            self.project_carried_arm(origin, representation, arm)?
                        {
                            for adjustment in steps.into_iter().rev() {
                                lookup.prepend_adjustment(&adjustment);
                            }
                            break;
                        }
                    }

                    candidates.extend(lookup.candidates);
                }

                Ok(candidates.into())
            }

            // answer with an empty candidate list for every other subject shape
            _ => Ok(MemberLookup::default()),
        }
    }

    /// Look up one member on a receiver known only by its bounds.
    fn lookup_bound_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        bounds: &[dir::GlobalTypeId],
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionFilter,
        active: &mut FxIndexSet<MemberLookupKey>,
    ) -> CompilerResult<MemberLookup> {
        for bound in bounds {
            let lookup = self.lookup_subject_member(
                origin, module, receiver, *bound, space, key, extensions, active,
            )?;
            if !lookup.is_empty() {
                return Ok(lookup);
            }
        }

        Ok(MemberLookup::default())
    }

    /// Look up one member through the receiver's apparent declaration instance.
    fn lookup_apparent_instance_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_type: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionFilter,
    ) -> CompilerResult<MemberLookup> {
        // consult the extensions at a structural subject's constructor, then its derived members
        let Some(instance) = self.apparent_instance(lookup_type)? else {
            let value = self.strip_form(origin, lookup_type)?;
            if extensions == ExtensionFilter::Include
                && let Some(root) = self.structural_root(value)?
            {
                let lookup = self.lookup_extension_member(
                    origin,
                    module,
                    receiver,
                    lookup_type,
                    root,
                    space,
                    key,
                )?;
                if !lookup.is_empty() {
                    return Ok(lookup);
                }
            }
            return self.lookup_derived_member(origin, value, space, key);
        };

        // look the key up on the named declaration
        self.lookup_symbol_member(
            origin,
            module,
            receiver,
            lookup_type,
            instance,
            space,
            key,
            extensions,
        )
    }

    /// Look up one static member on a declaration reference.
    fn lookup_declaration_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        reference: dir::TypeReference,
        mut arguments: &[dir::GlobalTypeId],
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionFilter,
        active: &mut FxIndexSet<MemberLookupKey>,
    ) -> CompilerResult<MemberLookup> {
        if space != dir::MemberSpace::Static {
            return Ok(MemberLookup::default());
        }

        // read the declaration the reference names
        let mut symbol = reference.symbol;

        // name a type alias's root declaration for statics, keeping the body for its own members
        let mut alias_body = None;
        if let Some(dir::Definition::TypeAlias(alias)) = self.definition(symbol)?.as_deref() {
            let body = alias.value;
            alias_body = Some(body);
            if let Some(named) = self.type_symbol(body)? {
                symbol = named;
                arguments = &[];
            }
        }

        // search inherent members first
        let is_alias = self.symbol_kind(symbol)?.is_type_alias();
        if !is_alias {
            let instance = ApparentInstance {
                symbol,
                arguments: arguments.iter().copied().collect(),
            };
            let inherent =
                self.lookup_inherent_symbol_member(origin, receiver, &instance, space, key)?;
            if !inherent.is_empty() {
                return Ok(inherent);
            }
        }

        // search extensions next: declared members shadow interface defaults
        let lookup = match extensions {
            ExtensionFilter::Include => {
                self.lookup_static_extension_member(origin, module, symbol, arguments, key)?
            }
            ExtensionFilter::Exclude => MemberLookup::default(),
        };

        // search the declaration's own interfaces when no extension matched
        if lookup.is_empty() && !is_alias {
            // search associated members through their declaring interface last
            let associated = self.lookup_associated_member(origin, receiver, space, key)?;
            if !associated.is_empty() {
                return Ok(associated);
            }

            // search the statics of the auto interfaces the declaration derives
            let instance = self.declaration_instance(reference.symbol)?;
            let subject = self.intern_type(dir::Type::Application(instance))?;
            let derived = self.lookup_derived_member(origin, subject, space, key)?;
            if !derived.is_empty() {
                return Ok(derived);
            }
        }

        // look the key up in the alias body when the root declaration missed it
        if lookup.is_empty()
            && let Some(body) = alias_body
        {
            return self.lookup_subject_member(
                origin, module, receiver, body, space, key, extensions, active,
            );
        }

        Ok(lookup)
    }

    /// Return whether one receiver's own base value is the subject.
    fn is_subject_receiver(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let receiver_base = self.form_chain(origin, receiver)?.base();
        let receiver_base = self.normalize(origin, receiver_base)?;

        Ok(receiver_base == self.normalize(origin, subject)?)
    }

    /// Join member lookups across union elements.
    fn lookup_union_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        elements: &[dir::GlobalTypeId],
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionFilter,
        active: &mut FxIndexSet<MemberLookupKey>,
    ) -> CompilerResult<MemberLookup> {
        let mut candidates = Vec::new();

        // runtime receiver unions settle each member under its enclosing forms
        let is_receiver_union = self.is_subject_receiver(origin, receiver, subject)?;

        // require every element to expose the member, tagging each candidate with its arm
        for element in elements {
            let arm_receiver = if is_receiver_union {
                self.replace_form_value(origin, receiver, *element)?
            } else {
                receiver
            };
            let lookup = self.lookup_subject_member(
                origin,
                module,
                arm_receiver,
                *element,
                space,
                key,
                extensions,
                active,
            )?;
            if lookup.is_empty() {
                return Ok(MemberLookup::default());
            }

            let arm = MemberArm {
                element: *element,
                receiver: arm_receiver,
            };
            for mut candidate in lookup.candidates {
                candidate.arm.get_or_insert(arm);
                candidates.push(candidate);
            }
        }

        // project distinct singleton fields through the physical union discriminant
        if space == dir::MemberSpace::Instance
            && let Some(projection) =
                self.select_union_discriminant(origin, receiver, elements, key, &candidates)?
        {
            return Ok(vec![MemberCandidate::projection(projection)].into());
        }

        Ok(candidates.into())
    }

    /// Select a shared required field as one physical union discriminant.
    fn select_union_discriminant(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        elements: &[dir::GlobalTypeId],
        key: dir::StaticKey,
        candidates: &[MemberCandidate],
    ) -> CompilerResult<Option<dir::Projection>> {
        // require every selected element to belong to one physical union representation
        let (representation, _) = self.project_newtype_receiver(origin, receiver)?;
        let representation = self.form_chain(origin, representation)?.base();
        let Some(representation_arms) = self.union_arms(origin, representation)? else {
            return Ok(None);
        };
        if !elements
            .iter()
            .all(|element| representation_arms.contains(element))
        {
            return Ok(None);
        }

        // require one distinct singleton-valued field from every selected arm
        let mut cases = Vec::with_capacity(elements.len());
        let mut types = Vec::with_capacity(elements.len());
        for element in elements {
            let arm = MemberArmGroup {
                arm: None,
                candidates: candidates
                    .iter()
                    .filter(|candidate| candidate.arm.is_some_and(|arm| arm.element == *element))
                    .collect(),
            };
            let Some(ty) = arm.direct_field_type() else {
                return Ok(None);
            };
            let base = self.strip_form(origin, ty)?;
            let Some(value) = self.ty(base)?.singleton_literal() else {
                return Ok(None);
            };

            // reject a repeated value, which matches several arms
            if cases
                .iter()
                .any(|case: &dir::DiscriminantCase| case.value == value)
            {
                return Ok(None);
            }

            cases.push(dir::DiscriminantCase {
                arm: self.canonical_union_leaf(
                    origin,
                    representation,
                    *element,
                    "a discriminant projection",
                )?,
                value,
            });
            types.push(ty);
        }

        // keep the declared field type beside the physical mapping
        let ty = self.normalized_union_type(types)?;
        let projection = dir::Projection::Discriminant {
            union: representation,
            key,
            cases,
            ty,
        };

        Ok(Some(projection))
    }

    /// Look up one member on a declaration reference.
    fn lookup_symbol_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        instance: ApparentInstance,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionFilter,
    ) -> CompilerResult<MemberLookup> {
        // search inherent members first, from stored bindings for closed subjects
        let is_closed = !self.type_flags(subject)?.has_variable();
        let inherent = if is_closed {
            self.stored_member_lookup(origin, receiver, &instance, space, key)?
        } else {
            self.lookup_inherent_symbol_member(origin, receiver, &instance, space, key)?
        };

        // prefer the inherent members the declaration exposes
        if !inherent.is_empty() {
            return Ok(inherent);
        }

        // search extensions next: declared members shadow interface defaults
        let mut lookup = MemberLookup::default();
        if extensions == ExtensionFilter::Include {
            lookup = self.lookup_extension_member(
                origin,
                module,
                receiver,
                subject,
                dir::TypeRoot::Declaration(instance.symbol),
                space,
                key,
            )?;
        }

        // answer with the matched extension member
        if !lookup.is_empty() {
            return Ok(lookup);
        }

        // search associated members through their declaring interface last
        let associated = self.lookup_associated_member(origin, receiver, space, key)?;
        if !associated.is_empty() {
            return Ok(associated);
        }

        // search the members of the auto interfaces the subject derives
        self.lookup_derived_member(origin, subject, space, key)
    }

    /// Look up one member declared by an auto interface the subject satisfies intrinsically.
    fn lookup_derived_member(
        &mut self,
        origin: Origin,
        subject: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        // walk the interfaces the compiler implements with members, in declaration order
        let derived = dir::AutoInterface::all()
            .filter(|interface| interface.has_builtin_implementation() && !interface.is_marker());
        for interface in derived {
            let Some(symbol) = self
                .environment_bound
                .language
                .symbol(dir::LanguageItem::from(interface))
            else {
                continue;
            };

            // pass the receiver as the interface's receiver argument
            let arguments = match interface.has_receiver_argument() {
                true => SmallVec::from_slice(&[subject]),
                false => SmallVec::new(),
            };
            let instance = ApparentInstance { symbol, arguments };

            // look the derived member up on the subject itself
            let lookup =
                self.lookup_inherent_symbol_member(origin, subject, &instance, space, key)?;
            if lookup.is_empty() {
                continue;
            }

            // admit the member once the subject satisfies the interface
            if self.decide_auto_interface(origin, subject, interface)? == Verdict::Holds {
                return Ok(lookup);
            }
        }

        Ok(MemberLookup::default())
    }

    /// Look up one associated member through its uniquely declaring interface.
    fn lookup_associated_member(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        if space != dir::MemberSpace::Static {
            return Ok(MemberLookup::default());
        }

        // require a qualifier for the projected key
        let Some(qualifier) = self.select_associated_qualifier(origin, receiver, key)? else {
            return Ok(MemberLookup::default());
        };

        // search the interface application selected for this projection
        let (module, qualifier) = self.nominal_application(qualifier)?;
        let arguments = self.type_ids(module, qualifier.arguments)?;
        let qualifier = ApparentInstance {
            symbol: qualifier.symbol,
            arguments: arguments.iter().copied().collect(),
        };

        self.lookup_inherent_symbol_member(origin, receiver, &qualifier, space, key)
    }

    /// Look up one inherent member on a declaration, walking its heritage.
    fn lookup_inherent_symbol_member(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance: &ApparentInstance,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        // require the declaration's definition
        let Some(definition) = self.definition(instance.symbol)? else {
            return Ok(MemberLookup::default());
        };

        // collect own members and heritage applications
        let members = definition
            .members_with_key(space, key)
            .cloned()
            .collect::<SmallVec<[_; 2]>>();
        let heritages = definition
            .bases()
            .iter()
            .map(|heritage| heritage.ty)
            .collect::<SmallVec<[_; 2]>>();

        // substitute applied arguments and the receiver's object
        let receiver_value = self.strip_form(origin, receiver)?;
        let substitution = instance.substitution(self)?.with_receiver(receiver_value);
        let candidates = self.instance_member_candidates(
            origin,
            receiver,
            instance,
            &substitution,
            &members,
            key,
        )?;
        if !candidates.is_empty() {
            return Ok(candidates.into());
        }

        // search substituted heritage applications
        for heritage in heritages {
            let heritage = self.substitute_type(heritage, &substitution)?;
            let (heritage_module, heritage) = self.nominal_application(heritage)?;
            let arguments = self.type_ids(heritage_module, heritage.arguments)?;
            let heritage = ApparentInstance {
                symbol: heritage.symbol,
                arguments: arguments.iter().copied().collect(),
            };

            let lookup =
                self.lookup_inherent_symbol_member(origin, receiver, &heritage, space, key)?;
            if !lookup.is_empty() {
                return Ok(lookup);
            }
        }

        Ok(MemberLookup::default())
    }

    /// Build the member candidates one instance declares for one key.
    pub(super) fn instance_member_candidates(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance: &ApparentInstance,
        substitution: &TypeSubstitution,
        members: &[dir::DefinitionMember],
        key: dir::StaticKey,
    ) -> CompilerResult<Vec<MemberCandidate>> {
        // build one candidate per declaration behind the key
        let mut candidates = Vec::new();
        for member in members.iter().cloned() {
            let Some(declared) = self.declared_member(&member)? else {
                continue;
            };

            // project value-less associated types through the receiver
            let symbol = declared.symbol;
            let is_associated = matches!(member, dir::DefinitionMember::AssociatedType(_));
            let is_rigid = self.is_rigid_projection_owner(receiver)?;
            let ty = match declared.ty {
                Some(ty) if !(is_associated && is_rigid) => ty,
                _ if is_associated => {
                    let arguments = self.intern_type_ids(&[])?;
                    let qualifier = Some(instance.qualifier(self)?);

                    self.intern_member(dir::MemberType {
                        owner: receiver,
                        key,
                        arguments,
                        qualifier,
                    })?
                }
                _ => continue,
            };

            // instantiate a value on an unapplied generic declaration at its use site
            let member = declared;
            let mut substitution = substitution.clone();
            let mut generic_arguments =
                self.symbol_generic_argument_bindings(instance.symbol, &instance.arguments)?;
            if instance.arguments.is_empty()
                && !member.role.is_callable()
                && let Some(template) = self.symbol_template(instance.symbol)?
            {
                let parameters = self.generic_template_parameters(template)?;
                let Some(instantiated) =
                    self.instantiate_parameters(origin, &parameters, &[], substitution)?
                else {
                    return Err(CompilerError::Internal {
                        message: format!("declaration template {template:?} cannot instantiate"),
                    });
                };
                for constraint in
                    self.substitute_application_constraints(origin, template, &instantiated)?
                {
                    self.push_relation(constraint)?;
                }
                generic_arguments = instantiated.bindings.to_vec();
                substitution = instantiated;
            }

            // apply the arguments and read the member's operations at the receiver
            let ty = self.substitute_type(ty, &substitution)?;
            let callable = member.callable_type(ty);
            let access = self.projected_member_access(origin, Some(receiver), &member, ty)?;

            // substitute static value types for projections
            let written = match self.static_value(symbol)? {
                Some(written) => Some(self.substitute_type(written, &substitution)?),
                None => None,
            };
            let mut declared = DeclaredSource::new(
                symbol,
                instance.symbol,
                dir::MemberOrigin::Declaration,
                generic_arguments,
            );
            declared.region_arguments = self.resolved_region_bindings(&substitution.bindings)?;
            declared.value_type = written;
            candidates.push(MemberCandidate::declared(
                declared, &member, access, callable,
            ));
        }

        Ok(candidates)
    }

    /// Return the exposed member key closest to one missing key.
    pub(in crate::sema) fn closest_member_key(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: &str,
    ) -> CompilerResult<Option<NameMatch<String>>> {
        // enumerate the receiver's keys like keyed lookup, in its selected space
        let module = origin.module();
        let subject = self.normalize_computation(origin, receiver)?;
        let space = match self.ty(subject)? {
            dir::Type::Reference(_) => dir::MemberSpace::Static,
            _ => dir::MemberSpace::Instance,
        };
        let mut keys = FxIndexSet::default();
        let mut visited = FxIndexSet::default();
        self.collect_subject_keys(origin, module, subject, space, &mut keys, &mut visited)?;
        let keys = keys
            .iter()
            .map(|candidate| self.format_static_key(candidate))
            .collect::<Vec<_>>();

        // suggest the closest declared key
        Ok(find_best_match(
            key,
            keys,
            diagnostic_suggestion_distance(key),
        ))
    }

    /// Collect declared member keys from one subject type.
    fn collect_subject_keys(
        &mut self,
        origin: Origin,
        module: ModuleId,
        subject: dir::GlobalTypeId,
        space: dir::MemberSpace,
        keys: &mut FxIndexSet<dir::StaticKey>,
        visited: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        // normalize the subject head like keyed lookup before collecting its keys
        let subject = self.normalize_computation(origin, subject)?;

        // stop cyclic paths through bounds, unions, and heritage
        if !visited.insert(subject) {
            return Ok(());
        }

        // collect the keys by the subject's own head
        match self.ty(subject)? {
            // follow a solved variable into its solution
            dir::Type::Variable(variable) => {
                if let Some(solution) = self.infer.solution(variable)? {
                    self.collect_subject_keys(origin, module, solution, space, keys, visited)?;
                }
            }
            // declarations expose the same keys as keyed lookup
            dir::Type::Reference(reference) => {
                self.collect_reference_keys(origin, module, reference, space, keys, visited)?;
            }
            // collect the instance and extension keys of an applied declaration
            dir::Type::Application(_) => {
                self.collect_instance_keys(origin, module, subject, space, keys)?;

                // include newtype payload keys
                if space == dir::MemberSpace::Instance
                    && let Some(instance) = self.apparent_instance(subject)?
                    && matches!(
                        self.definition(instance.symbol)?.as_deref(),
                        Some(dir::Definition::Newtype(_))
                    )
                    && let Some(payload) = self.decompose_newtype(origin, subject)?
                {
                    self.collect_subject_keys(
                        origin,
                        module,
                        payload.backing,
                        space,
                        keys,
                        visited,
                    )?;
                }
            }
            // refinements expose their refined key and base members
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(subject.module_id, refined)?;
                keys.insert(refined.key);
                self.collect_subject_keys(origin, module, refined.base, space, keys, visited)?;
            }
            // precise variants expose their enum owner's members
            dir::Type::Variant(variant) => {
                let dir::Type::Application(owner) = self.ty(variant.owner)? else {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "variant {:?} has non-application owner {:?}",
                            variant.variant, variant.owner
                        ),
                    });
                };
                self.collect_symbol_keys(module, owner.symbol, space, keys, visited)?;
            }
            // structural subjects expose their property keys
            dir::Type::Object(shape) => {
                for property in self.object_properties(subject.module_id, shape.properties)? {
                    keys.insert(property.key);
                }
            }
            // read primitive, literal, collection, and function keys from the apparent tables
            dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Slice(_)
            | dir::Type::FixedArray(_)
            | dir::Type::Function(_)
            | dir::Type::FunctionSignature(_) => {
                self.collect_instance_keys(origin, module, subject, space, keys)?;
            }

            // tuples expose their labeled elements and their extension members
            dir::Type::Tuple(tuple) => {
                for element in self.tuple_elements(subject.module_id, tuple.elements)? {
                    if let Some(label) = element.label {
                        keys.insert(dir::StaticKey::Name(label));
                    }
                }
                self.collect_instance_keys(origin, module, subject, space, keys)?;
            }
            // memory forms expose their pointee keys
            dir::Type::Form(form) => {
                self.collect_subject_keys(origin, module, form.value, space, keys, visited)?;
            }
            // erased subjects expose their constraint keys
            dir::Type::Dynamic(dynamic) => {
                self.collect_subject_keys(
                    origin,
                    module,
                    dynamic.constraint,
                    space,
                    keys,
                    visited,
                )?;
            }
            // parameters expose the keys of their declared bounds
            dir::Type::Parameter(parameter) => {
                for bound in self.parameter_bounds(origin, parameter)? {
                    self.collect_subject_keys(origin, module, bound, space, keys, visited)?;
                }
            }
            // composite subjects expose the keys of every element
            dir::Type::Union(dir::UnionType { elements })
            | dir::Type::Intersection(dir::IntersectionType { elements }) => {
                let elements: SmallVec<[_; 8]> = self.type_ids(subject.module_id, elements)?.into();
                for element in elements {
                    self.collect_subject_keys(origin, module, element, space, keys, visited)?;
                }
            }
            // remaining types expose no keyed members
            dir::Type::Error
            | dir::Type::Never
            | dir::Type::Unknown
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Key(_)
            | dir::Type::Region(_)
            | dir::Type::Static(_)
            | dir::Type::Intrinsic
            | dir::Type::Erased(_)
            | dir::Type::This
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::Range(_)
            | dir::Type::FunctionPointer(_) => {}
        }

        Ok(())
    }

    /// Collect the keys selected through one declaration reference.
    fn collect_reference_keys(
        &mut self,
        origin: Origin,
        module: ModuleId,
        reference: dir::TypeReference,
        space: dir::MemberSpace,
        keys: &mut FxIndexSet<dir::StaticKey>,
        visited: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        let symbol = reference.symbol;
        let alias = match self.definition(symbol)?.as_deref() {
            Some(dir::Definition::TypeAlias(alias)) => Some(alias.value),
            _ => None,
        };

        // expose the keys of a static alias's body
        if space == dir::MemberSpace::Static
            && let Some(alias) = alias
        {
            return self.collect_subject_keys(origin, module, alias, space, keys, visited);
        }

        self.collect_symbol_keys(module, symbol, space, keys, visited)
    }

    /// Collect one nominal subject's keys from its member tables.
    fn collect_instance_keys(
        &mut self,
        origin: Origin,
        module: ModuleId,
        subject: dir::GlobalTypeId,
        space: dir::MemberSpace,
        keys: &mut FxIndexSet<dir::StaticKey>,
    ) -> CompilerResult<()> {
        // collect the extension keys alone for a structural subject
        let Some(instance) = self.apparent_instance(subject)? else {
            let value = self.strip_form(origin, subject)?;
            if let Some(root) = self.structural_root(value)? {
                self.collect_root_extension_keys(origin, module, subject, root, space, keys)?;
            }

            return Ok(());
        };

        // enumerate declaration and heritage keys from the inherent table
        let inherent = self.inherent_member_table(origin, subject, &instance, space)?;
        keys.extend(inherent.keys().copied());

        let root = dir::TypeRoot::Declaration(instance.symbol);

        self.collect_root_extension_keys(origin, module, subject, root, space, keys)
    }

    /// Collect the member keys the visible extensions over one root expose for a subject.
    fn collect_root_extension_keys(
        &mut self,
        origin: Origin,
        module: ModuleId,
        subject: dir::GlobalTypeId,
        root: dir::TypeRoot,
        space: dir::MemberSpace,
        keys: &mut FxIndexSet<dir::StaticKey>,
    ) -> CompilerResult<()> {
        // enumerate the visible extension keys from the decided candidates
        let extensions = self.subject_extensions(origin, module, subject, subject, root)?;
        for extension in extensions {
            let Some(source) =
                self.decide_extension_source(origin, module, subject, subject, extension)?
            else {
                continue;
            };
            let matched =
                self.extension_candidates(origin, subject, subject, &source, space, None)?;
            keys.extend(matched.iter().map(|(key, _)| *key));
        }

        Ok(())
    }

    /// Collect the member keys one declaration exposes, with its heritage and extensions.
    fn collect_symbol_keys(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
        keys: &mut FxIndexSet<dir::StaticKey>,
        visited: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        // collect the declaration's own member keys
        self.collect_definition_keys(symbol, space, keys)?;

        // read the base declarations and interfaces this declaration inherits
        let heritages = match self.definition(symbol)?.as_deref() {
            Some(definition) => {
                let mut heritages = definition
                    .bases()
                    .into_iter()
                    .map(|heritage| heritage.ty)
                    .collect::<SmallVec<[_; 2]>>();
                heritages.extend(
                    definition
                        .implementations()
                        .map(|conformance| conformance.interface),
                );

                heritages
            }
            None => SmallVec::new(),
        };

        // collect the keys each heritage exposes
        for heritage in heritages {
            self.collect_subject_keys(
                Origin::Symbol(symbol),
                module,
                heritage,
                space,
                keys,
                visited,
            )?;
        }

        // collect extension keys targeting this declaration
        let head = ExtensionHead::Root(dir::TypeRoot::Declaration(symbol));
        for extension in self.extensions_over(module, &[head])? {
            self.collect_definition_keys(extension, space, keys)?;
        }

        Ok(())
    }

    /// Collect the member keys declared by one definition.
    fn collect_definition_keys(
        &mut self,
        symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
        keys: &mut FxIndexSet<dir::StaticKey>,
    ) -> CompilerResult<()> {
        // owners without a declaration body declare no keys
        let Some(definition) = self.definition(symbol)? else {
            return Ok(());
        };

        // collect each declared key in the requested space
        for member in definition.members() {
            if member.space() == space
                && let Some(key) = member.key()
            {
                keys.insert(key);
            }
        }

        Ok(())
    }

    /// Settle the checked type of each member declared through its own symbol.
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
}

impl dir::TypeFold for LookupReceiver {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(dir::GlobalTypeId) -> Result<dir::GlobalTypeId, E>,
    ) -> Result<(), E> {
        match self {
            Self::Direct(steps) => steps.map_types(map),
            Self::Dynamic {
                adjustments,
                constraint,
            } => {
                adjustments.map_types(map)?;
                constraint.map_types(map)
            }
        }
    }
}

impl dir::TypeFold for CandidateSource {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(dir::GlobalTypeId) -> Result<dir::GlobalTypeId, E>,
    ) -> Result<(), E> {
        match self {
            Self::Declared(declared) => {
                declared.value_type.map_types(map)?;
                declared.generic_arguments.map_types(map)?;
                declared.region_arguments.map_types(map)?;
                declared.bounds.map_types(map)?;
                declared.target.map_types(map)
            }
            Self::Structural { owner } => owner.map_types(map),
            Self::Projection(projection) => projection.map_types(map),
        }
    }
}

impl dir::TypeFold for MemberArm {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(dir::GlobalTypeId) -> Result<dir::GlobalTypeId, E>,
    ) -> Result<(), E> {
        self.element.map_types(map)?;
        self.receiver.map_types(map)
    }
}

impl dir::TypeFold for MemberCandidate {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(dir::GlobalTypeId) -> Result<dir::GlobalTypeId, E>,
    ) -> Result<(), E> {
        self.source.map_types(map)?;
        self.access.map_types(map)?;
        self.callable.map_types(map)?;
        self.receiver.map_types(map)?;
        self.arm.map_types(map)
    }
}
