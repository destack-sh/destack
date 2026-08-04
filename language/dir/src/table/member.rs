use std::mem;
use std::sync::Arc;

use destack_core::FxIndexMap as IndexMap;
use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    GlobalGenericTemplateId, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, PropertyAccess,
    SegmentView, StaticKey,
};

/// Cumulative checked member bindings for one DIR module.
#[derive(Debug, Clone)]
pub struct MemberTable<'a> {
    /// The module id of the member table.
    pub module_id: ModuleId,
    /// The ordered member table segments.
    segments: SegmentView<'a, MemberSegment>,
}

impl MemberTable<'static> {
    /// Create a member table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<MemberSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a member table from one segment.
    pub fn from_segment(segment: Arc<MemberSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> MemberTable<'a> {
    /// Create a member table from a segment view.
    pub fn from_view(segments: SegmentView<'a, MemberSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("member table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "member table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create a member table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b MemberSegment) -> MemberTable<'b> {
        MemberTable::from_view(self.segments.with_tail(tail))
    }

    /// Return the lookup subject selected at one source site.
    pub fn subject(&self, source: GlobalNodeIdAny) -> Option<MemberSubject> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.subject(source))
    }

    /// Return the checked member bindings available at one source site.
    pub fn members(&self, source: GlobalNodeIdAny) -> Option<&[MemberBinding]> {
        let subject = self.subject(source)?;

        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.members(subject))
    }

    /// Return whether this table has no member bindings.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(MemberSegment::is_empty)
    }
}

/// Checked member bindings added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct MemberSegment {
    /// The module id of the member segment.
    pub module_id: ModuleId,
    /// Lookup subjects selected at source member sites.
    subjects: IndexMap<GlobalNodeIdAny, MemberSubject>,
    /// Checked member bindings stored once per lookup subject.
    bindings: IndexMap<MemberSubject, Vec<MemberBinding>>,
}

/// One rollback position in a member segment.
#[derive(Debug, Clone, Copy)]
pub struct MemberMark {
    /// The recorded source count.
    subjects: usize,
    /// The recorded subject count.
    bindings: usize,
}

impl MemberSegment {
    /// Create an empty member segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            subjects: IndexMap::default(),
            bindings: IndexMap::default(),
        }
    }

    /// Record the member lookup subject selected at one source site.
    pub fn record_subject(&mut self, source: GlobalNodeIdAny, subject: MemberSubject) {
        // require one stable subject per source site
        if let Some(recorded) = self.subjects.get(&source) {
            assert_eq!(
                *recorded, subject,
                "member source was recorded with a different subject"
            );
        } else {
            self.subjects.insert(source, subject);
        }
    }

    /// Record the checked member binding resolved for one subject key.
    pub fn record_binding(&mut self, subject: MemberSubject, binding: MemberBinding) {
        let bindings = self.bindings.entry(subject).or_default();
        match bindings.iter_mut().find(|recorded| recorded.key == binding.key) {
            Some(recorded) => *recorded = binding,
            None => bindings.push(binding),
        }
    }

    /// Record the checked member bindings for one lookup subject.
    pub fn record_bindings(&mut self, subject: MemberSubject, bindings: Vec<MemberBinding>) {
        // require one stable binding list per subject
        if let Some(recorded) = self.bindings.get(&subject) {
            assert_eq!(
                recorded, &bindings,
                "member subject was recorded with different bindings"
            );
        } else {
            self.bindings.insert(subject, bindings);
        }
    }

    /// Iterate the member lookup subjects retained at source sites.
    pub fn iter_subjects(&self) -> impl Iterator<Item = (GlobalNodeIdAny, MemberSubject)> + '_ {
        self.subjects
            .iter()
            .map(|(source, subject)| (*source, *subject))
    }

    /// Return a rollback position for this segment.
    pub fn mark(&self) -> MemberMark {
        MemberMark {
            subjects: self.subjects.len(),
            bindings: self.bindings.len(),
        }
    }

    /// Truncate this segment to a previous rollback position.
    pub fn truncate_to(&mut self, mark: MemberMark) {
        self.subjects.truncate(mark.subjects);
        self.bindings.truncate(mark.bindings);
    }

    /// Return the lookup subject selected at one source site.
    pub fn subject(&self, source: GlobalNodeIdAny) -> Option<MemberSubject> {
        self.subjects.get(&source).copied()
    }

    /// Return the checked bindings stored for one lookup subject.
    pub fn members(&self, subject: MemberSubject) -> Option<&[MemberBinding]> {
        self.bindings.get(&subject).map(Vec::as_slice)
    }

    /// Return whether this segment has no member bindings.
    pub fn is_empty(&self) -> bool {
        self.subjects.is_empty() && self.bindings.is_empty()
    }

    /// Apply one mapping to every type id stored in this segment.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        // map subjects attached to source sites
        for subject in self.subjects.values_mut() {
            subject.map_type_ids(map);
        }

        // rebuild bindings under their mapped subjects
        let bindings = mem::take(&mut self.bindings);
        for (mut subject, mut members) in bindings {
            subject.map_type_ids(map);
            for member in &mut members {
                member.map_type_ids(map);
            }

            if let Some(recorded) = self.bindings.get(&subject) {
                assert_eq!(
                    recorded, &members,
                    "mapped member subjects have different bindings"
                );
            } else {
                self.bindings.insert(subject, members);
            }
        }
    }
}

/// The exact input to checked member lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MemberSubject {
    /// The non-nullish receiver type.
    pub receiver: GlobalTypeId,
    /// The type searched by member lookup.
    pub target: GlobalTypeId,
    /// The selected instance or static member space.
    pub space: MemberSpace,
    /// The generic template assumptions active at the source site.
    pub scope: Option<GlobalGenericTemplateId>,
}

impl MemberSubject {
    /// Create one member lookup subject.
    pub fn new(receiver: GlobalTypeId, target: GlobalTypeId, space: MemberSpace) -> Self {
        Self {
            receiver,
            target,
            space,
            scope: None,
        }
    }

    /// Attach the generic assumptions active for this lookup.
    pub fn with_scope(mut self, scope: Option<GlobalGenericTemplateId>) -> Self {
        self.scope = scope;

        self
    }

    /// Apply one mapping to every type id stored in this subject.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.receiver = map(self.receiver);
        self.target = map(self.target);
    }
}

/// One finite key bound by checked member lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemberBinding {
    /// The available member key.
    pub key: StaticKey,
    /// The available member kind.
    pub kind: MemberKind,
    /// The checked read and write types.
    pub access: PropertyAccess,
    /// Whether the member may be absent.
    pub is_optional: bool,
    /// The declarations contributing to this binding.
    pub declarations: Vec<MemberDeclaration>,
}

impl MemberBinding {
    /// Create one checked member binding.
    pub fn new(
        key: StaticKey,
        kind: MemberKind,
        access: PropertyAccess,
        is_optional: bool,
        declarations: Vec<MemberDeclaration>,
    ) -> Self {
        Self {
            key,
            kind,
            access,
            is_optional,
            declarations,
        }
    }

    /// Apply one mapping to every type id stored in this binding.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.access.map_type_ids(map);
        for declaration in &mut self.declarations {
            declaration.map_type_ids(map);
        }
    }
}

/// One declaration contributing to a checked member binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemberDeclaration {
    /// The selected declaration symbol.
    pub symbol: GlobalSymbolId,
    /// The declaration family that exposed this member.
    pub origin: MemberOrigin,
    /// The substituted callable type, when callable.
    pub callable_type: Option<GlobalTypeId>,
}

impl MemberDeclaration {
    /// Apply one mapping to every type id stored in this declaration.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.callable_type = self.callable_type.map(map);
    }
}

/// One member kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
pub enum MemberKind {
    /// Field member.
    Field,
    /// Getter or setter property member.
    Property,
    /// Method member.
    Method,
    /// Constructor member.
    Constructor,
    /// Call signature member.
    CallSignature,
    /// Construct signature member.
    ConstructSignature,
    /// Index signature member.
    IndexSignature,
    /// Associated type member.
    AssociatedType,
    /// Associated constant member.
    AssociatedConst,
    /// Enum variant member.
    Variant,
}

/// Member namespace selected by member lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum MemberSpace {
    /// Instance members selected from a runtime receiver.
    Instance,
    /// Static members selected from a declaration receiver.
    Static,
}

/// The declaration family one member came from.
///
/// Direct declarations shadow rooted extensions, which shadow blanket extensions.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub enum MemberOrigin {
    /// A member declared by the owning declaration itself.
    Declaration,
    /// A member declared by an extension rooted at its target.
    RootedExtension,
    /// A member declared by an open blanket extension.
    BlanketExtension,
}
