use std::hash::Hasher;

use tspp_core::{StableHasher, StringId};
use tspp_source::ModuleId;

use crate::{
    Access, Attribute, AttributeArgs, AttributeIdentifier, AttributeValue, Constant, Extent,
    FieldId, FloatType, GenericArgument, GenericParameter, GenericParameterDomain, Lifetime,
    LifetimeParameter, Multiplicity, Reference, SignatureParameter, Space, Static, StaticField,
    StaticId, StaticKey, Symbol, Tree, Type, TypeFingerprint, TypeId,
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
    /// Hash a generated function's signature, parameter domains, and selected application.
    pub(super) fn generated(
        base: Symbol,
        signature: TypeId,
        parameters: &[GenericParameter],
        receiver: Option<&GenericArgument>,
        arguments: &[GenericArgument],
        tree: &Tree,
    ) -> Symbol {
        // identify the generated declaration independently from generic instantiation
        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(b"tspp.mir.generated.v1");
        hasher.write_u64(base.raw());
        let mut hasher = Self { hasher };

        // hash the callable declaration without its parameter names
        hasher.hash_type(signature, tree);
        hasher.hash_length(parameters.len());
        for parameter in parameters {
            hasher.hash_generic_parameter(parameter, tree);
        }

        // hash the selected receiver and arguments separately
        hasher.hash_boolean(receiver.is_some());
        if let Some(receiver) = receiver {
            hasher.hash_argument(receiver, tree);
        }
        hasher.hash_length(arguments.len());
        for argument in arguments {
            hasher.hash_argument(argument, tree);
        }

        Symbol::from_raw(base.module(), hasher.hasher.finish_u64())
    }

    /// Derive one generic instance symbol from its concrete arguments.
    pub(super) fn symbol(base: Symbol, arguments: &[GenericArgument], tree: &Tree) -> Symbol {
        if arguments.is_empty() {
            return base;
        }

        let mut hasher = Self::for_instance(base);
        hasher.hash_length(arguments.len());
        for argument in arguments {
            hasher.hash_argument(argument, tree);
        }

        Symbol::from_raw(base.module(), hasher.hasher.finish_u64())
    }

    /// Derive one declaration symbol from its name and declaring identity.
    pub(super) fn declared(module: ModuleId, name: StringId, identity: u64) -> Symbol {
        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(b"tspp.mir.declaration.v1");
        hasher.write_u64(name.raw());
        hasher.write_u64(identity);

        Symbol::from_raw(Some(module), hasher.finish_u64())
    }

    /// Create a type fingerprint hasher.
    fn new() -> Self {
        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(b"tspp.mir.type.v1");

        Self { hasher }
    }

    /// Create a generic instance hasher.
    fn for_instance(base: Symbol) -> Self {
        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(b"tspp.mir.instance.v1");
        hasher.write_u64(base.raw());

        Self { hasher }
    }

    /// Hash one concrete static value.
    fn hash_static(&mut self, id: StaticId, tree: &Tree) {
        let value = tree.static_value(id);

        match value {
            Static::Parameter(index) => {
                self.hasher.write_u8(20);
                self.hasher.write_u32(*index);
            }
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
        }
    }

    /// Hash the persistent module and name of a symbol.
    fn hash_symbol(&mut self, symbol: Symbol) {
        match symbol.module() {
            Some(module) => {
                self.hasher.write_u8(1);
                self.hasher.write_u64(module.package_id.raw());
                self.hasher.write_u64(module.module_key.raw());
            }
            None => self.hasher.write_u8(0),
        }
        self.hasher.write_u64(symbol.raw());
    }

    /// Hash one type through structural or identified identity.
    fn hash_type(&mut self, id: TypeId, tree: &Tree) {
        match tree.get(id) {
            Type::Declaration { declaration } => {
                self.hasher.write_u8(0xff);
                self.hash_symbol(tree.get(*declaration).symbol);
            }
            Type::Error => self.hasher.write_u8(0),
            Type::Never => self.hasher.write_u8(1),
            Type::Void => self.hasher.write_u8(2),
            Type::Null => self.hasher.write_u8(45),
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
            Type::TypeId => self.hasher.write_u8(10),
            Type::Dynamic {
                kind,
                lifetime,
                constraint,
                access,
            } => {
                self.hasher.write_u8(12);
                self.hash_reference_kind(*kind);
                self.hash_lifetime(lifetime);
                self.hash_type(*constraint, tree);
                self.hash_access(*access);
            }
            Type::Reference {
                kind,
                lifetime,
                access,
                pointee,
            } => {
                self.hasher.write_u8(13);
                self.hash_reference_kind(*kind);
                self.hash_lifetime(lifetime);
                self.hash_access(*access);
                self.hash_type(*pointee, tree);
            }
            Type::Pointer { pointee, access } => {
                self.hasher.write_u8(31);
                self.hash_type(*pointee, tree);
                self.hash_access(*access);
            }
            Type::Slice {
                kind,
                lifetime,
                element,
                access,
            } => {
                self.hasher.write_u8(14);
                self.hash_reference_kind(*kind);
                self.hash_lifetime(lifetime);
                self.hash_type(*element, tree);
                self.hash_access(*access);
            }
            Type::Uninit { value } => {
                self.hasher.write_u8(15);
                self.hash_type(*value, tree);
            }
            Type::ManuallyDrop { value } => {
                self.hasher.write_u8(16);
                self.hash_type(*value, tree);
            }
            Type::FixedArray { element, length } => {
                self.hasher.write_u8(17);
                self.hash_type(*element, tree);
                self.hash_static(*length, tree);
            }
            Type::Tuple { elements } => {
                self.hasher.write_u8(18);
                self.hash_types(elements, tree);
            }
            Type::Struct { fields } => {
                self.hasher.write_u8(19);
                self.hash_length(fields.len());
                for field in fields {
                    self.hash_field(*field, tree);
                }
            }
            Type::Newtype { value } => {
                self.hasher.write_u8(20);
                self.hash_type(*value, tree);
            }
            Type::Variant {
                discriminant,
                cases,
            } => {
                self.hasher.write_u8(21);
                self.hash_type(*discriminant, tree);
                self.hash_length(cases.len());
                for case in cases {
                    self.hash_constant(&case.discriminant, tree);
                    self.hash_type(case.ty, tree);
                }
            }
            Type::Vector { element, lanes } => {
                self.hasher.write_u8(22);
                self.hash_type(*element, tree);
                self.hash_static(*lanes, tree);
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
                access,
            } => {
                self.hasher.write_u8(26);
                self.hash_type(*signature, tree);
                self.hasher.write_u8(match multiplicity {
                    Multiplicity::Repeatable => 0,
                    Multiplicity::Once => 1,
                });
                self.hash_reference_kind(*kind);
                self.hash_lifetime(lifetime);
                self.hash_access(*access);
            }
            Type::FunctionPointer { signature } => {
                self.hasher.write_u8(27);
                self.hash_type(*signature, tree);
            }
            Type::Witness {
                receiver,
                interface,
                member,
            } => {
                self.hasher.write_u8(46);
                self.hash_type(*receiver, tree);
                self.hash_type(*interface, tree);
                self.hasher.write_u64(member.raw());
            }
            Type::Application { base, arguments } => {
                self.hasher.write_u8(30);
                self.hash_type(*base, tree);
                self.hash_length(arguments.len());
                for argument in arguments {
                    self.hash_argument(argument, tree);
                }
            }
            Type::Parameter { index, referent } => {
                self.hasher.write_u8(32);
                self.hasher.write_u32(*index);
                self.hasher.write_u8(*referent as u8);
            }
        }
    }

    /// Hash one generic parameter's domain without its name.
    fn hash_generic_parameter(&mut self, parameter: &GenericParameter, tree: &Tree) {
        match &parameter.domain {
            GenericParameterDomain::Type { bounds } => {
                self.hasher.write_u8(0);
                self.hash_types(bounds, tree);
            }
            GenericParameterDomain::Region { outlives } => {
                self.hasher.write_u8(1);
                self.hash_length(outlives.len());
                for index in outlives {
                    self.hasher.write_u32(*index);
                }
            }
            GenericParameterDomain::Access => self.hasher.write_u8(3),
            GenericParameterDomain::Value { ty } => {
                self.hasher.write_u8(4);
                self.hash_type(*ty, tree);
            }
        }
    }

    /// Hash one generic argument.
    fn hash_argument(&mut self, argument: &GenericArgument, tree: &Tree) {
        match argument {
            GenericArgument::Type(ty) => {
                self.hasher.write_u8(0);
                self.hash_type(*ty, tree);
            }
            GenericArgument::Access(access) => {
                self.hasher.write_u8(2);
                self.hash_access(*access);
            }
            GenericArgument::Value(value) => {
                self.hasher.write_u8(3);
                self.hash_static(*value, tree);
            }
            GenericArgument::Region(lifetime) => {
                self.hasher.write_u8(4);
                self.hash_lifetime(lifetime);
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
    fn hash_field(&mut self, id: FieldId, tree: &Tree) {
        let field = tree.get(id);
        self.hash_string_maybe(field.name);
        self.hash_type(field.ty, tree);

        let attributes = &field.attributes;
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

    /// Hash one MIR space.
    fn hash_space(&mut self, space: Space) {
        match space {
            Space::Local => self.hasher.write_u8(0),
            Space::Shared => self.hasher.write_u8(1),
            Space::Constant => self.hasher.write_u8(2),
        }
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
        match access {
            Access::Readonly => self.hasher.write_u8(0),
            Access::Mutable => self.hasher.write_u8(1),
            Access::Immutable => self.hasher.write_u8(2),
            Access::Exclusive => self.hasher.write_u8(3),
            Access::Parameter(index) => {
                self.hasher.write_u8(4);
                self.hasher.write_u32(index);
            }
        }
    }

    /// Hash one MIR reference kind.
    fn hash_reference_kind(&mut self, kind: Reference) {
        match kind {
            Reference::Managed(space) => {
                self.hasher.write_u8(0);
                self.hash_space(space);
            }
            Reference::Unique => self.hasher.write_u8(1),
            Reference::Borrowed => self.hasher.write_u8(2),
            Reference::Raw => self.hasher.write_u8(3),
        }
    }

    /// Hash one concrete floating-point format.
    fn hash_float_type(&mut self, format: FloatType) {
        let tag = match format {
            FloatType::Float32 => 0,
            FloatType::Float64 => 1,
        };
        self.hasher.write_u8(tag);
    }

    /// Hash one applied MIR lifetime.
    fn hash_lifetime(&mut self, lifetime: &Lifetime) {
        self.hash_length(lifetime.extents.len());
        for term in &lifetime.extents {
            match term {
                Extent::Static => self.hasher.write_u8(0),
                Extent::Frame => self.hasher.write_u8(1),
                Extent::Bound(slot) => {
                    self.hasher.write_u8(2);
                    self.hasher.write_u32(slot.depth);
                    self.hasher.write_u32(slot.index);
                }
                Extent::Managed => self.hasher.write_u8(3),
                Extent::Parameter(index) => {
                    self.hasher.write_u8(4);
                    self.hasher.write_u32(*index);
                }
            }
        }
    }

    /// Hash lifetime structure without source-only parameter names.
    fn hash_lifetime_parameters(&mut self, lifetimes: &[LifetimeParameter]) {
        self.hash_length(lifetimes.len());
        for lifetime in lifetimes {
            self.hash_lifetime(&lifetime.outlives);
        }
    }

    /// Hash one executable scalar constant.
    fn hash_constant(&mut self, constant: &Constant, tree: &Tree) {
        match constant {
            Constant::Parameter(index) => {
                self.hasher.write_u8(9);
                self.hasher.write_u32(*index);
            }
            Constant::Null => self.hasher.write_u8(0),
            Constant::Undefined => self.hasher.write_u8(12),
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
            Constant::Layout { ty, measure } => {
                self.hasher.write_u8(10);
                self.hash_type(*ty, tree);
                self.hasher.write_u8(*measure as u8);
            }
            Constant::Witness {
                receiver,
                interface,
                member,
            } => {
                self.hasher.write_u8(11);
                self.hash_type(*receiver, tree);
                self.hash_type(*interface, tree);
                self.hasher.write_u64(member.raw());
            }
        }
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
