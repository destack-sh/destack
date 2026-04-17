use std::collections::{HashMap, HashSet};

use destack_dir::{
    GlobalSymbolId, LocalNodeIdAny, LocalTypeId, MappedTypeModifier, MappedTypeModifiers,
    MappedTypeParameter, NormalizationMode, PrimitiveType, ScalarLiteral, StaticKey, SymbolKey,
    SymbolType, Type, TypeField, TypeIndexSignature, TypeLiteral, TypeTable,
};

use super::key::KeySet;
use super::template::TemplateLiteralKeyShape;
use crate::analyze::common::{CanonicalSymbolMode, RelationMode, TypeContext};
use crate::{AnalyzeError, Compiler};

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

/// The resolved value types and missing keys for an index access.
#[derive(Debug, Clone)]
pub(crate) struct IndexAccessResolution {
    /// The resolved value types for the access.
    pub(crate) value_types: Vec<LocalTypeId>,
    /// The missing literal keys encountered during lookup.
    pub(crate) missing_keys: Vec<StaticKey>,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Report one recursive type instantiation for alias-driven mapped traversal.
    fn report_recursive_mapped_instantiation(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
    ) {
        let node = ctx
            .types
            .get_type_source(type_id)
            .into_global(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::RecursiveTypeInstantiation { node });
    }

    /// Normalize a `keyof` type expression.
    pub(crate) fn normalize_keyof_type(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        keyof_type_id: Option<LocalTypeId>,
        right: LocalTypeId,
        mode: NormalizationMode,
        _relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        // key queries should always use type operations semantics
        let relation_mode = RelationMode::TYPE_OPERATOR;

        // preserve keyof when the operand still depends on evaluation
        let (needs_evaluation, normalize_mode) =
            self.keyof_normalization_policy(ctx.type_view(), right, mode);

        // normalize the operand before extracting keys
        let normalized_right = self.normalize_type_inner(
            &mut ctx.reborrow(),
            right,
            normalize_mode,
            relation_mode,
            visited,
        );

        if needs_evaluation {
            if let Some(keyof_type_id) = keyof_type_id
                && normalized_right == right
            {
                return keyof_type_id;
            }
            let normalized = Type::KeyOf {
                target_type: normalized_right,
            };
            return ctx.types.insert_type_from_any(normalized, source_id);
        }

        // treat unconstrained type parameters as keyof any
        if let Type::Reference { symbol, .. } = ctx.types.get_type(normalized_right) {
            let symbol = *symbol;
            let is_static_parameter =
                self.symbol_is_static_parameter(ctx.symbol_type_view(), symbol);
            if is_static_parameter
                && let Some(constraint_id) =
                    self.static_parameter_constraint_type(&mut ctx.reborrow(), symbol, source_id)
                && matches!(
                    ctx.types.get_type(constraint_id),
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    }
                )
            {
                let mut key_set = KeySet::default();
                key_set.insert_index_kind(MappedIndexKind::String);
                key_set.insert_index_kind(MappedIndexKind::Number);
                key_set.insert_index_kind(MappedIndexKind::Symbol);
                return self.key_type_id_for_key_set(source_id, key_set, ctx.types);
            }
        }

        // collect keys for the normalized operand
        let mut visited_keys = HashSet::new();
        let key_set = self.key_set_for_type(
            &mut ctx.reborrow(),
            normalized_right,
            mode,
            relation_mode,
            visited,
            &mut visited_keys,
        );

        // turn the key set into a union type
        let key_type_id = self.key_type_id_for_key_set(source_id, key_set, ctx.types);
        self.normalize_type_inner(
            &mut ctx.reborrow(),
            key_type_id,
            mode,
            relation_mode,
            visited,
        )
    }

    /// Collect key information for a type id.
    fn key_set_for_type(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        mode: NormalizationMode,
        relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
        visited_keys: &mut HashSet<LocalTypeId>,
    ) -> KeySet {
        // resolve apparent types for key extraction
        let type_id = self.apparent_type(&mut ctx.reborrow(), type_id, relation_mode);

        // avoid cycles when traversing recursive types
        if !visited_keys.insert(type_id) {
            self.report_recursive_mapped_instantiation(&mut ctx.reborrow(), type_id);
            return KeySet::default();
        }

        let ty = ctx.types.get_type(type_id).clone();
        match ty {
            Type::TypeLiteral {
                value: TypeLiteral::Any,
            } => {
                // any exposes all key kinds
                self.key_set_for_all_index_kinds()
            }
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            } => {
                // primitives contribute their index kind
                self.key_set_for_index_kind(MappedIndexKind::String)
            }
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number),
            } => self.key_set_for_index_kind(MappedIndexKind::Number),
            Type::TypeLiteral {
                value:
                    TypeLiteral::Primitive(PrimitiveType::Symbol)
                    | TypeLiteral::Primitive(PrimitiveType::UniqueSymbol),
            } => self.key_set_for_index_kind(MappedIndexKind::Symbol),
            Type::Object {
                fields,
                index_signatures,
                ..
            } => {
                // collect literal keys and index signatures
                self.key_set_for_object(&fields, &index_signatures, ctx.types)
            }
            Type::Tuple { elements, .. } => {
                let mut keys = self.key_set_for_index_kind(MappedIndexKind::Number);
                // collect tuple element indexes as literal keys
                for (index, _) in elements.iter().enumerate() {
                    let index_id = self.repository.strings.intern(&index.to_string());
                    keys.insert_literal(StaticKey::Number(index_id));
                }
                keys
            }
            Type::Array { .. } | Type::ArraySized { .. } => {
                self.key_set_for_index_kind(MappedIndexKind::Number)
            }
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                let symbol = self.canonical_symbol_id(
                    ctx.module_symbol_view(),
                    symbol,
                    CanonicalSymbolMode::PreserveAliases,
                );

                // expand alias references with arguments when available
                if matches!(symbol.ty(), SymbolType::TypeAlias | SymbolType::Newtype)
                    && let Some(static_arguments) = static_arguments.as_ref()
                {
                    let source_id = ctx.types.get_type_source(type_id);
                    if let Some(expanded_id) = self.normalize_type_alias_reference_with_arguments(
                        &mut ctx.reborrow(),
                        source_id,
                        symbol,
                        static_arguments,
                        mode,
                        relation_mode,
                        visited,
                    ) {
                        return self.key_set_for_type(
                            &mut ctx.reborrow(),
                            expanded_id,
                            mode,
                            relation_mode,
                            visited,
                            visited_keys,
                        );
                    }
                }

                // resolve the apparent instance type for shape queries
                let source_id = ctx.types.get_type_source(type_id);
                if let Some(apparent_id) =
                    self.apparent_instance_type(&mut ctx.reborrow(), source_id, symbol)
                {
                    return self.key_set_for_type(
                        &mut ctx.reborrow(),
                        apparent_id,
                        mode,
                        relation_mode,
                        visited,
                        visited_keys,
                    );
                }

                // fall back to all key kinds
                self.key_set_for_all_index_kinds()
            }
            Type::Union { elements } => {
                // seed the intersection with the first union member
                let mut iter = elements.into_iter();
                let Some(first) = iter.next() else {
                    return KeySet::default();
                };

                // intersect keys across union members
                let normalized_first = self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    first,
                    mode,
                    relation_mode,
                    visited,
                );
                let mut keys = self.key_set_for_type(
                    &mut ctx.reborrow(),
                    normalized_first,
                    mode,
                    relation_mode,
                    visited,
                    visited_keys,
                );
                for element_id in iter {
                    let normalized_element = self.normalize_type_inner(
                        &mut ctx.reborrow(),
                        element_id,
                        mode,
                        relation_mode,
                        visited,
                    );
                    let element_keys = self.key_set_for_type(
                        &mut ctx.reborrow(),
                        normalized_element,
                        mode,
                        relation_mode,
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
                    let normalized_element = self.normalize_type_inner(
                        &mut ctx.reborrow(),
                        element_id,
                        mode,
                        relation_mode,
                        visited,
                    );
                    let element_keys = self.key_set_for_type(
                        &mut ctx.reborrow(),
                        normalized_element,
                        mode,
                        relation_mode,
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

    /// Build a key set for a single index kind.
    fn key_set_for_index_kind(&self, kind: MappedIndexKind) -> KeySet {
        // seed the requested index kind
        let mut keys = KeySet::default();
        keys.insert_index_kind(kind);
        keys
    }

    /// Build a key set with all index kinds.
    fn key_set_for_all_index_kinds(&self) -> KeySet {
        // seed all index kinds
        let mut keys = KeySet::default();
        keys.insert_index_kind(MappedIndexKind::String);
        keys.insert_index_kind(MappedIndexKind::Number);
        keys.insert_index_kind(MappedIndexKind::Symbol);
        keys
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
                if kind == MappedIndexKind::String {
                    keys.insert_index_kind(MappedIndexKind::Number);
                }
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
            let key_type_id = self.key_type_id_for_static_key(key, source_id, types);
            if !key_types.contains(&key_type_id) {
                key_types.push(key_type_id);
            }
        }

        // emit index key types
        if key_set.has_string {
            key_types.push(self.key_type_id_for_index_kind(
                MappedIndexKind::String,
                source_id,
                types,
            ));
        }
        if key_set.has_number {
            key_types.push(self.key_type_id_for_index_kind(
                MappedIndexKind::Number,
                source_id,
                types,
            ));
        }
        if key_set.has_symbol {
            key_types.push(self.key_type_id_for_index_kind(
                MappedIndexKind::Symbol,
                source_id,
                types,
            ));
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
    fn key_type_id_for_static_key(
        &self,
        key: StaticKey,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> LocalTypeId {
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
        types.insert_type_from_any(ty, source_id)
    }

    /// Convert an index kind into a primitive key type.
    fn key_type_id_for_index_kind(
        &self,
        kind: MappedIndexKind,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let primitive = match kind {
            MappedIndexKind::String => PrimitiveType::String,
            MappedIndexKind::Number => PrimitiveType::Number,
            MappedIndexKind::Symbol => PrimitiveType::Symbol,
        };
        types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(primitive),
            },
            source_id,
        )
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
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        source_id: LocalNodeIdAny,
        distributive_symbol: Option<GlobalSymbolId>,
        left: LocalTypeId,
        right: LocalTypeId,
        then_type: LocalTypeId,
        else_type: LocalTypeId,
        mode: NormalizationMode,
        _relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        // conditional ctx.types use type operations semantics
        let relation_mode = RelationMode::TYPE_OPERATOR;

        // normalize the condition operands
        let original_left = left;
        let original_right = right;
        let original_then = then_type;
        let original_else = else_type;
        let left =
            self.normalize_type_inner(&mut ctx.reborrow(), left, mode, relation_mode, visited);
        let right =
            self.normalize_type_inner(&mut ctx.reborrow(), right, mode, relation_mode, visited);

        // keep conditional types unresolved when they depend on static parameters
        let left_contains_static_parameters =
            self.type_contains_static_parameters(ctx.type_view(), left, &mut HashSet::new());
        if left_contains_static_parameters {
            let normalized_then = self.normalize_type_inner(
                &mut ctx.reborrow(),
                then_type,
                mode,
                relation_mode,
                visited,
            );
            let normalized_else = self.normalize_type_inner(
                &mut ctx.reborrow(),
                else_type,
                mode,
                relation_mode,
                visited,
            );
            let did_change = left != original_left
                || right != original_right
                || normalized_then != original_then
                || normalized_else != original_else;
            if !did_change {
                return type_id;
            }

            return ctx.types.insert_type_from_any(
                Type::Conditional {
                    distributive_symbol,
                    left,
                    right,
                    then_type: normalized_then,
                    else_type: normalized_else,
                },
                source_id,
            );
        }

        // distribute over unions for conditional typing
        if let Some(distributive_symbol) = distributive_symbol
            && let Type::Union { elements } = ctx.types.get_type(left).clone()
        {
            let mut branch_types = Vec::new();

            // evaluate each union element independently
            for element_id in elements {
                let mut substitutions = HashMap::new();
                substitutions.insert(distributive_symbol, element_id);
                let mut cache = HashMap::new();
                let mapped_right =
                    self.substitute_static_parameters(right, &substitutions, ctx.types, &mut cache);
                let mapped_then = self.substitute_static_parameters(
                    then_type,
                    &substitutions,
                    ctx.types,
                    &mut cache,
                );
                let mapped_else = self.substitute_static_parameters(
                    else_type,
                    &substitutions,
                    ctx.types,
                    &mut cache,
                );
                let branch = self.normalize_conditional_type(
                    &mut ctx.reborrow(),
                    type_id,
                    source_id,
                    None,
                    element_id,
                    mapped_right,
                    mapped_then,
                    mapped_else,
                    mode,
                    relation_mode,
                    visited,
                );
                branch_types.push(branch);
            }

            // union the distributed results
            return match branch_types.len() {
                0 => ctx.types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Never,
                    },
                    source_id,
                ),
                1 => branch_types[0],
                _ => {
                    let union_id = ctx.types.insert_type_from_any(
                        Type::Union {
                            elements: branch_types,
                        },
                        source_id,
                    );
                    self.normalize_type_inner(
                        &mut ctx.reborrow(),
                        union_id,
                        mode,
                        relation_mode,
                        visited,
                    )
                }
            };
        }

        // distribute over never as an empty union
        if distributive_symbol.is_some()
            && matches!(
                ctx.types.get_type(left),
                Type::TypeLiteral {
                    value: TypeLiteral::Never,
                }
            )
        {
            return ctx.types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Never,
                },
                source_id,
            );
        }

        // substitute distributive symbols into the branch types when needed
        let (right, then_type, else_type) = if let Some(distributive_symbol) = distributive_symbol {
            let mut substitutions = HashMap::new();
            substitutions.insert(distributive_symbol, left);
            let mut cache = HashMap::new();
            let mapped_right =
                self.substitute_static_parameters(right, &substitutions, ctx.types, &mut cache);
            let mapped_then =
                self.substitute_static_parameters(then_type, &substitutions, ctx.types, &mut cache);
            let mapped_else =
                self.substitute_static_parameters(else_type, &substitutions, ctx.types, &mut cache);
            (mapped_right, mapped_then, mapped_else)
        } else {
            (right, then_type, else_type)
        };

        // collect conditional relation facts once for infer and branch selection
        let contains_infer = self.type_contains_infer(right, ctx.types, &mut HashSet::new());
        let infer_substitutions = if contains_infer {
            self.infer_conditional_type_substitutions(
                &mut ctx.reborrow(),
                distributive_symbol.is_some(),
                left,
                right,
                source_id,
            )
        } else {
            None
        };
        let is_assignable = self
            .is_type_assignable(&mut ctx.reborrow(), right, left)
            .is_assignable();

        // infer conditional bindings before choosing a branch
        if contains_infer {
            if let Some(substitutions) = infer_substitutions {
                let substituted =
                    self.substitute_infer_types(&mut ctx.reborrow(), then_type, &substitutions);
                return self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    substituted,
                    mode,
                    relation_mode,
                    visited,
                );
            }

            return self.normalize_type_inner(
                &mut ctx.reborrow(),
                else_type,
                mode,
                relation_mode,
                visited,
            );
        }

        // any yields the union of both branches
        if matches!(
            ctx.types.get_type(left),
            Type::TypeLiteral {
                value: TypeLiteral::Any,
            }
        ) {
            let normalized_then = self.normalize_type_inner(
                &mut ctx.reborrow(),
                then_type,
                mode,
                relation_mode,
                visited,
            );
            let normalized_else = self.normalize_type_inner(
                &mut ctx.reborrow(),
                else_type,
                mode,
                relation_mode,
                visited,
            );
            if normalized_then == normalized_else {
                return normalized_then;
            }
            let union_id = ctx.types.insert_type_from_any(
                Type::Union {
                    elements: vec![normalized_then, normalized_else],
                },
                source_id,
            );
            return self.normalize_type_inner(
                &mut ctx.reborrow(),
                union_id,
                mode,
                relation_mode,
                visited,
            );
        }

        // choose the active branch based on assignability
        let branch_id = if is_assignable { then_type } else { else_type };

        self.normalize_type_inner(&mut ctx.reborrow(), branch_id, mode, relation_mode, visited)
    }

    /// Normalize indexed access types by resolving the accessed value types.
    pub(super) fn normalize_index_type(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        left: LocalTypeId,
        index: LocalTypeId,
        mode: NormalizationMode,
        _relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        // index access uses type operations semantics
        let relation_mode = RelationMode::INDEX_ACCESS;

        let resolution = self.resolve_index_access_types(
            &mut ctx.reborrow(),
            source_id,
            left,
            index,
            mode,
            relation_mode,
            visited,
        );
        let value_types = resolution.value_types;

        // collapse the collected value types
        let combined = match value_types.len() {
            0 => ctx.types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                source_id,
            ),
            1 => value_types[0],
            _ => ctx.types.insert_type_from_any(
                Type::Union {
                    elements: value_types,
                },
                source_id,
            ),
        };
        self.normalize_type_inner(&mut ctx.reborrow(), combined, mode, relation_mode, visited)
    }

    /// Resolve indexed access value types and missing keys.
    pub(crate) fn resolve_index_access_types(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        left: LocalTypeId,
        index: LocalTypeId,
        mode: NormalizationMode,
        _relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> IndexAccessResolution {
        // index access uses type operations semantics
        let relation_mode = RelationMode::INDEX_ACCESS;

        // resolve apparent operand types before index evaluation
        let left = self.apparent_type(&mut ctx.reborrow(), left, relation_mode);
        let index = self.apparent_type(&mut ctx.reborrow(), index, relation_mode);

        // normalize operands before evaluating the index
        let left =
            self.normalize_type_inner(&mut ctx.reborrow(), left, mode, relation_mode, visited);
        let index =
            self.normalize_type_inner(&mut ctx.reborrow(), index, mode, relation_mode, visited);

        // split union index types into individual keys
        let index_types = match ctx.types.get_type(index) {
            Type::Union { elements } => elements.clone(),
            _ => vec![index],
        };
        let mut value_types = Vec::new();
        let mut missing_keys = Vec::new();

        // compute the accessed type for each key
        for key_type_id in index_types {
            let value_type = self.index_access_type_for_key_type(
                &mut ctx.reborrow(),
                left,
                key_type_id,
                source_id,
                mode,
                relation_mode,
            );
            if let Some(value_type) = value_type {
                value_types.push(value_type);
            } else {
                if let Some(missing_key) = self.static_key_from_type(key_type_id, ctx.types) {
                    missing_keys.push(missing_key);
                }
                value_types.push(ctx.types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    },
                    source_id,
                ));
            }
        }

        IndexAccessResolution {
            value_types,
            missing_keys,
        }
    }

    /// Resolve an index access for a single key type.
    fn index_access_type_for_key_type(
        &self,
        ctx: &mut TypeContext<'_>,
        left: LocalTypeId,
        key_type_id: LocalTypeId,
        source_id: LocalNodeIdAny,
        mode: NormalizationMode,
        relation_mode: RelationMode,
    ) -> Option<LocalTypeId> {
        // handle literal key access first
        if let Some(static_key) = self.static_key_from_type(key_type_id, ctx.types) {
            return self.index_access_for_literal_key(
                &mut ctx.reborrow(),
                left,
                static_key,
                mode,
                relation_mode,
            );
        }

        // handle primitive index kinds
        if let Some(kind) = self.mapped_index_kind_for_type(key_type_id, ctx.types) {
            return self.index_access_for_index_kind(
                &mut ctx.reborrow(),
                left,
                kind,
                mode,
                relation_mode,
            );
        }

        match ctx.types.get_type(key_type_id) {
            Type::TypeLiteral {
                value: TypeLiteral::Any | TypeLiteral::Unknown,
            } => Some(ctx.types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                source_id,
            )),
            _ => None,
        }
    }

    /// Resolve a literal key access on a type id.
    fn index_access_for_literal_key(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        key: StaticKey,
        mode: NormalizationMode,
        relation_mode: RelationMode,
    ) -> Option<LocalTypeId> {
        // guard against recursive index access cycles
        let mut visited = HashSet::new();
        self.index_access_for_literal_key_inner(
            &mut ctx.reborrow(),
            type_id,
            key,
            mode,
            relation_mode,
            &mut visited,
        )
    }

    /// Resolve a literal key access on a type id with a recursion guard.
    fn index_access_for_literal_key_inner(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        key: StaticKey,
        mode: NormalizationMode,
        relation_mode: RelationMode,
        visited: &mut HashSet<LocalTypeId>,
    ) -> Option<LocalTypeId> {
        // stop when revisiting the same type
        if !visited.insert(type_id) {
            self.report_recursive_mapped_instantiation(&mut ctx.reborrow(), type_id);
            return None;
        }

        // use the receiver type as the source for synthesized unions
        let source_id = ctx.types.get_type_source(type_id);
        let ty = ctx.types.get_type(type_id).clone();
        match ty {
            Type::Union { elements } => {
                let mut value_types = Vec::new();

                // union values across elements
                for element_id in elements {
                    let value_type = self.index_access_for_literal_key_inner(
                        &mut ctx.reborrow(),
                        element_id,
                        key,
                        mode,
                        relation_mode,
                        visited,
                    )?;
                    value_types.push(value_type);
                }

                Some(self.union_type_ids_from_list(value_types, source_id, ctx.types))
            }
            Type::Intersection { elements } => {
                let mut value_types = Vec::new();

                // intersect values across elements
                for element_id in elements {
                    // skip descriptor wrappers when resolving member keys
                    if matches!(ctx.types.get_type(element_id), Type::Value { .. }) {
                        continue;
                    }
                    if let Some(value_type) = self.index_access_for_literal_key_inner(
                        &mut ctx.reborrow(),
                        element_id,
                        key,
                        mode,
                        relation_mode,
                        visited,
                    ) {
                        value_types.push(value_type);
                    }
                }
                if value_types.is_empty() {
                    None
                } else {
                    Some(self.intersection_type_ids_from_list(value_types, source_id, ctx.types))
                }
            }
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                let mut normalize_visited = Vec::new();
                let normalized_id = self.normalize_mapped_type(
                    &mut ctx.reborrow(),
                    source_id,
                    parameter,
                    modifiers,
                    value,
                    mode,
                    relation_mode,
                    &mut normalize_visited,
                );
                if matches!(ctx.types.get_type(normalized_id), Type::Mapped { .. }) {
                    None
                } else {
                    self.index_access_for_literal_key_inner(
                        &mut ctx.reborrow(),
                        normalized_id,
                        key,
                        mode,
                        relation_mode,
                        visited,
                    )
                }
            }
            _ => {
                let mut value_types = Vec::new();

                // add field type when present
                if let Some(field_type) =
                    self.field_type_for_key(&mut ctx.reborrow(), type_id, &key)
                {
                    value_types.push(field_type);
                }

                // add index signature candidates when compatible
                if let Some(kind) = self.mapped_index_kind_for_static_key(&key) {
                    let index_values = self.index_signature_value_types_for_kind(
                        &mut ctx.reborrow(),
                        type_id,
                        kind,
                    );
                    value_types.extend(index_values);
                }

                if value_types.is_empty() {
                    None
                } else {
                    Some(self.union_type_ids_from_list(value_types, source_id, ctx.types))
                }
            }
        }
    }

    /// Resolve an index access for a primitive index kind.
    fn index_access_for_index_kind(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        kind: MappedIndexKind,
        mode: NormalizationMode,
        relation_mode: RelationMode,
    ) -> Option<LocalTypeId> {
        // guard against recursive index access cycles
        let mut visited = HashSet::new();
        self.index_access_for_index_kind_inner(
            &mut ctx.reborrow(),
            type_id,
            kind,
            mode,
            relation_mode,
            &mut visited,
        )
    }

    /// Resolve an index access for a primitive index kind with a recursion guard.
    fn index_access_for_index_kind_inner(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        kind: MappedIndexKind,
        mode: NormalizationMode,
        relation_mode: RelationMode,
        visited: &mut HashSet<LocalTypeId>,
    ) -> Option<LocalTypeId> {
        // stop when revisiting the same type
        if !visited.insert(type_id) {
            self.report_recursive_mapped_instantiation(&mut ctx.reborrow(), type_id);
            return None;
        }

        // use the receiver type as the source for synthesized unions
        let source_id = ctx.types.get_type_source(type_id);
        let ty = ctx.types.get_type(type_id).clone();
        match ty {
            Type::Union { elements } => {
                let mut value_types = Vec::new();

                // union values across elements
                for element_id in elements {
                    let value_type = self.index_access_for_index_kind_inner(
                        &mut ctx.reborrow(),
                        element_id,
                        kind,
                        mode,
                        relation_mode,
                        visited,
                    )?;
                    value_types.push(value_type);
                }

                Some(self.union_type_ids_from_list(value_types, source_id, ctx.types))
            }
            Type::Intersection { elements } => {
                let mut value_types = Vec::new();

                // intersect values across elements
                for element_id in elements {
                    // skip descriptor wrappers when resolving member keys
                    if matches!(ctx.types.get_type(element_id), Type::Value { .. }) {
                        continue;
                    }
                    if let Some(value_type) = self.index_access_for_index_kind_inner(
                        &mut ctx.reborrow(),
                        element_id,
                        kind,
                        mode,
                        relation_mode,
                        visited,
                    ) {
                        value_types.push(value_type);
                    }
                }

                if value_types.is_empty() {
                    None
                } else {
                    Some(self.intersection_type_ids_from_list(value_types, source_id, ctx.types))
                }
            }
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                let mut normalize_visited = Vec::new();
                let normalized_id = self.normalize_mapped_type(
                    &mut ctx.reborrow(),
                    source_id,
                    parameter,
                    modifiers,
                    value,
                    mode,
                    relation_mode,
                    &mut normalize_visited,
                );
                if matches!(ctx.types.get_type(normalized_id), Type::Mapped { .. }) {
                    None
                } else {
                    self.index_access_for_index_kind_inner(
                        &mut ctx.reborrow(),
                        normalized_id,
                        kind,
                        mode,
                        relation_mode,
                        visited,
                    )
                }
            }
            _ => {
                let mut value_types = Vec::new();

                // add field types compatible with this kind
                let field_values =
                    self.field_types_for_index_kind(&mut ctx.reborrow(), type_id, kind);
                value_types.extend(field_values);

                // add index signature types
                let index_values =
                    self.index_signature_value_types_for_kind(&mut ctx.reborrow(), type_id, kind);
                value_types.extend(index_values);

                if value_types.is_empty() {
                    None
                } else {
                    Some(self.union_type_ids_from_list(value_types, source_id, ctx.types))
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
                let name = self.repository.strings.intern(&value.to_string());
                Some(StaticKey::Number(name))
            }
            ScalarLiteral::Float(value) => {
                let name = self.repository.strings.intern(&value.to_string());
                Some(StaticKey::Number(name))
            }
            ScalarLiteral::Bigint(value) => {
                let name = self.repository.strings.intern(&value.to_string());
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
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        key: &StaticKey,
    ) -> Option<LocalTypeId> {
        // unwrap alias references before walking fields
        let type_id = self.unwrap_normalization_alias_reference(type_id, ctx.types);
        let mut field_types = Vec::new();
        let mut pending = vec![type_id];
        let mut visited = Vec::new();

        // walk object and intersection shapes
        while let Some(current_id) = pending.pop() {
            if visited.contains(&current_id) {
                continue;
            }
            visited.push(current_id);
            let current_ty = ctx.types.get_type(current_id).clone();
            match current_ty {
                Type::Object { fields, .. } => {
                    for field in fields {
                        if field.key.matches(key) {
                            field_types.push(field.ty);
                        }
                    }
                }
                Type::Tuple { elements, .. } => {
                    // map numeric tuple keys to element types
                    let StaticKey::Number(name) = key else {
                        continue;
                    };
                    let index_str = self.repository.strings.get(*name);
                    let Ok(index) = index_str.as_ref().parse::<usize>() else {
                        continue;
                    };
                    if let Some(element) = elements.get(index) {
                        field_types.push(element.ty);
                    } else if let Some(rest) = elements.iter().find(|element| element.is_rest) {
                        field_types.push(rest.ty);
                    }
                }
                Type::Array { element, .. } => {
                    // arrays expose numeric indexed element access
                    if matches!(key, StaticKey::Number(_))
                        && let Some(element) = element
                    {
                        field_types.push(element);
                    }
                }
                Type::ArraySized { element, .. } => {
                    // fixed arrays expose numeric indexed element access
                    if matches!(key, StaticKey::Number(_)) {
                        field_types.push(element);
                    }
                }
                Type::Reference {
                    symbol,
                    static_arguments,
                } => {
                    let source_id = ctx.types.get_type_source(current_id);

                    // expand alias references with static arguments before apparent lookup
                    if static_arguments.is_some()
                        && matches!(symbol.ty(), SymbolType::TypeAlias | SymbolType::Newtype)
                    {
                        let mut visited_alias = Vec::new();
                        let arguments = static_arguments.as_deref().unwrap_or(&[]);
                        if let Some(expanded_id) = self
                            .normalize_type_alias_reference_with_arguments(
                                &mut ctx.reborrow(),
                                source_id,
                                symbol,
                                arguments,
                                NormalizationMode::Assign,
                                RelationMode::ALIAS_EXPANSION,
                                &mut visited_alias,
                            )
                        {
                            pending.push(expanded_id);
                            continue;
                        }
                    }

                    if let Some(instance_id) =
                        self.apparent_instance_type(&mut ctx.reborrow(), source_id, symbol)
                    {
                        pending.push(instance_id);
                    }
                }
                Type::Intersection { elements } => {
                    for element_id in elements {
                        pending.push(element_id);
                    }
                }
                _ => {}
            }
        }

        match field_types.len() {
            0 => None,
            1 => Some(field_types[0]),
            _ => Some(ctx.types.insert_type_from_type(
                Type::Intersection {
                    elements: field_types,
                },
                type_id,
            )),
        }
    }

    /// Collect field types compatible with an index kind.
    fn field_types_for_index_kind(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        kind: MappedIndexKind,
    ) -> Vec<LocalTypeId> {
        // unwrap alias references before collecting
        let type_id = self.unwrap_normalization_alias_reference(type_id, ctx.types);
        let mut field_types = Vec::new();
        let mut pending = vec![type_id];
        let mut visited = Vec::new();

        // traverse object and intersection shapes
        while let Some(current_id) = pending.pop() {
            if visited.contains(&current_id) {
                continue;
            }
            visited.push(current_id);
            let current_ty = ctx.types.get_type(current_id).clone();
            match current_ty {
                Type::Object { fields, .. } => {
                    for field in fields {
                        if self.static_key_matches_index_kind(&field.key, kind) {
                            field_types.push(field.ty);
                        }
                    }
                }
                Type::Tuple { elements, .. } => {
                    // map numeric tuple indices to element types
                    if kind == MappedIndexKind::Number {
                        for element in elements {
                            field_types.push(element.ty);
                        }
                    }
                }
                Type::Array { element, .. } => {
                    // arrays expose number index kind fields
                    if kind == MappedIndexKind::Number
                        && let Some(element) = element
                    {
                        field_types.push(element);
                    }
                }
                Type::ArraySized { element, .. } => {
                    // fixed arrays expose number index kind fields
                    if kind == MappedIndexKind::Number {
                        field_types.push(element);
                    }
                }
                Type::Reference {
                    symbol,
                    static_arguments,
                } => {
                    let source_id = ctx.types.get_type_source(current_id);

                    // expand alias references with static arguments before apparent lookup
                    if static_arguments.is_some()
                        && matches!(symbol.ty(), SymbolType::TypeAlias | SymbolType::Newtype)
                    {
                        let mut visited_alias = Vec::new();
                        let arguments = static_arguments.as_deref().unwrap_or(&[]);
                        if let Some(expanded_id) = self
                            .normalize_type_alias_reference_with_arguments(
                                &mut ctx.reborrow(),
                                source_id,
                                symbol,
                                arguments,
                                NormalizationMode::Assign,
                                RelationMode::ALIAS_EXPANSION,
                                &mut visited_alias,
                            )
                        {
                            pending.push(expanded_id);
                            continue;
                        }
                    }

                    if let Some(instance_id) =
                        self.apparent_instance_type(&mut ctx.reborrow(), source_id, symbol)
                    {
                        pending.push(instance_id);
                    }
                }
                Type::Intersection { elements } => {
                    for element_id in elements {
                        pending.push(element_id);
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
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        kind: MappedIndexKind,
    ) -> Vec<LocalTypeId> {
        // unwrap alias references before collecting
        let type_id = self.unwrap_normalization_alias_reference(type_id, ctx.types);
        let mut value_types = Vec::new();
        let mut pending = vec![type_id];
        let mut visited = Vec::new();

        // traverse object and intersection shapes
        while let Some(current_id) = pending.pop() {
            if visited.contains(&current_id) {
                continue;
            }
            visited.push(current_id);
            let current_ty = ctx.types.get_type(current_id).clone();
            match current_ty {
                Type::Object {
                    index_signatures, ..
                } => {
                    for signature in index_signatures {
                        let signature_kind =
                            self.mapped_index_kind_for_type(signature.key_type, ctx.types);
                        if let Some(signature_kind) = signature_kind
                            && self.index_kinds_compatible(signature_kind, kind)
                        {
                            value_types.push(signature.value_type);
                        }
                    }
                }
                Type::Array { element, .. } => {
                    // arrays have an implicit numeric index signature
                    if kind == MappedIndexKind::Number
                        && let Some(element) = element
                    {
                        value_types.push(element);
                    }
                }
                Type::ArraySized { element, .. } => {
                    // fixed arrays have an implicit numeric index signature
                    if kind == MappedIndexKind::Number {
                        value_types.push(element);
                    }
                }
                Type::Reference { symbol, .. } => {
                    let source_id = ctx.types.get_type_source(current_id);
                    if let Some(instance_id) =
                        self.apparent_instance_type(&mut ctx.reborrow(), source_id, symbol)
                    {
                        pending.push(instance_id);
                    }
                }
                Type::Intersection { elements } => {
                    for element_id in elements {
                        pending.push(element_id);
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
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // drop empty unions early
        if type_ids.is_empty() {
            return types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Never,
                },
                source_id,
            );
        }

        // collapse single element unions
        if type_ids.len() == 1 {
            return type_ids[0];
        }

        // keep union elements unique
        type_ids.sort_by_key(|id| id.0);
        type_ids.dedup();
        types.insert_type_from_any(Type::Union { elements: type_ids }, source_id)
    }

    /// Combine type ids into an intersection.
    fn intersection_type_ids_from_list(
        &self,
        mut type_ids: Vec<LocalTypeId>,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // drop empty intersections early
        if type_ids.is_empty() {
            return types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Never,
                },
                source_id,
            );
        }

        // collapse single element intersections
        if type_ids.len() == 1 {
            return type_ids[0];
        }

        // keep intersection elements unique
        type_ids.sort_by_key(|id| id.0);
        type_ids.dedup();
        types.insert_type_from_any(Type::Intersection { elements: type_ids }, source_id)
    }

    /// Normalize mapped types into object shapes.
    pub(crate) fn normalize_mapped_type(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        parameter: MappedTypeParameter,
        modifiers: MappedTypeModifiers,
        value: LocalTypeId,
        mode: NormalizationMode,
        _relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        // mapped key queries should use type operations semantics
        let relation_mode = RelationMode::OBJECT_SHAPE;

        let MappedTypeParameter {
            name,
            symbol,
            constraint,
            key_remap,
        } = parameter;

        // use the mapped parameter symbol for substitution
        let parameter_symbol = Some(symbol);

        // keep mapped types unresolved when they depend on free static parameters or inference
        let mut bound_static_parameters = HashSet::new();
        bound_static_parameters.insert(symbol);
        let mut static_visited = HashSet::new();
        let constraint_contains_static = self.type_contains_free_static_parameters(
            ctx.type_view(),
            constraint,
            &bound_static_parameters,
            &mut static_visited,
        );
        let mut infer_visited = HashSet::new();
        let constraint_contains_infer =
            self.type_contains_infer_vars(constraint, ctx.types, &mut infer_visited);
        let remap_contains_static = key_remap.is_some_and(|key_remap| {
            self.type_contains_free_static_parameters(
                ctx.type_view(),
                key_remap,
                &bound_static_parameters,
                &mut HashSet::new(),
            )
        });
        let remap_contains_infer = key_remap.is_some_and(|key_remap| {
            self.type_contains_infer_vars(key_remap, ctx.types, &mut HashSet::new())
        });
        if constraint_contains_static
            || constraint_contains_infer
            || remap_contains_static
            || remap_contains_infer
        {
            let normalized_value =
                self.normalize_type_inner(&mut ctx.reborrow(), value, mode, relation_mode, visited);
            let parameter = MappedTypeParameter {
                name,
                symbol,
                constraint,
                key_remap,
            };
            return ctx.types.insert_type_from_any(
                Type::Mapped {
                    parameter,
                    modifiers,
                    value: normalized_value,
                },
                source_id,
            );
        }

        // normalize the key constraint for evaluation
        let normalized_constraint = self.normalize_type_inner(
            &mut ctx.reborrow(),
            constraint,
            mode,
            relation_mode,
            visited,
        );
        let normalized_key_remap = key_remap.map(|key_remap| {
            self.normalize_type_inner(&mut ctx.reborrow(), key_remap, mode, relation_mode, visited)
        });

        // reuse one ctx context for mapped key collection and expansion
        let mut ctx = ctx.reborrow();
        {
            // collect mapped keys from the constraint
            let mut keys = Vec::new();
            self.collect_mapped_keys_for_type(
                &mut ctx.reborrow(),
                normalized_constraint,
                relation_mode,
                &mut keys,
            );
            if keys.is_empty() {
                // empty key set produces an empty object
                let normalized = Type::Object {
                    fields: Vec::new(),
                    call_signatures: Vec::new(),
                    construct_signatures: Vec::new(),
                    index_signatures: Vec::new(),
                };
                return ctx.types.insert_type_from_any(normalized, source_id);
            }

            // find the source type for modifier inheritance
            let source_type_id = parameter_symbol.and_then(|parameter_symbol| {
                self.mapped_source_type_id(parameter_symbol, constraint, value, ctx.types)
            });

            // expand each mapped key into fields or index signatures
            let mut fields: Vec<TypeField> = Vec::new();
            let mut index_values: HashMap<MappedIndexKind, Vec<LocalTypeId>> = HashMap::new();
            let mut index_optional: HashMap<MappedIndexKind, bool> = HashMap::new();
            let mut index_readonly: HashMap<MappedIndexKind, bool> = HashMap::new();

            // precompute normalized value and remap when the parameter is unused
            let normalized_value_without_param = if parameter_symbol.is_none() {
                Some(self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    value,
                    mode,
                    relation_mode,
                    visited,
                ))
            } else {
                None
            };
            let normalized_remap_without_param = if let Some(key_remap) = normalized_key_remap
                && parameter_symbol.is_none()
            {
                let mut remapped = Vec::new();
                self.collect_mapped_keys_for_type(
                    &mut ctx.reborrow(),
                    key_remap,
                    relation_mode,
                    &mut remapped,
                );
                Some(remapped)
            } else {
                None
            };

            for key in keys {
                let key_type_id = match &key {
                    MappedKey::Field { key_type, .. } => *key_type,
                    MappedKey::Index { key_type, .. } => *key_type,
                };

                // resolve base modifiers from the original key
                let (base_optional, base_readonly, index_base_readonly) = match &key {
                    MappedKey::Field { key, .. } => {
                        let (optional, readonly) = source_type_id
                            .and_then(|source| {
                                self.field_modifiers_for_key(&mut ctx.reborrow(), source, key)
                            })
                            .unwrap_or((false, false));
                        (optional, readonly, readonly)
                    }
                    MappedKey::Index { kind, .. } => {
                        let readonly = source_type_id
                            .and_then(|source| {
                                self.resolve_index_signature_readonly_for_kind(
                                    &mut ctx.reborrow(),
                                    source,
                                    *kind,
                                )
                            })
                            .unwrap_or(false);
                        (false, readonly, readonly)
                    }
                };

                // substitute the mapped parameter with the key type
                let normalized_value = if let Some(normalized) = normalized_value_without_param {
                    normalized
                } else {
                    let mut substitutions = HashMap::new();
                    if let Some(parameter_symbol) = parameter_symbol {
                        substitutions.insert(parameter_symbol, key_type_id);
                    }
                    let mut cache = HashMap::new();
                    let substituted_value = self.substitute_static_parameters(
                        value,
                        &substitutions,
                        ctx.types,
                        &mut cache,
                    );
                    self.normalize_type_inner(
                        &mut ctx.reborrow(),
                        substituted_value,
                        mode,
                        relation_mode,
                        visited,
                    )
                };

                // compute remapped keys when present
                let remapped_keys = if let Some(remapped) = normalized_remap_without_param.as_ref()
                {
                    remapped.clone()
                } else if let Some(key_remap) = key_remap {
                    let mut substitutions = HashMap::new();
                    if let Some(parameter_symbol) = parameter_symbol {
                        substitutions.insert(parameter_symbol, key_type_id);
                    }
                    let mut cache = HashMap::new();
                    let substituted_remap = self.substitute_static_parameters(
                        key_remap,
                        &substitutions,
                        ctx.types,
                        &mut cache,
                    );
                    let normalized_remap = self.normalize_type_inner(
                        &mut ctx.reborrow(),
                        substituted_remap,
                        mode,
                        relation_mode,
                        visited,
                    );
                    let mut remapped = Vec::new();
                    self.collect_mapped_keys_for_type(
                        &mut ctx.reborrow(),
                        normalized_remap,
                        relation_mode,
                        &mut remapped,
                    );
                    remapped
                } else {
                    vec![key.clone()]
                };

                // map every key into fields or index signatures
                for remapped_key in remapped_keys {
                    match remapped_key {
                        MappedKey::Field { key, .. } => {
                            let (is_optional, is_readonly) = self.apply_mapped_modifiers(
                                modifiers,
                                base_optional,
                                base_readonly,
                            );

                            // merge the mapped field into the output set
                            if let Some(existing) =
                                fields.iter_mut().find(|field| field.key.matches(&key))
                            {
                                if existing.ty != normalized_value {
                                    existing.ty = ctx.types.insert_type_from_type(
                                        Type::Union {
                                            elements: vec![existing.ty, normalized_value],
                                        },
                                        existing.ty,
                                    );
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
                            let (is_optional, is_readonly) =
                                self.apply_mapped_modifiers(modifiers, false, index_base_readonly);
                            if let Some(existing) = index_optional.get_mut(&kind) {
                                *existing = *existing && is_optional;
                            } else {
                                index_optional.insert(kind, is_optional);
                            }
                            if let Some(existing) = index_readonly.get_mut(&kind) {
                                *existing = *existing && is_readonly;
                            } else {
                                index_readonly.insert(kind, is_readonly);
                            }
                        }
                    }
                }
            }

            // build index signatures for mapped index keys
            let mut index_signatures = Vec::new();
            for (kind, values) in index_values {
                let value_type_id = self.union_type_ids_from_list(values, source_id, ctx.types);
                let key_type_id = self.key_type_id_for_index_kind(kind, source_id, ctx.types);
                let is_optional = index_optional.get(&kind).copied().unwrap_or(false);
                let is_readonly = index_readonly.get(&kind).copied().unwrap_or(false);
                index_signatures.push(TypeIndexSignature {
                    name,
                    key_type: key_type_id,
                    value_type: value_type_id,
                    is_optional,
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
            ctx.types.insert_type_from_any(normalized, source_id)
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
        if let Type::KeyOf { target_type: right } = types.get_type(constraint) {
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
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        relation_mode: RelationMode,
        keys: &mut Vec<MappedKey>,
    ) {
        // track visited types to avoid recursion cycles
        let mut visited = HashSet::new();
        self.collect_mapped_keys_for_type_inner(
            &mut ctx.reborrow(),
            type_id,
            relation_mode,
            keys,
            &mut visited,
        );
    }

    /// Collect mapped keys for a constraint type with a recursion guard.
    fn collect_mapped_keys_for_type_inner(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        relation_mode: RelationMode,
        keys: &mut Vec<MappedKey>,
        visited: &mut HashSet<LocalTypeId>,
    ) {
        // stop when revisiting the same type
        if !visited.insert(type_id) {
            return;
        }

        // reuse the current type source for synthesized key types
        let source_id = ctx.types.get_type_source(type_id);
        let ty = ctx.types.get_type(type_id).clone();
        match ty {
            Type::Union { elements } => {
                // expand each union member
                for element_id in elements {
                    self.collect_mapped_keys_for_type_inner(
                        &mut ctx.reborrow(),
                        element_id,
                        relation_mode,
                        keys,
                        visited,
                    );
                }
            }
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                // follow constraint bounds for static parameter references
                if self.symbol_is_static_parameter(ctx.symbol_type_view(), symbol) {
                    let mut ctx = ctx.reborrow();
                    if let Some(constraint_id) =
                        self.static_parameter_constraint_type(&mut ctx, symbol, source_id)
                    {
                        self.collect_mapped_keys_for_type_inner(
                            &mut ctx.reborrow(),
                            constraint_id,
                            relation_mode,
                            keys,
                            visited,
                        );
                    }
                } else if static_arguments.is_none()
                    && let Some(instance_id) =
                        self.apparent_instance_type(&mut ctx.reborrow(), source_id, symbol)
                {
                    // prefer instance shapes for type parameter and alias references
                    self.collect_mapped_keys_for_type_inner(
                        &mut ctx.reborrow(),
                        instance_id,
                        relation_mode,
                        keys,
                        visited,
                    );
                } else if symbol.ty() == SymbolType::TypeAlias {
                    let static_arguments = static_arguments
                        .clone()
                        .map(|arguments| self.canonicalize_instance_arguments_for_key(arguments));
                    let static_arguments = static_arguments.unwrap_or_default();

                    if ctx.types.is_normalization_alias_in_progress(
                        symbol,
                        NormalizationMode::Assign,
                        relation_mode.cache_key(),
                        &static_arguments,
                    ) {
                        return;
                    }

                    // expand alias references to collect mapped keys from utility types
                    let mut normalize_visited = Vec::new();
                    let normalized = self.normalize_type_alias_reference_with_arguments(
                        &mut ctx.reborrow(),
                        source_id,
                        symbol,
                        &static_arguments,
                        NormalizationMode::Assign,
                        relation_mode,
                        &mut normalize_visited,
                    );
                    if let Some(normalized) = normalized {
                        let normalized = self.normalize_type_inner(
                            &mut ctx.reborrow(),
                            normalized,
                            NormalizationMode::Assign,
                            relation_mode,
                            &mut Vec::new(),
                        );
                        self.collect_mapped_keys_for_type_inner(
                            &mut ctx.reborrow(),
                            normalized,
                            relation_mode,
                            keys,
                            visited,
                        );
                    }
                }
            }
            Type::KeyOf { target_type: right } => {
                let mut normalize_visited = Vec::new();
                let normalized = self.normalize_keyof_type(
                    &mut ctx.reborrow(),
                    source_id,
                    Some(type_id),
                    right,
                    NormalizationMode::Assign,
                    relation_mode,
                    &mut normalize_visited,
                );
                if self.type_is_deferred_keyof_projection(ctx.type_view(), normalized) {
                    // avoid recursing on deferred keyof projections
                    return;
                }
                self.collect_mapped_keys_for_type_inner(
                    &mut ctx.reborrow(),
                    normalized,
                    relation_mode,
                    keys,
                    visited,
                );
            }
            Type::Conditional { .. } => {
                let mut normalize_visited = Vec::new();
                let normalized = self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    type_id,
                    NormalizationMode::Assign,
                    relation_mode,
                    &mut normalize_visited,
                );
                if normalized != type_id {
                    self.collect_mapped_keys_for_type_inner(
                        &mut ctx.reborrow(),
                        normalized,
                        relation_mode,
                        keys,
                        visited,
                    );
                }
            }
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(_),
            } => {
                // literal keys map directly to field keys
                if let Some(key) = self.static_key_from_type(type_id, ctx.types) {
                    let key_type = self.key_type_id_for_static_key(key, source_id, ctx.types);
                    keys.push(MappedKey::Field { key, key_type });
                }
            }
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            } => {
                // primitive strings map to string index keys
                keys.push(MappedKey::Index {
                    kind: MappedIndexKind::String,
                    key_type: self.key_type_id_for_index_kind(
                        MappedIndexKind::String,
                        source_id,
                        ctx.types,
                    ),
                });
            }
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number),
            } => {
                // primitive numbers map to number index keys
                keys.push(MappedKey::Index {
                    kind: MappedIndexKind::Number,
                    key_type: self.key_type_id_for_index_kind(
                        MappedIndexKind::Number,
                        source_id,
                        ctx.types,
                    ),
                });
            }
            Type::TypeLiteral {
                value:
                    TypeLiteral::Primitive(PrimitiveType::Symbol)
                    | TypeLiteral::Primitive(PrimitiveType::UniqueSymbol),
            } => {
                // primitive symbols map to symbol index keys
                keys.push(MappedKey::Index {
                    kind: MappedIndexKind::Symbol,
                    key_type: self.key_type_id_for_index_kind(
                        MappedIndexKind::Symbol,
                        source_id,
                        ctx.types,
                    ),
                });
            }
            Type::TypeLiteral {
                value: TypeLiteral::Any,
            } => {
                // include all index kinds for any
                keys.push(MappedKey::Index {
                    kind: MappedIndexKind::String,
                    key_type: self.key_type_id_for_index_kind(
                        MappedIndexKind::String,
                        source_id,
                        ctx.types,
                    ),
                });
                keys.push(MappedKey::Index {
                    kind: MappedIndexKind::Number,
                    key_type: self.key_type_id_for_index_kind(
                        MappedIndexKind::Number,
                        source_id,
                        ctx.types,
                    ),
                });
                keys.push(MappedKey::Index {
                    kind: MappedIndexKind::Symbol,
                    key_type: self.key_type_id_for_index_kind(
                        MappedIndexKind::Symbol,
                        source_id,
                        ctx.types,
                    ),
                });
            }
            Type::TemplateLiteral { strings, spans } => {
                // expand template literal keys into literal fields when possible
                if let Some(shape) = self.template_literal_key_shape(
                    &mut ctx.reborrow(),
                    &strings,
                    &spans,
                    relation_mode,
                ) {
                    match shape {
                        TemplateLiteralKeyShape::Literal(literal_keys) => {
                            for key in literal_keys {
                                let key_type =
                                    self.key_type_id_for_static_key(key, source_id, ctx.types);
                                keys.push(MappedKey::Field { key, key_type });
                            }
                        }
                        TemplateLiteralKeyShape::StringIndex => {
                            keys.push(MappedKey::Index {
                                kind: MappedIndexKind::String,
                                key_type: self.key_type_id_for_index_kind(
                                    MappedIndexKind::String,
                                    source_id,
                                    ctx.types,
                                ),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Apply mapped type modifiers to base modifiers. Returns (is_optional, is_readonly).
    fn apply_mapped_modifiers(
        &self,
        modifiers: MappedTypeModifiers,
        base_optional: bool,
        base_readonly: bool,
    ) -> (bool, bool) {
        // resolve optional modifiers
        let is_optional = match modifiers.optional {
            MappedTypeModifier::Present => true,
            MappedTypeModifier::Add => true,
            MappedTypeModifier::Remove => false,
            MappedTypeModifier::None => base_optional,
        };

        // resolve readonly modifiers
        let is_readonly = match modifiers.readonly {
            MappedTypeModifier::Present => true,
            MappedTypeModifier::Add => true,
            MappedTypeModifier::Remove => false,
            MappedTypeModifier::None => base_readonly,
        };

        (is_optional, is_readonly)
    }

    /// Resolve base modifiers for a field key on a type.
    pub(crate) fn field_modifiers_for_key(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        key: &StaticKey,
    ) -> Option<(bool, bool)> {
        // unwrap alias references before walking
        let type_id = self.unwrap_normalization_alias_reference(type_id, ctx.types);
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
            let current_ty = ctx.types.get_type(current_id).clone();
            match current_ty {
                Type::Object { fields, .. } => {
                    for field in fields {
                        if field.key.matches(key) {
                            found = true;
                            is_optional = is_optional && field.is_optional;
                            is_readonly = is_readonly && field.is_readonly;
                        }
                    }
                }
                Type::Reference {
                    symbol,
                    static_arguments,
                } => {
                    let source_id = ctx.types.get_type_source(current_id);

                    // expand alias references with static arguments before apparent lookup
                    if static_arguments.is_some()
                        && matches!(symbol.ty(), SymbolType::TypeAlias | SymbolType::Newtype)
                    {
                        let mut visited_alias = Vec::new();
                        let arguments = static_arguments.as_deref().unwrap_or(&[]);
                        if let Some(expanded_id) = self
                            .normalize_type_alias_reference_with_arguments(
                                &mut ctx.reborrow(),
                                source_id,
                                symbol,
                                arguments,
                                NormalizationMode::Assign,
                                RelationMode::ALIAS_EXPANSION,
                                &mut visited_alias,
                            )
                        {
                            pending.push(expanded_id);
                            continue;
                        }
                    }

                    if let Some(instance_id) =
                        self.apparent_instance_type(&mut ctx.reborrow(), source_id, symbol)
                    {
                        pending.push(instance_id);
                    }
                }
                Type::Intersection { elements } => {
                    for element_id in elements {
                        pending.push(element_id);
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
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        kind: MappedIndexKind,
    ) -> Option<bool> {
        // unwrap alias references before walking
        let type_id = self.unwrap_normalization_alias_reference(type_id, ctx.types);
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
            let current_ty = ctx.types.get_type(current_id).clone();
            match current_ty {
                Type::Object {
                    index_signatures, ..
                } => {
                    for signature in index_signatures {
                        let signature_kind =
                            self.mapped_index_kind_for_type(signature.key_type, ctx.types);
                        if let Some(signature_kind) = signature_kind
                            && self.index_kinds_compatible(signature_kind, kind)
                        {
                            found = true;
                            is_readonly = is_readonly && signature.is_readonly;
                        }
                    }
                }
                Type::Reference {
                    symbol,
                    static_arguments,
                } => {
                    let source_id = ctx.types.get_type_source(current_id);

                    // expand alias references with static arguments before apparent lookup
                    if static_arguments.is_some()
                        && matches!(symbol.ty(), SymbolType::TypeAlias | SymbolType::Newtype)
                    {
                        let mut visited_alias = Vec::new();
                        let arguments = static_arguments.as_deref().unwrap_or(&[]);
                        if let Some(expanded_id) = self
                            .normalize_type_alias_reference_with_arguments(
                                &mut ctx.reborrow(),
                                source_id,
                                symbol,
                                arguments,
                                NormalizationMode::Assign,
                                RelationMode::ALIAS_EXPANSION,
                                &mut visited_alias,
                            )
                        {
                            pending.push(expanded_id);
                            continue;
                        }
                    }

                    if let Some(instance_id) =
                        self.apparent_instance_type(&mut ctx.reborrow(), source_id, symbol)
                    {
                        pending.push(instance_id);
                    }
                }
                Type::Intersection { elements } => {
                    for element_id in elements {
                        pending.push(element_id);
                    }
                }
                _ => {}
            }
        }

        found.then_some(is_readonly)
    }
}
