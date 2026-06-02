use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{Arena, GlobalNodeIdAny, GlobalStaticId, GlobalSymbolId, LanguageItem, SegmentView};

/// Cumulative annotation invocations for one DIR module.
#[derive(Debug, Clone)]
pub struct AnnotationTable<'a> {
    /// The module id of the annotation table.
    pub module_id: ModuleId,
    /// The ordered annotation table segments.
    segments: SegmentView<'a, AnnotationSegment>,
}

impl AnnotationTable<'static> {
    /// Create an annotation table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<AnnotationSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create an annotation table from one segment.
    pub fn from_segment(segment: Arc<AnnotationSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> AnnotationTable<'a> {
    /// Create an annotation table from a segment view.
    pub fn from_view(segments: SegmentView<'a, AnnotationSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("annotation table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "annotation table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Iterate visible annotation invocations.
    pub fn iter_invocations(
        &self,
    ) -> impl Iterator<Item = (LocalAnnotationId, &AnnotationInvocation)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_invocations())
    }

    /// Iterate visible annotation invocations attached to one owner.
    pub fn invocations_for_owner(
        &self,
        owner: GlobalNodeIdAny,
    ) -> impl Iterator<Item = &AnnotationInvocation> + '_ {
        let mut ids = Vec::new();

        // collect owner ids in segment order
        for segment in self.segments.iter() {
            if let Some(segment_ids) = segment.invocations_by_owner.get(&owner) {
                ids.extend(segment_ids.iter().copied());
            }
        }

        ids.into_iter().map(|id| self.get_invocation(id))
    }

    /// Get an annotation invocation by id.
    pub fn get_invocation(&self, invocation_id: LocalAnnotationId) -> &AnnotationInvocation {
        for segment in self.segments.iter() {
            if let Some(invocation) = segment.get_local_invocation(invocation_id) {
                return invocation;
            }
        }

        panic!("DIR annotation invocation {invocation_id:?} is not visible")
    }

    /// Get the number of annotation invocations in the table.
    pub fn invocation_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.invocation_count())
            .unwrap_or(0)
    }
}

/// Annotation invocations added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationSegment {
    /// The module id of the annotation segment.
    pub module_id: ModuleId,
    /// The first annotation invocation id owned by this table segment.
    pub(crate) first_invocation_id: u32,
    /// Annotation invocations owned by this segment.
    pub(crate) invocations: Arena<AnnotationInvocation>,
    /// Annotation invocations attached to each owner.
    pub(crate) invocations_by_owner: IndexMap<GlobalNodeIdAny, Vec<LocalAnnotationId>>,
}

impl AnnotationSegment {
    /// Create an empty annotation segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            first_invocation_id: 0,
            invocations: Arena::new(),
            invocations_by_owner: IndexMap::new(),
        }
    }

    /// Create a new empty segment after an existing annotation table segment.
    pub fn from_base(base: &Self) -> Self {
        Self {
            module_id: base.module_id,
            first_invocation_id: base.invocation_count(),
            invocations: Arena::new(),
            invocations_by_owner: IndexMap::new(),
        }
    }

    /// Insert one annotation invocation.
    pub fn insert_invocation(&mut self, invocation: AnnotationInvocation) -> LocalAnnotationId {
        let invocation_id = LocalAnnotationId::new(self.invocation_count());
        let owner = invocation.owner;

        self.invocations.allocate(invocation);
        self.invocations_by_owner
            .entry(owner)
            .or_default()
            .push(invocation_id);

        invocation_id
    }

    /// Iterate annotation invocations owned by this segment.
    pub fn iter_invocations(
        &self,
    ) -> impl Iterator<Item = (LocalAnnotationId, &AnnotationInvocation)> + '_ {
        (self.first_invocation_id..self.invocation_count()).map(|index| {
            let invocation_id = LocalAnnotationId::new(index);
            (invocation_id, self.get_invocation(invocation_id))
        })
    }

    /// Get an annotation invocation by id.
    pub fn get_invocation(&self, invocation_id: LocalAnnotationId) -> &AnnotationInvocation {
        self.get_local_invocation(invocation_id).unwrap_or_else(|| {
            panic!("DIR annotation invocation {invocation_id:?} is not allocated in this segment")
        })
    }

    /// Get the number of annotation invocations in the segment.
    pub fn invocation_count(&self) -> u32 {
        self.first_invocation_id + self.invocations.len() as u32
    }

    /// Return true when this segment has no annotation invocations.
    pub fn is_empty(&self) -> bool {
        self.invocations.is_empty()
    }

    /// Get an invocation owned by this table segment.
    pub(crate) fn get_local_invocation(
        &self,
        invocation_id: LocalAnnotationId,
    ) -> Option<&AnnotationInvocation> {
        self.contains_invocation_id(invocation_id).then(|| {
            self.invocations
                .get(invocation_id.0 - self.first_invocation_id)
        })
    }

    /// Return whether this segment contains the given annotation invocation id.
    fn contains_invocation_id(&self, invocation_id: LocalAnnotationId) -> bool {
        invocation_id.0 >= self.first_invocation_id && invocation_id.0 < self.invocation_count()
    }
}

/// Unique identifier for an annotation invocation.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalAnnotationId(pub u32);

impl LocalAnnotationId {
    /// Wrap an id as a local annotation id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Checked annotation invocation attached to one owner node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotationInvocation {
    /// The decorator node.
    pub source: GlobalNodeIdAny,
    /// The node annotated by this invocation.
    pub owner: GlobalNodeIdAny,
    /// The callee expression.
    pub callee: GlobalNodeIdAny,
    /// The resolved annotation target.
    pub target: AnnotationTarget,
    /// The invocation arguments.
    pub arguments: Vec<AnnotationArgument>,
}

/// Checked annotation argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotationArgument {
    /// The argument node.
    pub source: GlobalNodeIdAny,
    /// The committed static value when one exists.
    pub value: Option<GlobalStaticId>,
}

/// Resolved annotation target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnnotationTarget {
    /// Compiler language item annotation.
    LanguageItem(LanguageItem),
    /// User-defined annotation symbol.
    Symbol(GlobalSymbolId),
    /// Unresolved or non-symbol annotation target.
    Unknown,
}
