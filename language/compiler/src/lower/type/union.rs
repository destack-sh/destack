use std::collections::{HashMap, HashSet};
use {destack_dir as dir, destack_mir as mir};

use destack_artifact::DiagnosticAnchor;
use destack_core::{StringId, StringPool};
use destack_query::format::format_unique_symbol_qualified_name;
use destack_source::ModuleId;

use super::{FieldInput, FieldLayoutKind, LayoutPolicy, TypeLowerer};
use crate::{LowerError, LowerResult};

const UNION_TAG_FIELD_NAME: &str = "tag";
const UNION_PAYLOAD_FIELD_NAME: &str = "payload";

/// Layout metadata for a lowered union type.
#[derive(Debug, Clone)]
pub(crate) struct UnionLayout {
    /// The tag field type.
    pub(crate) tag_type: mir::LocalNodeId<mir::Type>,
    /// The payload field type.
    pub(crate) payload_type: mir::LocalNodeId<mir::Type>,
    /// The payload storage strategy.
    pub(crate) payload: VariantPayload,
    /// The source union type ids in tag order.
    pub(crate) source_types: Vec<dir::LocalTypeId>,
    /// The tag field index in layout order.
    pub(crate) tag_field_index: u32,
    /// The payload field index in layout order.
    pub(crate) payload_field_index: u32,
    /// Discriminant field metadata when present.
    pub(crate) discriminant: Option<UnionDiscriminant>,
}

/// Payload storage strategy for union layouts.
#[derive(Debug, Clone, Copy)]
pub(crate) enum VariantPayload {
    /// Store the payload inline inside the union struct.
    Inline,
    /// Store the payload as a managed box.
    Boxed,
}

/// Discriminant metadata for a tagged union.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct UnionDiscriminant {
    /// The primary discriminant key used for tag ordering.
    pub(crate) primary_key: dir::StaticKey,
    /// Discriminant fields indexed by static key.
    pub(crate) fields: Vec<UnionDiscriminantField>,
}

/// Discriminant values for a field in tag order.
#[derive(Debug, Clone)]
pub(crate) struct UnionDiscriminantField {
    /// The field key shared by all union variants.
    pub(crate) key: dir::StaticKey,
    /// Literal values ordered by tag value.
    pub(crate) values: Vec<DiscriminantLiteral>,
    /// Tag mapping for literal values.
    pub(crate) tag_by_value: HashMap<DiscriminantKey, u32>,
}

/// A discriminant literal value tied to its declared type.
#[derive(Debug, Clone)]
pub(crate) struct DiscriminantLiteral {
    /// The literal value.
    pub(crate) value: DiscriminantValue,
    /// The declared type id for this literal.
    pub(crate) type_id: dir::LocalTypeId,
    /// The canonical key for tag ordering and lookup.
    pub(crate) key: DiscriminantKey,
}

/// Canonical discriminant value for ordering and lookup.
#[derive(Debug, Clone)]
pub(crate) enum DiscriminantValue {
    /// Null literal value.
    Null,
    /// Undefined literal value.
    Undefined,
    /// Boolean literal value.
    Boolean(bool),
    /// Number literal value.
    Number { value: f64 },
    /// Bigint literal value.
    Bigint(i64),
    /// String literal value.
    String(StringId),
    /// Unique symbol literal value.
    UniqueSymbol,
}

/// Canonical discriminant key used for ordering and lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum DiscriminantKey {
    /// Null literal key.
    Null,
    /// Undefined literal key.
    Undefined,
    /// Boolean literal key.
    Boolean(bool),
    /// Number literal key, stored as canonical f64 bits.
    Number(u64),
    /// Bigint literal key.
    Bigint(i64),
    /// String literal key.
    String(StringId),
    /// Unique symbol literal key.
    UniqueSymbol,
}

impl TypeLowerer<'_> {
    /// Return cached union layout metadata.
    pub(crate) fn union_layout(&self, type_id: dir::LocalTypeId) -> Option<&UnionLayout> {
        self.union_cache.get(&type_id)
    }

    /// Lower a DIR union type into a tagged union layout.
    pub(crate) fn lower_union_type(
        &mut self,
        types: &dir::TypeTable,
        type_id: dir::LocalTypeId,
        elements: &[dir::LocalTypeId],
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // collect union elements with deduplication
        let mut collected = Vec::new();
        let mut visited = HashSet::new();
        for element_id in elements {
            self.collect_union_element(*element_id, types, &mut visited, &mut collected);
        }

        // reject empty unions
        if collected.is_empty() {
            return Err(LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: "union has no elements".to_string(),
            }
            .into());
        }

        // use null references for unions of one reference type and null
        if let Some(null_reference) =
            self.try_lower_null_reference_union(types, &collected, module_id, node, builder)?
        {
            return Ok(null_reference);
        }

        // resolve discriminant metadata and tag ordering
        let (source_types, discriminant) =
            self.order_union_elements_by_discriminant(types, &collected, node, builder.strings())?;

        // lower union element types for copy and layout bounds
        let mut element_types = Vec::with_capacity(source_types.len());
        let mut copy = mir::Copy::Yes;
        let mut max_payload_size = 0;
        let mut max_payload_alignment = 1;
        for element_id in &source_types {
            // null and undefined are tag-only variants
            let element_type = match types.get_type(*element_id) {
                dir::Type::Literal(dir::LiteralType::Null | dir::LiteralType::Undefined) => {
                    self.ty_void
                }
                _ => self.lower_type(types, *element_id, module_id, node, builder)?,
            };

            // combine layout sizing and copy
            element_types.push(element_type);
            let element = builder.tree().get(element_type);
            copy = copy.combine(element.copy());
            let (size, alignment) = self
                .size_and_align_of_type(element, builder.tree())
                .ok_or_else(|| LowerError::UnsupportedType {
                    anchor: self.diagnostic_anchor(node),
                    ty: element_id.into_global(module_id),
                    message: "union layout requires concrete nested types".to_string(),
                })?;
            max_payload_size = max_payload_size.max(size);
            max_payload_alignment = max_payload_alignment.max(alignment);
        }

        // define tag and payload field types
        let tag_name = builder.intern(UNION_TAG_FIELD_NAME);
        let payload_name = builder.intern(UNION_PAYLOAD_FIELD_NAME);
        let tag_width = self.tag_width_for_discriminant_count(source_types.len(), node)?;
        let tag_type = self.union_tag_type(tag_width, builder);
        let payload = self.union_payload(copy, max_payload_size, max_payload_alignment);
        let payload_type = match payload {
            VariantPayload::Inline => self.inline_union_payload_type(max_payload_size, builder),
            VariantPayload::Boxed => builder.type_managed_reference(self.ty_void),
        };

        // compute field sizes and alignments
        let (tag_size, tag_alignment) = self
            .size_and_align_of_type(builder.tree().get(tag_type), builder.tree())
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(node),
                message: "union layout requires concrete nested types".to_string(),
            })?;
        let (payload_size, payload_alignment) = self
            .size_and_align_of_type(builder.tree().get(payload_type), builder.tree())
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(node),
                message: "union layout requires concrete nested types".to_string(),
            })?;

        // assemble field inputs
        let fields = vec![
            FieldInput {
                name: tag_name,
                ty: tag_type,
                size: tag_size,
                alignment: tag_alignment,
                source_index: Some(0),
                kind: FieldLayoutKind::Synthetic,
            },
            FieldInput {
                name: payload_name,
                ty: payload_type,
                size: payload_size,
                alignment: payload_alignment,
                source_index: Some(1),
                kind: FieldLayoutKind::Synthetic,
            },
        ];

        // compute layout and create the mir variant type
        let layout = Self::compute_struct_layout(fields, LayoutPolicy::Source);
        let cases = element_types
            .iter()
            .copied()
            .enumerate()
            .map(|(tag, ty)| {
                (
                    mir::Constant::UInt {
                        value: tag as u128,
                        width: tag_width,
                    },
                    ty,
                )
            })
            .collect();
        let mir_type = builder.type_variant(tag_type, payload_type, cases, copy);

        // cache layout for later field lookups
        self.layout_cache.insert(mir_type, layout.clone());

        // resolve tag and payload field indices
        let tag_field_index =
            layout
                .field_index(tag_name)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node),
                    message: "missing union tag field".to_string(),
                })?;
        let payload_field_index =
            layout
                .field_index(payload_name)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node),
                    message: "missing union payload field".to_string(),
                })?;

        // cache union layout metadata
        self.union_cache.insert(
            type_id,
            UnionLayout {
                tag_type,
                payload_type,
                payload,
                source_types,
                tag_field_index,
                payload_field_index,
                discriminant,
            },
        );

        // return the union type
        Ok(mir_type)
    }

    /// Resolve the tag type for a union layout.
    fn union_tag_type(
        &mut self,
        tag_width: u16,
        builder: &mut mir::ModuleBuilder,
    ) -> mir::LocalNodeId<mir::Type> {
        // select the smallest unsigned integer width
        match tag_width {
            8 => builder.type_int(8, false),
            16 => builder.type_int(16, false),
            32 => self.ty_u32,
            64 => builder.type_int(64, false),
            _ => builder.type_int(tag_width, false),
        }
    }

    /// Select the payload storage strategy for a union layout.
    fn union_payload(
        &self,
        copy: mir::Copy,
        payload_size: u32,
        payload_alignment: u32,
    ) -> VariantPayload {
        // require trivial copy for inline payloads
        if self.layout_policy.inline_union_requires_trivial_copyability && copy != mir::Copy::Yes {
            return VariantPayload::Boxed;
        }

        // keep inline payloads aligned within the policy
        if payload_alignment > self.layout_policy.inline_union_max_alignment {
            return VariantPayload::Boxed;
        }
        if payload_size <= self.layout_policy.inline_union_budget_bytes {
            return VariantPayload::Inline;
        }

        VariantPayload::Boxed
    }

    /// Build the inline payload type for a union.
    fn inline_union_payload_type(
        &mut self,
        payload_size: u32,
        builder: &mut mir::ModuleBuilder,
    ) -> mir::LocalNodeId<mir::Type> {
        // store inline payloads as pointer sized words
        let pointer_size = u32::from(self.pointer_bytes()).max(1);
        let slot_count = if payload_size == 0 {
            0
        } else {
            payload_size.div_ceil(pointer_size)
        };
        builder.type_array(self.ty_usize, slot_count as u64, mir::Copy::Yes)
    }

    /// Lower union types that can be represented as references that allow null.
    fn try_lower_null_reference_union(
        &mut self,
        types: &dir::TypeTable,
        elements: &[dir::LocalTypeId],
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<Option<mir::LocalNodeId<mir::Type>>> {
        let mut non_null = None;
        let mut has_null = false;

        for element_id in elements {
            match types.get_type(*element_id) {
                dir::Type::Literal(dir::LiteralType::Null) => {
                    has_null = true;
                }
                _ => {
                    if non_null.is_some() {
                        return Ok(None);
                    }
                    non_null = Some(*element_id);
                }
            }
        }

        if !has_null {
            return Ok(None);
        }

        let Some(non_null) = non_null else {
            return Ok(None);
        };
        if elements.len() != 2 {
            return Ok(None);
        }

        let non_null_type = self.lower_type(types, non_null, module_id, node, builder)?;
        let mir::Type::Reference {
            kind,
            lifetime,
            address_space,
            access,
            pointee,
            nullability,
        } = builder.tree().get(non_null_type)
        else {
            return Ok(None);
        };

        if nullability.allows_null() {
            return Ok(Some(non_null_type));
        }

        let Some(pointee) = pointee.ty() else {
            return Ok(None);
        };

        let null_reference = builder.type_reference_with_lifetime(
            *kind,
            lifetime.clone(),
            pointee,
            *access,
            address_space.clone(),
            mir::Nullability::Null,
        );
        Ok(Some(null_reference))
    }

    /// Collect union elements with deduplication.
    fn collect_union_element(
        &self,
        type_id: dir::LocalTypeId,
        types: &dir::TypeTable,
        visited: &mut HashSet<dir::LocalTypeId>,
        collected: &mut Vec<dir::LocalTypeId>,
    ) {
        // stop on already visited types
        if !visited.insert(type_id) {
            return;
        }

        // expand nested unions or record distinct types
        match types.get_type(type_id) {
            dir::Type::Union(union) => {
                for element_id in &union.elements {
                    self.collect_union_element(*element_id, types, visited, collected);
                }
            }
            _ => {
                let is_duplicate = collected.iter().any(|existing| *existing == type_id);
                if !is_duplicate {
                    collected.push(type_id);
                }
            }
        }
    }

    /// Order union elements by discriminant tags when available.
    fn order_union_elements_by_discriminant(
        &self,
        types: &dir::TypeTable,
        elements: &[dir::LocalTypeId],
        node: dir::AnchoredGlobalNodeId,
        strings: &StringPool,
    ) -> LowerResult<(Vec<dir::LocalTypeId>, Option<UnionDiscriminant>)> {
        // collect discriminant literal fields for each element
        let mut element_fields = Vec::with_capacity(elements.len());
        for element_id in elements {
            let fields = self.discriminant_fields_for_type(types, *element_id, node)?;
            let Some(fields) = fields else {
                return Ok((elements.to_vec(), None));
            };
            element_fields.push(fields);
        }

        // intersect field keys across all elements
        let Some((first_map, rest_maps)) = element_fields.split_first() else {
            return Ok((elements.to_vec(), None));
        };
        let mut common_keys: Vec<dir::StaticKey> = first_map.keys().copied().collect();
        for map in rest_maps {
            common_keys.retain(|key| map.contains_key(key));
        }

        if common_keys.is_empty() {
            return Ok((elements.to_vec(), None));
        }

        // sort keys to pick a stable primary discriminant
        common_keys.sort_by(|left, right| self.compare_static_keys(*left, *right, strings));

        // collect discriminant values per key
        let mut fields = Vec::with_capacity(common_keys.len());
        for key in &common_keys {
            let mut values = Vec::with_capacity(elements.len());
            let mut seen_values = HashSet::new();

            for map in &element_fields {
                let literal = map
                    .get(key)
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(node),
                        message: "missing discriminant literal value".to_string(),
                    })?;
                if !seen_values.insert(literal.key) {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(node),
                        message: "duplicate discriminant value in union".to_string(),
                    }
                    .into());
                }
                values.push(literal.clone());
            }

            fields.push(UnionDiscriminantField {
                key: *key,
                values,
                tag_by_value: HashMap::new(),
            });
        }

        // build tag order from the primary discriminant
        let primary_key = fields
            .first()
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(node),
                message: "missing union discriminant field".to_string(),
            })?
            .key;
        let primary_values = fields
            .iter()
            .find(|field| field.key == primary_key)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(node),
                message: "missing primary discriminant field".to_string(),
            })?
            .values
            .clone();
        let mut order: Vec<usize> = (0..elements.len()).collect();
        order.sort_by(|left, right| {
            DiscriminantKey::compare(
                primary_values[*left].key,
                primary_values[*right].key,
                strings,
            )
        });

        // reorder elements and discriminant values
        let ordered_elements = order.iter().map(|index| elements[*index]).collect();
        for field in &mut fields {
            let ordered_values = order
                .iter()
                .map(|index| field.values[*index].clone())
                .collect::<Vec<_>>();
            field.values = ordered_values;

            let mut tag_by_value = HashMap::new();
            for (tag, literal) in field.values.iter().enumerate() {
                if tag_by_value.insert(literal.key, tag as u32).is_some() {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(node),
                        message: "duplicate discriminant value in union".to_string(),
                    }
                    .into());
                }
            }
            field.tag_by_value = tag_by_value;
        }

        // return the ordered elements and discriminant metadata
        Ok((
            ordered_elements,
            Some(UnionDiscriminant {
                primary_key,
                fields,
            }),
        ))
    }

    /// Collect discriminant literal fields for an object-like type.
    fn discriminant_fields_for_type(
        &self,
        types: &dir::TypeTable,
        type_id: dir::LocalTypeId,
        node: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<Option<HashMap<dir::StaticKey, DiscriminantLiteral>>> {
        // track visited types to avoid recursion
        let mut visited = HashSet::new();
        self.discriminant_fields_for_type_inner(types, type_id, node, &mut visited)
    }

    /// Collect discriminant fields with recursion and alias expansion.
    fn discriminant_fields_for_type_inner(
        &self,
        types: &dir::TypeTable,
        type_id: dir::LocalTypeId,
        node: dir::AnchoredGlobalNodeId,
        visited: &mut HashSet<dir::LocalTypeId>,
    ) -> LowerResult<Option<HashMap<dir::StaticKey, DiscriminantLiteral>>> {
        // stop recursion on cycles
        if !visited.insert(type_id) {
            return Ok(Some(HashMap::new()));
        }

        // unwrap alias references before inspecting shape
        let dir_type = types.get_type(type_id);
        match dir_type {
            dir::Type::Reference(reference) => {
                if self.symbol_is(reference.symbol, dir::SymbolForm::TypeAlias)
                    && let Some(target) = types.get_alias_target_type_id(reference.symbol)
                {
                    return self.discriminant_fields_for_type_inner(types, target, node, visited);
                }

                if let Some(instance_id) = types.get_instance_type_id(reference.symbol) {
                    return self.discriminant_fields_for_type_inner(
                        types,
                        instance_id,
                        node,
                        visited,
                    );
                }

                Ok(None)
            }
            dir::Type::Object(object) => {
                let mut map = HashMap::new();
                for field in &object.fields {
                    if field.is_optional {
                        continue;
                    }

                    let literal = self.discriminant_literal_for_type(types, field.ty, node)?;
                    let Some(literal) = literal else {
                        continue;
                    };
                    map.insert(field.key, literal);
                }

                Ok(Some(map))
            }
            dir::Type::Intersection(intersection) => {
                let mut maps = Vec::with_capacity(intersection.elements.len());
                for element_id in &intersection.elements {
                    let Some(map) =
                        self.discriminant_fields_for_type_inner(types, *element_id, node, visited)?
                    else {
                        return Ok(None);
                    };
                    maps.push(map);
                }

                let Some((first, rest)) = maps.split_first() else {
                    return Ok(Some(HashMap::new()));
                };
                let mut merged = first.clone();
                for map in rest {
                    merged.retain(|key, literal| {
                        map.get(key).is_some_and(|other| other.key == literal.key)
                    });
                }

                Ok(Some(merged))
            }
            dir::Type::Value(value) => {
                self.discriminant_fields_for_type_inner(types, value.value, node, visited)
            }
            _ => Ok(None),
        }
    }

    /// Resolve a discriminant literal for a type when possible.
    fn discriminant_literal_for_type(
        &self,
        types: &dir::TypeTable,
        type_id: dir::LocalTypeId,
        node: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<Option<DiscriminantLiteral>> {
        // track visited types to avoid recursion
        let mut visited = HashSet::new();
        self.discriminant_literal_for_type_inner(types, type_id, node, &mut visited)
    }

    /// Resolve a discriminant literal with recursion tracking.
    fn discriminant_literal_for_type_inner(
        &self,
        types: &dir::TypeTable,
        type_id: dir::LocalTypeId,
        node: dir::AnchoredGlobalNodeId,
        visited: &mut HashSet<dir::LocalTypeId>,
    ) -> LowerResult<Option<DiscriminantLiteral>> {
        // stop recursion on cycles
        if !visited.insert(type_id) {
            return Ok(None);
        }

        // unwrap alias references
        let dir_type = types.get_type(type_id);
        match dir_type {
            dir::Type::Reference(reference) => {
                if self.symbol_is(reference.symbol, dir::SymbolForm::TypeAlias)
                    && let Some(target) = types.get_alias_target_type_id(reference.symbol)
                {
                    return self.discriminant_literal_for_type_inner(types, target, node, visited);
                }

                Ok(None)
            }
            dir::Type::Literal(value) => {
                let literal = match value {
                    dir::LiteralType::Null => DiscriminantValue::Null,
                    dir::LiteralType::Undefined => DiscriminantValue::Undefined,
                    dir::LiteralType::ScalarLiteral(scalar) => match scalar {
                        dir::ScalarLiteral::Null => DiscriminantValue::Null,
                        dir::ScalarLiteral::Boolean(value) => DiscriminantValue::Boolean(*value),
                        dir::ScalarLiteral::Integer(value) => {
                            let (bits, number) = DiscriminantKey::canonical_number_bits(
                                *value as f64,
                                self.diagnostic_anchor(node),
                            )?;
                            return Ok(Some(DiscriminantLiteral {
                                value: DiscriminantValue::Number { value: number },
                                type_id,
                                key: DiscriminantKey::Number(bits),
                            }));
                        }
                        dir::ScalarLiteral::Float(value) => {
                            let (bits, number) = DiscriminantKey::canonical_number_bits(
                                *value,
                                self.diagnostic_anchor(node),
                            )?;
                            return Ok(Some(DiscriminantLiteral {
                                value: DiscriminantValue::Number { value: number },
                                type_id,
                                key: DiscriminantKey::Number(bits),
                            }));
                        }
                        dir::ScalarLiteral::Bigint(value) => DiscriminantValue::Bigint(*value),
                        dir::ScalarLiteral::String(value) => DiscriminantValue::String(*value),
                        dir::ScalarLiteral::Character(_)
                        | dir::ScalarLiteral::RegexString { .. } => {
                            return Ok(None);
                        }
                    },
                    dir::LiteralType::Primitive(dir::PrimitiveType::UniqueSymbol) => {
                        DiscriminantValue::UniqueSymbol
                    }
                    _ => return Ok(None),
                };

                let key = DiscriminantKey::from_value(&literal, self.diagnostic_anchor(node))?;
                Ok(Some(DiscriminantLiteral {
                    value: literal,
                    type_id,
                    key,
                }))
            }
            dir::Type::Value(value) => {
                self.discriminant_literal_for_type_inner(types, value.value, node, visited)
            }
            _ => Ok(None),
        }
    }

    /// Compute the minimal unsigned tag width for a tag count.
    fn tag_width_for_discriminant_count(
        &self,
        count: usize,
        node: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<u16> {
        if count <= u8::MAX as usize {
            return Ok(8);
        }
        if count <= u16::MAX as usize {
            return Ok(16);
        }
        if count <= u32::MAX as usize {
            return Ok(32);
        }
        if count <= u64::MAX as usize {
            return Ok(64);
        }

        Err(LowerError::UnsupportedConstruct {
            anchor: self.diagnostic_anchor(node),
            message: "union tag count exceeds supported width".to_string(),
        }
        .into())
    }

    /// Compare static keys for deterministic discriminant field selection.
    fn compare_static_keys(
        &self,
        left: dir::StaticKey,
        right: dir::StaticKey,
        strings: &StringPool,
    ) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        match (left, right) {
            (dir::StaticKey::Name(left), dir::StaticKey::Name(right)) => {
                strings.get(left).cmp(&strings.get(right))
            }
            (dir::StaticKey::Number(left), dir::StaticKey::Number(right)) => {
                strings.get(left).cmp(&strings.get(right))
            }
            (dir::StaticKey::Name(left), dir::StaticKey::Number(right)) => {
                let ordering = strings.get(left).cmp(&strings.get(right));
                if ordering == Ordering::Equal {
                    Ordering::Less
                } else {
                    ordering
                }
            }
            (dir::StaticKey::Number(left), dir::StaticKey::Name(right)) => {
                let ordering = strings.get(left).cmp(&strings.get(right));
                if ordering == Ordering::Equal {
                    Ordering::Greater
                } else {
                    ordering
                }
            }
            (dir::StaticKey::Symbol(left), dir::StaticKey::Symbol(right)) => {
                self.compare_symbol_keys(left, right, strings)
            }
            (dir::StaticKey::Symbol(_), _) => Ordering::Greater,
            (_, dir::StaticKey::Symbol(_)) => Ordering::Less,
        }
    }

    /// Compare symbol keys for deterministic ordering.
    fn compare_symbol_keys(
        &self,
        left: dir::SymbolKey,
        right: dir::SymbolKey,
        strings: &StringPool,
    ) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        match (left, right) {
            (dir::SymbolKey::Registry(left), dir::SymbolKey::Registry(right)) => {
                strings.get(left).cmp(&strings.get(right))
            }
            (dir::SymbolKey::Unique(left), dir::SymbolKey::Unique(right)) => {
                self.compare_unique_symbol_keys(left, right, strings)
            }
            (dir::SymbolKey::Registry(_), dir::SymbolKey::Unique(_)) => Ordering::Less,
            (dir::SymbolKey::Unique(_), _) => Ordering::Greater,
        }
    }

    /// Compare unique symbol keys using qualified names when available.
    fn compare_unique_symbol_keys(
        &self,
        left: dir::GlobalSymbolId,
        right: dir::GlobalSymbolId,
        strings: &StringPool,
    ) -> std::cmp::Ordering {
        // build qualified names for unique symbols
        let revision = self.context.revision();
        let left_name =
            format_unique_symbol_qualified_name(left, &self.compiler.repository, revision, strings);
        let right_name = format_unique_symbol_qualified_name(
            right,
            &self.compiler.repository,
            revision,
            strings,
        );

        // prefer qualified ordering with a stable fallback
        match (left_name, right_name) {
            (Some(left_name), Some(right_name)) => left_name.cmp(&right_name),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => left.cmp(&right),
        }
    }
}

impl DiscriminantKey {
    /// Compare two discriminant keys using canonical ordering rules.
    fn compare(
        left: DiscriminantKey,
        right: DiscriminantKey,
        strings: &StringPool,
    ) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        let left_rank = left.rank();
        let right_rank = right.rank();
        if left_rank != right_rank {
            return left_rank.cmp(&right_rank);
        }

        match (left, right) {
            (DiscriminantKey::Boolean(left), DiscriminantKey::Boolean(right)) => left.cmp(&right),
            (DiscriminantKey::Number(left), DiscriminantKey::Number(right)) => {
                let left = f64::from_bits(left);
                let right = f64::from_bits(right);
                left.partial_cmp(&right).unwrap_or(Ordering::Equal)
            }
            (DiscriminantKey::Bigint(left), DiscriminantKey::Bigint(right)) => left.cmp(&right),
            (DiscriminantKey::String(left), DiscriminantKey::String(right)) => {
                strings.get(left).cmp(&strings.get(right))
            }
            (DiscriminantKey::UniqueSymbol, DiscriminantKey::UniqueSymbol) => Ordering::Equal,
            _ => Ordering::Equal,
        }
    }

    /// Build a canonical key for a discriminant value.
    fn from_value(value: &DiscriminantValue, anchor: DiagnosticAnchor) -> LowerResult<Self> {
        match value {
            DiscriminantValue::Null => Ok(DiscriminantKey::Null),
            DiscriminantValue::Undefined => Ok(DiscriminantKey::Undefined),
            DiscriminantValue::Boolean(value) => Ok(DiscriminantKey::Boolean(*value)),
            DiscriminantValue::Number { value } => {
                let (bits, _) = Self::canonical_number_bits(*value, anchor)?;
                Ok(DiscriminantKey::Number(bits))
            }
            DiscriminantValue::Bigint(value) => Ok(DiscriminantKey::Bigint(*value)),
            DiscriminantValue::String(value) => Ok(DiscriminantKey::String(*value)),
            DiscriminantValue::UniqueSymbol => Ok(DiscriminantKey::UniqueSymbol),
        }
    }

    /// Build a canonical key from a scalar literal expression.
    pub(crate) fn from_scalar_literal(
        literal: &dir::ScalarLiteral,
        anchor: DiagnosticAnchor,
    ) -> LowerResult<Option<Self>> {
        let key = match literal {
            dir::ScalarLiteral::Null => DiscriminantKey::Null,
            dir::ScalarLiteral::Boolean(value) => DiscriminantKey::Boolean(*value),
            dir::ScalarLiteral::Integer(value) => {
                let (bits, _) = Self::canonical_number_bits(*value as f64, anchor.clone())?;
                DiscriminantKey::Number(bits)
            }
            dir::ScalarLiteral::Float(value) => {
                let (bits, _) = Self::canonical_number_bits(*value, anchor)?;
                DiscriminantKey::Number(bits)
            }
            dir::ScalarLiteral::Bigint(value) => DiscriminantKey::Bigint(*value),
            dir::ScalarLiteral::String(value) => DiscriminantKey::String(*value),
            dir::ScalarLiteral::Character(_) | dir::ScalarLiteral::RegexString { .. } => {
                return Ok(None);
            }
        };

        Ok(Some(key))
    }

    /// Compute the canonical rank for discriminant ordering.
    fn rank(self) -> u8 {
        match self {
            DiscriminantKey::Null => 0,
            DiscriminantKey::Undefined => 1,
            DiscriminantKey::Boolean(false) => 2,
            DiscriminantKey::Boolean(true) => 3,
            DiscriminantKey::Number(_) => 4,
            DiscriminantKey::Bigint(_) => 5,
            DiscriminantKey::String(_) => 6,
            DiscriminantKey::UniqueSymbol => 7,
        }
    }

    /// Normalize a number for discriminant ordering and lookup.
    fn canonical_number_bits(value: f64, anchor: DiagnosticAnchor) -> LowerResult<(u64, f64)> {
        if value.is_nan() {
            return Err(LowerError::UnsupportedConstruct {
                anchor,
                message: "NaN is not a valid discriminant literal".to_string(),
            }
            .into());
        }

        let value = if value == 0.0 { 0.0 } else { value };
        Ok((value.to_bits(), value))
    }
}
