use destack_dir::{
    LocalTypeId, PrimitiveType, ScalarLiteral, StaticKey, Type, TypeLiteral, TypeTable,
};

/// Canonical kind for index signature key matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IndexKeyKind {
    /// String index signature.
    String,
    /// Number index signature.
    Number,
    /// Symbol index signature.
    Symbol,
    /// Any other key type.
    Unknown,
}

/// Get the key kind for a type used in an index signature.
pub(super) fn index_key_kind_for_type(key_type: LocalTypeId, types: &TypeTable) -> IndexKeyKind {
    match types.get_type(key_type) {
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        } => IndexKeyKind::String,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        } => IndexKeyKind::Number,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Symbol),
        } => IndexKeyKind::Symbol,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::UniqueSymbol),
        } => IndexKeyKind::Symbol,
        _ => IndexKeyKind::Unknown,
    }
}

/// Get the key kind for an index expression value.
pub(super) fn index_key_kind_for_index(
    index_type: LocalTypeId,
    types: &TypeTable,
) -> Option<IndexKeyKind> {
    match types.get_type(index_type) {
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        } => Some(IndexKeyKind::String),
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        } => Some(IndexKeyKind::Number),
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Symbol),
        } => Some(IndexKeyKind::Symbol),
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::UniqueSymbol),
        } => Some(IndexKeyKind::Symbol),
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_)),
        } => Some(IndexKeyKind::String),
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_)),
        } => Some(IndexKeyKind::Number),
        _ => None,
    }
}

/// Get the key kind for a static member key.
pub(super) fn index_key_kind_for_member(member_key: &StaticKey) -> IndexKeyKind {
    match member_key {
        StaticKey::Name(_) => IndexKeyKind::String,
        StaticKey::Number(_) => IndexKeyKind::Number,
        StaticKey::UniqueSymbol(_) | StaticKey::GlobalSymbol(_) => IndexKeyKind::Symbol,
    }
}

/// Check if an index signature key kind is compatible with an access key kind.
pub(super) fn index_key_kinds_compatible(signature: IndexKeyKind, access: IndexKeyKind) -> bool {
    match (signature, access) {
        (IndexKeyKind::String, IndexKeyKind::String) => true,
        (IndexKeyKind::Number, IndexKeyKind::Number) => true,
        (IndexKeyKind::Symbol, IndexKeyKind::Symbol) => true,
        // string and number indexers are compatible in ts
        (IndexKeyKind::String, IndexKeyKind::Number) => true,
        (IndexKeyKind::Number, IndexKeyKind::String) => true,
        _ => false,
    }
}

/// Check if a field key is covered by an index signature key kind.
pub(super) fn field_key_matches_index_kind(key: &StaticKey, kind: IndexKeyKind) -> bool {
    match kind {
        IndexKeyKind::String => matches!(key, StaticKey::Name(_) | StaticKey::Number(_)),
        IndexKeyKind::Number => matches!(key, StaticKey::Number(_)),
        IndexKeyKind::Symbol => {
            matches!(key, StaticKey::UniqueSymbol(_) | StaticKey::GlobalSymbol(_))
        }
        IndexKeyKind::Unknown => false,
    }
}
