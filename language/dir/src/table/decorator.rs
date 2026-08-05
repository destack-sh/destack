use destack_serde::Reflect;
use std::sync::Arc;

use destack_core::FxIndexMap as IndexMap;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    Arena, Argument, ArgumentBinding, Decorator, Expression, GlobalNodeId, GlobalNodeIdAny,
    GlobalStaticId, GlobalSymbolId, GlobalTypeId, LanguageItem, LocalNodeId, NewtypeSelection,
    SegmentView,
};

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

    /// Return the application attached to one authored decorator.
    pub fn application_for_decorator(
        &self,
        decorator: LocalNodeId<Decorator>,
    ) -> Option<&DecoratorApplication> {
        self.iter_applications()
            .map(|(_, application)| application)
            .find(|application| application.source.local_id == decorator)
    }

    /// Return the application selected by one decorator expression.
    pub fn application_for_expression(
        &self,
        expression: LocalNodeId<Expression>,
    ) -> Option<&DecoratorApplication> {
        self.iter_applications()
            .map(|(_, application)| application)
            .find(|application| application.expression.local_id == expression)
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
            applications_by_owner: IndexMap::default(),
        }
    }

    /// Create a new empty segment after an existing decorator table segment.
    pub fn from_base(base: &Self) -> Self {
        Self {
            module_id: base.module_id,
            first_application_id: base.application_count(),
            applications: Arena::new(),
            applications_by_owner: IndexMap::default(),
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

    /// Iterate decorator applications attached to one owner.
    pub fn applications_for_owner(
        &self,
        owner: GlobalNodeIdAny,
    ) -> impl Iterator<Item = &DecoratorApplication> + '_ {
        self.applications_by_owner
            .get(&owner)
            .into_iter()
            .flatten()
            .map(|id| self.get_application(*id))
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

    /// Apply one mapping to every type id stored in this segment.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        for application in self.applications.iter_mut() {
            application.map_type_ids(map);
        }
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
    pub source: GlobalNodeId<Decorator>,
    /// The node decorated by this application.
    pub owner: GlobalNodeIdAny,
    /// The decorator target expression.
    pub expression: GlobalNodeId<Expression>,
    /// The checked decorator selection.
    pub resolution: DecoratorResolution,
    /// The checked decorator value.
    pub value: GlobalStaticId,
}

impl DecoratorApplication {
    /// Apply one mapping to every type id stored in this application.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.resolution.map_type_ids(map);
    }
}

/// Checked resolution of one decorator application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DecoratorResolution {
    /// The resolved decorator declaration.
    pub target: DecoratorTarget,
    /// The selection used to construct the decorator value.
    pub selection: DecoratorSelection,
    /// The nominal decorator value type.
    pub ty: GlobalTypeId,
}

impl DecoratorResolution {
    /// Apply one mapping to every type id stored in this resolution.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.selection.map_type_ids(map);
        self.ty = map(self.ty);
    }
}

/// Resolved decorator declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DecoratorTarget {
    /// Toolchain language item decorator.
    LanguageItem {
        /// The resolved decorator symbol.
        symbol: GlobalSymbolId,
        /// The well-known decorator identity.
        item: LanguageItem,
    },
    /// User-defined decorator symbol.
    Symbol {
        /// The resolved decorator symbol.
        symbol: GlobalSymbolId,
    },
}

impl DecoratorTarget {
    /// Return the well-known decorator identity, when present.
    pub fn language_item(self) -> Option<LanguageItem> {
        match self {
            Self::LanguageItem { item, .. } => Some(item),
            Self::Symbol { .. } => None,
        }
    }
}

/// Selection used to construct one checked decorator value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DecoratorSelection {
    /// Arguments matched against one newtype backing.
    Newtype {
        /// The selected newtype backing.
        newtype: NewtypeSelection,
        /// The source arguments bound to the selected parameters.
        arguments: Vec<ArgumentBinding>,
    },
    /// Providers selected by the compiler-owned derive dispatcher.
    Derive {
        /// The selected providers in argument order.
        providers: Vec<DeriveProvider>,
    },
}

impl DecoratorSelection {
    /// Apply one mapping to every type id stored in this selection.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Newtype { newtype, arguments } => {
                newtype.map_type_ids(map);
                for argument in arguments {
                    argument.map_type_ids(map);
                }
            }
            Self::Derive { providers } => {
                for provider in providers {
                    provider.map_type_ids(map);
                }
            }
        }
    }
}

/// One provider selected by a compiler-owned derive decorator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DeriveProvider {
    /// The source provider argument.
    pub argument: GlobalNodeId<Argument>,
    /// The selected provider backing.
    pub newtype: NewtypeSelection,
    /// The instantiated provider type.
    pub ty: GlobalTypeId,
}

impl DeriveProvider {
    /// Apply one mapping to every type id stored in this provider.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        self.newtype.map_type_ids(map);
        self.ty = map(self.ty);
    }
}
