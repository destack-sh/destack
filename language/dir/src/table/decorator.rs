use destack_serde::Reflect;
use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{Arena, GlobalNodeIdAny, GlobalSymbolId, LanguageItem, SegmentView};

/// Cumulative decorator applications for one DIR module.
#[derive(Debug, Clone)]
pub struct DecoratorTable<'a> {
    /// The module id of the decorator table.
    pub module_id: ModuleId,
    /// The ordered decorator table segments.
    segments: SegmentView<'a, DecoratorSegment>,
}

impl DecoratorTable<'static> {
    /// Create a decorator table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<DecoratorSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a decorator table from one segment.
    pub fn from_segment(segment: Arc<DecoratorSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> DecoratorTable<'a> {
    /// Create a decorator table from a segment view.
    pub fn from_view(segments: SegmentView<'a, DecoratorSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("decorator table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "decorator table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Iterate visible decorator applications.
    pub fn iter_applications(
        &self,
    ) -> impl Iterator<Item = (LocalDecoratorId, &DecoratorApplication)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_applications())
    }

    /// Iterate visible decorator applications attached to one owner.
    pub fn applications_for_owner(
        &self,
        owner: GlobalNodeIdAny,
    ) -> impl Iterator<Item = &DecoratorApplication> + '_ {
        let mut ids = Vec::new();

        // collect owner ids in segment order
        for segment in self.segments.iter() {
            if let Some(segment_ids) = segment.applications_by_owner.get(&owner) {
                ids.extend(segment_ids.iter().copied());
            }
        }

        ids.into_iter().map(|id| self.get_application(id))
    }

    /// Get a decorator application by id.
    pub fn get_application(&self, application_id: LocalDecoratorId) -> &DecoratorApplication {
        for segment in self.segments.iter() {
            if let Some(application) = segment.get_local_application(application_id) {
                return application;
            }
        }

        panic!("DIR decorator application {application_id:?} is not visible")
    }

    /// Get the number of decorator applications in the table.
    pub fn application_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.application_count())
            .unwrap_or(0)
    }
}

/// Decorator applications added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DecoratorSegment {
    /// The module id of the decorator segment.
    pub module_id: ModuleId,
    /// The first decorator application id owned by this table segment.
    pub(crate) first_application_id: u32,
    /// Decorator applications owned by this segment.
    pub(crate) applications: Arena<DecoratorApplication>,
    /// Decorator applications attached to each owner.
    pub(crate) applications_by_owner: IndexMap<GlobalNodeIdAny, Vec<LocalDecoratorId>>,
}

impl DecoratorSegment {
    /// Create an empty decorator segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            first_application_id: 0,
            applications: Arena::new(),
            applications_by_owner: IndexMap::new(),
        }
    }

    /// Create a new empty segment after an existing decorator table segment.
    pub fn from_base(base: &Self) -> Self {
        Self {
            module_id: base.module_id,
            first_application_id: base.application_count(),
            applications: Arena::new(),
            applications_by_owner: IndexMap::new(),
        }
    }

    /// Insert one decorator application.
    pub fn insert_application(&mut self, application: DecoratorApplication) -> LocalDecoratorId {
        let application_id = LocalDecoratorId::new(self.application_count());
        let owner = application.owner;

        self.applications.allocate(application);
        self.applications_by_owner
            .entry(owner)
            .or_default()
            .push(application_id);

        application_id
    }

    /// Iterate decorator applications owned by this segment.
    pub fn iter_applications(
        &self,
    ) -> impl Iterator<Item = (LocalDecoratorId, &DecoratorApplication)> + '_ {
        (self.first_application_id..self.application_count()).map(|index| {
            let application_id = LocalDecoratorId::new(index);
            (application_id, self.get_application(application_id))
        })
    }

    /// Get a decorator application by id.
    pub fn get_application(&self, application_id: LocalDecoratorId) -> &DecoratorApplication {
        self.get_local_application(application_id)
            .unwrap_or_else(|| {
                panic!(
                    "DIR decorator application {application_id:?} is not allocated in this segment"
                )
            })
    }

    /// Get the number of decorator applications in the segment.
    pub fn application_count(&self) -> u32 {
        self.first_application_id + self.applications.len() as u32
    }

    /// Return true when this segment has no decorator applications.
    pub fn is_empty(&self) -> bool {
        self.applications.is_empty()
    }

    /// Get an application owned by this table segment.
    pub(crate) fn get_local_application(
        &self,
        application_id: LocalDecoratorId,
    ) -> Option<&DecoratorApplication> {
        self.contains_application_id(application_id).then(|| {
            self.applications
                .get(application_id.0 - self.first_application_id)
        })
    }

    /// Return whether this segment contains the given decorator application id.
    fn contains_application_id(&self, application_id: LocalDecoratorId) -> bool {
        application_id.0 >= self.first_application_id && application_id.0 < self.application_count()
    }
}

/// Unique identifier for a decorator application.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct LocalDecoratorId(pub u32);

impl LocalDecoratorId {
    /// Wrap an id as a local decorator id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Checked decorator application attached to one owner node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DecoratorApplication {
    /// The decorator node.
    pub source: GlobalNodeIdAny,
    /// The node decorated by this application.
    pub owner: GlobalNodeIdAny,
    /// The decorator target expression.
    pub target: GlobalNodeIdAny,
    /// The resolved decorator target.
    pub resolution: DecoratorResolution,
    /// The application arguments.
    pub arguments: Vec<DecoratorArgument>,
}

/// Checked decorator argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DecoratorArgument {
    /// The argument node.
    pub source: GlobalNodeIdAny,
}

/// Resolved decorator target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DecoratorResolution {
    /// Compiler language item decorator.
    LanguageItem(LanguageItem),
    /// User-defined decorator symbol.
    Symbol(GlobalSymbolId),
    /// Unresolved or non-symbol decorator target.
    Unresolved,
}
