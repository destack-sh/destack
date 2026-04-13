use destack_source::{AdaptImage, ModuleId};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::{
    AttributionTable, ExtensionTable, GenericTable, HeritageTable, InstanceTable, ResolutionTable,
    TypeRelationCache,
};
use crate::{
    Arena, FloatType, GlobalSymbolId, IntType, IntrinsicType, LocalNodeId, LocalNodeIdAny,
    LocalTypeId, Node, PrimitiveType, ScalarLiteral, Type, TypeLiteral,
};

/// TypeTable stores all type-related analysis results for a module. NOT THREAD-SAFE.
/// NOTE #Cleanup #Architecture: revisit TypeTable.*_in_progress markers
#[derive(Debug, Clone, Serialize, Deserialize, AdaptImage)]
pub struct TypeTable {
    /// The module id of the type table.
    pub module_id: ModuleId,

    // types
    /// The next type id to allocate.
    pub(crate) next_type_id: u32,
    /// The types.
    pub(crate) types: Arena<Type>,
    /// The metadata for each type id.
    pub(crate) type_metadata_by_id: Vec<TypeMetadata>,
    /// The version number for symbol to type mappings.
    pub(crate) symbol_version_by_id: IndexMap<GlobalSymbolId, u64>,
    /// The relation and normalization caches.
    pub(crate) relation: TypeRelationCache,
    /// The static parameter metadata table.
    pub(crate) generic: GenericTable,
    /// The type attribution table.
    pub(crate) attribution: AttributionTable,
    /// The instance table.
    pub(crate) instance: InstanceTable,
    /// The resolution table.
    pub(crate) resolution: ResolutionTable,
    /// The heritage table.
    pub(crate) heritage: HeritageTable,
    /// The extension table.
    pub(crate) extension: ExtensionTable,
}

/// The provenance of one type slot in the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, AdaptImage)]
pub enum TypeOrigin {
    /// The type was produced in the local module.
    Local,
    /// The type was imported from another module.
    Imported,
}

/// The freshness state attached to one type slot.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord, AdaptImage,
)]
pub enum Freshness {
    /// No freshness information is attached.
    None,
    /// The type slot is regularized and should not widen at commitment.
    Regular,
    /// The type slot is fresh and may widen at commitment.
    Fresh,
}

/// The metadata stored for one type id.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, AdaptImage)]
pub struct TypeMetadata {
    /// The source node id that produced this type slot.
    pub source_id: LocalNodeIdAny,
    /// The provenance of this type slot.
    pub origin: TypeOrigin,
    /// The freshness state for this type slot.
    pub freshness: Freshness,
    /// The revision for this type slot.
    pub revision: u64,
}

/// Compact index key for instance interning candidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, AdaptImage)]
pub(super) struct InstanceInternerKey {
    /// The symbol that owns the instance.
    symbol_id: GlobalSymbolId,
    /// The number of static arguments.
    static_argument_count: usize,
}

/// Build one compact key for instance interning.
pub(super) fn instance_interner_key(
    symbol_id: GlobalSymbolId,
    static_argument_count: usize,
) -> InstanceInternerKey {
    InstanceInternerKey {
        symbol_id,
        static_argument_count,
    }
}

/// Canonical key for one static-parameter declaration slot.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, AdaptImage,
)]
pub(crate) struct StaticParameterSymbolKey {
    /// The defining module id for this static-parameter slot.
    module_id: ModuleId,
    /// The local symbol id for this static-parameter slot.
    local_symbol_id: u32,
}

/// Build one canonical key for static-parameter metadata.
pub(super) fn static_parameter_symbol_key(symbol_id: GlobalSymbolId) -> StaticParameterSymbolKey {
    StaticParameterSymbolKey {
        module_id: symbol_id.module_id,
        local_symbol_id: symbol_id.local_id.id,
    }
}

impl TypeTable {
    /// Create a new TypeTable.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            // types
            next_type_id: 0,
            types: Arena::new(),
            type_metadata_by_id: Vec::new(),
            symbol_version_by_id: IndexMap::new(),
            relation: TypeRelationCache::new(),
            generic: GenericTable::new(),
            attribution: AttributionTable::new(),
            instance: InstanceTable::new(),
            resolution: ResolutionTable::new(),
            heritage: HeritageTable::new(),
            extension: ExtensionTable::new(),
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

        let type_id = LocalTypeId::new(self.next_type_id);
        self.next_type_id += 1;

        self.types.allocate(ty);
        self.type_metadata_by_id.push(TypeMetadata {
            source_id,
            origin,
            freshness: Freshness::None,
            revision: 1,
        });
        self.initialize_relation_cache_for_new_type();
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
        let freshness = self.type_freshness(source_type_id);
        let type_id = self.allocate_type(ty, source_id, origin);
        if freshness != Freshness::None {
            self.set_type_freshness(type_id, freshness);
        }
        type_id
    }

    /// Get or insert a literal type id.
    pub fn intern_literal_type(
        &mut self,
        source_type_id: LocalTypeId,
        literal: TypeLiteral,
    ) -> LocalTypeId {
        let literal_hash = literal_hash_key(&literal);

        // reuse cached literal ids first
        if let Some(cached_entries) = self
            .relation
            .interner
            .literal_type_id_by_hash
            .get(&literal_hash)
        {
            for (cached_literal, cached_type_id) in cached_entries {
                if cached_literal == &literal {
                    return *cached_type_id;
                }
            }
        }

        // reuse an existing literal type when available
        for (index, ty) in self.types.iter().enumerate() {
            if let Type::TypeLiteral { value } = ty
                && value == &literal
            {
                let literal_type_id = LocalTypeId::new(index as u32);
                let literal_entries = self
                    .relation
                    .interner
                    .literal_type_id_by_hash
                    .entry(literal_hash)
                    .or_default();
                literal_entries.push((literal.clone(), literal_type_id));
                return literal_type_id;
            }
        }

        // insert a new literal type
        let literal_type_id = self.insert_type_from_type(
            Type::TypeLiteral {
                value: literal.clone(),
            },
            source_type_id,
        );
        let literal_entries = self
            .relation
            .interner
            .literal_type_id_by_hash
            .entry(literal_hash)
            .or_default();
        literal_entries.push((literal, literal_type_id));
        literal_type_id
    }

    /// Get a type by its id.
    pub fn get_type(&self, type_id: LocalTypeId) -> &Type {
        self.types.get(type_id.0)
    }

    /// Get a type by its id when present.
    pub fn get_type_maybe(&self, type_id: LocalTypeId) -> Option<&Type> {
        self.types.get_maybe(type_id.0)
    }

    /// Strip value wrapper types to reach the underlying type id.
    pub fn unwrap_value_type_id(&self, type_id: LocalTypeId) -> LocalTypeId {
        let mut current = type_id;
        loop {
            match self.get_type(current) {
                Type::Value { value } => current = *value,
                _ => return current,
            }
        }
    }

    /// Iterate over all type ids.
    pub fn iter_type_ids(&self) -> impl Iterator<Item = LocalTypeId> + '_ {
        (0..self.types.len()).map(|id| LocalTypeId::new(id as u32))
    }

    /// Get a mutable type by its id without bumping its version.
    pub fn get_type_mut(&mut self, type_id: LocalTypeId) -> &mut Type {
        self.types.get_mut(type_id.0)
    }

    /// Return metadata for a type id.
    fn type_metadata(&self, type_id: LocalTypeId) -> TypeMetadata {
        self.type_metadata_by_id
            .get(type_id.0 as usize)
            .copied()
            .unwrap_or_else(|| {
                panic!("missing type metadata for type id {type_id:?}");
            })
    }

    /// Return mutable metadata for a type id.
    fn type_metadata_mut(&mut self, type_id: LocalTypeId) -> &mut TypeMetadata {
        self.type_metadata_by_id
            .get_mut(type_id.0 as usize)
            .unwrap_or_else(|| {
                panic!("missing mutable type metadata for type id {type_id:?}");
            })
    }

    /// Update a type and bump its version.
    pub fn update_type(&mut self, type_id: LocalTypeId, ty: Type) {
        self.assert_type_table_invariants_debug("update_type:start");
        *self.types.get_mut(type_id.0) = ty;
        self.bump_type_version(type_id);
        self.assert_type_table_invariants_debug("update_type:end");
    }

    /// Return the current version for a type id.
    pub fn type_version(&self, type_id: LocalTypeId) -> u64 {
        self.type_metadata(type_id).revision
    }

    /// Bump the version for a type id.
    pub fn bump_type_version(&mut self, type_id: LocalTypeId) {
        let metadata = self.type_metadata_mut(type_id);
        metadata.revision = metadata.revision.wrapping_add(1);
    }

    /// Return the current version for a symbol mapping.
    pub fn symbol_version(&self, symbol_id: GlobalSymbolId) -> u64 {
        self.symbol_version_by_id
            .get(&symbol_id)
            .copied()
            .unwrap_or(0)
    }

    /// Bump the version for a symbol mapping.
    pub fn bump_symbol_version(&mut self, symbol_id: GlobalSymbolId) {
        let version = self.symbol_version_by_id.entry(symbol_id).or_insert(0);
        *version = version.wrapping_add(1);
    }

    /// Get the source id for a type.
    pub fn get_type_source(&self, type_id: LocalTypeId) -> LocalNodeIdAny {
        self.type_metadata(type_id).source_id
    }

    /// Get the provenance for a type.
    pub fn type_origin(&self, type_id: LocalTypeId) -> TypeOrigin {
        self.type_metadata(type_id).origin
    }

    /// Return true when a type originated from an imported module.
    pub fn is_imported_type(&self, type_id: LocalTypeId) -> bool {
        matches!(self.type_origin(type_id), TypeOrigin::Imported)
    }

    /// Return the freshness for one type slot.
    pub fn type_freshness(&self, type_id: LocalTypeId) -> Freshness {
        self.type_metadata(type_id).freshness
    }

    /// Return true when one type slot is marked as fresh.
    pub fn is_fresh_type(&self, type_id: LocalTypeId) -> bool {
        self.type_freshness(type_id) == Freshness::Fresh
    }

    /// Set the freshness for one type slot.
    pub fn set_type_freshness(&mut self, type_id: LocalTypeId, freshness: Freshness) {
        let metadata = self.type_metadata_mut(type_id);
        if metadata.freshness == freshness {
            return;
        }
        metadata.freshness = freshness;
        self.bump_type_version(type_id);
    }

    /// Get the number of types in the table.
    pub fn type_count(&self) -> u32 {
        self.next_type_id
    }

    /// Assert internal table invariants only in debug builds.
    fn assert_type_table_invariants_debug(&self, _context: &str) {
        #[cfg(debug_assertions)]
        self.assert_type_table_invariants(_context);
    }

    /// Assert internal table invariants for type storage and relation slots.
    #[cfg(debug_assertions)]
    fn assert_type_table_invariants(&self, context: &str) {
        let type_slot_count = self.types.len();
        let metadata_slot_count = self.type_metadata_by_id.len();
        let assign_cache_slot_count = self
            .relation
            .normalization
            .normalized_assignability_type_by_id
            .len();
        let flow_cache_slot_count = self.relation.normalization.normalized_flow_type_by_id.len();
        let next_type_id = self.next_type_id as usize;

        assert_eq!(
            metadata_slot_count, type_slot_count,
            "type metadata slot mismatch in {context}: metadata={metadata_slot_count}, types={type_slot_count}",
        );
        assert_eq!(
            assign_cache_slot_count, type_slot_count,
            "assign normalization slot mismatch in {context}: assign={assign_cache_slot_count}, types={type_slot_count}",
        );
        assert_eq!(
            flow_cache_slot_count, type_slot_count,
            "flow normalization slot mismatch in {context}: flow={flow_cache_slot_count}, types={type_slot_count}",
        );
        assert_eq!(
            next_type_id, type_slot_count,
            "next_type_id mismatch in {context}: next={next_type_id}, types={type_slot_count}",
        );
    }
}

/// Build one deterministic hash key for literal interning.
fn literal_hash_key(literal: &TypeLiteral) -> u64 {
    let mut hasher = DefaultHasher::new();
    hash_type_literal(literal, &mut hasher);
    hasher.finish()
}

/// Hash one type literal into the provided hasher.
fn hash_type_literal(literal: &TypeLiteral, hasher: &mut impl Hasher) {
    match literal {
        TypeLiteral::Never => 0u8.hash(hasher),
        TypeLiteral::Any => 1u8.hash(hasher),
        TypeLiteral::Infer => 2u8.hash(hasher),
        TypeLiteral::Undefined => 3u8.hash(hasher),
        TypeLiteral::Unknown => 4u8.hash(hasher),
        TypeLiteral::Object => 5u8.hash(hasher),
        TypeLiteral::Void => 6u8.hash(hasher),
        TypeLiteral::Null => 7u8.hash(hasher),
        TypeLiteral::Primitive(primitive) => {
            8u8.hash(hasher);
            hash_primitive_type(*primitive, hasher);
        }
        TypeLiteral::Intrinsic(intrinsic) => {
            9u8.hash(hasher);
            hash_intrinsic_type(*intrinsic, hasher);
        }
        TypeLiteral::ScalarLiteral(literal) => {
            10u8.hash(hasher);
            hash_scalar_literal(literal, hasher);
        }
    }
}

/// Hash one intrinsic type into the provided hasher.
fn hash_intrinsic_type(intrinsic: IntrinsicType, hasher: &mut impl Hasher) {
    match intrinsic {
        IntrinsicType::Uppercase => 0u8.hash(hasher),
        IntrinsicType::Lowercase => 1u8.hash(hasher),
        IntrinsicType::Capitalize => 2u8.hash(hasher),
        IntrinsicType::Uncapitalize => 3u8.hash(hasher),
        IntrinsicType::NoInfer => 4u8.hash(hasher),
        IntrinsicType::BuiltinIteratorReturn => 5u8.hash(hasher),
    }
}

/// Hash one primitive type into the provided hasher.
fn hash_primitive_type(primitive: PrimitiveType, hasher: &mut impl Hasher) {
    match primitive {
        PrimitiveType::Boolean => 0u8.hash(hasher),
        PrimitiveType::Character => 1u8.hash(hasher),
        PrimitiveType::String => 2u8.hash(hasher),
        PrimitiveType::Bigint => 3u8.hash(hasher),
        PrimitiveType::Number => 4u8.hash(hasher),
        PrimitiveType::Int(int_type) => {
            5u8.hash(hasher);
            hash_int_type(int_type, hasher);
        }
        PrimitiveType::Float(float_type) => {
            6u8.hash(hasher);
            hash_float_type(float_type, hasher);
        }
        PrimitiveType::Symbol => 7u8.hash(hasher),
        PrimitiveType::UniqueSymbol => 8u8.hash(hasher),
    }
}

/// Hash one int type into the provided hasher.
fn hash_int_type(int_type: IntType, hasher: &mut impl Hasher) {
    match int_type {
        IntType::Int8 => 0u8.hash(hasher),
        IntType::Int16 => 1u8.hash(hasher),
        IntType::Int32 => 2u8.hash(hasher),
        IntType::Int64 => 3u8.hash(hasher),
        IntType::Int128 => 4u8.hash(hasher),
        IntType::Int256 => 5u8.hash(hasher),
        IntType::Isize => 6u8.hash(hasher),
        IntType::Uint8 => 7u8.hash(hasher),
        IntType::Uint16 => 8u8.hash(hasher),
        IntType::Uint32 => 9u8.hash(hasher),
        IntType::Uint64 => 10u8.hash(hasher),
        IntType::Uint128 => 11u8.hash(hasher),
        IntType::Uint256 => 12u8.hash(hasher),
        IntType::Usize => 13u8.hash(hasher),
        IntType::Arbitrary { width, is_signed } => {
            14u8.hash(hasher);
            width.hash(hasher);
            is_signed.hash(hasher);
        }
    }
}

/// Hash one float type into the provided hasher.
fn hash_float_type(float_type: FloatType, hasher: &mut impl Hasher) {
    match float_type {
        FloatType::Float32 => 0u8.hash(hasher),
        FloatType::Float64 => 1u8.hash(hasher),
        FloatType::Arbitrary { width } => {
            2u8.hash(hasher);
            width.hash(hasher);
        }
    }
}

/// Hash one scalar literal into the provided hasher.
fn hash_scalar_literal(literal: &ScalarLiteral, hasher: &mut impl Hasher) {
    match literal {
        ScalarLiteral::Null => {
            0u8.hash(hasher);
        }
        ScalarLiteral::Boolean(value) => {
            1u8.hash(hasher);
            value.hash(hasher);
        }
        ScalarLiteral::Integer(value) => {
            2u8.hash(hasher);
            value.hash(hasher);
        }
        ScalarLiteral::Bigint(value) => {
            3u8.hash(hasher);
            value.hash(hasher);
        }
        ScalarLiteral::Float(value) => {
            4u8.hash(hasher);
            value.to_bits().hash(hasher);
        }
        ScalarLiteral::Character(value) => {
            5u8.hash(hasher);
            value.hash(hasher);
        }
        ScalarLiteral::String(value) => {
            6u8.hash(hasher);
            value.hash(hasher);
        }
        ScalarLiteral::RegexString { content, flags } => {
            7u8.hash(hasher);
            content.hash(hasher);
            flags.hash(hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_source::ModuleId;

    use super::{Freshness, TypeOrigin, TypeTable};
    use crate::{LocalNodeIdAny, NodeType, PrimitiveType, ScalarLiteral, Type, TypeLiteral};

    #[test]
    fn test_insert_type_from_any_tracks_local_type_metadata() {
        let mut types = TypeTable::new(ModuleId::EPHEMERAL);
        let source_id = LocalNodeIdAny::new(7, NodeType::Expression);
        let type_id = types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Any,
            },
            source_id,
        );

        assert_eq!(types.get_type_source(type_id), source_id);
        assert_eq!(types.type_origin(type_id), TypeOrigin::Local);
        assert!(!types.is_imported_type(type_id));
        assert_eq!(types.type_freshness(type_id), Freshness::None);
        assert_eq!(types.type_version(type_id), 1);
    }

    #[test]
    fn test_insert_imported_type_from_any_tracks_imported_type_metadata() {
        let mut types = TypeTable::new(ModuleId::EPHEMERAL);
        let source_id = LocalNodeIdAny::new(9, NodeType::Expression);
        let type_id = types.insert_imported_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number),
            },
            source_id,
        );

        assert_eq!(types.get_type_source(type_id), source_id);
        assert_eq!(types.type_origin(type_id), TypeOrigin::Imported);
        assert!(types.is_imported_type(type_id));
        assert_eq!(types.type_freshness(type_id), Freshness::None);
        assert_eq!(types.type_version(type_id), 1);
    }

    #[test]
    fn test_insert_type_from_type_preserves_origin_and_source_metadata() {
        let mut types = TypeTable::new(ModuleId::EPHEMERAL);
        let source_id = LocalNodeIdAny::new(13, NodeType::Expression);
        let source_type_id = types.insert_imported_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            },
            source_id,
        );

        let mapped_type_id = types.insert_type_from_type(
            Type::Array {
                element: Some(source_type_id),
                is_readonly: false,
            },
            source_type_id,
        );

        assert_eq!(types.get_type_source(mapped_type_id), source_id);
        assert_eq!(types.type_origin(mapped_type_id), TypeOrigin::Imported);
        assert_eq!(types.type_freshness(mapped_type_id), Freshness::None);
        assert_eq!(types.type_version(mapped_type_id), 1);
    }

    #[test]
    fn test_insert_type_from_type_preserves_type_freshness() {
        let mut types = TypeTable::new(ModuleId::EPHEMERAL);
        let source_id = LocalNodeIdAny::new(17, NodeType::Expression);
        let source_type_id = types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1)),
            },
            source_id,
        );
        types.set_type_freshness(source_type_id, Freshness::Fresh);

        let mapped_type_id = types.insert_type_from_type(
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(2)),
            },
            source_type_id,
        );

        assert_eq!(types.type_freshness(mapped_type_id), Freshness::Fresh);
    }

    #[test]
    fn test_update_type_bumps_type_metadata_revision() {
        let mut types = TypeTable::new(ModuleId::EPHEMERAL);
        let source_id = LocalNodeIdAny::new(21, NodeType::Expression);
        let type_id = types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            },
            source_id,
        );

        assert_eq!(types.type_version(type_id), 1);

        types.update_type(
            type_id,
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
        );

        assert_eq!(types.type_version(type_id), 2);
        assert_eq!(types.get_type_source(type_id), source_id);
        assert_eq!(types.type_origin(type_id), TypeOrigin::Local);
    }

    #[test]
    fn test_set_type_freshness_bumps_type_metadata_revision() {
        let mut types = TypeTable::new(ModuleId::EPHEMERAL);
        let source_id = LocalNodeIdAny::new(23, NodeType::Expression);
        let type_id = types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1)),
            },
            source_id,
        );

        assert_eq!(types.type_version(type_id), 1);
        assert_eq!(types.type_freshness(type_id), Freshness::None);

        types.set_type_freshness(type_id, Freshness::Fresh);

        assert_eq!(types.type_version(type_id), 2);
        assert_eq!(types.type_freshness(type_id), Freshness::Fresh);
    }
}
