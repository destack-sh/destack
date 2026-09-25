use std::sync::Arc;
use tspp_serde::Reflect;

use serde::{Deserialize, Serialize};
use tspp_core::FxIndexMap as IndexMap;
use tspp_source::ModuleId;

use crate::{
    Arena, Argument, ArgumentBinding, AutoInterface, Decorator, Expression, GenericArgument,
    GlobalNodeId, GlobalNodeIdAny, GlobalStaticId, GlobalSymbolId, GlobalTypeId, InstanceKey,
    LanguageItem, LocalNodeId, SegmentView, TypeFold,
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
    /// Declared decorator uses awaiting selection and evaluation.
    pub(crate) uses: Vec<DecoratorUse>,
}

impl DecoratorSegment {
    /// Create an empty decorator segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            first_application_id: 0,
            applications: Arena::new(),
            applications_by_owner: IndexMap::default(),
            uses: Vec::new(),
        }
    }

    /// Create a new empty segment after an existing decorator table segment.
    pub fn from_base(base: &Self) -> Self {
        Self {
            module_id: base.module_id,
            first_application_id: base.application_count(),
            applications: Arena::new(),
            applications_by_owner: IndexMap::default(),
            uses: Vec::new(),
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

    /// Insert one declared decorator use.
    pub fn insert_use(&mut self, decorator_use: DecoratorUse) {
        self.uses.push(decorator_use);
    }

    /// Iterate declared decorator uses.
    pub fn iter_uses(&self) -> impl Iterator<Item = &DecoratorUse> + '_ {
        self.uses.iter()
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

impl TypeFold for DecoratorSegment {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        for application in self.applications.iter_mut() {
            application.map_types(map)?;
        }

        Ok(())
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

/// Declared decorator use awaiting selection and evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DecoratorUse {
    /// The decorator node.
    pub source: GlobalNodeId<Decorator>,
    /// The node decorated by this use.
    pub owner: GlobalNodeIdAny,
    /// The decorator target expression.
    pub target: GlobalNodeId<Expression>,
    /// The resolved decorator symbol.
    pub symbol: GlobalSymbolId,
    /// The explicit generic arguments.
    pub generic_arguments: Vec<LocalNodeId<GenericArgument>>,
    /// The decorator application arguments.
    pub arguments: Vec<LocalNodeId<Argument>>,
}

/// Checked decorator application attached to one owner node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
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

/// Checked resolution of one decorator application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct DecoratorResolution {
    /// The resolved decorator declaration.
    pub target: DecoratorTarget,
    /// The selection used to construct the decorator value.
    pub selection: DecoratorSelection,
    /// The nominal decorator value type.
    pub ty: GlobalTypeId,
}

/// Resolved decorator declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
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

/// InstanceKey used to construct one checked decorator value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold)]
pub enum DecoratorSelection {
    /// Arguments matched against one newtype backing.
    Newtype {
        /// The selected newtype declaration and its generic arguments.
        key: InstanceKey,
        /// The selected instantiated backing alternative.
        backing: GlobalTypeId,
        /// The source arguments bound to the selected parameters.
        arguments: Vec<ArgumentBinding>,
    },
    /// Interfaces selected by the compiler-owned derive dispatcher.
    Derive {
        /// The selected interfaces in argument order.
        interfaces: Vec<AutoInterface>,
    },
}
