use std::collections::{HashMap, HashSet};

use destack_dir::{
    Expression, GlobalSymbolId, LocalNodeIdAny, LocalTypeId, NormalizationMode, PrimitiveType,
    ScalarLiteral, StaticKey, StringId, SymbolKey, SymbolTable, Type, TypeField,
    TypeIndexSignature, TypeLiteral, TypeMappedModifiers, TypeMappedParameter, TypeModifier,
    TypeTable, TypeUnaryOperator,
};
use destack_workspace::{Module, ProfileId};

use super::key::KeySet;
use crate::Compiler;

/// A mapped key produced when expanding mapped types.
#[derive(Debug, Clone)]
enum MappedKey {
    /// A literal property key with its type representation.
    Field {
        key: StaticKey,
        key_type: LocalTypeId,
    },
    /// An index signature key kind with its type representation.
    Index {
        kind: MappedIndexKind,
        key_type: LocalTypeId,
    },
}

/// Canonical key kinds used for mapped types and `keyof`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum MappedIndexKind {
    /// String keys.
    String,
    /// Number keys.
    Number,
    /// Symbol keys.
    Symbol,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Normalize a `keyof` type expression.
    pub(super) fn normalize_keyof_type(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        right: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        mode: NormalizationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        // normalize the operand before extracting keys
        let normalized_right =
            self.normalize_type_inner(module, profile, right, symbols, types, mode, visited);

        // collect keys for the normalized operand
        let mut visited_keys = HashSet::new();
        let key_set = self.key_set_for_type(
            module,
            profile,
            normalized_right,
            symbols,
            types,
            mode,
            visited,
            &mut visited_keys,
        );

        // turn the key set into a union type
        let key_type_id = self.key_type_id_for_key_set(source_id, key_set, types);
        self.normalize_type_inner(module, profile, key_type_id, symbols, types, mode, visited)
    }

    /// Collect key information for a type id.
    fn key_set_for_type(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        mode: NormalizationMode,
        visited: &mut Vec<LocalTypeId>,
        visited_keys: &mut HashSet<LocalTypeId>,
    ) -> KeySet {
        // avoid cycles when traversing recursive types
        if !visited_keys.insert(type_id) {
            return KeySet::default();
        }

        let ty = types.get_type(type_id).clone();
        match ty {
            Type::TypeLiteral {
                value: TypeLiteral::Any,
            } => {
                // any exposes all key kinds
                let mut keys = KeySet::default();
                // seed all index kinds
                keys.insert_index_kind(MappedIndexKind::String);
                keys.insert_index_kind(MappedIndexKind::Number);
                keys.insert_index_kind(MappedIndexKind::Symbol);
                keys
            }
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            } => {
                // primitives contribute their index kind
                let mut keys = KeySet::default();
                // seed string index kind
                keys.insert_index_kind(MappedIndexKind::String);
                keys
            }
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number),
            } => {
                let mut keys = KeySet::default();
                // seed number index kind
                keys.insert_index_kind(MappedIndexKind::Number);
                keys
            }
            Type::TypeLiteral {
                value:
                    TypeLiteral::Primitive(PrimitiveType::Symbol)
                    | TypeLiteral::Primitive(PrimitiveType::UniqueSymbol),
            } => {
                let mut keys = KeySet::default();
                // seed symbol index kind
                keys.insert_index_kind(MappedIndexKind::Symbol);
                keys
            }
            Type::Object {
                fields,
                index_signatures,
                ..
            } => {
                // collect literal keys and index signatures
                self.key_set_for_object(&fields, &index_signatures, types)
            }
            Type::Tuple { elements } => {
                let mut keys = KeySet::default();
                // seed number index kind
                keys.insert_index_kind(MappedIndexKind::Number);
                // collect tuple element indexes as literal keys
                for (index, _) in elements.iter().enumerate() {
                    let index_id = self.program.strings.intern(&index.to_string());
                    keys.insert_literal(StaticKey::Number(index_id));
                }
                keys
            }
            Type::Array { .. } | Type::ArraySized { .. } => {
                let mut keys = KeySet::default();
                // seed number index kind
                keys.insert_index_kind(MappedIndexKind::Number);
                keys
            }
            Type::Reference { symbol, .. } => {
                // use instance shapes when possible
                if let Some(instance_id) = types.get_instance_type_id(symbol) {
                    return self.key_set_for_type(
                        module,
                        profile,
                        instance_id,
                        symbols,
                        types,
                        mode,
                        visited,
                        visited_keys,
                    );
                }
                KeySet::default()
            }
            Type::Union { elements } => {
                // seed the intersection with the first union member
                let mut iter = elements.into_iter();
                let Some(first) = iter.next() else {
                    return KeySet::default();
                };

                // intersect keys across union members
                let mut keys = self.key_set_for_type(
                    module,
                    profile,
                    self.normalize_type_inner(
                        module, profile, first, symbols, types, mode, visited,
                    ),
                    symbols,
                    types,
                    mode,
                    visited,
                    visited_keys,
                );
                for element_id in iter {
                    let element_keys = self.key_set_for_type(
                        module,
                        profile,
                        self.normalize_type_inner(
                            module, profile, element_id, symbols, types, mode, visited,
                        ),
                        symbols,
                        types,
                        mode,
                        visited,
                        visited_keys,
                    );
                    keys.intersect_with(&element_keys);
                }

                keys
            }
            Type::Intersection { elements } => {
                // union keys across intersection members
                let mut keys = KeySet::default();
                for element_id in elements {
                    let element_keys = self.key_set_for_type(
                        module,
                        profile,
                        self.normalize_type_inner(
                            module, profile, element_id, symbols, types, mode, visited,
                        ),
                        symbols,
                        types,
                        mode,
                        visited,
                        visited_keys,
                    );
                    keys.union_with(&element_keys);
                }
                keys
            }
            _ => KeySet::default(),
        }
    }

    /// Collect key information from an object type.
    fn key_set_for_object(
        &self,
        fields: &[TypeField],
        index_signatures: &[TypeIndexSignature],
        types: &TypeTable,
    ) -> KeySet {
        let mut keys = KeySet::default();

        // add literal field keys
        for field in fields {
            keys.insert_literal(field.key);
        }

        // add index signature key kinds
        for signature in index_signatures {
            if let Some(kind) = self.mapped_index_kind_for_type(signature.key_type, types) {
                keys.insert_index_kind(kind);
            }
        }

        keys
    }

    /// Convert a key set into a type id.
    fn key_type_id_for_key_set(
        &self,
        source_id: LocalNodeIdAny,
        key_set: KeySet,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let mut key_types = Vec::new();

        // emit literal key types
        for key in key_set.literal_keys() {
            let key_type_id = self.key_type_id_for_static_key(key, types);
            if !key_types.contains(&key_type_id) {
                key_types.push(key_type_id);
            }
        }

        // emit index key types
        if key_set.has_string {
            key_types.push(self.key_type_id_for_index_kind(MappedIndexKind::String, types));
        }
        if key_set.has_number {
            key_types.push(self.key_type_id_for_index_kind(MappedIndexKind::Number, types));
        }
        if key_set.has_symbol {
            key_types.push(self.key_type_id_for_index_kind(MappedIndexKind::Symbol, types));
        }

        // collapse to a single type when possible
        match key_types.len() {
            0 => types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Never,
                },
                source_id,
            ),
            1 => key_types[0],
            _ => types.insert_type_from_any(
                Type::Union {
                    elements: key_types,
                },
                source_id,
            ),
        }
    }

    /// Convert a static key into a literal key type.
    fn key_type_id_for_static_key(&self, key: StaticKey, types: &mut TypeTable) -> LocalTypeId {
        let ty = match key {
            StaticKey::Name(name) | StaticKey::Number(name) => Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(name)),
            },
            StaticKey::Symbol(SymbolKey::Unique(_) | SymbolKey::WellKnown(_)) => {
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::UniqueSymbol),
                }
            }
            StaticKey::Symbol(SymbolKey::Registry(_)) => Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Symbol),
            },
        };
        types.insert_type(ty)
    }

    /// Convert an index kind into a primitive key type.
    fn key_type_id_for_index_kind(
        &self,
        kind: MappedIndexKind,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let primitive = match kind {
            MappedIndexKind::String => PrimitiveType::String,
            MappedIndexKind::Number => PrimitiveType::Number,
            MappedIndexKind::Symbol => PrimitiveType::Symbol,
        };
        types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(primitive),
        })
    }

    /// Map a key type into a mapped index kind.
    fn mapped_index_kind_for_type(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<MappedIndexKind> {
        match types.get_type(type_id) {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            } => Some(MappedIndexKind::String),
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number),
            } => Some(MappedIndexKind::Number),
            Type::TypeLiteral {
                value:
                    TypeLiteral::Primitive(PrimitiveType::Symbol)
                    | TypeLiteral::Primitive(PrimitiveType::UniqueSymbol),
            } => Some(MappedIndexKind::Symbol),
            _ => None,
        }
    }

    /// Normalize a conditional type by evaluating its branches.
    pub(super) fn normalize_conditional_type(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        left: LocalTypeId,
        right: LocalTypeId,
        then_type: LocalTypeId,
        else_type: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        mode: NormalizationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        // normalize the condition operands
        let left = self.normalize_type_inner(module, profile, left, symbols, types, mode, visited);
        let right =
            self.normalize_type_inner(module, profile, right, symbols, types, mode, visited);

        // distribute over unions for conditional typing
        if let Type::Union { elements } = types.get_type(left).clone() {
            let mut branch_types = Vec::new();

            // evaluate each union element independently
            for element_id in elements {
                let branch = self.normalize_conditional_type(
                    module, profile, source_id, element_id, right, then_type, else_type, symbols,
                    types, mode, visited,
                );
                branch_types.push(branch);
            }

            // union the distributed results
            return match branch_types.len() {
                0 => types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Never,
                    },
                    source_id,
                ),
                1 => branch_types[0],
                _ => {
                    let union_id = types.insert_type_from_any(
                        Type::Union {
                            elements: branch_types,
                        },
                        source_id,
                    );
                    self.normalize_type_inner(
                        module, profile, union_id, symbols, types, mode, visited,
                    )
                }
            };
        }

        // choose the active branch based on assignability
        let options = self.analyze_context_options_for_module(module.id);
        let branch_id = if self
            .is_type_assignable(module, profile, symbols, right, left, types, &options)
            .is_assignable()
        {
            then_type
        } else {
            else_type
        };

        self.normalize_type_inner(module, profile, branch_id, symbols, types, mode, visited)
    }

    /// Normalize indexed access types by resolving the accessed value types.
    pub(super) fn normalize_index_type(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        left: LocalTypeId,
        index: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        mode: NormalizationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        // normalize operands before evaluating the index
        let left = self.normalize_type_inner(module, profile, left, symbols, types, mode, visited);
        let index =
            self.normalize_type_inner(module, profile, index, symbols, types, mode, visited);

        // split union index types into individual keys
        let index_types = match types.get_type(index) {
            Type::Union { elements } => elements.clone(),
            _ => vec![index],
        };
        let mut value_types = Vec::new();

        // compute the accessed type for each key
        for key_type_id in index_types {
            let value_type = self.index_access_type_for_key_type(left, key_type_id, types);
            if let Some(value_type) = value_type {
                value_types.push(value_type);
            } else {
                value_types.push(types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    },
                    source_id,
                ));
            }
        }

        // collapse the collected value types
        let combined = match value_types.len() {
            0 => types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                source_id,
            ),
            1 => value_types[0],
            _ => types.insert_type_from_any(
                Type::Union {
                    elements: value_types,
                },
                source_id,
            ),
        };
        self.normalize_type_inner(module, profile, combined, symbols, types, mode, visited)
    }

    /// Resolve an index access for a single key type.
    fn index_access_type_for_key_type(
        &self,
        left: LocalTypeId,
        key_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        // handle literal key access first
        if let Some(static_key) = self.static_key_from_type(key_type_id, types) {
            return self.index_access_for_literal_key(left, static_key, types);
        }

        // handle primitive index kinds
        if let Some(kind) = self.mapped_index_kind_for_type(key_type_id, types) {
            return self.index_access_for_index_kind(left, kind, types);
        }

        match types.get_type(key_type_id) {
            Type::TypeLiteral {
                value: TypeLiteral::Any | TypeLiteral::Unknown,
            } => Some(types.insert_type(Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            })),
            _ => None,
        }
    }

    /// Resolve a literal key access on a type id.
    fn index_access_for_literal_key(
        &self,
        type_id: LocalTypeId,
        key: StaticKey,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let ty = types.get_type(type_id).clone();
        match ty {
            Type::Union { elements } => {
                let mut value_types = Vec::new();

                // union values across elements
                for element_id in elements {
                    let value_type = self.index_access_for_literal_key(element_id, key, types)?;
                    value_types.push(value_type);
                }

                Some(self.union_type_ids_from_list(value_types, types))
            }
            Type::Intersection { elements } => {
                let mut value_types = Vec::new();

                // intersect values across elements
                for element_id in elements {
                    let value_type = self.index_access_for_literal_key(element_id, key, types)?;
                    value_types.push(value_type);
                }

                Some(self.intersection_type_ids_from_list(value_types, types))
            }
            _ => {
                let mut value_types = Vec::new();

                // add field type when present
                if let Some(field_type) = self.field_type_for_key(type_id, &key, types) {
                    value_types.push(field_type);
                }

                // add index signature fallbacks when compatible
                if let Some(kind) = self.mapped_index_kind_for_static_key(&key) {
                    let index_values =
                        self.index_signature_value_types_for_kind(type_id, kind, types);
                    value_types.extend(index_values);
                }

                if value_types.is_empty() {
                    None
                } else {
                    Some(self.union_type_ids_from_list(value_types, types))
                }
            }
        }
    }

    /// Resolve an index access for a primitive index kind.
    fn index_access_for_index_kind(
        &self,
        type_id: LocalTypeId,
        kind: MappedIndexKind,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let ty = types.get_type(type_id).clone();
        match ty {
            Type::Union { elements } => {
                let mut value_types = Vec::new();

                // union values across elements
                for element_id in elements {
                    let value_type = self.index_access_for_index_kind(element_id, kind, types)?;
                    value_types.push(value_type);
                }

                Some(self.union_type_ids_from_list(value_types, types))
            }
            Type::Intersection { elements } => {
                let mut value_types = Vec::new();

                // intersect values across elements
                for element_id in elements {
                    let value_type = self.index_access_for_index_kind(element_id, kind, types)?;
                    value_types.push(value_type);
                }

                Some(self.intersection_type_ids_from_list(value_types, types))
            }
            _ => {
                let mut value_types = Vec::new();

                // add field types compatible with this kind
                let field_values = self.field_types_for_index_kind(type_id, kind, types);
                value_types.extend(field_values);

                // add index signature types
                let index_values = self.index_signature_value_types_for_kind(type_id, kind, types);
                value_types.extend(index_values);

                if value_types.is_empty() {
                    None
                } else {
                    Some(self.union_type_ids_from_list(value_types, types))
                }
            }
        }
    }

    /// Convert a literal key type into a StaticKey.
    fn static_key_from_type(&self, type_id: LocalTypeId, types: &TypeTable) -> Option<StaticKey> {
        let Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(literal),
        } = types.get_type(type_id)
        else {
            return None;
        };

        match literal {
            ScalarLiteral::String(name) => Some(StaticKey::Name(*name)),
            ScalarLiteral::Integer(value) => {
                let name = self.program.strings.intern(&value.to_string());
                Some(StaticKey::Number(name))
            }
            ScalarLiteral::Float(value) => {
                let name = self.program.strings.intern(&value.to_string());
                Some(StaticKey::Number(name))
            }
            ScalarLiteral::Bigint(value) => {
                let name = self.program.strings.intern(&value.to_string());
                Some(StaticKey::Number(name))
            }
            _ => None,
        }
    }

    /// Map a static key into an index kind.
    fn mapped_index_kind_for_static_key(&self, key: &StaticKey) -> Option<MappedIndexKind> {
        match key {
            StaticKey::Name(_) => Some(MappedIndexKind::String),
            StaticKey::Number(_) => Some(MappedIndexKind::Number),
            StaticKey::Symbol(_) => Some(MappedIndexKind::Symbol),
        }
    }

    /// Resolve the field type for a specific key.
    fn field_type_for_key(
        &self,
        type_id: LocalTypeId,
        key: &StaticKey,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        // unwrap alias references before walking fields
        let type_id = self.unwrap_normalization_alias_reference(type_id, types);
        let mut field_types = Vec::new();
        let mut pending = vec![type_id];
        let mut visited = Vec::new();

        // walk object and intersection shapes
        while let Some(current_id) = pending.pop() {
            if visited.contains(&current_id) {
                continue;
            }
            visited.push(current_id);
            match types.get_type(current_id) {
                Type::Object { fields, .. } => {
                    for field in fields {
                        if field.key.matches(key) {
                            field_types.push(field.ty);
                        }
                    }
                }
                Type::Reference { symbol, .. } => {
                    if let Some(instance_id) = types.get_instance_type_id(*symbol) {
                        pending.push(instance_id);
                    }
                }
                Type::Intersection { elements } => {
                    for element_id in elements {
                        pending.push(*element_id);
                    }
                }
                _ => {}
            }
        }

        match field_types.len() {
            0 => None,
            1 => Some(field_types[0]),
            _ => Some(types.insert_type(Type::Intersection {
                elements: field_types,
            })),
        }
    }

    /// Collect field types compatible with an index kind.
    fn field_types_for_index_kind(
        &self,
        type_id: LocalTypeId,
        kind: MappedIndexKind,
        types: &mut TypeTable,
    ) -> Vec<LocalTypeId> {
        // unwrap alias references before collecting
        let type_id = self.unwrap_normalization_alias_reference(type_id, types);
        let mut field_types = Vec::new();
        let mut pending = vec![type_id];
        let mut visited = Vec::new();

        // traverse object and intersection shapes
        while let Some(current_id) = pending.pop() {
            if visited.contains(&current_id) {
                continue;
            }
            visited.push(current_id);
            match types.get_type(current_id) {
                Type::Object { fields, .. } => {
                    for field in fields {
                        if self.static_key_matches_index_kind(&field.key, kind) {
                            field_types.push(field.ty);
                        }
                    }
                }
                Type::Reference { symbol, .. } => {
                    if let Some(instance_id) = types.get_instance_type_id(*symbol) {
                        pending.push(instance_id);
                    }
                }
                Type::Intersection { elements } => {
                    for element_id in elements {
                        pending.push(*element_id);
                    }
                }
                _ => {}
            }
        }

        field_types
    }

    /// Collect index signature value types compatible with an index kind.
    fn index_signature_value_types_for_kind(
        &self,
        type_id: LocalTypeId,
        kind: MappedIndexKind,
        types: &mut TypeTable,
    ) -> Vec<LocalTypeId> {
        // unwrap alias references before collecting
        let type_id = self.unwrap_normalization_alias_reference(type_id, types);
        let mut value_types = Vec::new();
        let mut pending = vec![type_id];
        let mut visited = Vec::new();

        // traverse object and intersection shapes
        while let Some(current_id) = pending.pop() {
            if visited.contains(&current_id) {
                continue;
            }
            visited.push(current_id);
            match types.get_type(current_id) {
                Type::Object {
                    index_signatures, ..
                } => {
                    for signature in index_signatures {
                        let signature_kind =
                            self.mapped_index_kind_for_type(signature.key_type, types);
                        if let Some(signature_kind) = signature_kind
                            && self.index_kinds_compatible(signature_kind, kind)
                        {
                            value_types.push(signature.value_type);
                        }
                    }
                }
                Type::Reference { symbol, .. } => {
                    if let Some(instance_id) = types.get_instance_type_id(*symbol) {
                        pending.push(instance_id);
                    }
                }
                Type::Intersection { elements } => {
                    for element_id in elements {
                        pending.push(*element_id);
                    }
                }
                _ => {}
            }
        }

        value_types
    }

    /// Check if a key matches an index kind.
    fn static_key_matches_index_kind(&self, key: &StaticKey, kind: MappedIndexKind) -> bool {
        match kind {
            MappedIndexKind::String => matches!(key, StaticKey::Name(_) | StaticKey::Number(_)),
            MappedIndexKind::Number => matches!(key, StaticKey::Number(_)),
            MappedIndexKind::Symbol => {
                matches!(key, StaticKey::Symbol(_))
            }
        }
    }

    /// Check if an index signature kind can satisfy an access kind.
    fn index_kinds_compatible(&self, signature: MappedIndexKind, access: MappedIndexKind) -> bool {
        match (signature, access) {
            (MappedIndexKind::String, MappedIndexKind::String) => true,
            (MappedIndexKind::Number, MappedIndexKind::Number) => true,
            (MappedIndexKind::Symbol, MappedIndexKind::Symbol) => true,
            // string and number indexers are compatible in ts
            (MappedIndexKind::String, MappedIndexKind::Number) => true,
            (MappedIndexKind::Number, MappedIndexKind::String) => true,
            _ => false,
        }
    }

    /// Combine type ids into a union.
    fn union_type_ids_from_list(
        &self,
        mut type_ids: Vec<LocalTypeId>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // drop empty unions early
        if type_ids.is_empty() {
            return types.insert_type(Type::TypeLiteral {
                value: TypeLiteral::Never,
            });
        }

        // collapse single element unions
        if type_ids.len() == 1 {
            return type_ids[0];
        }

        // keep union elements unique
        type_ids.sort_by_key(|id| id.0);
        type_ids.dedup();
        types.insert_type(Type::Union { elements: type_ids })
    }

    /// Combine type ids into an intersection.
    fn intersection_type_ids_from_list(
        &self,
        mut type_ids: Vec<LocalTypeId>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // drop empty intersections early
        if type_ids.is_empty() {
            return types.insert_type(Type::TypeLiteral {
                value: TypeLiteral::Never,
            });
        }

        // collapse single element intersections
        if type_ids.len() == 1 {
            return type_ids[0];
        }

        // keep intersection elements unique
        type_ids.sort_by_key(|id| id.0);
        type_ids.dedup();
        types.insert_type(Type::Intersection { elements: type_ids })
    }

    /// Normalize mapped types into object shapes.
    pub(super) fn normalize_mapped_type(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        parameter: TypeMappedParameter,
        modifiers: TypeMappedModifiers,
        value: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        mode: NormalizationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        let TypeMappedParameter {
            name,
            constraint,
            key_remap,
        } = parameter;

        // resolve the mapped parameter symbol
        let parameter_symbol = self.resolve_mapped_parameter_symbol(
            module, profile, source_id, name, value, key_remap, symbols, types,
        );
        let Some(parameter_symbol) = parameter_symbol else {
            // fallback to normalized mapped type when symbol resolution fails
            let normalized_constraint = self
                .normalize_type_inner(module, profile, constraint, symbols, types, mode, visited);
            let normalized_key_remap = key_remap.map(|key_remap| {
                self.normalize_type_inner(module, profile, key_remap, symbols, types, mode, visited)
            });
            let normalized_value =
                self.normalize_type_inner(module, profile, value, symbols, types, mode, visited);
            let normalized = Type::Mapped {
                parameter: TypeMappedParameter {
                    name,
                    constraint: normalized_constraint,
                    key_remap: normalized_key_remap,
                },
                modifiers,
                value: normalized_value,
            };
            return types.insert_type_from_any(normalized, source_id);
        };

        // normalize the key constraint for evaluation
        let normalized_constraint =
            self.normalize_type_inner(module, profile, constraint, symbols, types, mode, visited);

        // collect mapped keys from the constraint
        let mut keys = Vec::new();
        self.collect_mapped_keys_for_type(normalized_constraint, types, &mut keys);
        if keys.is_empty() {
            // empty key set produces an empty object
            let normalized = Type::Object {
                fields: Vec::new(),
                call_signatures: Vec::new(),
                construct_signatures: Vec::new(),
                index_signatures: Vec::new(),
            };
            return types.insert_type_from_any(normalized, source_id);
        }

        // find the source type for modifier inheritance
        let source_type_id = self.mapped_source_type_id(parameter_symbol, constraint, value, types);

        // expand each mapped key into fields or index signatures
        let mut fields: Vec<TypeField> = Vec::new();
        let mut index_values: HashMap<MappedIndexKind, Vec<LocalTypeId>> = HashMap::new();
        for key in keys {
            let key_type_id = match &key {
                MappedKey::Field { key_type, .. } => *key_type,
                MappedKey::Index { key_type, .. } => *key_type,
            };

            // substitute the mapped parameter with the key type
            let mut substitutions = HashMap::new();
            substitutions.insert(parameter_symbol, key_type_id);
            let mut cache = HashMap::new();
            let substituted_value =
                self.substitute_static_parameters(value, &substitutions, types, &mut cache);
            let normalized_value = self.normalize_type_inner(
                module,
                profile,
                substituted_value,
                symbols,
                types,
                mode,
                visited,
            );

            // compute remapped keys when present
            let remapped_keys = if let Some(key_remap) = key_remap {
                let substituted_remap =
                    self.substitute_static_parameters(key_remap, &substitutions, types, &mut cache);
                let normalized_remap = self.normalize_type_inner(
                    module,
                    profile,
                    substituted_remap,
                    symbols,
                    types,
                    mode,
                    visited,
                );
                let mut remapped = Vec::new();
                self.collect_mapped_keys_for_type(normalized_remap, types, &mut remapped);
                remapped
            } else {
                vec![key.clone()]
            };

            // map every key into fields or index signatures
            for remapped_key in remapped_keys {
                match remapped_key {
                    MappedKey::Field { key, .. } => {
                        let (base_optional, base_readonly) = source_type_id
                            .and_then(|source| self.field_modifiers_for_key(source, &key, types))
                            .unwrap_or((false, false));
                        let (is_optional, is_readonly) =
                            self.apply_mapped_modifiers(modifiers, base_optional, base_readonly);

                        // merge the mapped field into the output set
                        if let Some(existing) =
                            fields.iter_mut().find(|field| field.key.matches(&key))
                        {
                            if existing.ty != normalized_value {
                                existing.ty = types.insert_type(Type::Union {
                                    elements: vec![existing.ty, normalized_value],
                                });
                            }
                            existing.is_optional = existing.is_optional && is_optional;
                            existing.is_readonly = existing.is_readonly && is_readonly;
                        } else {
                            fields.push(TypeField {
                                key,
                                ty: normalized_value,
                                is_optional,
                                is_readonly,
                            });
                        }
                    }
                    MappedKey::Index { kind, .. } => {
                        index_values.entry(kind).or_default().push(normalized_value);
                    }
                }
            }
        }

        // build index signatures for mapped index keys
        let mut index_signatures = Vec::new();
        for (kind, values) in index_values {
            let value_type_id = self.union_type_ids_from_list(values, types);
            let key_type_id = self.key_type_id_for_index_kind(kind, types);
            let base_readonly = source_type_id
                .and_then(|source| {
                    self.resolve_index_signature_readonly_for_kind(source, kind, types)
                })
                .unwrap_or(false);
            let (_, is_readonly) = self.apply_mapped_modifiers(modifiers, false, base_readonly);
            index_signatures.push(TypeIndexSignature {
                name,
                key_type: key_type_id,
                value_type: value_type_id,
                is_readonly,
            });
        }

        // build the normalized object type
        let normalized = Type::Object {
            fields,
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures,
        };
        types.insert_type_from_any(normalized, source_id)
    }

    /// Find the mapped parameter symbol in scope or referenced types.
    fn resolve_mapped_parameter_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        name: StringId,
        value: LocalTypeId,
        key_remap: Option<LocalTypeId>,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Option<GlobalSymbolId> {
        // try scope lookup first
        if let Ok(expression_id) = source_id.try_into_typed::<Expression>() {
            let tree = module.dir(profile).tree.read();
            let (_, scope, mark) = symbols.get_scope(expression_id, &tree);
            let key = StaticKey::Name(name);
            if let Some(symbol_id) = scope.find_up_to(key, mark).or_else(|| scope.find(key)) {
                return Some(GlobalSymbolId::new(module.id, symbol_id));
            }
        }

        // fall back to scanning referenced symbols
        let mut visited = HashSet::new();
        if let Some(symbol) = self.find_type_reference_symbol(
            module,
            profile,
            name,
            value,
            symbols,
            types,
            &mut visited,
        ) {
            return Some(symbol);
        }

        // try the key remap if present
        if let Some(key_remap) = key_remap
            && let Some(symbol) = self.find_type_reference_symbol(
                module,
                profile,
                name,
                key_remap,
                symbols,
                types,
                &mut visited,
            )
        {
            return Some(symbol);
        }

        None
    }

    /// Find a referenced symbol by name inside a type tree.
    fn find_type_reference_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        name: StringId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> Option<GlobalSymbolId> {
        // avoid cycles while traversing
        if !visited.insert(type_id) {
            return None;
        }

        // walk the referenced type structure
        match types.get_type(type_id) {
            Type::Reference { symbol, .. } => {
                // resolve the symbol name from the owning module
                let symbol_name = if symbol.module_id == module.id {
                    symbols.get_symbol(symbol.local_id).name()
                } else {
                    let remote_module = self.program.modules.get(symbol.module_id);
                    let remote_module = remote_module.read();
                    let remote_symbols = remote_module.dir(profile).symbols.read();
                    remote_symbols.get_symbol(symbol.local_id).name()
                };

                // compare the resolved name
                if symbol_name == Some(name) {
                    Some(*symbol)
                } else {
                    None
                }
            }
            Type::Value { value }
            | Type::Unary { right: value, .. }
            | Type::Mutable { right: value, .. }
            | Type::ValueOf { right: value, .. }
            | Type::ReferenceOf { right: value, .. }
            | Type::PointerOf { right: value, .. } => self
                .find_type_reference_symbol(module, profile, name, *value, symbols, types, visited),
            Type::Binary { left, right, .. } => self
                .find_type_reference_symbol(module, profile, name, *left, symbols, types, visited)
                .or_else(|| {
                    self.find_type_reference_symbol(
                        module, profile, name, *right, symbols, types, visited,
                    )
                }),
            Type::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => self
                .find_type_reference_symbol(module, profile, name, *left, symbols, types, visited)
                .or_else(|| {
                    self.find_type_reference_symbol(
                        module, profile, name, *right, symbols, types, visited,
                    )
                })
                .or_else(|| {
                    self.find_type_reference_symbol(
                        module, profile, name, *then_type, symbols, types, visited,
                    )
                })
                .or_else(|| {
                    self.find_type_reference_symbol(
                        module, profile, name, *else_type, symbols, types, visited,
                    )
                }),
            Type::Mapped {
                parameter, value, ..
            } => {
                let mapped_symbol = self.find_type_reference_symbol(
                    module,
                    profile,
                    name,
                    parameter.constraint,
                    symbols,
                    types,
                    visited,
                );
                mapped_symbol
                    .or_else(|| {
                        if let Some(key_remap) = parameter.key_remap {
                            self.find_type_reference_symbol(
                                module, profile, name, key_remap, symbols, types, visited,
                            )
                        } else {
                            None
                        }
                    })
                    .or_else(|| {
                        self.find_type_reference_symbol(
                            module, profile, name, *value, symbols, types, visited,
                        )
                    })
            }
            Type::Index { left, index } => self
                .find_type_reference_symbol(module, profile, name, *left, symbols, types, visited)
                .or_else(|| {
                    self.find_type_reference_symbol(
                        module, profile, name, *index, symbols, types, visited,
                    )
                }),
            Type::TemplateLiteral { spans, .. } => spans.iter().find_map(|span| {
                self.find_type_reference_symbol(
                    module, profile, name, *span, symbols, types, visited,
                )
            }),
            Type::Infer { constraint, .. } => constraint.and_then(|constraint| {
                self.find_type_reference_symbol(
                    module, profile, name, constraint, symbols, types, visited,
                )
            }),
            Type::Predicate { target, .. } => target.and_then(|target| {
                self.find_type_reference_symbol(
                    module, profile, name, target, symbols, types, visited,
                )
            }),
            Type::Array { element } => element.and_then(|element| {
                self.find_type_reference_symbol(
                    module, profile, name, element, symbols, types, visited,
                )
            }),
            Type::ArraySized { element, .. } => self.find_type_reference_symbol(
                module, profile, name, *element, symbols, types, visited,
            ),
            Type::Tuple { elements } => elements.iter().find_map(|element| {
                self.find_type_reference_symbol(
                    module, profile, name, element.ty, symbols, types, visited,
                )
            }),
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => fields
                .iter()
                .find_map(|field| {
                    self.find_type_reference_symbol(
                        module, profile, name, field.ty, symbols, types, visited,
                    )
                })
                .or_else(|| {
                    call_signatures.iter().find_map(|signature| {
                        self.find_type_reference_symbol(
                            module, profile, name, *signature, symbols, types, visited,
                        )
                    })
                })
                .or_else(|| {
                    construct_signatures.iter().find_map(|signature| {
                        self.find_type_reference_symbol(
                            module, profile, name, *signature, symbols, types, visited,
                        )
                    })
                })
                .or_else(|| {
                    index_signatures.iter().find_map(|signature| {
                        self.find_type_reference_symbol(
                            module,
                            profile,
                            name,
                            signature.key_type,
                            symbols,
                            types,
                            visited,
                        )
                        .or_else(|| {
                            self.find_type_reference_symbol(
                                module,
                                profile,
                                name,
                                signature.value_type,
                                symbols,
                                types,
                                visited,
                            )
                        })
                    })
                }),
            Type::Union { elements } | Type::Intersection { elements } => {
                elements.iter().find_map(|element| {
                    self.find_type_reference_symbol(
                        module, profile, name, *element, symbols, types, visited,
                    )
                })
            }
            _ => None,
        }
    }

    /// Derive the source type used for modifier inheritance.
    fn mapped_source_type_id(
        &self,
        parameter_symbol: GlobalSymbolId,
        constraint: LocalTypeId,
        value: LocalTypeId,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        // prefer `keyof T` constraints
        if let Type::Unary {
            operator: TypeUnaryOperator::Keyof,
            right,
        } = types.get_type(constraint)
        {
            return Some(*right);
        }

        // infer from `T[K]` value shapes
        if let Type::Index { left, index } = types.get_type(value)
            && let Type::Reference { symbol, .. } = types.get_type(*index)
            && *symbol == parameter_symbol
        {
            return Some(*left);
        }

        None
    }

    /// Collect mapped keys for a constraint type.
    fn collect_mapped_keys_for_type(
        &self,
        type_id: LocalTypeId,
        types: &mut TypeTable,
        keys: &mut Vec<MappedKey>,
    ) {
        let ty = types.get_type(type_id).clone();
        match ty {
            Type::Union { elements } => {
                // expand each union member
                for element_id in elements {
                    self.collect_mapped_keys_for_type(element_id, types, keys);
                }
            }
            Type::TypeLiteral {
                value: TypeLiteral::Any,
            } => {
                // include all index kinds for any
                keys.push(MappedKey::Index {
                    kind: MappedIndexKind::String,
                    key_type: self.key_type_id_for_index_kind(MappedIndexKind::String, types),
                });
                keys.push(MappedKey::Index {
                    kind: MappedIndexKind::Number,
                    key_type: self.key_type_id_for_index_kind(MappedIndexKind::Number, types),
                });
                keys.push(MappedKey::Index {
                    kind: MappedIndexKind::Symbol,
                    key_type: self.key_type_id_for_index_kind(MappedIndexKind::Symbol, types),
                });
            }
            Type::TypeLiteral {
                value:
                    TypeLiteral::Primitive(PrimitiveType::String)
                    | TypeLiteral::Primitive(PrimitiveType::Number)
                    | TypeLiteral::Primitive(PrimitiveType::Symbol)
                    | TypeLiteral::Primitive(PrimitiveType::UniqueSymbol),
            } => {
                // include the matching primitive index kind
                if let Some(kind) = self.mapped_index_kind_for_type(type_id, types) {
                    keys.push(MappedKey::Index {
                        kind,
                        key_type: type_id,
                    });
                }
            }
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(_),
            } => {
                // include literal keys for scalar literal types
                if let Some(key) = self.static_key_from_type(type_id, types) {
                    keys.push(MappedKey::Field {
                        key,
                        key_type: type_id,
                    });
                }
            }
            _ => {}
        }
    }

    /// Apply mapped type modifiers to base modifiers. Returns (is_optional, is_readonly).
    fn apply_mapped_modifiers(
        &self,
        modifiers: TypeMappedModifiers,
        base_optional: bool,
        base_readonly: bool,
    ) -> (bool, bool) {
        // resolve optional modifiers
        let is_optional = match modifiers.optional {
            TypeModifier::Add => true,
            TypeModifier::Remove => false,
            TypeModifier::None => base_optional,
        };
        // resolve readonly modifiers
        let is_readonly = match modifiers.readonly {
            TypeModifier::Add => true,
            TypeModifier::Remove => false,
            TypeModifier::None => base_readonly,
        };

        (is_optional, is_readonly)
    }

    /// Resolve base modifiers for a field key on a type.
    fn field_modifiers_for_key(
        &self,
        type_id: LocalTypeId,
        key: &StaticKey,
        types: &mut TypeTable,
    ) -> Option<(bool, bool)> {
        // unwrap alias references before walking
        let type_id = self.unwrap_normalization_alias_reference(type_id, types);
        let mut is_optional = true;
        let mut is_readonly = true;
        let mut found = false;
        let mut pending = vec![type_id];
        let mut visited = Vec::new();

        // traverse object and intersection shapes
        while let Some(current_id) = pending.pop() {
            if visited.contains(&current_id) {
                continue;
            }
            visited.push(current_id);
            match types.get_type(current_id) {
                Type::Object { fields, .. } => {
                    for field in fields {
                        if field.key.matches(key) {
                            found = true;
                            is_optional = is_optional && field.is_optional;
                            is_readonly = is_readonly && field.is_readonly;
                        }
                    }
                }
                Type::Reference { symbol, .. } => {
                    if let Some(instance_id) = types.get_instance_type_id(*symbol) {
                        pending.push(instance_id);
                    }
                }
                Type::Intersection { elements } => {
                    for element_id in elements {
                        pending.push(*element_id);
                    }
                }
                _ => {}
            }
        }

        found.then_some((is_optional, is_readonly))
    }

    /// Resolve readonly modifiers for index signatures of a given kind.
    fn resolve_index_signature_readonly_for_kind(
        &self,
        type_id: LocalTypeId,
        kind: MappedIndexKind,
        types: &mut TypeTable,
    ) -> Option<bool> {
        // unwrap alias references before walking
        let type_id = self.unwrap_normalization_alias_reference(type_id, types);
        let mut is_readonly = true;
        let mut found = false;
        let mut pending = vec![type_id];
        let mut visited = Vec::new();

        // traverse object and intersection shapes
        while let Some(current_id) = pending.pop() {
            if visited.contains(&current_id) {
                continue;
            }
            visited.push(current_id);
            match types.get_type(current_id) {
                Type::Object {
                    index_signatures, ..
                } => {
                    for signature in index_signatures {
                        let signature_kind =
                            self.mapped_index_kind_for_type(signature.key_type, types);
                        if let Some(signature_kind) = signature_kind
                            && self.index_kinds_compatible(signature_kind, kind)
                        {
                            found = true;
                            is_readonly = is_readonly && signature.is_readonly;
                        }
                    }
                }
                Type::Reference { symbol, .. } => {
                    if let Some(instance_id) = types.get_instance_type_id(*symbol) {
                        pending.push(instance_id);
                    }
                }
                Type::Intersection { elements } => {
                    for element_id in elements {
                        pending.push(*element_id);
                    }
                }
                _ => {}
            }
        }

        found.then_some(is_readonly)
    }
}
