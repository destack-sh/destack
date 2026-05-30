use std::collections::HashSet;

use destack_core::StringId;
use destack_mir as mir;

use super::ValueTypeMap;

/// Structural type representation for CSE matching.
///
/// Unlike `LocalNodeId<Type>`, this compares by type structure rather than node
/// identity. Two types with different node IDs but identical structure will
/// produce equal keys. This enables CSE of casts and other type-dependent
/// operations across independently-created but structurally equivalent types.
///
/// Recursive types are resolved eagerly during construction, so the key is
/// self-contained and can be hashed/compared without access to the tree.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeKey {
    /// Non concrete type structure.
    NonConcrete,
    /// The void/unit type.
    Void,
    /// Boolean type.
    Boolean,
    /// Integer type with width and signedness.
    Int { width: u16, signed: bool },
    /// Pointer-sized signed integer type.
    Isize,
    /// Pointer-sized unsigned integer type.
    Usize,
    /// Floating-point type with format.
    Float { format: mir::FloatType },
    /// Type descriptor handle.
    TypeDescriptor,
    /// Compact runtime type id.
    TypeId,
    /// Atomic storage type.
    Atomic { value: Box<TypeKey> },
    /// Erased dynamic value type.
    Dynamic { constraint: Box<TypeKey> },
    /// Linear uninitialized allocation token type.
    Uninit { value: Box<TypeKey> },
    /// Reference or pointer type.
    Reference {
        kind: mir::ReferenceKind,
        lifetime: mir::Lifetime,
        space: mir::Space,
        access: mir::Access,
        pointee: Box<TypeKey>,
        nullability: mir::Nullability,
    },
    /// Fixed-size array type.
    Array {
        element: Box<TypeKey>,
        length: u64,
        copy: mir::Copy,
    },
    /// Slice type.
    Slice {
        kind: mir::ReferenceKind,
        lifetime: mir::Lifetime,
        element: Box<TypeKey>,
        space: mir::Space,
        access: mir::Access,
        nullability: mir::Nullability,
    },
    /// Tuple type with ordered elements.
    Tuple {
        elements: Vec<TypeKey>,
        copy: mir::Copy,
    },
    /// Struct type with optionally named fields.
    Struct {
        fields: Vec<(Option<StringId>, TypeKey)>,
        copy: mir::Copy,
    },
    /// Nominal newtype wrapper.
    Newtype {
        inner: Box<TypeKey>,
        copy: mir::Copy,
    },
    /// Variant type with ordered cases.
    Variant {
        tag: Box<TypeKey>,
        storage: Box<TypeKey>,
        cases: Vec<(mir::Constant, TypeKey)>,
        copy: mir::Copy,
    },
    /// Vector type with fixed lanes.
    Vector {
        element: Box<TypeKey>,
        lanes: u32,
        copy: mir::Copy,
    },
    /// Tensor value type with a shape.
    Tensor {
        element: Box<TypeKey>,
        shape: Vec<mir::TensorDimension>,
        layout: mir::TensorLayout,
        copy: mir::Copy,
    },
    /// Tensor view type.
    TensorView {
        kind: mir::ReferenceKind,
        lifetime: mir::Lifetime,
        space: mir::Space,
        access: mir::Access,
        element: Box<TypeKey>,
        shape: Vec<mir::TensorDimension>,
        layout: mir::TensorViewLayout,
        nullability: mir::Nullability,
    },
    /// Bare function signature.
    FunctionSignature {
        parameters: Vec<TypeKey>,
        result: Box<TypeKey>,
        borrow_obligations: Vec<mir::BorrowObligation>,
    },
    /// Function pointer type.
    FunctionPointer { signature: Box<TypeKey> },
    /// Closure value type.
    Closure {
        /// The bare function signature.
        signature: Box<TypeKey>,
        /// The captured environment representation.
        environment: Box<TypeKey>,
    },
    /// Recursive reference to a previously visited type id.
    Recursive { id: mir::LocalNodeId<mir::Type> },
}

impl TypeKey {
    /// Build a type key from a MIR type id, resolving nested types.
    pub fn from_type(type_id: mir::LocalNodeId<mir::Type>, tree: &mir::Tree) -> Self {
        let mut visiting = Vec::new();
        Self::from_type_inner(type_id, tree, &mut visiting)
    }

    /// Build a type key from a recoverable MIR type reference.
    pub fn from_type_reference(type_id: mir::TypeReference, tree: &mir::Tree) -> Self {
        let Some(type_id) = type_id.ty() else {
            return TypeKey::NonConcrete;
        };

        Self::from_type(type_id, tree)
    }

    fn from_type_inner(
        type_id: mir::LocalNodeId<mir::Type>,
        tree: &mir::Tree,
        visiting: &mut Vec<mir::LocalNodeId<mir::Type>>,
    ) -> Self {
        if visiting.contains(&type_id) {
            return TypeKey::Recursive { id: type_id };
        }

        visiting.push(type_id);

        let ty = tree.get(type_id);
        let key = match ty {
            mir::Type::Void => TypeKey::Void,
            mir::Type::Boolean => TypeKey::Boolean,
            mir::Type::Int {
                width,
                is_signed: signed,
            } => TypeKey::Int {
                width: *width,
                signed: *signed,
            },
            mir::Type::Isize => TypeKey::Isize,
            mir::Type::Usize => TypeKey::Usize,
            mir::Type::Float(float_type) => TypeKey::Float {
                format: *float_type,
            },
            mir::Type::TypeDescriptor => TypeKey::TypeDescriptor,
            mir::Type::TypeId => TypeKey::TypeId,
            mir::Type::Atomic { value } => TypeKey::Atomic {
                value: Box::new(Self::from_type_reference(*value, tree)),
            },
            mir::Type::Dynamic { constraint } => TypeKey::Dynamic {
                constraint: Box::new(Self::from_type_reference(*constraint, tree)),
            },
            mir::Type::Uninit { value } => TypeKey::Uninit {
                value: Box::new(Self::from_type_reference(*value, tree)),
            },

            mir::Type::Reference {
                kind,
                lifetime,
                space,
                access,
                pointee,
                nullability,
            } => TypeKey::Reference {
                kind: *kind,
                lifetime: lifetime.clone(),
                space: space.clone(),
                access: *access,
                pointee: Box::new(Self::from_type_reference(*pointee, tree)),
                nullability: *nullability,
            },

            mir::Type::Array {
                element,
                length,
                copy,
            } => TypeKey::Array {
                element: Box::new(Self::from_type_reference(*element, tree)),
                length: *length,
                copy: *copy,
            },
            mir::Type::Slice {
                kind,
                lifetime,
                element,
                space,
                access,
                nullability,
            } => TypeKey::Slice {
                kind: *kind,
                lifetime: lifetime.clone(),
                element: Box::new(Self::from_type_reference(*element, tree)),
                space: space.clone(),
                access: *access,
                nullability: *nullability,
            },

            mir::Type::Tuple { elements, copy } => {
                let elements = elements
                    .iter()
                    .map(|element| Self::from_type_reference(*element, tree))
                    .collect();
                TypeKey::Tuple {
                    elements,
                    copy: *copy,
                }
            }

            mir::Type::Struct { fields, copy } => {
                let fields = fields
                    .iter()
                    .map(|field_id| {
                        let field = tree.get(*field_id);
                        let field_ty = Self::from_type_reference(field.ty, tree);
                        (field.name, field_ty)
                    })
                    .collect();
                TypeKey::Struct {
                    fields,
                    copy: *copy,
                }
            }

            mir::Type::Newtype { inner, copy } => TypeKey::Newtype {
                inner: Box::new(Self::from_type_reference(*inner, tree)),
                copy: *copy,
            },

            mir::Type::Variant {
                tag,
                storage,
                cases,
                copy,
            } => {
                let tag = Box::new(Self::from_type_reference(*tag, tree));
                let storage = Box::new(Self::from_type_reference(*storage, tree));
                let cases = cases
                    .iter()
                    .map(|case| (case.tag.clone(), Self::from_type_reference(case.ty, tree)))
                    .collect();
                TypeKey::Variant {
                    tag,
                    storage,
                    cases,
                    copy: *copy,
                }
            }

            mir::Type::Vector {
                element,
                lanes,
                copy,
            } => TypeKey::Vector {
                element: Box::new(Self::from_type_reference(*element, tree)),
                lanes: *lanes,
                copy: *copy,
            },

            mir::Type::Tensor {
                element,
                shape,
                layout,
                copy,
            } => TypeKey::Tensor {
                element: Box::new(Self::from_type_reference(*element, tree)),
                shape: shape.clone(),
                layout: layout.clone(),
                copy: *copy,
            },

            mir::Type::TensorView {
                kind,
                lifetime,
                space,
                access,
                element,
                shape,
                layout,
                nullability,
            } => TypeKey::TensorView {
                kind: *kind,
                lifetime: lifetime.clone(),
                space: space.clone(),
                access: *access,
                element: Box::new(Self::from_type_reference(*element, tree)),
                shape: shape.clone(),
                layout: layout.clone(),
                nullability: *nullability,
            },

            mir::Type::FunctionSignature {
                parameters,
                result,
                borrow_obligations,
            } => {
                let parameters = parameters
                    .iter()
                    .map(|param| Self::from_type_reference(*param, tree))
                    .collect();
                TypeKey::FunctionSignature {
                    parameters,
                    result: Box::new(Self::from_type_reference(*result, tree)),
                    borrow_obligations: borrow_obligations.clone(),
                }
            }
            mir::Type::FunctionPointer { signature } => TypeKey::FunctionPointer {
                signature: Box::new(Self::from_type_reference(*signature, tree)),
            },
            mir::Type::Closure {
                signature,
                environment,
            } => TypeKey::Closure {
                signature: Box::new(Self::from_type_reference(*signature, tree)),
                environment: Box::new(Self::from_type_reference(*environment, tree)),
            },
        };

        visiting.pop();
        key
    }

    /// Check if this is a scalar type (no heap allocation needed to construct).
    pub fn is_scalar(&self) -> bool {
        matches!(
            self,
            TypeKey::NonConcrete
                | TypeKey::Void
                | TypeKey::Boolean
                | TypeKey::Int { .. }
                | TypeKey::Isize
                | TypeKey::Usize
                | TypeKey::Float { .. }
                | TypeKey::TypeDescriptor
                | TypeKey::TypeId
        )
    }

    /// Return the byte size when it is unambiguous.
    pub fn byte_size(&self, pointer_width_bits: u16) -> Option<u64> {
        match self {
            TypeKey::Int { width, .. } => bytes_for_width(*width),
            TypeKey::Isize | TypeKey::Usize => bytes_for_width(pointer_width_bits),
            TypeKey::Float { format } => bytes_for_width(format.width()),
            TypeKey::Uninit { value } => value.byte_size(pointer_width_bits),
            TypeKey::Newtype { inner, .. } => inner.byte_size(pointer_width_bits),
            TypeKey::Array {
                element,
                length,
                copy: _,
            } => {
                let element_size = element.byte_size(pointer_width_bits)?;
                element_size.checked_mul(*length)
            }
            TypeKey::Slice { .. } => None,
            _ => None,
        }
    }
}

/// Return the unsigned integer width for a value when it is known.
pub fn unsigned_int_width_for_value(
    value: mir::Value,
    value_types: &ValueTypeMap,
    pointer_width_bits: u16,
    tree: &mir::Tree,
) -> Option<u16> {
    // look up the value type
    let type_id = value_types.require_value_type(value);
    let ty = tree.get(type_id);

    // accept unsigned integer types
    match ty {
        mir::Type::Int {
            width,
            is_signed: signed,
        } if !*signed => Some(*width),
        mir::Type::Usize => Some(pointer_width_bits),
        _ => None,
    }
}

/// Return true when two values can be safely substituted.
pub fn can_substitute_value(
    destination: mir::Value,
    replacement: mir::Value,
    value_types: &ValueTypeMap,
    tree: &mir::Tree,
) -> bool {
    // enforce type equality when substituting values
    let destination_type = value_types.require_value_type(destination);
    let replacement_type = value_types.require_value_type(replacement);

    types_are_equal(destination_type, replacement_type, tree)
}

/// Convert a bit width into bytes when the width is byte aligned.
fn bytes_for_width(width: u16) -> Option<u64> {
    if width.is_multiple_of(8) {
        Some(u64::from(width / 8))
    } else {
        None
    }
}

/// Check if two MIR types are structurally equal.
///
/// This performs deep structural comparison, resolving `LocalNodeId<Type>`
/// references through the tree. Two types are equal if they have the
/// same structure, regardless of whether they have different node IDs.
pub fn types_are_equal(
    a: mir::LocalNodeId<mir::Type>,
    b: mir::LocalNodeId<mir::Type>,
    tree: &mir::Tree,
) -> bool {
    // fast path: same node ID
    if a == b {
        return true;
    }

    // compare underlying type structures
    let mut visiting = HashSet::new();
    types_are_equal_inner(a, b, tree, &mut visiting)
}

/// Check if two MIR type structures are equal.
fn types_are_equal_inner(
    a: mir::LocalNodeId<mir::Type>,
    b: mir::LocalNodeId<mir::Type>,
    tree: &mir::Tree,
    visiting: &mut HashSet<(mir::LocalNodeId<mir::Type>, mir::LocalNodeId<mir::Type>)>,
) -> bool {
    if !visiting.insert((a, b)) {
        return true;
    }

    let ty_a = tree.get(a);
    let ty_b = tree.get(b);
    let result = match (ty_a, ty_b) {
        // simple scalar types: direct comparison
        (mir::Type::Void, mir::Type::Void) => true,
        (mir::Type::Boolean, mir::Type::Boolean) => true,
        (
            mir::Type::Int {
                width: w1,
                is_signed: s1,
            },
            mir::Type::Int {
                width: w2,
                is_signed: s2,
            },
        ) => w1 == w2 && s1 == s2,
        (mir::Type::Isize, mir::Type::Isize) => true,
        (mir::Type::Usize, mir::Type::Usize) => true,
        (mir::Type::Float(left), mir::Type::Float(right)) => left == right,
        (mir::Type::TypeDescriptor, mir::Type::TypeDescriptor)
        | (mir::Type::TypeId, mir::Type::TypeId) => true,

        // references: compare all fields recursively
        (
            mir::Type::Reference {
                kind: k1,
                lifetime: l1,
                space: a1,
                access: m1,
                pointee: p1,
                nullability: n1,
            },
            mir::Type::Reference {
                kind: k2,
                lifetime: l2,
                space: a2,
                access: m2,
                pointee: p2,
                nullability: n2,
            },
        ) => {
            k1 == k2
                && l1 == l2
                && a1 == a2
                && m1 == m2
                && n1 == n2
                && type_references_are_equal(*p1, *p2, tree, visiting)
        }

        // arrays: compare element type and length
        (
            mir::Type::Array {
                element: e1,
                length: l1,
                copy: c1,
            },
            mir::Type::Array {
                element: e2,
                length: l2,
                copy: c2,
            },
        ) => c1 == c2 && l1 == l2 && type_references_are_equal(*e1, *e2, tree, visiting),
        (
            mir::Type::Slice {
                kind: k1,
                lifetime: l1,
                element: e1,
                space: a1,
                access: m1,
                nullability: n1,
            },
            mir::Type::Slice {
                kind: k2,
                lifetime: l2,
                element: e2,
                space: a2,
                access: m2,
                nullability: n2,
            },
        ) => {
            k1 == k2
                && l1 == l2
                && a1 == a2
                && m1 == m2
                && n1 == n2
                && type_references_are_equal(*e1, *e2, tree, visiting)
        }

        // tuples: compare element types
        (
            mir::Type::Tuple {
                elements: e1,
                copy: c1,
            },
            mir::Type::Tuple {
                elements: e2,
                copy: c2,
            },
        ) => {
            c1 == c2
                && e1.len() == e2.len()
                && e1
                    .iter()
                    .zip(e2.iter())
                    .all(|(a, b)| type_references_are_equal(*a, *b, tree, visiting))
        }

        // structs: compare field types
        (
            mir::Type::Struct {
                fields: f1,
                copy: c1,
            },
            mir::Type::Struct {
                fields: f2,
                copy: c2,
            },
        ) => {
            c1 == c2
                && f1.len() == f2.len()
                && f1.iter().zip(f2.iter()).all(|(a, b)| {
                    let field_a = tree.get(*a);
                    let field_b = tree.get(*b);
                    field_a.name == field_b.name
                        && type_references_are_equal(field_a.ty, field_b.ty, tree, visiting)
                })
        }

        // newtypes: compare inner type
        (
            mir::Type::Newtype {
                inner: i1,
                copy: c1,
            },
            mir::Type::Newtype {
                inner: i2,
                copy: c2,
            },
        ) => c1 == c2 && type_references_are_equal(*i1, *i2, tree, visiting),

        // function pointers: compare parameter and result types
        (
            mir::Type::FunctionSignature {
                parameters: p1,
                result: r1,
                borrow_obligations: o1,
            },
            mir::Type::FunctionSignature {
                parameters: p2,
                result: r2,
                borrow_obligations: o2,
            },
        ) => {
            o1 == o2
                && p1.len() == p2.len()
                && p1
                    .iter()
                    .zip(p2.iter())
                    .all(|(a, b)| type_references_are_equal(*a, *b, tree, visiting))
                && type_references_are_equal(*r1, *r2, tree, visiting)
        }

        // function pointers: compare signatures
        (
            mir::Type::FunctionPointer { signature: s1 },
            mir::Type::FunctionPointer { signature: s2 },
        ) => type_references_are_equal(*s1, *s2, tree, visiting),

        // closures: compare signature and environment
        (
            mir::Type::Closure {
                signature: s1,
                environment: e1,
            },
            mir::Type::Closure {
                signature: s2,
                environment: e2,
            },
        ) => {
            type_references_are_equal(*s1, *s2, tree, visiting)
                && type_references_are_equal(*e1, *e2, tree, visiting)
        }

        // different type cases are never equal
        _ => false,
    };

    visiting.remove(&(a, b));
    result
}

/// Check if two type references resolve to equal concrete types.
fn type_references_are_equal(
    left: mir::TypeReference,
    right: mir::TypeReference,
    tree: &mir::Tree,
    visiting: &mut HashSet<(mir::LocalNodeId<mir::Type>, mir::LocalNodeId<mir::Type>)>,
) -> bool {
    match (left.ty(), right.ty()) {
        (Some(left), Some(right)) => types_are_equal_inner(left, right, tree, visiting),
        (None, None) => left == right,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scalar types produce scalar keys.
    #[test]
    fn test_type_key_scalar_types() {
        let mut tree = mir::Tree::new();
        let void_id = tree.insert_type(mir::Type::Void);
        let boolean_id = tree.insert_type(mir::Type::Boolean);
        let int32_id = tree.insert_type(mir::Type::INT32);
        let isize_id = tree.insert_type(mir::Type::Isize);
        let usize_id = tree.insert_type(mir::Type::Usize);
        let uint64_id = tree.insert_type(mir::Type::UINT64);
        let float64_id = tree.insert_type(mir::Type::FLOAT64);
        let type_tag_id = tree.insert_type(mir::Type::TypeDescriptor);

        assert_eq!(TypeKey::from_type(void_id, &tree), TypeKey::Void);
        assert_eq!(TypeKey::from_type(boolean_id, &tree), TypeKey::Boolean);
        assert_eq!(
            TypeKey::from_type(int32_id, &tree),
            TypeKey::Int {
                width: 32,
                signed: true
            }
        );
        assert_eq!(TypeKey::from_type(isize_id, &tree), TypeKey::Isize);
        assert_eq!(TypeKey::from_type(usize_id, &tree), TypeKey::Usize);
        assert_eq!(
            TypeKey::from_type(uint64_id, &tree),
            TypeKey::Int {
                width: 64,
                signed: false
            }
        );
        assert_eq!(
            TypeKey::from_type(float64_id, &tree),
            TypeKey::Float {
                format: mir::FloatType::Float64
            }
        );
        assert_eq!(
            TypeKey::from_type(type_tag_id, &tree),
            TypeKey::TypeDescriptor
        );
    }

    /// Scalar types are identified as scalar.
    #[test]
    fn test_type_key_is_scalar() {
        let mut tree = mir::Tree::new();
        let void_id = tree.insert_type(mir::Type::Void);
        let boolean_id = tree.insert_type(mir::Type::Boolean);
        let int32_id = tree.insert_type(mir::Type::INT32);
        let isize_id = tree.insert_type(mir::Type::Isize);
        let usize_id = tree.insert_type(mir::Type::Usize);
        let float64_id = tree.insert_type(mir::Type::FLOAT64);
        let type_tag_id = tree.insert_type(mir::Type::TypeDescriptor);

        assert!(TypeKey::from_type(void_id, &tree).is_scalar());
        assert!(TypeKey::from_type(boolean_id, &tree).is_scalar());
        assert!(TypeKey::from_type(int32_id, &tree).is_scalar());
        assert!(TypeKey::from_type(isize_id, &tree).is_scalar());
        assert!(TypeKey::from_type(usize_id, &tree).is_scalar());
        assert!(TypeKey::from_type(float64_id, &tree).is_scalar());
        assert!(TypeKey::from_type(type_tag_id, &tree).is_scalar());
    }

    /// Complex types produce complex keys.
    #[test]
    fn test_type_key_complex_types() {
        let mut tree = mir::Tree::new();

        // array type
        let i32_id = tree.insert_type(mir::Type::INT32);
        let array_id = tree.insert_type(mir::Type::Array {
            element: i32_id.into(),
            length: 10,
            copy: mir::Copy::Yes,
        });
        let key = TypeKey::from_type(array_id, &tree);
        assert_eq!(
            key,
            TypeKey::Array {
                element: Box::new(TypeKey::Int {
                    width: 32,
                    signed: true
                }),
                length: 10,
                copy: mir::Copy::Yes
            }
        );
        assert!(!key.is_scalar());
    }

    /// Structurally equal types produce equal keys.
    #[test]
    fn test_type_key_structural_equality() {
        let mut tree = mir::Tree::new();

        // create two structurally identical array types with different node IDs
        let i32_id_1 = tree.insert_type(mir::Type::INT32);
        let i32_id_2 = tree.insert_type(mir::Type::INT32);
        assert_ne!(i32_id_1, i32_id_2);

        let array_id_1 = tree.insert_type(mir::Type::Array {
            element: i32_id_1.into(),
            length: 5,
            copy: mir::Copy::Yes,
        });
        let array_id_2 = tree.insert_type(mir::Type::Array {
            element: i32_id_2.into(),
            length: 5,
            copy: mir::Copy::Yes,
        });

        let key_1 = TypeKey::from_type(array_id_1, &tree);
        let key_2 = TypeKey::from_type(array_id_2, &tree);
        assert_eq!(key_1, key_2);
    }

    /// Copy differences yield distinct keys.
    #[test]
    fn test_type_key_copyability_distinguishes() {
        let mut tree = mir::Tree::new();

        let i32_id = tree.insert_type(mir::Type::INT32);
        let array_trivial_id = tree.insert_type(mir::Type::Array {
            element: i32_id.into(),
            length: 4,
            copy: mir::Copy::Yes,
        });
        let array_linear_id = tree.insert_type(mir::Type::Array {
            element: i32_id.into(),
            length: 4,
            copy: mir::Copy::No,
        });

        let key_trivial = TypeKey::from_type(array_trivial_id, &tree);
        let key_linear = TypeKey::from_type(array_linear_id, &tree);

        assert_ne!(key_trivial, key_linear);
    }
}
