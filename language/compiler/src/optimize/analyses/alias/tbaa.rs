use destack_base::StringId;

use crate::optimize::common::{MemoryLocation, TypeKey};

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
pub(crate) struct TypeBasedAA {
    /// Whether to use strict TBAA rules.
    strict: bool,
}

impl TypeBasedAA {
    /// Create a new TBAA analysis.
    pub(super) fn new(strict: bool) -> Self {
        Self { strict }
    }

    /// Query if two memory locations may alias based on their types.
    pub(super) fn alias(&self, loc_a: &MemoryLocation, loc_b: &MemoryLocation) -> AliasResult {
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
        if !self.strict {
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
            (TypeKey::Struct { fields: f1 }, TypeKey::Struct { fields: f2 }) => {
                self.structs_cannot_alias(f1, f2)
            }

            // different tuple layouts cannot alias
            (TypeKey::Tuple { elements: e1 }, TypeKey::Tuple { elements: e2 }) => {
                self.tuples_cannot_alias(e1, e2)
            }

            // array vs non-array aggregates cannot alias
            (TypeKey::Array { .. }, TypeKey::Struct { .. })
            | (TypeKey::Struct { .. }, TypeKey::Array { .. })
            | (TypeKey::Array { .. }, TypeKey::Tuple { .. })
            | (TypeKey::Tuple { .. }, TypeKey::Array { .. }) => true,

            // arrays with different element types
            (TypeKey::Array { element: e1, .. }, TypeKey::Array { element: e2, .. }) => {
                self.types_cannot_alias(e1, e2)
            }

            // references with different pointee types
            (TypeKey::Reference { pointee: p1, .. }, TypeKey::Reference { pointee: p2, .. }) => {
                self.types_cannot_alias(p1, p2)
            }

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
        // strict TBAA by default
        Self::new(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_loc_with_type(ptr_id: u32, ty: TypeKey) -> MemoryLocation {
        MemoryLocation::with_type(destack_mir::Value::new(ptr_id), ty)
    }

    #[test]
    fn test_int_vs_float_no_alias() {
        let tbaa = TypeBasedAA::default();

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
        let tbaa = TypeBasedAA::default();

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
        let tbaa = TypeBasedAA::default();

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
        let tbaa = TypeBasedAA::default();

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
        };

        let loc_struct = make_loc_with_type(0, struct_ty);
        let loc_tuple = make_loc_with_type(1, tuple_ty);

        assert_eq!(tbaa.alias(&loc_struct, &loc_tuple), AliasResult::NoAlias);
    }

    #[test]
    fn test_different_struct_layouts_no_alias() {
        let tbaa = TypeBasedAA::default();

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
        };

        let loc1 = make_loc_with_type(0, struct_2field);
        let loc2 = make_loc_with_type(1, struct_3field);

        assert_eq!(tbaa.alias(&loc1, &loc2), AliasResult::NoAlias);
    }

    #[test]
    fn test_array_vs_struct_no_alias() {
        let tbaa = TypeBasedAA::default();

        let array_ty = TypeKey::Array {
            element: Box::new(TypeKey::Int {
                width: 32,
                signed: true,
            }),
            length: 10,
        };
        let struct_ty = TypeKey::Struct {
            fields: vec![(
                None,
                TypeKey::Int {
                    width: 32,
                    signed: true,
                },
            )],
        };

        let loc_array = make_loc_with_type(0, array_ty);
        let loc_struct = make_loc_with_type(1, struct_ty);

        assert_eq!(tbaa.alias(&loc_array, &loc_struct), AliasResult::NoAlias);
    }

    #[test]
    fn test_no_type_info_may_alias() {
        let tbaa = TypeBasedAA::default();

        let loc1 = MemoryLocation::from_ptr(destack_mir::Value::new(0));
        let loc2 = MemoryLocation::from_ptr(destack_mir::Value::new(1));

        // without type info, we can't prove no-alias
        assert_eq!(tbaa.alias(&loc1, &loc2), AliasResult::MayAlias);
    }

    #[test]
    fn test_non_strict_always_may_alias() {
        let tbaa = TypeBasedAA::new(false);

        let int_ty = TypeKey::Int {
            width: 32,
            signed: true,
        };
        let float_ty = TypeKey::Float { width: 64 };

        let loc_int = make_loc_with_type(0, int_ty);
        let loc_float = make_loc_with_type(1, float_ty);

        // non-strict TBAA doesn't prove anything
        assert_eq!(tbaa.alias(&loc_int, &loc_float), AliasResult::MayAlias);
    }

    #[test]
    fn test_array_different_element_types_no_alias() {
        let tbaa = TypeBasedAA::default();

        let array_i32 = TypeKey::Array {
            element: Box::new(TypeKey::Int {
                width: 32,
                signed: true,
            }),
            length: 10,
        };
        let array_f64 = TypeKey::Array {
            element: Box::new(TypeKey::Float { width: 64 }),
            length: 10,
        };

        let loc1 = make_loc_with_type(0, array_i32);
        let loc2 = make_loc_with_type(1, array_f64);

        // arrays with incompatible element types cannot alias
        assert_eq!(tbaa.alias(&loc1, &loc2), AliasResult::NoAlias);
    }

    #[test]
    fn test_reference_vs_scalar_no_alias() {
        let tbaa = TypeBasedAA::default();

        let ref_ty = TypeKey::Reference {
            kind: destack_mir::ReferenceKind::Raw,
            mutability: destack_mir::Mutability::Mutable,
            pointee: Box::new(TypeKey::Int {
                width: 32,
                signed: true,
            }),
            is_nullable: false,
        };
        let int_ty = TypeKey::Int {
            width: 32,
            signed: true,
        };

        let loc_ref = make_loc_with_type(0, ref_ty);
        let loc_int = make_loc_with_type(1, int_ty);

        // reference type vs scalar type cannot alias
        assert_eq!(tbaa.alias(&loc_ref, &loc_int), AliasResult::NoAlias);
    }

    #[test]
    fn test_references_same_pointee_may_alias() {
        let tbaa = TypeBasedAA::default();

        let ref_ty = TypeKey::Reference {
            kind: destack_mir::ReferenceKind::Raw,
            mutability: destack_mir::Mutability::Mutable,
            pointee: Box::new(TypeKey::Int {
                width: 32,
                signed: true,
            }),
            is_nullable: false,
        };

        let loc1 = make_loc_with_type(0, ref_ty.clone());
        let loc2 = make_loc_with_type(1, ref_ty);

        // references to same type may alias
        assert_eq!(tbaa.alias(&loc1, &loc2), AliasResult::MayAlias);
    }

    #[test]
    fn test_function_pointer_vs_data_no_alias() {
        let tbaa = TypeBasedAA::default();

        let fn_ptr_ty = TypeKey::FunctionPointer {
            parameters: vec![TypeKey::Int {
                width: 32,
                signed: true,
            }],
            result: Box::new(TypeKey::Void),
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
        let tbaa = TypeBasedAA::default();

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
        let tbaa = TypeBasedAA::default();

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
