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
/// self-contained and can be hashed/compared without access to the node tree.
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
    /// Floating-point type with width.
    Float { width: u16 },
    /// Type descriptor handle.
    TypeDescriptor,
    /// Compact runtime type id.
    TypeId,
    /// Reference or pointer type.
    Reference {
        kind: mir::ReferenceKind,
        address_space: mir::AddressSpace,
        mutability: mir::Mutability,
        pointee: Box<TypeKey>,
        is_nullable: bool,
    },
    /// Fixed-size array type.
    Array {
        element: Box<TypeKey>,
        length: u64,
        copyability: mir::Copyability,
    },
    /// Tuple type with ordered elements.
    Tuple {
        elements: Vec<TypeKey>,
        copyability: mir::Copyability,
    },
    /// Struct type with optionally named fields.
    Struct {
        fields: Vec<(Option<StringId>, TypeKey)>,
        copyability: mir::Copyability,
    },
    /// Nominal newtype wrapper.
    Newtype {
        inner: Box<TypeKey>,
        copyability: mir::Copyability,
    },
    /// Vector type with fixed lanes.
    Vector {
        element: Box<TypeKey>,
        lanes: u32,
        copyability: mir::Copyability,
    },
    /// Tensor value type with a shape.
    Tensor {
        element: Box<TypeKey>,
        shape: Vec<mir::TensorDimension>,
        layout: mir::TensorLayout,
        copyability: mir::Copyability,
    },
    /// Tensor view type.
    TensorReference {
        kind: mir::ReferenceKind,
        address_space: mir::AddressSpace,
        mutability: mir::Mutability,
        element: Box<TypeKey>,
        shape: Vec<mir::TensorDimension>,
        layout: mir::TensorLayout,
        is_nullable: bool,
    },
    /// Function pointer type.
    FunctionPointer {
        parameters: Vec<TypeKey>,
        result: Box<TypeKey>,
    },
    /// Callable closure value type.
    Closure { signature: Box<TypeKey> },
    /// Recursive reference to a previously visited type id.
    Recursive { id: mir::LocalNodeId<mir::Type> },
}

impl TypeKey {
    /// Build a type key from a MIR type id, resolving nested types.
    pub fn from_type(type_id: mir::LocalNodeId<mir::Type>, tree: &mir::NodeTree) -> Self {
        let mut visiting = Vec::new();
        Self::from_type_inner(type_id, tree, &mut visiting)
    }

    /// Build a type key from a recoverable MIR type reference.
    pub fn from_type_reference(type_id: mir::TypeReference, tree: &mir::NodeTree) -> Self {
        let Some(type_id) = type_id.ty() else {
            return TypeKey::NonConcrete;
        };

        Self::from_type(type_id, tree)
    }

    fn from_type_inner(
        type_id: mir::LocalNodeId<mir::Type>,
        tree: &mir::NodeTree,
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
            mir::Type::Float { width } => TypeKey::Float { width: *width },
            mir::Type::TypeDescriptor => TypeKey::TypeDescriptor,
            mir::Type::TypeId => TypeKey::TypeId,

            mir::Type::Reference {
                kind,
                address_space,
                mutability,
                pointee,
                is_nullable,
            } => TypeKey::Reference {
                kind: *kind,
                address_space: *address_space,
                mutability: *mutability,
                pointee: Box::new(Self::from_type_reference(*pointee, tree)),
                is_nullable: *is_nullable,
            },

            mir::Type::Array {
                element,
                length,
                copyability,
            } => TypeKey::Array {
                element: Box::new(Self::from_type_reference(*element, tree)),
                length: *length,
                copyability: *copyability,
            },

            mir::Type::Tuple {
                elements,
                copyability,
            } => {
                let elements = elements
                    .iter()
                    .map(|element| Self::from_type_reference(*element, tree))
                    .collect();
                TypeKey::Tuple {
                    elements,
                    copyability: *copyability,
                }
            }

            mir::Type::Struct {
                fields,
                copyability,
            } => {
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
                    copyability: *copyability,
                }
            }

            mir::Type::Newtype { inner, copyability } => TypeKey::Newtype {
                inner: Box::new(Self::from_type_reference(*inner, tree)),
                copyability: *copyability,
            },

            mir::Type::Vector {
                element,
                lanes,
                copyability,
            } => TypeKey::Vector {
                element: Box::new(Self::from_type_reference(*element, tree)),
                lanes: *lanes,
                copyability: *copyability,
            },

            mir::Type::Tensor {
                element,
                shape,
                layout,
                copyability,
            } => TypeKey::Tensor {
                element: Box::new(Self::from_type_reference(*element, tree)),
                shape: shape.clone(),
                layout: layout.clone(),
                copyability: *copyability,
            },

            mir::Type::TensorReference {
                kind,
                address_space,
                mutability,
                element,
                shape,
                layout,
                is_nullable,
            } => TypeKey::TensorReference {
                kind: *kind,
                address_space: *address_space,
                mutability: *mutability,
                element: Box::new(Self::from_type_reference(*element, tree)),
                shape: shape.clone(),
                layout: layout.clone(),
                is_nullable: *is_nullable,
            },

            mir::Type::FunctionPointer { parameters, result } => {
                let parameters = parameters
                    .iter()
                    .map(|param| Self::from_type_reference(*param, tree))
                    .collect();
                TypeKey::FunctionPointer {
                    parameters,
                    result: Box::new(Self::from_type_reference(*result, tree)),
                }
            }
            mir::Type::Closure { signature } => TypeKey::Closure {
                signature: Box::new(Self::from_type_reference(*signature, tree)),
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
            TypeKey::Float { width } => bytes_for_width(*width),
            TypeKey::Newtype { inner, .. } => inner.byte_size(pointer_width_bits),
            TypeKey::Array {
                element,
                length,
                copyability: _,
            } => {
                let element_size = element.byte_size(pointer_width_bits)?;
                element_size.checked_mul(*length)
            }
            _ => None,
        }
    }
}

/// Return the unsigned integer width for a value when it is known.
pub fn unsigned_int_width_for_value(
    value: mir::Value,
    value_types: &ValueTypeMap,
    pointer_width_bits: u16,
    tree: &mir::NodeTree,
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
    tree: &mir::NodeTree,
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
/// references through the node tree. Two types are equal if they have the
/// same structure, regardless of whether they have different node IDs.
pub fn types_are_equal(
    a: mir::LocalNodeId<mir::Type>,
    b: mir::LocalNodeId<mir::Type>,
    tree: &mir::NodeTree,
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
    tree: &mir::NodeTree,
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
        (mir::Type::Float { width: w1 }, mir::Type::Float { width: w2 }) => w1 == w2,
        (mir::Type::TypeDescriptor, mir::Type::TypeDescriptor)
        | (mir::Type::TypeId, mir::Type::TypeId) => true,

        // references: compare all fields recursively
        (
            mir::Type::Reference {
                kind: k1,
                address_space: a1,
                mutability: m1,
                pointee: p1,
                is_nullable: n1,
            },
            mir::Type::Reference {
                kind: k2,
                address_space: a2,
                mutability: m2,
                pointee: p2,
                is_nullable: n2,
            },
        ) => {
            k1 == k2
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
                copyability: c1,
            },
            mir::Type::Array {
                element: e2,
                length: l2,
                copyability: c2,
            },
        ) => c1 == c2 && l1 == l2 && type_references_are_equal(*e1, *e2, tree, visiting),

        // tuples: compare element types
        (
            mir::Type::Tuple {
                elements: e1,
                copyability: c1,
            },
            mir::Type::Tuple {
                elements: e2,
                copyability: c2,
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
                copyability: c1,
            },
            mir::Type::Struct {
                fields: f2,
                copyability: c2,
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
                copyability: c1,
            },
            mir::Type::Newtype {
                inner: i2,
                copyability: c2,
            },
        ) => c1 == c2 && type_references_are_equal(*i1, *i2, tree, visiting),

        // function pointers: compare parameter and result types
        (
            mir::Type::FunctionPointer {
                parameters: p1,
                result: r1,
            },
            mir::Type::FunctionPointer {
                parameters: p2,
                result: r2,
            },
        ) => {
            p1.len() == p2.len()
                && p1
                    .iter()
                    .zip(p2.iter())
                    .all(|(a, b)| type_references_are_equal(*a, *b, tree, visiting))
                && type_references_are_equal(*r1, *r2, tree, visiting)
        }

        // function values: compare signatures
        (mir::Type::Closure { signature: s1 }, mir::Type::Closure { signature: s2 }) => {
            type_references_are_equal(*s1, *s2, tree, visiting)
        }

        // different type variants are never equal
        _ => false,
    };

    visiting.remove(&(a, b));
    result
}

/// Check if two type references resolve to equal concrete types.
fn type_references_are_equal(
    left: mir::TypeReference,
    right: mir::TypeReference,
    tree: &mir::NodeTree,
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
        let mut tree = mir::NodeTree::new();
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
            TypeKey::Float { width: 64 }
        );
        assert_eq!(
            TypeKey::from_type(type_tag_id, &tree),
            TypeKey::TypeDescriptor
        );
    }

    /// Scalar types are identified as scalar.
    #[test]
    fn test_type_key_is_scalar() {
        let mut tree = mir::NodeTree::new();
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
        let mut tree = mir::NodeTree::new();

        // array type
        let i32_id = tree.insert_type(mir::Type::INT32);
        let array_id = tree.insert_type(mir::Type::Array {
            element: i32_id,
            length: 10,
            copyability: mir::Copyability::Trivial,
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
                copyability: mir::Copyability::Trivial
            }
        );
        assert!(!key.is_scalar());
    }

    /// Structurally equal types produce equal keys.
    #[test]
    fn test_type_key_structural_equality() {
        let mut tree = mir::NodeTree::new();

        // create two structurally identical array types with different node IDs
        let i32_id_1 = tree.insert_type(mir::Type::INT32);
        let i32_id_2 = tree.insert_type(mir::Type::INT32);
        assert_ne!(i32_id_1, i32_id_2);

        let array_id_1 = tree.insert_type(mir::Type::Array {
            element: i32_id_1,
            length: 5,
            copyability: mir::Copyability::Trivial,
        });
        let array_id_2 = tree.insert_type(mir::Type::Array {
            element: i32_id_2,
            length: 5,
            copyability: mir::Copyability::Trivial,
        });

        let key_1 = TypeKey::from_type(array_id_1, &tree);
        let key_2 = TypeKey::from_type(array_id_2, &tree);
        assert_eq!(key_1, key_2);
    }

    /// Copyability differences yield distinct keys.
    #[test]
    fn test_type_key_copyability_distinguishes() {
        let mut tree = mir::NodeTree::new();

        let i32_id = tree.insert_type(mir::Type::INT32);
        let array_trivial_id = tree.insert_type(mir::Type::Array {
            element: i32_id,
            length: 4,
            copyability: mir::Copyability::Trivial,
        });
        let array_linear_id = tree.insert_type(mir::Type::Array {
            element: i32_id,
            length: 4,
            copyability: mir::Copyability::Linear,
        });

        let key_trivial = TypeKey::from_type(array_trivial_id, &tree);
        let key_linear = TypeKey::from_type(array_linear_id, &tree);

        assert_ne!(key_trivial, key_linear);
    }
}
