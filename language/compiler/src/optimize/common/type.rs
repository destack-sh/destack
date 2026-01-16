use destack_base::StringId;
use destack_mir as mir;

use super::constant_matches_type;
use crate::optimize::analyses::OwnershipAnalysis;

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
    TypeTag,
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
    /// Function pointer type.
    FunctionPointer {
        parameters: Vec<TypeKey>,
        result: Box<TypeKey>,
    },
}

impl TypeKey {
    /// Build a type key from a MIR type, recursively resolving nested types.
    pub fn from_type(ty: &mir::Type, tree: &mir::NodeTree) -> Self {
        match ty {
            mir::Type::Void => TypeKey::Void,
            mir::Type::Boolean => TypeKey::Boolean,
            mir::Type::Int { width, signed } => TypeKey::Int {
                width: *width,
                signed: *signed,
            },
            mir::Type::Isize => TypeKey::Isize,
            mir::Type::Usize => TypeKey::Usize,
            mir::Type::Float { width } => TypeKey::Float { width: *width },
            mir::Type::TypeTag => TypeKey::TypeTag,

            mir::Type::Reference {
                kind,
                address_space,
                mutability,
                pointee,
                is_nullable,
            } => {
                let pointee_ty = tree.get(*pointee);
                TypeKey::Reference {
                    kind: *kind,
                    address_space: *address_space,
                    mutability: *mutability,
                    pointee: Box::new(TypeKey::from_type(pointee_ty, tree)),
                    is_nullable: *is_nullable,
                }
            }

            mir::Type::Array {
                element,
                length,
                copyability,
            } => {
                let element_ty = tree.get(*element);
                TypeKey::Array {
                    element: Box::new(TypeKey::from_type(element_ty, tree)),
                    length: *length,
                    copyability: *copyability,
                }
            }

            mir::Type::Tuple {
                elements,
                copyability,
            } => {
                let elements = elements
                    .iter()
                    .map(|e| TypeKey::from_type(tree.get(*e), tree))
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
                    .map(|f| {
                        let field = tree.get(*f);
                        let field_ty = TypeKey::from_type(tree.get(field.ty), tree);
                        (field.name, field_ty)
                    })
                    .collect();
                TypeKey::Struct {
                    fields,
                    copyability: *copyability,
                }
            }

            mir::Type::FunctionPointer { parameters, result } => {
                let parameters = parameters
                    .iter()
                    .map(|p| TypeKey::from_type(tree.get(*p), tree))
                    .collect();
                let result_ty = tree.get(*result);
                TypeKey::FunctionPointer {
                    parameters,
                    result: Box::new(TypeKey::from_type(result_ty, tree)),
                }
            }
        }
    }

    /// Check if this is a scalar type (no heap allocation needed to construct).
    pub fn is_scalar(&self) -> bool {
        matches!(
            self,
            TypeKey::Void
                | TypeKey::Boolean
                | TypeKey::Int { .. }
                | TypeKey::Isize
                | TypeKey::Usize
                | TypeKey::Float { .. }
                | TypeKey::TypeTag
        )
    }

    /// Return the byte size when it is unambiguous.
    pub fn byte_size(&self, pointer_width_bits: u16) -> Option<u64> {
        match self {
            TypeKey::Int { width, .. } => bytes_for_width(*width),
            TypeKey::Isize | TypeKey::Usize => bytes_for_width(pointer_width_bits),
            TypeKey::Float { width } => bytes_for_width(*width),
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
    ownership: &OwnershipAnalysis,
    pointer_width_bits: u16,
    tree: &mir::NodeTree,
) -> Option<u16> {
    // look up the value type
    let type_id = ownership.value_type(value)?;
    let ty = tree.get(type_id);

    // accept unsigned integer types
    match ty {
        mir::Type::Int { width, signed } if !*signed => Some(*width),
        mir::Type::Usize => Some(pointer_width_bits),
        _ => None,
    }
}

/// Return true when two values can be safely substituted.
pub fn can_substitute_value(
    destination: mir::Value,
    replacement: mir::Value,
    ownership: &OwnershipAnalysis,
    pointer_width_bits: u16,
    tree: &mir::NodeTree,
) -> bool {
    // resolve destination and replacement type keys
    let destination_key = ownership.value_type_key(destination);
    let replacement_key = ownership.value_type_key(replacement);

    // require structural equivalence when both are known
    if let (Some(destination_key), Some(replacement_key)) = (destination_key, replacement_key) {
        return destination_key == replacement_key;
    }

    // allow constants when they match the destination type
    if let Some(destination_type) = ownership.value_type(destination)
        && let Some(constant_type) = ownership.constant_type(replacement)
    {
        return constant_matches_type(constant_type, destination_type, pointer_width_bits, tree);
    }

    false
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
    let ty_a = tree.get(a);
    let ty_b = tree.get(b);
    types_are_equal_inner(ty_a, ty_b, tree)
}

/// Check if two MIR type structures are equal.
fn types_are_equal_inner(a: &mir::Type, b: &mir::Type, tree: &mir::NodeTree) -> bool {
    match (a, b) {
        // simple scalar types: direct comparison
        (mir::Type::Void, mir::Type::Void) => true,
        (mir::Type::Boolean, mir::Type::Boolean) => true,
        (
            mir::Type::Int {
                width: w1,
                signed: s1,
            },
            mir::Type::Int {
                width: w2,
                signed: s2,
            },
        ) => w1 == w2 && s1 == s2,
        (mir::Type::Isize, mir::Type::Isize) => true,
        (mir::Type::Usize, mir::Type::Usize) => true,
        (mir::Type::Float { width: w1 }, mir::Type::Float { width: w2 }) => w1 == w2,
        (mir::Type::TypeTag, mir::Type::TypeTag) => true,

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
        ) => k1 == k2 && a1 == a2 && m1 == m2 && n1 == n2 && types_are_equal(*p1, *p2, tree),

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
        ) => c1 == c2 && l1 == l2 && types_are_equal(*e1, *e2, tree),

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
                    .all(|(a, b)| types_are_equal(*a, *b, tree))
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
                    field_a.name == field_b.name && types_are_equal(field_a.ty, field_b.ty, tree)
                })
        }

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
                    .all(|(a, b)| types_are_equal(*a, *b, tree))
                && types_are_equal(*r1, *r2, tree)
        }

        // different type variants are never equal
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scalar types produce scalar keys.
    #[test]
    fn test_type_key_scalar_types() {
        let tree = mir::NodeTree::new();

        assert_eq!(TypeKey::from_type(&mir::Type::Void, &tree), TypeKey::Void);
        assert_eq!(
            TypeKey::from_type(&mir::Type::Boolean, &tree),
            TypeKey::Boolean
        );
        assert_eq!(
            TypeKey::from_type(&mir::Type::INT32, &tree),
            TypeKey::Int {
                width: 32,
                signed: true
            }
        );
        assert_eq!(TypeKey::from_type(&mir::Type::Isize, &tree), TypeKey::Isize);
        assert_eq!(TypeKey::from_type(&mir::Type::Usize, &tree), TypeKey::Usize);
        assert_eq!(
            TypeKey::from_type(&mir::Type::UINT64, &tree),
            TypeKey::Int {
                width: 64,
                signed: false
            }
        );
        assert_eq!(
            TypeKey::from_type(&mir::Type::FLOAT64, &tree),
            TypeKey::Float { width: 64 }
        );
        assert_eq!(
            TypeKey::from_type(&mir::Type::TypeTag, &tree),
            TypeKey::TypeTag
        );
    }

    /// Scalar types are identified as scalar.
    #[test]
    fn test_type_key_is_scalar() {
        let tree = mir::NodeTree::new();

        assert!(TypeKey::from_type(&mir::Type::Void, &tree).is_scalar());
        assert!(TypeKey::from_type(&mir::Type::Boolean, &tree).is_scalar());
        assert!(TypeKey::from_type(&mir::Type::INT32, &tree).is_scalar());
        assert!(TypeKey::from_type(&mir::Type::Isize, &tree).is_scalar());
        assert!(TypeKey::from_type(&mir::Type::Usize, &tree).is_scalar());
        assert!(TypeKey::from_type(&mir::Type::FLOAT64, &tree).is_scalar());
        assert!(TypeKey::from_type(&mir::Type::TypeTag, &tree).is_scalar());
    }

    /// Complex types produce complex keys.
    #[test]
    fn test_type_key_complex_types() {
        let mut tree = mir::NodeTree::new();

        // array type
        let i32_id = tree.insert(mir::Type::INT32);
        let array_ty = mir::Type::Array {
            element: i32_id,
            length: 10,
            copyability: mir::Copyability::Trivial,
        };
        let key = TypeKey::from_type(&array_ty, &tree);
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
        let i32_id_1 = tree.insert(mir::Type::INT32);
        let i32_id_2 = tree.insert(mir::Type::INT32);
        assert_ne!(i32_id_1, i32_id_2);

        let array_1 = mir::Type::Array {
            element: i32_id_1,
            length: 5,
            copyability: mir::Copyability::Trivial,
        };
        let array_2 = mir::Type::Array {
            element: i32_id_2,
            length: 5,
            copyability: mir::Copyability::Trivial,
        };

        let key_1 = TypeKey::from_type(&array_1, &tree);
        let key_2 = TypeKey::from_type(&array_2, &tree);
        assert_eq!(key_1, key_2);
    }

    /// Copyability differences yield distinct keys.
    #[test]
    fn test_type_key_copyability_distinguishes() {
        let mut tree = mir::NodeTree::new();

        let i32_id = tree.insert(mir::Type::INT32);
        let array_trivial = mir::Type::Array {
            element: i32_id,
            length: 4,
            copyability: mir::Copyability::Trivial,
        };
        let array_linear = mir::Type::Array {
            element: i32_id,
            length: 4,
            copyability: mir::Copyability::Linear,
        };

        let key_trivial = TypeKey::from_type(&array_trivial, &tree);
        let key_linear = TypeKey::from_type(&array_linear, &tree);

        assert_ne!(key_trivial, key_linear);
    }
}
