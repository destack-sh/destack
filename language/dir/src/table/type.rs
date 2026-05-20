use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    Arena, GlobalNodeIdAny, GlobalSymbolId, IntersectionType, LocalNodeId, LocalNodeIdAny,
    LocalTypeId, Node, SegmentView, Type, UnionType,
};

/// Cumulative type slots for one DIR module.
#[derive(Debug, Clone)]
pub struct TypeTable<'a> {
    /// The module id of the type table.
    pub module_id: ModuleId,
    /// The ordered type table segments.
    segments: SegmentView<'a, TypeSegment>,
}

impl TypeTable<'static> {
    /// Create a type table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<TypeSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a type table from one segment.
    pub fn from_segment(segment: Arc<TypeSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> TypeTable<'a> {
    /// Create a type table from a segment view.
    pub fn from_view(segments: SegmentView<'a, TypeSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("type table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "type table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create a type table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b TypeSegment) -> TypeTable<'b> {
        TypeTable::from_view(self.segments.with_tail(tail))
    }

    /// Iterate effective checked types keyed by DIR node.
    pub fn node_types(&self) -> impl Iterator<Item = (GlobalNodeIdAny, LocalTypeId)> + '_ {
        let mut entries = IndexMap::new();

        // apply later segment values over earlier ones
        for segment in self.segments.iter() {
            for (node_id, type_id) in &segment.node_types {
                entries.insert(*node_id, *type_id);
            }
        }

        entries.into_iter()
    }

    /// Iterate solved symbol types.
    pub fn symbol_types(&self) -> impl Iterator<Item = (GlobalSymbolId, LocalTypeId)> + '_ {
        let mut entries = IndexMap::new();

        // apply later segment values over earlier ones
        for segment in self.segments.iter() {
            for (symbol_id, type_id) in &segment.symbol_types {
                entries.insert(*symbol_id, *type_id);
            }
        }

        entries.into_iter()
    }

    /// Get the effective checked type id for a node.
    pub fn get_node_type_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        for segment in self.segments.iter().rev() {
            if let Some(type_id) = segment.get_node_type_id(node_id) {
                return Some(type_id);
            }
        }

        None
    }

    /// Get the solved type id for a symbol.
    pub fn get_symbol_type_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalTypeId> {
        for segment in self.segments.iter().rev() {
            if let Some(type_id) = segment.get_symbol_type_id(symbol_id) {
                return Some(type_id);
            }
        }

        None
    }

    /// Get a type by its id.
    pub fn get_type(&self, type_id: LocalTypeId) -> &Type {
        self.get_type_maybe(type_id)
            .unwrap_or_else(|| panic!("DIR type {type_id:?} is not visible"))
    }

    /// Get a type by its id when present.
    pub fn get_type_maybe(&self, type_id: LocalTypeId) -> Option<&Type> {
        for segment in self.segments.iter().rev() {
            if let Some(ty) = segment.get_type_maybe(type_id) {
                return Some(ty);
            }
        }

        None
    }

    /// Strip outer form types to reach the payload type id.
    pub fn unwrap_form_payload_type_id(&self, type_id: LocalTypeId) -> LocalTypeId {
        let mut current = type_id;
        loop {
            match self.get_type(current) {
                Type::Form(form) => current = form.value,
                _ => return current,
            }
        }
    }

    /// Iterate over all type ids.
    pub fn iter_type_ids(&self) -> impl Iterator<Item = LocalTypeId> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.iter_type_ids())
    }

    /// Return the origin for a type id.
    pub fn type_origin(&self, type_id: LocalTypeId) -> TypeOrigin {
        self.type_source(type_id).origin
    }

    /// Get the source id for a type.
    pub fn get_type_source(&self, type_id: LocalTypeId) -> LocalNodeIdAny {
        self.type_source(type_id).source_id
    }

    /// Return true when a type originated from an imported module.
    pub fn is_imported_type(&self, type_id: LocalTypeId) -> bool {
        matches!(self.type_origin(type_id), TypeOrigin::Imported)
    }

    /// Get the number of types in the table.
    pub fn type_count(&self) -> u32 {
        self.segments
            .last()
            .map(|segment| segment.type_count())
            .unwrap_or(0)
    }

    /// Return the number of entries in this table.
    pub fn len(&self) -> u32 {
        self.type_count()
    }

    /// Return true when this table has no entries.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }

    /// Return the source for a type id.
    fn type_source(&self, type_id: LocalTypeId) -> TypeSource {
        for segment in self.segments.iter() {
            if segment.contains_type_id(type_id) {
                return segment.type_source(type_id);
            }
        }

        panic!("missing type source for type id {type_id:?}");
    }
}

/// Type slots added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeSegment {
    /// The module id of the type segment.
    pub module_id: ModuleId,
    /// The first type id owned by this table segment.
    pub(crate) first_type_id: u32,
    /// Canonical type entries.
    pub(crate) types: Arena<Type>,

    /// The source for each type id.
    pub(crate) sources: Arena<TypeSource>,
    /// Effective checked type keyed by DIR node occurrence.
    pub(crate) node_types: IndexMap<GlobalNodeIdAny, LocalTypeId>,
    /// Checked declaration type keyed by symbol.
    pub(crate) symbol_types: IndexMap<GlobalSymbolId, LocalTypeId>,
}

impl TypeSegment {
    /// Create a new type segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            first_type_id: 0,
            types: Arena::new(),
            sources: Arena::new(),
            node_types: IndexMap::new(),
            symbol_types: IndexMap::new(),
        }
    }

    /// Create a new empty segment after an existing type table segment.
    pub fn from_base(base: &Self) -> Self {
        Self {
            module_id: base.module_id,
            first_type_id: base.type_count(),
            types: Arena::new(),
            sources: Arena::new(),
            node_types: IndexMap::new(),
            symbol_types: IndexMap::new(),
        }
    }

    /// Allocate one type slot with canonical metadata.
    fn allocate_type(
        &mut self,
        ty: Type,
        source_id: LocalNodeIdAny,
        origin: TypeOrigin,
    ) -> LocalTypeId {
        self.assert_type_table_invariants_debug("allocate_type:start");

        let type_id = LocalTypeId::new(self.type_count());

        self.types.allocate(ty);
        self.sources.allocate(TypeSource { source_id, origin });
        self.assert_type_table_invariants_debug("allocate_type:end");

        type_id
    }

    /// Insert a type derived from some source node.
    pub fn insert_type_from<T: Node>(&mut self, ty: Type, node_id: LocalNodeId<T>) -> LocalTypeId {
        self.allocate_type(ty, node_id.into_any(), TypeOrigin::Local)
    }

    /// Insert a type derived from some source node (any node type).
    pub fn insert_type_from_any(&mut self, ty: Type, node_id: LocalNodeIdAny) -> LocalTypeId {
        self.allocate_type(ty, node_id, TypeOrigin::Local)
    }

    /// Insert a type that originates from an imported module.
    pub fn insert_imported_type_from_any(
        &mut self,
        ty: Type,
        node_id: LocalNodeIdAny,
    ) -> LocalTypeId {
        self.allocate_type(ty, node_id, TypeOrigin::Imported)
    }

    /// Insert a type derived from another type id.
    pub fn insert_type_from_type(&mut self, ty: Type, source_type_id: LocalTypeId) -> LocalTypeId {
        let source_id = self.get_type_source(source_type_id);
        let origin = self.type_origin(source_type_id);

        self.allocate_type(ty, source_id, origin)
    }

    /// Intern one union type by deterministic structural scan.
    pub fn intern_union_type(
        &mut self,
        source_type_id: LocalTypeId,
        elements: Vec<LocalTypeId>,
    ) -> LocalTypeId {
        for type_id in self.iter_type_ids() {
            if let Type::Union(existing) = self.get_type(type_id)
                && existing.elements == elements
            {
                return type_id;
            }
        }

        self.insert_type_from_type(Type::Union(UnionType { elements }), source_type_id)
    }

    /// Intern one intersection type by deterministic structural scan.
    pub fn intern_intersection_type(
        &mut self,
        source_type_id: LocalTypeId,
        elements: Vec<LocalTypeId>,
    ) -> LocalTypeId {
        for type_id in self.iter_type_ids() {
            if let Type::Intersection(existing) = self.get_type(type_id)
                && existing.elements == elements
            {
                return type_id;
            }
        }

        self.insert_type_from_type(
            Type::Intersection(IntersectionType { elements }),
            source_type_id,
        )
    }

    /// Iterate effective checked types keyed by DIR node.
    pub fn node_types(&self) -> impl Iterator<Item = (GlobalNodeIdAny, LocalTypeId)> + '_ {
        self.node_types
            .iter()
            .map(|(node_id, type_id)| (*node_id, *type_id))
    }

    /// Iterate solved symbol types.
    pub fn symbol_types(&self) -> impl Iterator<Item = (GlobalSymbolId, LocalTypeId)> + '_ {
        self.symbol_types
            .iter()
            .map(|(symbol_id, type_id)| (*symbol_id, *type_id))
    }

    /// Set the effective checked type for a node.
    pub fn set_node_type(&mut self, node_id: GlobalNodeIdAny, ty: LocalTypeId) {
        self.node_types.insert(node_id, ty);
    }

    /// Get the effective checked type id for a node.
    pub fn get_node_type_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.node_types.get(&node_id).copied()
    }

    /// Set the solved type for a symbol.
    pub fn set_symbol_type(&mut self, symbol_id: GlobalSymbolId, ty: LocalTypeId) {
        self.symbol_types.insert(symbol_id, ty);
    }

    /// Get the solved type id for a symbol.
    pub fn get_symbol_type_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalTypeId> {
        self.symbol_types.get(&symbol_id).copied()
    }

    /// Get a type by its id.
    pub fn get_type(&self, type_id: LocalTypeId) -> &Type {
        self.get_type_maybe(type_id)
            .unwrap_or_else(|| panic!("DIR type {type_id:?} is not allocated in this segment"))
    }

    /// Get a type by its id when present.
    pub fn get_type_maybe(&self, type_id: LocalTypeId) -> Option<&Type> {
        self.contains_type_id(type_id)
            .then(|| self.types.get(type_id.0 - self.first_type_id))
    }

    /// Strip outer form types to reach the payload type id.
    pub fn unwrap_form_payload_type_id(&self, type_id: LocalTypeId) -> LocalTypeId {
        let mut current = type_id;
        loop {
            match self.get_type(current) {
                Type::Form(form) => current = form.value,
                _ => return current,
            }
        }
    }

    /// Iterate over all type ids.
    pub fn iter_type_ids(&self) -> impl Iterator<Item = LocalTypeId> + '_ {
        let end = self.type_count();

        (self.first_type_id..end).map(LocalTypeId::new)
    }

    /// Get a mutable type by its id.
    pub fn get_type_mut(&mut self, type_id: LocalTypeId) -> &mut Type {
        assert!(
            self.contains_type_id(type_id),
            "DIR type {type_id:?} is not mutable in this segment"
        );

        self.types.get_mut(type_id.0 - self.first_type_id)
    }

    /// Return the source for a type id.
    fn type_source(&self, type_id: LocalTypeId) -> TypeSource {
        if self.contains_type_id(type_id) {
            let slot = type_id.0 - self.first_type_id;
            return *self.sources.get(slot);
        }

        panic!("missing type source for type id {type_id:?}");
    }

    /// Update a type in place.
    pub fn update_type(&mut self, type_id: LocalTypeId, ty: Type) {
        self.assert_type_table_invariants_debug("update_type:start");
        *self.get_type_mut(type_id) = ty;
        self.assert_type_table_invariants_debug("update_type:end");
    }

    /// Get the source id for a type.
    pub fn get_type_source(&self, type_id: LocalTypeId) -> LocalNodeIdAny {
        self.type_source(type_id).source_id
    }

    /// Return the origin for a type.
    pub fn type_origin(&self, type_id: LocalTypeId) -> TypeOrigin {
        self.type_source(type_id).origin
    }

    /// Return true when a type originated from an imported module.
    pub fn is_imported_type(&self, type_id: LocalTypeId) -> bool {
        matches!(self.type_origin(type_id), TypeOrigin::Imported)
    }

    /// Get the number of types in the table.
    pub fn type_count(&self) -> u32 {
        self.first_type_id + self.types.len() as u32
    }

    /// Return the number of entries in this table.
    pub fn len(&self) -> u32 {
        self.type_count()
    }

    /// Return true when this table has no entries.
    pub fn is_empty(&self) -> bool {
        self.types.is_empty() && self.node_types.is_empty() && self.symbol_types.is_empty()
    }

    /// Return whether this segment contains the given type id.
    fn contains_type_id(&self, type_id: LocalTypeId) -> bool {
        type_id.0 >= self.first_type_id && type_id.0 < self.type_count()
    }

    /// Assert internal table invariants only in debug builds.
    fn assert_type_table_invariants_debug(&self, _context: &str) {
        #[cfg(debug_assertions)]
        self.assert_type_table_invariants(_context);
    }

    /// Assert internal store invariants.
    #[cfg(debug_assertions)]
    fn assert_type_table_invariants(&self, context: &str) {
        let type_slot_count = self.types.len();
        let source_slot_count = self.sources.len();

        assert_eq!(
            source_slot_count, type_slot_count,
            "type source slot mismatch in {context}: source={source_slot_count}, types={type_slot_count}",
        );
    }
}

/// The origin of one type slot in the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeOrigin {
    /// The type was produced in the local module.
    Local,
    /// The type was imported from another module.
    Imported,
}

/// The source stored for one type id.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TypeSource {
    /// The source node id that produced this type slot.
    pub source_id: LocalNodeIdAny,
    /// The origin of this type slot.
    pub origin: TypeOrigin,
}
