use std::hash::Hasher;

use destack_core::{StableHasher, StringId};

use crate::{
    Access, Attribute, AttributeArgs, AttributeIdentifier, AttributeValue, Constant, Copy, Field,
    FloatType, GlobalStorage, Lifetime, LifetimeParameter, LifetimeTerm, LocalNodeId, Multiplicity,
    Nullability, ReferenceKind, SignatureParameter, Space, Static, StaticField, StaticId,
    StaticKey, Storage, Symbol, TensorDimension, TensorDimensionOrder, TensorFormat,
    TensorReduction, TensorSharding, TensorShardingAxis, TensorViewFormat, Tree, Type,
    TypeFingerprint, TypeId,
};

impl Tree {
    /// Compute the stable canonical fingerprint of one type.
    pub fn type_fingerprint(&self, ty: TypeId) -> TypeFingerprint {
        let mut hasher = TypeHasher::new();
        hasher.hash_type(ty, self);

        TypeFingerprint::from_raw(hasher.hasher.finish_u128())
    }
}

/// Stable structural type hasher.
pub(super) struct TypeHasher {
    /// The stable hash under construction.
    hasher: StableHasher,
}

impl TypeHasher {
    /// Derive one generic instance symbol from its concrete arguments.
    pub(super) fn symbol(base: Symbol, arguments: &[StaticId], tree: &Tree) -> Symbol {
        if arguments.is_empty() {
            return base;
        }

        let mut hasher = Self::for_instance(base);
        hasher.hash_length(arguments.len());
        for argument in arguments {
            hasher.hash_static(*argument, tree);
        }

        Symbol::from_raw(hasher.hasher.finish_u64())
    }

    /// Create a type fingerprint hasher.
    fn new() -> Self {
        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(b"destack.mir.type.v1");

        Self { hasher }
    }

    /// Create a generic instance hasher.
    fn for_instance(base: Symbol) -> Self {
        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(b"destack.mir.instance.v1");
        hasher.write_u64(base.raw());

        Self { hasher }
    }

    /// Hash one concrete static value.
    fn hash_static(&mut self, id: StaticId, tree: &Tree) {
        let value = tree.static_value(id);

        match value {
            Static::Null => self.hasher.write_u8(0),
            Static::Undefined => self.hasher.write_u8(1),
            Static::Boolean(value) => {
                self.hasher.write_u8(2);
                self.hash_boolean(*value);
            }
            Static::Integer(value) => {
                self.hasher.write_u8(3);
                self.hasher.write_i64(*value);
            }
            Static::Bigint(value) => {
                self.hasher.write_u8(4);
                self.hasher.write_i64(*value);
            }
            Static::Float(bits) => {
                self.hasher.write_u8(5);
                self.hasher.write_u64(*bits);
            }
            Static::Character(value) => {
                self.hasher.write_u8(6);
                self.hasher.write_u32(u32::from(*value));
            }
            Static::String(value) => {
                self.hasher.write_u8(7);
                self.hasher.write_u64(value.raw());
            }
            Static::Regex { content, flags } => {
                self.hasher.write_u8(8);
                self.hasher.write_u64(content.raw());
                self.hash_string_maybe(*flags);
            }
            Static::Type(ty) => {
                self.hasher.write_u8(9);
                self.hash_type(*ty, tree);
            }
            Static::Array(elements) => {
                self.hasher.write_u8(10);
                self.hash_static_values(elements, tree);
            }
            Static::FixedArray { value, length } => {
                self.hasher.write_u8(11);
                self.hash_static(*value, tree);
                self.hasher.write_u64(*length);
            }
            Static::Tuple(elements) => {
                self.hasher.write_u8(12);
                self.hash_static_values(elements, tree);
            }
            Static::Newtype { ty, value } => {
                self.hasher.write_u8(13);
                self.hash_type(*ty, tree);
                self.hash_static(*value, tree);
            }
            Static::Object(fields) => {
                self.hasher.write_u8(14);
                self.hash_static_fields(fields, tree);
            }
            Static::Struct { ty, fields } => {
                self.hasher.write_u8(15);
                self.hash_type(*ty, tree);
                self.hash_static_fields(fields, tree);
            }
        }
    }

    /// Hash one ordered static value sequence.
    fn hash_static_values(&mut self, values: &[StaticId], tree: &Tree) {
        self.hash_length(values.len());
        for value in values {
            self.hash_static(*value, tree);
        }
    }

    /// Hash one ordered static field set.
    fn hash_static_fields(&mut self, fields: &[StaticField], tree: &Tree) {
        self.hash_length(fields.len());
        for field in fields {
            self.hash_static_key(&field.key);
            self.hash_static(field.value, tree);
        }
    }

    /// Hash one static object key.
    fn hash_static_key(&mut self, key: &StaticKey) {
        match key {
            StaticKey::Name(name) => {
                self.hasher.write_u8(0);
                self.hasher.write_u64(name.raw());
            }
            StaticKey::Index(index) => {
                self.hasher.write_u8(1);
                self.hasher.write_u64(*index);
            }
            StaticKey::Unique(symbol) => {
                self.hasher.write_u8(2);
                self.hasher.write_u64(symbol.raw());
            }
            StaticKey::Registry(name) => {
                self.hasher.write_u8(3);
                self.hasher.write_u64(name.raw());
            }
        }
    }

    /// Hash one type through structural or identified identity.
    fn hash_type(&mut self, id: TypeId, tree: &Tree) {
        // terminate identified recursion at the declaration symbol
        if let Some(symbol) = tree.type_symbol(id) {
            self.hasher.write_u8(0xff);
            self.hasher.write_u64(symbol.raw());

            return;
        }

        match tree.get(id) {
            Type::Error => self.hasher.write_u8(0),
            Type::Never => self.hasher.write_u8(1),
            Type::Void => self.hasher.write_u8(2),
            Type::Boolean => self.hasher.write_u8(3),
            Type::Character => self.hasher.write_u8(4),
            Type::Int { width, is_signed } => {
                self.hasher.write_u8(5);
                self.hasher.write_u16(*width);
                self.hash_boolean(*is_signed);
            }
            Type::Isize => self.hasher.write_u8(6),
            Type::Usize => self.hasher.write_u8(7),
            Type::Float(format) => {
                self.hasher.write_u8(8);
                self.hash_float_type(*format);
            }
            Type::TypeDescriptor => self.hasher.write_u8(9),
            Type::TypeId => self.hasher.write_u8(10),
            Type::Atomic { value } => {
                self.hasher.write_u8(11);
                self.hash_type(*value, tree);
            }
            Type::Dynamic {
                kind,
                lifetime,
                constraint,
                storage,
                access,
                nullability,
            } => {
                self.hasher.write_u8(12);
                self.hash_reference_kind(*kind);
                self.hash_lifetime(lifetime);
                self.hash_type(*constraint, tree);
                self.hash_storage(*storage);
                self.hash_access(*access);
                self.hash_nullability(*nullability);
            }
            Type::Reference {
                kind,
                lifetime,
                storage,
                access,
                pointee,
                nullability,
            } => {
                self.hasher.write_u8(13);
                self.hash_reference_kind(*kind);
                self.hash_lifetime(lifetime);
                self.hash_storage(*storage);
                self.hash_access(*access);
                self.hash_type(*pointee, tree);
                self.hash_nullability(*nullability);
            }
            Type::Pointer {
                pointee,
                access,
                nullability,
            } => {
                self.hasher.write_u8(31);
                self.hash_type(*pointee, tree);
                self.hash_access(*access);
                self.hash_nullability(*nullability);
            }
            Type::Slice {
                kind,
                lifetime,
                element,
                storage,
                access,
                nullability,
            } => {
                self.hasher.write_u8(14);
                self.hash_reference_kind(*kind);
                self.hash_lifetime(lifetime);
                self.hash_type(*element, tree);
                self.hash_storage(*storage);
                self.hash_access(*access);
                self.hash_nullability(*nullability);
            }
            Type::Uninit { value } => {
                self.hasher.write_u8(15);
                self.hash_type(*value, tree);
            }
            Type::ManuallyDrop { value } => {
                self.hasher.write_u8(16);
                self.hash_type(*value, tree);
            }
            Type::FixedArray {
                element,
                length,
                copy,
            } => {
                self.hasher.write_u8(17);
                self.hash_type(*element, tree);
                self.hasher.write_u64(*length);
                self.hash_copy(*copy);
            }
            Type::Tuple { elements, copy } => {
                self.hasher.write_u8(18);
                self.hash_types(elements, tree);
                self.hash_copy(*copy);
            }
            Type::Struct { fields, copy } => {
                self.hasher.write_u8(19);
                self.hash_length(fields.len());
                for field in fields {
                    self.hash_field(*field, tree);
                }
                self.hash_copy(*copy);
            }
            Type::Newtype { inner, copy } => {
                self.hasher.write_u8(20);
                self.hash_type(*inner, tree);
                self.hash_copy(*copy);
            }
            Type::Variant {
                discriminant,
                cases,
                copy,
            } => {
                self.hasher.write_u8(21);
                self.hash_type(*discriminant, tree);
                self.hash_length(cases.len());
                for case in cases {
                    self.hash_constant(&case.discriminant);
                    self.hash_type(case.ty, tree);
                }
                self.hash_copy(*copy);
            }
            Type::Vector {
                element,
                lanes,
                copy,
            } => {
                self.hasher.write_u8(22);
                self.hash_type(*element, tree);
                self.hasher.write_u32(*lanes);
                self.hash_copy(*copy);
            }
            Type::Tensor {
                kind,
                lifetime,
                storage,
                access,
                element,
                shape,
                format,
                sharding,
                nullability,
            } => {
                self.hasher.write_u8(23);
                self.hash_reference_kind(*kind);
                self.hash_lifetime(lifetime);
                self.hash_storage(*storage);
                self.hash_access(*access);
                self.hash_type(*element, tree);
                self.hash_tensor_shape(shape);
                self.hash_tensor_format(*format);
                self.hash_tensor_sharding(sharding);
                self.hash_nullability(*nullability);
            }
            Type::TensorView {
                kind,
                lifetime,
                storage,
                access,
                element,
                shape,
                format,
                sharding,
                nullability,
            } => {
                self.hasher.write_u8(24);
                self.hash_reference_kind(*kind);
                self.hash_lifetime(lifetime);
                self.hash_storage(*storage);
                self.hash_access(*access);
                self.hash_type(*element, tree);
                self.hash_tensor_shape(shape);
                self.hash_tensor_view_format(*format);
                self.hash_tensor_sharding(sharding);
                self.hash_nullability(*nullability);
            }
            Type::FunctionSignature {
                lifetimes,
                parameters,
                result,
            } => {
                self.hasher.write_u8(25);
                self.hash_lifetime_parameters(lifetimes);
                self.hash_length(parameters.len());
                for parameter in parameters {
                    self.hash_parameter(parameter, tree);
                }
                self.hash_type(*result, tree);
            }
            Type::Function {
                signature,
                multiplicity,
                kind,
                lifetime,
                storage,
                access,
                nullability,
            } => {
                self.hasher.write_u8(26);
                self.hash_type(*signature, tree);
                self.hasher.write_u8(match multiplicity {
                    Multiplicity::Repeatable => 0,
                    Multiplicity::Once => 1,
                });
                self.hash_reference_kind(*kind);
                self.hash_lifetime(lifetime);
                self.hash_storage(*storage);
                self.hash_access(*access);
                self.hash_nullability(*nullability);
            }
            Type::FunctionPointer { signature } => {
                self.hasher.write_u8(27);
                self.hash_type(*signature, tree);
            }
            Type::Application { base, lifetimes } => {
                self.hasher.write_u8(30);
                self.hash_type(*base, tree);
                self.hash_lifetimes(lifetimes);
            }
        }
    }

    /// Hash an ordered type sequence.
    fn hash_types(&mut self, types: &[TypeId], tree: &Tree) {
        self.hash_length(types.len());
        for ty in types {
            self.hash_type(*ty, tree);
        }
    }

    /// Hash one structural field declaration.
    fn hash_field(&mut self, id: LocalNodeId<Field>, tree: &Tree) {
        let field = tree.get(id);
        self.hash_string_maybe(field.name);
        self.hash_type(field.ty, tree);

        let attributes = tree.attributes(id);
        self.hash_length(attributes.len());
        for attribute in attributes {
            self.hash_attribute(attribute, tree);
        }
    }

    /// Hash one callable parameter type.
    fn hash_parameter(&mut self, parameter: &SignatureParameter, tree: &Tree) {
        self.hash_type(parameter.ty, tree);
    }

    /// Hash one field attribute, recursively hashing type ids.
    fn hash_attribute(&mut self, attribute: &Attribute, tree: &Tree) {
        self.hash_attribute_identifier(attribute.name);
        match &attribute.args {
            AttributeArgs::None => self.hasher.write_u8(0),
            AttributeArgs::Value(value) => {
                self.hasher.write_u8(1);
                self.hash_attribute_value(value, tree);
            }
            AttributeArgs::Values(values) => {
                self.hasher.write_u8(2);
                self.hash_length(values.len());
                for value in values {
                    self.hash_attribute_value(value, tree);
                }
            }
            AttributeArgs::KeyValues(values) => {
                self.hasher.write_u8(3);
                self.hash_length(values.len());
                for value in values {
                    self.hash_attribute_identifier(value.key);
                    self.hash_attribute_value(&value.value, tree);
                }
            }
        }
    }

    /// Hash one attribute value, recursively hashing type ids.
    fn hash_attribute_value(&mut self, value: &AttributeValue, tree: &Tree) {
        match value {
            AttributeValue::Identifier(value) => {
                self.hasher.write_u8(0);
                self.hash_attribute_identifier(*value);
            }
            AttributeValue::Type(ty) => {
                self.hasher.write_u8(1);
                self.hash_type(*ty, tree);
            }
            AttributeValue::Integer(value) => {
                self.hasher.write_u8(2);
                self.hasher.write_i128(*value);
            }
            AttributeValue::Float(value) => {
                self.hasher.write_u8(3);
                self.hasher.write_u64(value.bits);
            }
            AttributeValue::Boolean(value) => {
                self.hasher.write_u8(4);
                self.hash_boolean(*value);
            }
            AttributeValue::String(value) => {
                self.hasher.write_u8(5);
                self.hasher.write_u64(value.raw());
            }
            AttributeValue::List(values) => {
                self.hasher.write_u8(6);
                self.hash_length(values.len());
                for value in values {
                    self.hash_attribute_value(value, tree);
                }
            }
            AttributeValue::Object(values) => {
                self.hasher.write_u8(9);
                self.hash_length(values.len());
                for value in values {
                    self.hash_attribute_identifier(value.key);
                    self.hash_attribute_value(&value.value, tree);
                }
            }
            AttributeValue::Missing => self.hasher.write_u8(7),
            AttributeValue::Error => self.hasher.write_u8(8),
        }
    }

    /// Hash one sequence length independently of the host pointer width.
    fn hash_length(&mut self, length: usize) {
        self.hasher.write_u64(length as u64);
    }

    /// Hash one boolean with a canonical one-byte encoding.
    fn hash_boolean(&mut self, value: bool) {
        self.hasher.write_u8(u8::from(value));
    }

    /// Hash one optional interned string.
    fn hash_string_maybe(&mut self, value: Option<StringId>) {
        match value {
            Some(value) => {
                self.hasher.write_u8(1);
                self.hasher.write_u64(value.raw());
            }
            None => self.hasher.write_u8(0),
        }
    }

    /// Hash one MIR access mode.
    fn hash_access(&mut self, access: Access) {
        let tag = match access {
            Access::Readonly => 0,
            Access::Mutable => 1,
            Access::Exclusive => 2,
        };
        self.hasher.write_u8(tag);
    }

    /// Hash one MIR storage space.
    fn hash_space(&mut self, space: Space) {
        let tag = match space {
            Space::Local => 0,
            Space::Shared => 1,
        };
        self.hasher.write_u8(tag);
    }

    /// Hash one MIR reference storage.
    fn hash_storage(&mut self, storage: Storage) {
        match storage {
            Storage::Heap(space) => {
                self.hasher.write_u8(0);
                self.hash_space(space);
            }
            Storage::Frame => self.hasher.write_u8(1),
            Storage::Global(global) => {
                self.hasher.write_u8(2);
                let tag = match global {
                    GlobalStorage::Constant => 0,
                    GlobalStorage::Local => 1,
                    GlobalStorage::Shared => 2,
                    GlobalStorage::Immortal => 3,
                };
                self.hasher.write_u8(tag);
            }
        }
    }

    /// Hash one MIR reference kind.
    fn hash_reference_kind(&mut self, kind: ReferenceKind) {
        let tag = match kind {
            ReferenceKind::Managed => 0,
            ReferenceKind::Unique => 1,
            ReferenceKind::Borrowed => 2,
        };
        self.hasher.write_u8(tag);
    }

    /// Hash one MIR nullability mode.
    fn hash_nullability(&mut self, nullability: Nullability) {
        let tag = match nullability {
            Nullability::None => 0,
            Nullability::Null => 1,
            Nullability::Undefined => 2,
            Nullability::NullOrUndefined => 3,
        };
        self.hasher.write_u8(tag);
    }

    /// Hash one MIR copy property.
    fn hash_copy(&mut self, copy: Copy) {
        let tag = match copy {
            Copy::Yes => 0,
            Copy::No => 1,
        };
        self.hasher.write_u8(tag);
    }

    /// Hash one concrete floating-point format.
    fn hash_float_type(&mut self, format: FloatType) {
        let tag = match format {
            FloatType::Float16 => 0,
            FloatType::Bfloat16 => 1,
            FloatType::Float32 => 2,
            FloatType::Float64 => 3,
        };
        self.hasher.write_u8(tag);
    }

    /// Hash one applied MIR lifetime.
    fn hash_lifetime(&mut self, lifetime: &Lifetime) {
        self.hash_length(lifetime.terms.len());
        for term in &lifetime.terms {
            match term {
                LifetimeTerm::Static => self.hasher.write_u8(0),
                LifetimeTerm::Frame => self.hasher.write_u8(1),
                LifetimeTerm::Slot(slot) => {
                    self.hasher.write_u8(2);
                    self.hasher.write_u32(slot.0);
                }
            }
        }
    }

    /// Hash one ordered applied lifetime sequence.
    fn hash_lifetimes(&mut self, lifetimes: &[Lifetime]) {
        self.hash_length(lifetimes.len());
        for lifetime in lifetimes {
            self.hash_lifetime(lifetime);
        }
    }

    /// Hash lifetime structure without source-only parameter names.
    fn hash_lifetime_parameters(&mut self, lifetimes: &[LifetimeParameter]) {
        self.hash_length(lifetimes.len());
        for lifetime in lifetimes {
            self.hash_length(lifetime.outlives.len());
            for target in &lifetime.outlives {
                self.hasher.write_u32(target.0);
            }
        }
    }

    /// Hash one executable scalar constant.
    fn hash_constant(&mut self, constant: &Constant) {
        match constant {
            Constant::Null => self.hasher.write_u8(0),
            Constant::Undefined => self.hasher.write_u8(1),
            Constant::Boolean { value } => {
                self.hasher.write_u8(2);
                self.hash_boolean(*value);
            }
            Constant::Int {
                value,
                width,
                is_signed,
            } => {
                self.hasher.write_u8(3);
                self.hasher.write_i128(*value);
                self.hasher.write_u16(*width);
                self.hash_boolean(*is_signed);
            }
            Constant::UInt { value, width } => {
                self.hasher.write_u8(4);
                self.hasher.write_u128(*value);
                self.hasher.write_u16(*width);
            }
            Constant::Float { bits, format } => {
                self.hasher.write_u8(5);
                self.hasher.write_u64(*bits);
                self.hash_float_type(*format);
            }
            Constant::Char { value } => {
                self.hasher.write_u8(6);
                self.hasher.write_u32(u32::from(*value));
            }
            Constant::Uninit => self.hasher.write_u8(7),
            Constant::Zeroed => self.hasher.write_u8(8),
        }
    }

    /// Hash one tensor shape.
    fn hash_tensor_shape(&mut self, shape: &[TensorDimension]) {
        self.hash_length(shape.len());
        for dimension in shape {
            match dimension {
                TensorDimension::Static(size) => {
                    self.hasher.write_u8(0);
                    self.hasher.write_u64(*size);
                }
                TensorDimension::Symbol(name) => {
                    self.hasher.write_u8(1);
                    self.hasher.update_len_prefixed(name.as_bytes());
                }
                TensorDimension::Dynamic => self.hasher.write_u8(2),
            }
        }
    }

    /// Hash one tensor dimension order.
    fn hash_tensor_order(&mut self, order: TensorDimensionOrder) {
        let tag = match order {
            TensorDimensionOrder::RowMajor => 0,
            TensorDimensionOrder::ColumnMajor => 1,
        };
        self.hasher.write_u8(tag);
    }

    /// Hash one tensor format.
    fn hash_tensor_format(&mut self, format: TensorFormat) {
        match format {
            TensorFormat::Dense { order } => {
                self.hasher.write_u8(0);
                self.hash_tensor_order(order);
            }
        }
    }

    /// Hash one tensor view format.
    fn hash_tensor_view_format(&mut self, format: TensorViewFormat) {
        match format {
            TensorViewFormat::Dense { order } => {
                self.hasher.write_u8(0);
                self.hash_tensor_order(order);
            }
            TensorViewFormat::Strided => self.hasher.write_u8(1),
        }
    }

    /// Hash one tensor placement.
    fn hash_tensor_sharding(&mut self, sharding: &TensorSharding) {
        match sharding {
            TensorSharding::Unsharded => self.hasher.write_u8(0),
            TensorSharding::Sharding { axes } => {
                self.hasher.write_u8(1);
                self.hash_length(axes.len());
                for axis in axes {
                    self.hash_tensor_axis(*axis);
                }
            }
        }
    }

    /// Hash one tensor placement axis.
    fn hash_tensor_axis(&mut self, axis: TensorShardingAxis) {
        match axis {
            TensorShardingAxis::Shard { axis } => {
                self.hasher.write_u8(0);
                self.hasher.write_i32(axis);
            }
            TensorShardingAxis::Replicate => self.hasher.write_u8(1),
            TensorShardingAxis::Partial { reduction } => {
                self.hasher.write_u8(2);
                self.hash_tensor_reduction(reduction);
            }
        }
    }

    /// Hash one tensor reduction.
    fn hash_tensor_reduction(&mut self, reduction: TensorReduction) {
        let tag = match reduction {
            TensorReduction::Add => 0,
            TensorReduction::Multiply => 1,
            TensorReduction::Minimum => 2,
            TensorReduction::Maximum => 3,
            TensorReduction::And => 4,
            TensorReduction::Or => 5,
        };
        self.hasher.write_u8(tag);
    }

    /// Hash one parsed attribute identifier.
    fn hash_attribute_identifier(&mut self, identifier: AttributeIdentifier) {
        match identifier {
            AttributeIdentifier::Identifier(name) => {
                self.hasher.write_u8(0);
                self.hasher.write_u64(name.raw());
            }
            AttributeIdentifier::Missing => self.hasher.write_u8(1),
            AttributeIdentifier::Error => self.hasher.write_u8(2),
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_core::StringId;

    use crate::{
        Access, Copy, Field, Lifetime, Nullability, ReferenceKind, Space, Static, Storage, Symbol,
        Tree, Type,
    };

    /// Structural instance symbols are independent of local type allocation order.
    #[test]
    fn test_mangle_structural_instances_across_trees() {
        let base = Symbol::named(StringId::for_text("library.pick"));

        let mut first = Tree::new();
        let first_int = first.intern_type(Type::INT32);
        let first_bool = first.intern_type(Type::Boolean);
        let first_tuple = first.intern_type(Type::Tuple {
            elements: vec![first_int, first_bool],
            copy: Copy::Yes,
        });
        let first_int = first.intern_static(Static::Type(first_int));
        let first_bool = first.intern_static(Static::Type(first_bool));
        let first_tuple = first.intern_static(Static::Type(first_tuple));

        let mut second = Tree::new();
        let second_bool = second.intern_type(Type::Boolean);
        let second_int = second.intern_type(Type::INT32);
        let second_tuple = second.intern_type(Type::Tuple {
            elements: vec![second_int, second_bool],
            copy: Copy::Yes,
        });
        let second_tuple = second.intern_static(Static::Type(second_tuple));

        assert_eq!(
            base.instantiate(&[first_tuple], &first),
            base.instantiate(&[second_tuple], &second)
        );
        assert_ne!(
            base.instantiate(&[first_int], &first),
            base.instantiate(&[first_bool], &first)
        );
    }

    /// Nominal instance symbols depend on declaration identity rather than recursive layout.
    #[test]
    fn test_mangle_identified_instances_by_symbol() {
        let base = Symbol::named(StringId::for_text("library.consume"));
        let first_name = Symbol::named(StringId::for_text("library.First"));
        let second_name = Symbol::named(StringId::for_text("library.Second"));
        let mut tree = Tree::new();

        let first = tree.reserve_type(first_name);
        let first_field = tree.intern_field(
            Field {
                name: None,
                ty: first,
            },
            Vec::new(),
        );
        tree.define_type(
            first,
            Type::Struct {
                fields: vec![first_field],
                copy: Copy::No,
            },
        );

        let second = tree.reserve_type(second_name);
        tree.define_type(second, Type::Void);

        let mut foreign_tree = Tree::new();
        let foreign_first = foreign_tree.reserve_type(first_name);
        foreign_tree.define_type(foreign_first, Type::Void);
        let first = tree.intern_static(Static::Type(first));
        let second = tree.intern_static(Static::Type(second));
        let foreign_first = foreign_tree.intern_static(Static::Type(foreign_first));
        assert_ne!(
            base.instantiate(&[first], &tree),
            base.instantiate(&[second], &tree)
        );
        assert_eq!(
            base.instantiate(&[first], &tree),
            base.instantiate(&[foreign_first], &foreign_tree)
        );
    }

    /// Lifetime variants select one shared runtime instance symbol.
    #[test]
    fn test_mangle_lifetime_variants_as_one_runtime_instance() {
        let mut tree = Tree::new();
        let pointee = tree.intern_type(Type::INT32);
        let local = tree.intern_type(Type::Reference {
            kind: ReferenceKind::Borrowed,
            lifetime: Lifetime::slot(0),
            storage: Storage::Heap(Space::Local),
            access: Access::Readonly,
            pointee,
            nullability: Nullability::None,
        });
        let static_ = tree.intern_type(Type::Reference {
            kind: ReferenceKind::Borrowed,
            lifetime: Lifetime::static_storage(),
            storage: Storage::Heap(Space::Local),
            access: Access::Readonly,
            pointee,
            nullability: Nullability::None,
        });
        let local = tree.intern_representation(local);
        let static_ = tree.intern_representation(static_);
        assert_eq!(local, static_);

        let local = tree.intern_static(Static::Type(local));
        let static_ = tree.intern_static(Static::Type(static_));
        let base = Symbol::named(StringId::for_text("library.inspect"));

        assert_eq!(
            base.instantiate(&[local], &tree),
            base.instantiate(&[static_], &tree)
        );
    }

    /// Static values contribute their exact value identity to instance symbols.
    #[test]
    fn test_mangle_static_instances_by_value() {
        let mut tree = Tree::new();
        let base = Symbol::named(StringId::for_text("library.take"));
        let first = tree.intern_static(Static::Integer(4));
        let second = tree.intern_static(Static::Integer(8));

        assert_ne!(
            base.instantiate(&[first], &tree),
            base.instantiate(&[second], &tree)
        );
    }

    /// Static instance symbols are independent of local static allocation order.
    #[test]
    fn test_mangle_static_instances_across_trees() {
        let base = Symbol::named(StringId::for_text("library.buffer"));

        let mut first = Tree::new();
        let first_length = first.intern_static(Static::Integer(64));

        let mut second = Tree::new();
        second.intern_static(Static::Integer(32));
        let second_length = second.intern_static(Static::Integer(64));

        assert_eq!(
            base.instantiate(&[first_length], &first),
            base.instantiate(&[second_length], &second)
        );
    }
}
