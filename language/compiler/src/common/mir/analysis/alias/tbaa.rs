use destack_core::StringId;
use destack_mir as mir;

use crate::common::mir::{MemoryLocation, TypeKey};

use super::result::AliasResult;

/// Type-based alias analysis.
///
/// Uses type information to prove that memory accesses cannot alias.
/// Based on strict aliasing rules: different incompatible types cannot alias.
///
/// The type hierarchy for TBAA:
/// - Scalars (int, float, bool) are distinct leaves
/// - Aggregates (struct, tuple, array) are distinct from scalars
/// - Pointers may alias other pointers but not non-pointers
#[derive(Debug)]
pub(crate) struct TypeBasedAA;

impl TypeBasedAA {
    /// Create a new TBAA analysis.
    pub(super) fn new() -> Self {
        Self
    }

    /// Query if two memory locations may alias based on their types.
    pub(super) fn alias(&self, loc_a: &MemoryLocation, loc_b: &MemoryLocation) -> AliasResult {
        // raw pointers can alias anything
        if matches!(
            (loc_a.pointer_kind, loc_b.pointer_kind),
            (Some(mir::ReferenceKind::Raw), _) | (_, Some(mir::ReferenceKind::Raw))
        ) {
            if let (Some(space_a), Some(space_b)) = (
                loc_a.pointer_address_space.clone(),
                loc_b.pointer_address_space.clone(),
            ) && space_a != space_b
            {
                return AliasResult::NoAlias;
            }

            return AliasResult::MayAlias;
        }

        // need type info for both
        let (ty_a, ty_b) = match (&loc_a.access_type, &loc_b.access_type) {
            (Some(a), Some(b)) => (a, b),
            _ => return AliasResult::MayAlias,
        };

        if self.types_cannot_alias(ty_a, ty_b) {
            AliasResult::NoAlias
        } else {
            AliasResult::MayAlias
        }
    }

    /// Check if two types cannot alias based on strict aliasing rules.
    ///
    /// Types cannot alias if they are fundamentally incompatible:
    /// - Different scalar types (int vs float)
    /// - Different aggregate types (struct vs tuple)
    /// - Scalars vs aggregates
    fn types_cannot_alias(&self, ty_a: &TypeKey, ty_b: &TypeKey) -> bool {
        if let (Some(space_a), Some(space_b)) =
            (Self::address_space_of(ty_a), Self::address_space_of(ty_b))
            && space_a != space_b
        {
            return true;
        }

        if Self::is_raw_reference(ty_a) || Self::is_raw_reference(ty_b) {
            return false;
        }

        match (ty_a, ty_b) {
            // different scalar categories cannot alias
            (TypeKey::Int { .. }, TypeKey::Float { .. })
            | (TypeKey::Float { .. }, TypeKey::Int { .. }) => true,

            // boolean vs numeric cannot alias
            (TypeKey::Boolean, TypeKey::Int { .. })
            | (TypeKey::Int { .. }, TypeKey::Boolean)
            | (TypeKey::Boolean, TypeKey::Float { .. })
            | (TypeKey::Float { .. }, TypeKey::Boolean) => true,

            // different integer widths/signedness may alias (byte aliasing)
            // this is conservative, matching C's char* can alias anything rule
            (TypeKey::Int { .. }, TypeKey::Int { .. }) => false,

            // different float widths may alias
            (TypeKey::Float { .. }, TypeKey::Float { .. }) => false,

            // struct vs tuple cannot alias
            (TypeKey::Tuple { .. }, TypeKey::Struct { .. })
            | (TypeKey::Struct { .. }, TypeKey::Tuple { .. }) => true,

            // different struct layouts cannot alias
            (
                TypeKey::Struct {
                    fields: f1,
                    copy: _,
                },
                TypeKey::Struct {
                    fields: f2,
                    copy: _,
                },
            ) => self.structs_cannot_alias(f1, f2),

            // different tuple layouts cannot alias
            (
                TypeKey::Tuple {
                    elements: e1,
                    copy: _,
                },
                TypeKey::Tuple {
                    elements: e2,
                    copy: _,
                },
            ) => self.tuples_cannot_alias(e1, e2),

            // array vs non-array aggregates cannot alias
            (TypeKey::Array { .. }, TypeKey::Struct { .. })
            | (TypeKey::Struct { .. }, TypeKey::Array { .. })
            | (TypeKey::Array { .. }, TypeKey::Tuple { .. })
            | (TypeKey::Tuple { .. }, TypeKey::Array { .. }) => true,

            // arrays with different element types
            (
                TypeKey::Array {
                    element: e1,
                    copy: _,
                    ..
                },
                TypeKey::Array {
                    element: e2,
                    copy: _,
                    ..
                },
            ) => self.types_cannot_alias(e1, e2),

            // references with different address spaces or pointee types
            (
                TypeKey::Reference {
                    address_space: a1,
                    pointee: p1,
                    ..
                },
                TypeKey::Reference {
                    address_space: a2,
                    pointee: p2,
                    ..
                },
            ) => a1 != a2 || self.types_cannot_alias(p1, p2),

            // reference vs non-reference
            (TypeKey::Reference { .. }, _) | (_, TypeKey::Reference { .. }) => {
                // references are pointers, they don't alias non-pointer data
                // but we're conservative here because the pointee might
                !matches!(
                    (ty_a, ty_b),
                    (TypeKey::Reference { .. }, TypeKey::Reference { .. })
                )
            }

            // function pointer types don't alias data
            (TypeKey::FunctionPointer { .. }, _) | (_, TypeKey::FunctionPointer { .. }) => {
                !matches!(
                    (ty_a, ty_b),
                    (
                        TypeKey::FunctionPointer { .. },
                        TypeKey::FunctionPointer { .. }
                    )
                )
            }

            // void doesn't alias anything (zero-sized)
            (TypeKey::Void, _) | (_, TypeKey::Void) => true,

            // conservative default
            _ => false,
        }
    }

    /// Return the address space for reference-like types.
    fn address_space_of(ty: &TypeKey) -> Option<mir::AddressSpace> {
        match ty {
            TypeKey::Reference { address_space, .. } => Some(address_space.clone()),
            TypeKey::TensorView { address_space, .. } => Some(address_space.clone()),
            _ => None,
        }
    }

    /// Return true when a type key represents a raw pointer.
    fn is_raw_reference(ty: &TypeKey) -> bool {
        matches!(
            ty,
            TypeKey::Reference {
                kind: mir::ReferenceKind::Raw,
                ..
            } | TypeKey::TensorView {
                kind: mir::ReferenceKind::Raw,
                ..
            }
        )
    }

    /// Check if two struct types cannot alias.
    fn structs_cannot_alias(
        &self,
        fields_a: &[(Option<StringId>, TypeKey)],
        fields_b: &[(Option<StringId>, TypeKey)],
    ) -> bool {
        // different field counts means different types
        if fields_a.len() != fields_b.len() {
            return true;
        }

        // check if any field types are incompatible
        for ((name_a, ty_a), (name_b, ty_b)) in fields_a.iter().zip(fields_b.iter()) {
            // different field names at same position
            if name_a != name_b {
                return true;
            }

            // recursively check field types
            if self.types_cannot_alias(ty_a, ty_b) {
                return true;
            }
        }

        false
    }

    /// Check if two tuple types cannot alias.
    fn tuples_cannot_alias(&self, elements_a: &[TypeKey], elements_b: &[TypeKey]) -> bool {
        // different element counts means different types
        if elements_a.len() != elements_b.len() {
            return true;
        }

        // check if any element types are incompatible
        for (ty_a, ty_b) in elements_a.iter().zip(elements_b.iter()) {
            if self.types_cannot_alias(ty_a, ty_b) {
                return true;
            }
        }

        false
    }
}

impl Default for TypeBasedAA {
    fn default() -> Self {
        // default to strict aliasing
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn make_loc_with_type(ptr_id: u32, ty: TypeKey) -> MemoryLocation {
        MemoryLocation::with_type(mir::Value::new(ptr_id), ty)
    }

    #[test]
    fn test_int_vs_float_no_alias() {
        let tbaa = TypeBasedAA::new();

        let int_ty = TypeKey::Int {
            width: 32,
            signed: true,
        };
        let float_ty = TypeKey::Float { width: 64 };

        let loc_int = make_loc_with_type(0, int_ty);
        let loc_float = make_loc_with_type(1, float_ty);

        assert_eq!(tbaa.alias(&loc_int, &loc_float), AliasResult::NoAlias);
    }

    #[test]
    fn test_same_int_may_alias() {
        let tbaa = TypeBasedAA::new();

        let int_ty = TypeKey::Int {
            width: 32,
            signed: true,
        };

        let loc1 = make_loc_with_type(0, int_ty.clone());
        let loc2 = make_loc_with_type(1, int_ty);

        assert_eq!(tbaa.alias(&loc1, &loc2), AliasResult::MayAlias);
    }

    #[test]
    fn test_bool_vs_int_no_alias() {
        let tbaa = TypeBasedAA::new();

        let bool_ty = TypeKey::Boolean;
        let int_ty = TypeKey::Int {
            width: 32,
            signed: true,
        };

        let loc_bool = make_loc_with_type(0, bool_ty);
        let loc_int = make_loc_with_type(1, int_ty);

        assert_eq!(tbaa.alias(&loc_bool, &loc_int), AliasResult::NoAlias);
    }

    #[test]
    fn test_struct_vs_tuple_no_alias() {
        let tbaa = TypeBasedAA::new();

        let struct_ty = TypeKey::Struct {
            fields: vec![
                (
                    None,
                    TypeKey::Int {
                        width: 32,
                        signed: true,
                    },
                ),
                (
                    None,
                    TypeKey::Int {
                        width: 32,
                        signed: true,
                    },
                ),
            ],
            copy: mir::Copy::Yes,
        };
        let tuple_ty = TypeKey::Tuple {
            elements: vec![
                TypeKey::Int {
                    width: 32,
                    signed: true,
                },
                TypeKey::Int {
                    width: 32,
                    signed: true,
                },
            ],
            copy: mir::Copy::Yes,
        };

        let loc_struct = make_loc_with_type(0, struct_ty);
        let loc_tuple = make_loc_with_type(1, tuple_ty);

        assert_eq!(tbaa.alias(&loc_struct, &loc_tuple), AliasResult::NoAlias);
    }

    #[test]
    fn test_different_struct_layouts_no_alias() {
        let tbaa = TypeBasedAA::new();

        let struct_2field = TypeKey::Struct {
            fields: vec![
                (
                    None,
                    TypeKey::Int {
                        width: 32,
                        signed: true,
                    },
                ),
                (
                    None,
                    TypeKey::Int {
                        width: 32,
                        signed: true,
                    },
                ),
            ],
            copy: mir::Copy::Yes,
        };
        let struct_3field = TypeKey::Struct {
            fields: vec![
                (
                    None,
                    TypeKey::Int {
                        width: 32,
                        signed: true,
                    },
                ),
                (
                    None,
                    TypeKey::Int {
                        width: 32,
                        signed: true,
                    },
                ),
                (
                    None,
                    TypeKey::Int {
                        width: 32,
                        signed: true,
                    },
                ),
            ],
            copy: mir::Copy::Yes,
        };

        let loc1 = make_loc_with_type(0, struct_2field);
        let loc2 = make_loc_with_type(1, struct_3field);

        assert_eq!(tbaa.alias(&loc1, &loc2), AliasResult::NoAlias);
    }

    #[test]
    fn test_array_vs_struct_no_alias() {
        let tbaa = TypeBasedAA::new();

        let array_ty = TypeKey::Array {
            element: Box::new(TypeKey::Int {
                width: 32,
                signed: true,
            }),
            length: 10,
            copy: mir::Copy::Yes,
        };
        let struct_ty = TypeKey::Struct {
            fields: vec![(
                None,
                TypeKey::Int {
                    width: 32,
                    signed: true,
                },
            )],
            copy: mir::Copy::Yes,
        };

        let loc_array = make_loc_with_type(0, array_ty);
        let loc_struct = make_loc_with_type(1, struct_ty);

        assert_eq!(tbaa.alias(&loc_array, &loc_struct), AliasResult::NoAlias);
    }

    #[test]
    fn test_no_type_info_may_alias() {
        let tbaa = TypeBasedAA::new();

        let loc1 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc2 = MemoryLocation::from_ptr(mir::Value::new(1));

        // without type info, we can't prove no-alias
        assert_eq!(tbaa.alias(&loc1, &loc2), AliasResult::MayAlias);
    }

    /// Strict TBAA still treats raw references as may-alias.
    #[test]
    fn test_strict_raw_refs_may_alias() {
        let tbaa = TypeBasedAA::new();

        let ref_i32 = TypeKey::Reference {
            kind: mir::ReferenceKind::Raw,
            lifetime: mir::Lifetime::empty(),
            address_space: mir::AddressSpace::Local,
            access: mir::Access::Mutable,
            pointee: Box::new(TypeKey::Int {
                width: 32,
                signed: true,
            }),
            nullability: mir::Nullability::None,
        };
        let ref_f64 = TypeKey::Reference {
            kind: mir::ReferenceKind::Raw,
            lifetime: mir::Lifetime::empty(),
            address_space: mir::AddressSpace::Local,
            access: mir::Access::Mutable,
            pointee: Box::new(TypeKey::Float { width: 64 }),
            nullability: mir::Nullability::None,
        };

        let loc_ref_i32 = make_loc_with_type(0, ref_i32);
        let loc_ref_f64 = make_loc_with_type(1, ref_f64);

        assert_eq!(
            tbaa.alias(&loc_ref_i32, &loc_ref_f64),
            AliasResult::MayAlias
        );
    }

    #[test]
    fn test_array_different_element_types_no_alias() {
        let tbaa = TypeBasedAA::new();

        let array_i32 = TypeKey::Array {
            element: Box::new(TypeKey::Int {
                width: 32,
                signed: true,
            }),
            length: 10,
            copy: mir::Copy::Yes,
        };
        let array_f64 = TypeKey::Array {
            element: Box::new(TypeKey::Float { width: 64 }),
            length: 10,
            copy: mir::Copy::Yes,
        };

        let loc1 = make_loc_with_type(0, array_i32);
        let loc2 = make_loc_with_type(1, array_f64);

        // arrays with incompatible element types cannot alias
        assert_eq!(tbaa.alias(&loc1, &loc2), AliasResult::NoAlias);
    }

    #[test]
    fn test_reference_vs_scalar_may_alias() {
        let tbaa = TypeBasedAA::new();

        let ref_ty = TypeKey::Reference {
            kind: mir::ReferenceKind::Raw,
            lifetime: mir::Lifetime::empty(),
            address_space: mir::AddressSpace::Local,
            access: mir::Access::Mutable,
            pointee: Box::new(TypeKey::Int {
                width: 32,
                signed: true,
            }),
            nullability: mir::Nullability::None,
        };
        let int_ty = TypeKey::Int {
            width: 32,
            signed: true,
        };

        let loc_ref = make_loc_with_type(0, ref_ty);
        let loc_int = make_loc_with_type(1, int_ty);

        // raw references can alias scalar data
        assert_eq!(tbaa.alias(&loc_ref, &loc_int), AliasResult::MayAlias);
    }

    #[test]
    fn test_references_same_pointee_may_alias() {
        let tbaa = TypeBasedAA::new();

        let ref_ty = TypeKey::Reference {
            kind: mir::ReferenceKind::Raw,
            lifetime: mir::Lifetime::empty(),
            address_space: mir::AddressSpace::Local,
            access: mir::Access::Mutable,
            pointee: Box::new(TypeKey::Int {
                width: 32,
                signed: true,
            }),
            nullability: mir::Nullability::None,
        };

        let loc1 = make_loc_with_type(0, ref_ty.clone());
        let loc2 = make_loc_with_type(1, ref_ty);

        // references to same type may alias
        assert_eq!(tbaa.alias(&loc1, &loc2), AliasResult::MayAlias);
    }

    /// References in different address spaces do not alias.
    #[test]
    fn test_references_different_address_spaces_no_alias() {
        let tbaa = TypeBasedAA::new();

        let ref_generic = TypeKey::Reference {
            kind: mir::ReferenceKind::Raw,
            lifetime: mir::Lifetime::empty(),
            address_space: mir::AddressSpace::Local,
            access: mir::Access::Mutable,
            pointee: Box::new(TypeKey::Int {
                width: 32,
                signed: true,
            }),
            nullability: mir::Nullability::None,
        };
        let ref_shared = TypeKey::Reference {
            kind: mir::ReferenceKind::Raw,
            lifetime: mir::Lifetime::empty(),
            address_space: mir::AddressSpace::Shared,
            access: mir::Access::Mutable,
            pointee: Box::new(TypeKey::Int {
                width: 32,
                signed: true,
            }),
            nullability: mir::Nullability::None,
        };

        let loc1 = make_loc_with_type(0, ref_generic);
        let loc2 = make_loc_with_type(1, ref_shared);

        // distinct address spaces cannot alias
        assert_eq!(tbaa.alias(&loc1, &loc2), AliasResult::NoAlias);
    }

    #[test]
    fn test_function_pointer_vs_data_no_alias() {
        let tbaa = TypeBasedAA::new();

        let fn_ptr_ty = TypeKey::FunctionPointer {
            signature: Box::new(TypeKey::FunctionSignature {
                parameters: vec![TypeKey::Int {
                    width: 32,
                    signed: true,
                }],
                result: Box::new(TypeKey::Void),
                borrow_obligations: Vec::new(),
            }),
        };
        let int_ty = TypeKey::Int {
            width: 64,
            signed: false,
        };

        let loc_fn = make_loc_with_type(0, fn_ptr_ty);
        let loc_int = make_loc_with_type(1, int_ty);

        // function pointer vs data type cannot alias
        assert_eq!(tbaa.alias(&loc_fn, &loc_int), AliasResult::NoAlias);
    }

    #[test]
    fn test_void_vs_anything_no_alias() {
        let tbaa = TypeBasedAA::new();

        let void_ty = TypeKey::Void;
        let int_ty = TypeKey::Int {
            width: 32,
            signed: true,
        };

        let loc_void = make_loc_with_type(0, void_ty);
        let loc_int = make_loc_with_type(1, int_ty);

        // void is zero-sized, cannot alias anything
        assert_eq!(tbaa.alias(&loc_void, &loc_int), AliasResult::NoAlias);
    }

    #[test]
    fn test_different_width_ints_may_alias() {
        let tbaa = TypeBasedAA::new();

        // different integer widths may alias (byte aliasing rule)
        let i32_ty = TypeKey::Int {
            width: 32,
            signed: true,
        };
        let i64_ty = TypeKey::Int {
            width: 64,
            signed: true,
        };

        let loc32 = make_loc_with_type(0, i32_ty);
        let loc64 = make_loc_with_type(1, i64_ty);

        // like C's char* can alias anything, different int widths may alias
        assert_eq!(tbaa.alias(&loc32, &loc64), AliasResult::MayAlias);
    }
}
