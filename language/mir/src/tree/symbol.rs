use std::hash::{Hash, Hasher};

use destack_core::{StableHasher, StringId};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    Attribute, AttributeArgs, AttributeValue, Field, SignatureParameter, Tree, Type, TypeId,
};

/// Persistent, mangled identity of a function, global, or type.
///
/// Unique and stable across builds, minted from a resolved declaration path
/// and any runtime representation arguments. Cross-module references link by
/// symbol equality, and analyses and profile data key on it.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct Symbol(u64);

/// Stable structural fingerprint of one MIR type.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct TypeFingerprint(u128);

impl Symbol {
    /// Create the symbol of one resolved declaration name.
    pub fn named(name: StringId) -> Self {
        Self(name.raw())
    }

    /// Derive one generic instance symbol from its runtime representations.
    pub fn instantiate(self, representations: &[TypeId], tree: &Tree) -> Self {
        if representations.is_empty() {
            return self;
        }

        let mut hasher = TypeHasher::for_instance(self);
        representations.len().hash(&mut hasher.hasher);
        for representation in representations {
            hasher.write_type(*representation, tree);
        }

        Self(hasher.hasher.finish_u64())
    }

    /// Return the stable symbol bits.
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl Tree {
    /// Compute the stable structural fingerprint of one type.
    pub fn type_fingerprint(&self, ty: TypeId) -> TypeFingerprint {
        let mut hasher = TypeHasher::new();
        hasher.write_type(ty, self);

        TypeFingerprint(hasher.hasher.finish_u128())
    }
}

/// Stable structural type hasher.
struct TypeHasher {
    /// The stable hash under construction.
    hasher: StableHasher,
}

impl TypeHasher {
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

    /// Write one type through structural or identified identity.
    fn write_type(&mut self, id: TypeId, tree: &Tree) {
        // identified types terminate recursive representations at their symbol
        if let Some(symbol) = tree.type_symbol(id) {
            self.hasher.write_u8(0xff);
            self.hasher.write_u64(symbol.raw());

            return;
        }

        match tree.get(id) {
            Type::Error => self.hasher.write_u8(0),
            Type::Never => self.hasher.write_u8(27),
            Type::Void => self.hasher.write_u8(1),
            Type::Boolean => self.hasher.write_u8(2),
            Type::Int { width, is_signed } => {
                self.hasher.write_u8(3);
                width.hash(&mut self.hasher);
                is_signed.hash(&mut self.hasher);
            }
            Type::Isize => self.hasher.write_u8(4),
            Type::Usize => self.hasher.write_u8(5),
            Type::Float(format) => {
                self.hasher.write_u8(6);
                format.hash(&mut self.hasher);
            }
            Type::TypeDescriptor => self.hasher.write_u8(7),
            Type::TypeId => self.hasher.write_u8(8),
            Type::Atomic { value } => {
                self.hasher.write_u8(9);
                self.write_type(*value, tree);
            }
            Type::Dynamic {
                constraint,
                nullability,
                space,
            } => {
                self.hasher.write_u8(10);
                self.write_type(*constraint, tree);
                nullability.hash(&mut self.hasher);
                space.hash(&mut self.hasher);
            }
            Type::WithLifetimes { base, lifetimes } => {
                self.hasher.write_u8(11);
                self.write_type(*base, tree);
                lifetimes.hash(&mut self.hasher);
            }
            Type::Reference {
                kind,
                lifetime,
                space,
                access,
                pointee,
                nullability,
            } => {
                self.hasher.write_u8(12);
                kind.hash(&mut self.hasher);
                lifetime.hash(&mut self.hasher);
                space.hash(&mut self.hasher);
                access.hash(&mut self.hasher);
                self.write_type(*pointee, tree);
                nullability.hash(&mut self.hasher);
            }
            Type::Slice {
                kind,
                lifetime,
                element,
                space,
                access,
                nullability,
            } => {
                self.hasher.write_u8(13);
                kind.hash(&mut self.hasher);
                lifetime.hash(&mut self.hasher);
                self.write_type(*element, tree);
                space.hash(&mut self.hasher);
                access.hash(&mut self.hasher);
                nullability.hash(&mut self.hasher);
            }
            Type::Uninit { value } => {
                self.hasher.write_u8(14);
                self.write_type(*value, tree);
            }
            Type::ManuallyDrop { value } => {
                self.hasher.write_u8(15);
                self.write_type(*value, tree);
            }
            Type::FixedArray {
                element,
                length,
                copy,
            } => {
                self.hasher.write_u8(16);
                self.write_type(*element, tree);
                length.hash(&mut self.hasher);
                copy.hash(&mut self.hasher);
            }
            Type::Tuple { elements, copy } => {
                self.hasher.write_u8(17);
                self.write_types(elements, tree);
                copy.hash(&mut self.hasher);
            }
            Type::Struct { fields, copy } => {
                self.hasher.write_u8(18);
                fields.len().hash(&mut self.hasher);
                for field in fields {
                    self.write_field(*field, tree);
                }
                copy.hash(&mut self.hasher);
            }
            Type::Newtype { inner, copy } => {
                self.hasher.write_u8(19);
                self.write_type(*inner, tree);
                copy.hash(&mut self.hasher);
            }
            Type::Variant {
                discriminant,
                storage,
                cases,
                copy,
            } => {
                self.hasher.write_u8(20);
                self.write_type(*discriminant, tree);
                self.write_type(*storage, tree);
                cases.len().hash(&mut self.hasher);
                for case in cases {
                    case.discriminant.hash(&mut self.hasher);
                    self.write_type(case.ty, tree);
                }
                copy.hash(&mut self.hasher);
            }
            Type::Vector {
                element,
                lanes,
                copy,
            } => {
                self.hasher.write_u8(21);
                self.write_type(*element, tree);
                lanes.hash(&mut self.hasher);
                copy.hash(&mut self.hasher);
            }
            Type::Tensor {
                element,
                space,
                shape,
                format,
                sharding,
                copy,
            } => {
                self.hasher.write_u8(22);
                self.write_type(*element, tree);
                space.hash(&mut self.hasher);
                shape.hash(&mut self.hasher);
                format.hash(&mut self.hasher);
                sharding.hash(&mut self.hasher);
                copy.hash(&mut self.hasher);
            }
            Type::TensorView {
                kind,
                lifetime,
                space,
                access,
                element,
                shape,
                format,
                sharding,
                nullability,
            } => {
                self.hasher.write_u8(23);
                kind.hash(&mut self.hasher);
                lifetime.hash(&mut self.hasher);
                space.hash(&mut self.hasher);
                access.hash(&mut self.hasher);
                self.write_type(*element, tree);
                shape.hash(&mut self.hasher);
                format.hash(&mut self.hasher);
                sharding.hash(&mut self.hasher);
                nullability.hash(&mut self.hasher);
            }
            Type::FunctionSignature {
                lifetimes,
                parameters,
                result,
            } => {
                self.hasher.write_u8(24);
                lifetimes.hash(&mut self.hasher);
                parameters.len().hash(&mut self.hasher);
                for parameter in parameters {
                    self.write_parameter(parameter, tree);
                }
                self.write_type(*result, tree);
            }
            Type::Function {
                signature,
                environment,
            } => {
                self.hasher.write_u8(25);
                self.write_type(*signature, tree);
                self.write_type(*environment, tree);
            }
            Type::FunctionPointer { signature } => {
                self.hasher.write_u8(26);
                self.write_type(*signature, tree);
            }
            Type::Character => self.hasher.write_u8(27),
            Type::Never => self.hasher.write_u8(28),
            Type::Continuation {
                resume_type,
                yield_type,
                return_type,
            } => {
                self.hasher.write_u8(29);
                self.write_type(*resume_type, tree);
                self.write_type(*yield_type, tree);
                self.write_type(*return_type, tree);
            }
            Type::Waiter { value_type } => {
                self.hasher.write_u8(30);
                self.write_type(*value_type, tree);
            }
        }
    }

    /// Write an ordered type sequence.
    fn write_types(&mut self, types: &[TypeId], tree: &Tree) {
        types.len().hash(&mut self.hasher);
        for ty in types {
            self.write_type(*ty, tree);
        }
    }

    /// Write one structural field declaration.
    fn write_field(&mut self, id: crate::LocalNodeId<Field>, tree: &Tree) {
        let field = tree.get(id);
        field.name.hash(&mut self.hasher);
        self.write_type(field.ty, tree);

        let attributes = tree.attributes(id);
        attributes.len().hash(&mut self.hasher);
        for attribute in attributes {
            self.write_attribute(attribute, tree);
        }
    }

    /// Write one callable parameter representation.
    fn write_parameter(&mut self, parameter: &SignatureParameter, tree: &Tree) {
        self.write_type(parameter.ty, tree);
    }

    /// Write one field attribute, recursively replacing type ids.
    fn write_attribute(&mut self, attribute: &Attribute, tree: &Tree) {
        attribute.name.hash(&mut self.hasher);
        match &attribute.args {
            AttributeArgs::None => self.hasher.write_u8(0),
            AttributeArgs::Value(value) => {
                self.hasher.write_u8(1);
                self.write_attribute_value(value, tree);
            }
            AttributeArgs::Values(values) => {
                self.hasher.write_u8(2);
                values.len().hash(&mut self.hasher);
                for value in values {
                    self.write_attribute_value(value, tree);
                }
            }
            AttributeArgs::KeyValues(values) => {
                self.hasher.write_u8(3);
                values.len().hash(&mut self.hasher);
                for value in values {
                    value.key.hash(&mut self.hasher);
                    self.write_attribute_value(&value.value, tree);
                }
            }
        }
    }

    /// Write one attribute value, recursively replacing type ids.
    fn write_attribute_value(&mut self, value: &AttributeValue, tree: &Tree) {
        match value {
            AttributeValue::Identifier(value) => {
                self.hasher.write_u8(0);
                value.hash(&mut self.hasher);
            }
            AttributeValue::Type(ty) => {
                self.hasher.write_u8(1);
                self.write_type(*ty, tree);
            }
            AttributeValue::Integer(value) => {
                self.hasher.write_u8(2);
                value.hash(&mut self.hasher);
            }
            AttributeValue::Float(value) => {
                self.hasher.write_u8(3);
                value.hash(&mut self.hasher);
            }
            AttributeValue::Boolean(value) => {
                self.hasher.write_u8(4);
                value.hash(&mut self.hasher);
            }
            AttributeValue::String(value) => {
                self.hasher.write_u8(5);
                value.hash(&mut self.hasher);
            }
            AttributeValue::List(values) => {
                self.hasher.write_u8(6);
                values.len().hash(&mut self.hasher);
                for value in values {
                    self.write_attribute_value(value, tree);
                }
            }
            AttributeValue::Missing => self.hasher.write_u8(7),
            AttributeValue::Error => self.hasher.write_u8(8),
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_core::StringId;

    use crate::{
        Access, Copy, Field, Lifetime, Nullability, ReferenceKind, Space, Symbol, Tree, Type,
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

        let mut second = Tree::new();
        let second_bool = second.intern_type(Type::Boolean);
        let second_int = second.intern_type(Type::INT32);
        let second_tuple = second.intern_type(Type::Tuple {
            elements: vec![second_int, second_bool],
            copy: Copy::Yes,
        });

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
            space: Space::Local,
            access: Access::Readonly,
            pointee,
            nullability: Nullability::None,
        });
        let static_ = tree.intern_type(Type::Reference {
            kind: ReferenceKind::Borrowed,
            lifetime: Lifetime::static_storage(),
            space: Space::Local,
            access: Access::Readonly,
            pointee,
            nullability: Nullability::None,
        });
        let local = tree.intern_representation(local);
        let static_ = tree.intern_representation(static_);
        let base = Symbol::named(StringId::for_text("library.inspect"));

        assert_eq!(local, static_);
        assert_eq!(
            base.instantiate(&[local], &tree),
            base.instantiate(&[static_], &tree)
        );
    }
}
