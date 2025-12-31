use destack_dir::{
    GlobalSymbolId, IntType, LocalTypeId, PrimitiveType, ScalarLiteral, SymbolTable, SymbolType,
    Type, TypeField, TypeIndexSignature, TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ProfileId};

use super::super::common::NormalizationMode;
use super::{field_key_matches_index_kind, index_key_kind_for_type, index_key_kinds_compatible};
use crate::{AnalyzeOptions, Compiler};

/// Result of a type assignability check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assignability {
    /// Types are assignable
    Assignable,
    /// Types are not assignable
    NotAssignable,
    // ..Undecidable?
}

impl Assignability {
    /// Whether the types are assignable.
    pub fn is_assignable(self) -> bool {
        matches!(self, Assignability::Assignable)
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Check if `source` type is assignable to `target` type.
    /// Returns true if a value of type `source` can be assigned to a location of type `target`.
    pub fn is_type_assignable(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Assignability {
        // same type id: trivially assignable
        if target_id == source_id {
            return Assignability::Assignable;
        }

        let target_id = self.normalize_type(
            module,
            profile,
            target_id,
            symbols,
            types,
            NormalizationMode::Assignability,
        );
        let source_id = self.normalize_type(
            module,
            profile,
            source_id,
            symbols,
            types,
            NormalizationMode::Assignability,
        );

        if target_id == source_id {
            return Assignability::Assignable;
        }

        self.is_type_assignable_inner(
            module, profile, symbols, target_id, source_id, types, options,
        )
    }

    /// Inner assignability check on Type values.
    fn is_type_assignable_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Assignability {
        // follow cached alias instances for assignability comparisons
        let target_id = self.unwrap_assignability_alias_type(target_id, types);
        let source_id = self.unwrap_assignability_alias_type(source_id, types);

        let target = types.get_type(target_id).clone();
        let source = types.get_type(source_id).clone();

        // handle special target types first
        match &target {
            Type::InferVar { .. } => return Assignability::Assignable,
            // any accepts everything
            Type::TypeLiteral {
                value: TypeLiteral::Any,
            } => return Assignability::Assignable,

            // unknown accepts everything
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            } => return Assignability::Assignable,

            // never accepts nothing
            Type::TypeLiteral {
                value: TypeLiteral::Never,
            } => return Assignability::NotAssignable,

            _ => {}
        }

        // handle special source types
        match &source {
            Type::InferVar { .. } => return Assignability::Assignable,
            // never is assignable to everything (bottom type)
            Type::TypeLiteral {
                value: TypeLiteral::Never,
            } => return Assignability::Assignable,

            // any is assignable to everything (escape hatch)
            Type::TypeLiteral {
                value: TypeLiteral::Any,
            } => return Assignability::Assignable,

            _ => {}
        }

        // structural comparison
        match (target, source) {
            // type literals: must match exactly (with some exceptions)
            (Type::TypeLiteral { value: target_lit }, Type::TypeLiteral { value: source_lit }) => {
                self.is_type_literal_assignable(&target_lit, &source_lit)
            }

            // arrays: covariant in element type
            (
                Type::Array {
                    element: Some(target_elem),
                },
                Type::Array {
                    element: Some(source_elem),
                },
            ) => self.is_type_assignable(
                module,
                profile,
                symbols,
                target_elem,
                source_elem,
                types,
                options,
            ),

            // empty array is assignable to any array
            (Type::Array { element: Some(_) }, Type::Array { element: None }) => {
                Assignability::Assignable
            }

            // tuples: same length and each element assignable
            (
                Type::Tuple {
                    elements: target_elems,
                },
                Type::Tuple {
                    elements: source_elems,
                },
            ) => {
                if target_elems.len() != source_elems.len() {
                    return Assignability::NotAssignable;
                }
                for (target_elem, source_elem) in target_elems.iter().zip(source_elems.iter()) {
                    if !self
                        .is_type_assignable(
                            module,
                            profile,
                            symbols,
                            target_elem.ty,
                            source_elem.ty,
                            types,
                            options,
                        )
                        .is_assignable()
                    {
                        return Assignability::NotAssignable;
                    }
                }
                Assignability::Assignable
            }

            // tuple assignable to array if all elements are assignable to array element type
            (
                Type::Array {
                    element: Some(target_elem),
                },
                Type::Tuple {
                    elements: source_elems,
                },
            ) => {
                for source_elem in source_elems {
                    if !self
                        .is_type_assignable(
                            module,
                            profile,
                            symbols,
                            target_elem,
                            source_elem.ty,
                            types,
                            options,
                        )
                        .is_assignable()
                    {
                        return Assignability::NotAssignable;
                    }
                }
                Assignability::Assignable
            }

            // objects: structural subtyping (source must have all target fields)
            (
                Type::Object {
                    fields: target_fields,
                    call_signatures: target_call_signatures,
                    construct_signatures: target_construct_signatures,
                    index_signatures: target_index_signatures,
                },
                Type::Object {
                    fields: source_fields,
                    call_signatures: source_call_signatures,
                    construct_signatures: source_construct_signatures,
                    index_signatures: source_index_signatures,
                },
            ) => self.is_object_type_assignable(
                module,
                profile,
                symbols,
                &target_fields,
                &target_call_signatures,
                &target_construct_signatures,
                &target_index_signatures,
                &source_fields,
                &source_call_signatures,
                &source_construct_signatures,
                &source_index_signatures,
                types,
                options,
            ),

            // functions: contravariant params, covariant return
            (
                Type::Function {
                    dynamic_parameters: target_params,
                    this_parameter: target_this,
                    return_type: target_return,
                    ..
                },
                Type::Function {
                    dynamic_parameters: source_params,
                    this_parameter: source_this,
                    return_type: source_return,
                    ..
                },
            ) => self.is_function_type_assignable(
                module,
                profile,
                symbols,
                &target_params,
                &target_this,
                &target_return,
                &source_params,
                &source_this,
                &source_return,
                types,
                options,
            ),

            // callable objects: function values can satisfy call signatures
            (
                Type::Object {
                    fields: target_fields,
                    call_signatures: target_call_signatures,
                    construct_signatures: target_construct_signatures,
                    index_signatures: target_index_signatures,
                },
                Type::Function {
                    dynamic_parameters: source_params,
                    this_parameter: source_this,
                    return_type: source_return,
                    ..
                },
            ) => self.is_object_assignable_from_function(
                module,
                profile,
                symbols,
                &target_fields,
                &target_call_signatures,
                &target_construct_signatures,
                &target_index_signatures,
                &source_params,
                &source_this,
                &source_return,
                types,
                options,
            ),

            // functions: callable object sources must provide a compatible signature
            (
                Type::Function {
                    dynamic_parameters: target_params,
                    this_parameter: target_this,
                    return_type: target_return,
                    ..
                },
                Type::Object {
                    call_signatures: source_call_signatures,
                    ..
                },
            ) => self.is_function_assignable_from_object(
                module,
                profile,
                symbols,
                &target_params,
                &target_this,
                &target_return,
                &source_call_signatures,
                types,
                options,
            ),

            // union to union: each source element must fit a target element
            (
                Type::Union {
                    elements: target_elems,
                },
                Type::Union {
                    elements: source_elems,
                },
            ) => {
                for source_elem in source_elems {
                    let mut is_assignable = false;
                    for target_elem in &target_elems {
                        if self
                            .is_type_assignable_inner(
                                module,
                                profile,
                                symbols,
                                *target_elem,
                                source_elem,
                                types,
                                options,
                            )
                            .is_assignable()
                        {
                            is_assignable = true;
                            break;
                        }
                    }
                    if !is_assignable {
                        return Assignability::NotAssignable;
                    }
                }
                Assignability::Assignable
            }

            // union target: source must be assignable to at least one element
            (
                Type::Union {
                    elements: target_elems,
                },
                _,
            ) => {
                for target_elem in target_elems {
                    if self
                        .is_type_assignable_inner(
                            module,
                            profile,
                            symbols,
                            target_elem,
                            source_id,
                            types,
                            options,
                        )
                        .is_assignable()
                    {
                        return Assignability::Assignable;
                    }
                }
                Assignability::NotAssignable
            }

            // pointers: invariant in mutability and pointee type
            (
                Type::PointerOf {
                    mutability: target_mutability,
                    right: target_right,
                },
                Type::PointerOf {
                    mutability: source_mutability,
                    right: source_right,
                },
            ) => {
                if target_mutability != source_mutability {
                    return Assignability::NotAssignable;
                }

                let target_assignable = self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    target_right,
                    source_right,
                    types,
                    options,
                );
                let source_assignable = self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    source_right,
                    target_right,
                    types,
                    options,
                );
                if target_assignable.is_assignable() && source_assignable.is_assignable() {
                    Assignability::Assignable
                } else {
                    Assignability::NotAssignable
                }
            }

            // interface target: allow structural assignability from object source
            (
                Type::Reference {
                    symbol: target_symbol,
                    ..
                },
                Type::Object {
                    fields: source_fields,
                    call_signatures: source_call_signatures,
                    construct_signatures: source_construct_signatures,
                    index_signatures: source_index_signatures,
                },
            ) => {
                if target_symbol.ty().is_interface()
                    && let Some(target_instance_id) = types.get_instance_type_id(target_symbol)
                {
                    let target_instance = types.get_type(target_instance_id).clone();
                    if let Type::Object {
                        fields: target_fields,
                        call_signatures: target_call_signatures,
                        construct_signatures: target_construct_signatures,
                        index_signatures: target_index_signatures,
                    } = target_instance
                    {
                        return self.is_object_type_assignable(
                            module,
                            profile,
                            symbols,
                            &target_fields,
                            &target_call_signatures,
                            &target_construct_signatures,
                            &target_index_signatures,
                            &source_fields,
                            &source_call_signatures,
                            &source_construct_signatures,
                            &source_index_signatures,
                            types,
                            options,
                        );
                    }
                }

                Assignability::NotAssignable
            }

            // interface target: allow function values to satisfy call signatures
            (
                Type::Reference {
                    symbol: target_symbol,
                    ..
                },
                Type::Function {
                    dynamic_parameters: source_params,
                    this_parameter: source_this,
                    return_type: source_return,
                    ..
                },
            ) => {
                if target_symbol.ty().is_interface()
                    && let Some(target_instance_id) = types.get_instance_type_id(target_symbol)
                {
                    let target_instance = types.get_type(target_instance_id).clone();
                    if let Type::Object {
                        fields: target_fields,
                        call_signatures: target_call_signatures,
                        construct_signatures: target_construct_signatures,
                        index_signatures: target_index_signatures,
                    } = target_instance
                    {
                        return self.is_object_assignable_from_function(
                            module,
                            profile,
                            symbols,
                            &target_fields,
                            &target_call_signatures,
                            &target_construct_signatures,
                            &target_index_signatures,
                            &source_params,
                            &source_this,
                            &source_return,
                            types,
                            options,
                        );
                    }
                }

                Assignability::NotAssignable
            }

            // object target: allow interface sources with structural shape
            (
                Type::Object {
                    fields: target_fields,
                    call_signatures: target_call_signatures,
                    construct_signatures: target_construct_signatures,
                    index_signatures: target_index_signatures,
                },
                Type::Reference {
                    symbol: source_symbol,
                    ..
                },
            ) => {
                if source_symbol.ty().is_interface()
                    && let Some(source_instance_id) = types.get_instance_type_id(source_symbol)
                {
                    let source_instance = types.get_type(source_instance_id).clone();
                    if let Type::Object {
                        fields: source_fields,
                        call_signatures: source_call_signatures,
                        construct_signatures: source_construct_signatures,
                        index_signatures: source_index_signatures,
                    } = source_instance
                    {
                        return self.is_object_type_assignable(
                            module,
                            profile,
                            symbols,
                            &target_fields,
                            &target_call_signatures,
                            &target_construct_signatures,
                            &target_index_signatures,
                            &source_fields,
                            &source_call_signatures,
                            &source_construct_signatures,
                            &source_index_signatures,
                            types,
                            options,
                        );
                    }
                }

                Assignability::NotAssignable
            }

            // function target: accept callable interface sources
            (
                Type::Function {
                    dynamic_parameters: target_params,
                    this_parameter: target_this,
                    return_type: target_return,
                    ..
                },
                Type::Reference {
                    symbol: source_symbol,
                    ..
                },
            ) => {
                if source_symbol.ty().is_interface()
                    && let Some(source_instance_id) = types.get_instance_type_id(source_symbol)
                {
                    let source_instance = types.get_type(source_instance_id).clone();
                    if let Type::Object {
                        call_signatures: source_call_signatures,
                        ..
                    } = source_instance
                    {
                        return self.is_function_assignable_from_object(
                            module,
                            profile,
                            symbols,
                            &target_params,
                            &target_this,
                            &target_return,
                            &source_call_signatures,
                            types,
                            options,
                        );
                    }
                }

                Assignability::NotAssignable
            }

            // union source: all elements must be assignable to target
            (
                _,
                Type::Union {
                    elements: source_elems,
                },
            ) => {
                for source_elem in source_elems {
                    if !self
                        .is_type_assignable_inner(
                            module,
                            profile,
                            symbols,
                            target_id,
                            source_elem,
                            types,
                            options,
                        )
                        .is_assignable()
                    {
                        return Assignability::NotAssignable;
                    }
                }
                Assignability::Assignable
            }

            // intersection target: source must be assignable to all elements
            (
                Type::Intersection {
                    elements: target_elems,
                },
                _,
            ) => {
                for target_elem in target_elems {
                    if !self
                        .is_type_assignable_inner(
                            module,
                            profile,
                            symbols,
                            target_elem,
                            source_id,
                            types,
                            options,
                        )
                        .is_assignable()
                    {
                        return Assignability::NotAssignable;
                    }
                }
                Assignability::Assignable
            }

            // intersection source: at least one element must be assignable to target
            (
                _,
                Type::Intersection {
                    elements: source_elems,
                },
            ) => {
                for source_elem in source_elems {
                    if self
                        .is_type_assignable_inner(
                            module,
                            profile,
                            symbols,
                            target_id,
                            source_elem,
                            types,
                            options,
                        )
                        .is_assignable()
                    {
                        return Assignability::Assignable;
                    }
                }
                Assignability::NotAssignable
            }

            // references: same symbol OR source is subtype of target via lineage OR structurally compatible
            // NOTE #Incomplete: should also check type arguments
            (
                Type::Reference {
                    symbol: target_symbol,
                    ..
                },
                Type::Reference {
                    symbol: source_symbol,
                    ..
                },
            ) => {
                // nominal check: same symbol or lineage
                if target_symbol == source_symbol
                    || self.is_type_lineage_assignable(source_symbol, target_symbol, types)
                {
                    return Assignability::Assignable;
                }

                // structural check: only for interfaces
                if target_symbol.ty().is_interface()
                    && let (Some(target_instance_id), Some(source_instance_id)) = (
                        types.get_instance_type_id(target_symbol),
                        types.get_instance_type_id(source_symbol),
                    )
                {
                    let target_instance = types.get_type(target_instance_id).clone();
                    let source_instance = types.get_type(source_instance_id).clone();
                    if let (
                        Type::Object {
                            fields: target_fields,
                            call_signatures: target_call_signatures,
                            construct_signatures: target_construct_signatures,
                            index_signatures: target_index_signatures,
                        },
                        Type::Object {
                            fields: source_fields,
                            call_signatures: source_call_signatures,
                            construct_signatures: source_construct_signatures,
                            index_signatures: source_index_signatures,
                        },
                    ) = (target_instance, source_instance)
                    {
                        return self.is_object_type_assignable(
                            module,
                            profile,
                            symbols,
                            &target_fields,
                            &target_call_signatures,
                            &target_construct_signatures,
                            &target_index_signatures,
                            &source_fields,
                            &source_call_signatures,
                            &source_construct_signatures,
                            &source_index_signatures,
                            types,
                            options,
                        );
                    }
                }

                Assignability::NotAssignable
            }

            // error types: always assignable (to suppress cascading errors)
            (Type::Error, _) | (_, Type::Error) => Assignability::Assignable,

            // everything else: not assignable
            _ => Assignability::NotAssignable,
        }
    }

    /// Check type literal assignability.
    fn is_type_literal_assignable(
        &self,
        target: &TypeLiteral,
        source: &TypeLiteral,
    ) -> Assignability {
        // exact match
        if target == source {
            return Assignability::Assignable;
        }

        match (target, source) {
            // any/unknown already handled above, but handle for completeness
            (TypeLiteral::Any, _) | (TypeLiteral::Unknown, _) => Assignability::Assignable,
            (_, TypeLiteral::Never) | (_, TypeLiteral::Any) => Assignability::Assignable,

            // null is only assignable to null (or any/unknown)
            (TypeLiteral::Null, TypeLiteral::Null) => Assignability::Assignable,

            // undefined is only assignable to undefined or void
            (TypeLiteral::Void, TypeLiteral::Undefined) => Assignability::Assignable,
            (TypeLiteral::Undefined, TypeLiteral::Undefined) => Assignability::Assignable,

            // primitives: check for exact match or numeric widening
            (
                TypeLiteral::Primitive(target_primitive),
                TypeLiteral::Primitive(source_primitive),
            ) => {
                if target_primitive == source_primitive
                    || self.is_primitive_numeric_assignable(target_primitive, source_primitive)
                {
                    Assignability::Assignable
                } else {
                    Assignability::NotAssignable
                }
            }

            // scalar literal to primitive: check if literal is of that primitive type
            (TypeLiteral::Primitive(primitive_type), TypeLiteral::ScalarLiteral(literal)) => {
                if self.is_scalar_literal_assignable(literal, primitive_type) {
                    Assignability::Assignable
                } else {
                    Assignability::NotAssignable
                }
            }

            // everything else: not assignable
            _ => Assignability::NotAssignable,
        }
    }

    /// Check if a scalar literal value matches a primitive type.
    fn is_scalar_literal_assignable(&self, literal: &ScalarLiteral, ty: &PrimitiveType) -> bool {
        match (literal, ty) {
            // boolean
            (ScalarLiteral::Boolean(_), PrimitiveType::Boolean) => true,
            // string (including regex strings)
            (
                ScalarLiteral::String(_) | ScalarLiteral::RegexString { .. },
                PrimitiveType::String,
            ) => true,
            // integer and float to number (JavaScript style)
            (ScalarLiteral::Integer(_) | ScalarLiteral::Float(_), PrimitiveType::Number) => true,
            // integer literal to specific int type: check range
            (ScalarLiteral::Integer(value), PrimitiveType::Int(int_type)) => {
                self.is_integer_literal_assignable(*value as i128, int_type)
            }
            // float literal to specific float type: always allowed (may lose precision)
            (ScalarLiteral::Float(_), PrimitiveType::Float(_)) => true,
            // integer literal to float type: always allowed (implicit conversion)
            (ScalarLiteral::Integer(_), PrimitiveType::Float(_)) => true,
            // bigint
            (ScalarLiteral::Bigint(_), PrimitiveType::Bigint) => true,
            // character
            (ScalarLiteral::Character(_), PrimitiveType::Character) => true,
            _ => false,
        }
    }

    /// Check if an integer literal value fits within the range of a specific int type.
    fn is_integer_literal_assignable(&self, value: i128, int_type: &IntType) -> bool {
        let (min, max) = match int_type {
            IntType::Int8 => (i8::MIN as i128, i8::MAX as i128),
            IntType::Int16 => (i16::MIN as i128, i16::MAX as i128),
            IntType::Int32 => (i32::MIN as i128, i32::MAX as i128),
            IntType::Int64 => (i64::MIN as i128, i64::MAX as i128),
            IntType::Int128 | IntType::Int256 => (i128::MIN, i128::MAX),
            IntType::Uint8 => (0, u8::MAX as i128),
            IntType::Uint16 => (0, u16::MAX as i128),
            IntType::Uint32 => (0, u32::MAX as i128),
            IntType::Uint64 => (0, u64::MAX as i128),
            IntType::Uint128 | IntType::Uint256 => (0, i128::MAX), // (can't represent u128::MAX in i128)
            IntType::Isize | IntType::Usize => {
                // pointer sized integers: use target platform pointer size
                // for now, assume 64 bit
                if int_type.is_signed() {
                    (i64::MIN as i128, i64::MAX as i128)
                } else {
                    (0, u64::MAX as i128)
                }
            }
            IntType::Arbitrary { width, is_signed } => {
                if *is_signed {
                    let half_range = 1i128 << (width - 1);
                    (-half_range, half_range - 1)
                } else {
                    let max_val = if *width >= 128 {
                        i128::MAX
                    } else {
                        (1i128 << width) - 1
                    };
                    (0, max_val)
                }
            }
        };
        value >= min && value <= max
    }

    /// Check if numeric widening from source to target is allowed.
    /// Widening is allowed when assigning a smaller numeric type to a larger one.
    fn is_primitive_numeric_assignable(
        &self,
        target: &PrimitiveType,
        source: &PrimitiveType,
    ) -> bool {
        match (target, source) {
            // number accepts any numeric type (JS compatibility)
            (PrimitiveType::Number, PrimitiveType::Int(_) | PrimitiveType::Float(_)) => true,

            // float widening: float32 to float64
            (PrimitiveType::Float(target_float), PrimitiveType::Float(source_float)) => {
                target_float.width() >= source_float.width()
            }

            // int to float: always allowed (may lose precision for large ints)
            (PrimitiveType::Float(_), PrimitiveType::Int(_)) => true,

            // signed int widening: int8 to int16 to int32 to int64 to int128
            (PrimitiveType::Int(target_int), PrimitiveType::Int(source_int)) => {
                match (target_int.width(), source_int.width()) {
                    (Some(tw), Some(sw)) => {
                        if target_int.is_signed() == source_int.is_signed() {
                            // same signedness: target must be at least as wide
                            tw >= sw
                        } else if target_int.is_signed() && !source_int.is_signed() {
                            // unsigned to signed: target must be strictly wider
                            // (uint8 max 255 fits in int16, but not int8)
                            tw > sw
                        } else {
                            // signed to unsigned: not safe (negative values)
                            false
                        }
                    }
                    // pointer sized ints: only allow same signedness
                    _ => false,
                }
            }

            _ => false,
        }
    }

    /// Check object type assignability (structural subtyping).
    fn is_object_type_assignable(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_fields: &[TypeField],
        target_call_signatures: &[LocalTypeId],
        target_construct_signatures: &[LocalTypeId],
        target_index_signatures: &[TypeIndexSignature],
        source_fields: &[TypeField],
        source_call_signatures: &[LocalTypeId],
        source_construct_signatures: &[LocalTypeId],
        source_index_signatures: &[TypeIndexSignature],
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Assignability {
        if self.is_object_fields_assignable(
            module,
            profile,
            symbols,
            target_fields,
            source_fields,
            types,
            options,
        ) == Assignability::NotAssignable
        {
            return Assignability::NotAssignable;
        }

        if !self.is_signature_set_assignable(
            module,
            profile,
            symbols,
            target_call_signatures,
            source_call_signatures,
            types,
            options,
        ) {
            return Assignability::NotAssignable;
        }

        if !self.is_signature_set_assignable(
            module,
            profile,
            symbols,
            target_construct_signatures,
            source_construct_signatures,
            types,
            options,
        ) {
            return Assignability::NotAssignable;
        }

        if !self.is_index_signatures_assignable(
            module,
            profile,
            symbols,
            target_index_signatures,
            source_index_signatures,
            source_fields,
            types,
            options,
        ) {
            return Assignability::NotAssignable;
        }

        Assignability::Assignable
    }

    /// Check callable object assignability from a function type.
    fn is_object_assignable_from_function(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_fields: &[TypeField],
        target_call_signatures: &[LocalTypeId],
        target_construct_signatures: &[LocalTypeId],
        target_index_signatures: &[TypeIndexSignature],
        source_params: &[LocalTypeId],
        source_this: &Option<LocalTypeId>,
        source_return: &Option<LocalTypeId>,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Assignability {
        if self.is_object_fields_assignable(
            module,
            profile,
            symbols,
            target_fields,
            &[],
            types,
            options,
        ) == Assignability::NotAssignable
        {
            return Assignability::NotAssignable;
        }

        if !self.is_call_signatures_assignable_from_function(
            module,
            profile,
            symbols,
            target_call_signatures,
            source_params,
            source_this,
            source_return,
            types,
            options,
        ) {
            return Assignability::NotAssignable;
        }

        if !self.is_signature_set_assignable(
            module,
            profile,
            symbols,
            target_construct_signatures,
            &[],
            types,
            options,
        ) {
            return Assignability::NotAssignable;
        }

        if !self.is_index_signatures_assignable(
            module,
            profile,
            symbols,
            target_index_signatures,
            &[],
            &[],
            types,
            options,
        ) {
            return Assignability::NotAssignable;
        }

        Assignability::Assignable
    }

    /// Check function assignability from callable object signatures.
    fn is_function_assignable_from_object(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_params: &[LocalTypeId],
        target_this: &Option<LocalTypeId>,
        target_return: &Option<LocalTypeId>,
        source_call_signatures: &[LocalTypeId],
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Assignability {
        if source_call_signatures.is_empty() {
            return Assignability::NotAssignable;
        }

        for source_signature in source_call_signatures {
            let signature = types.get_type(*source_signature).clone();
            let Type::Function {
                dynamic_parameters: source_params,
                this_parameter: source_this,
                return_type: source_return,
                ..
            } = signature
            else {
                continue;
            };

            if self
                .is_function_type_assignable(
                    module,
                    profile,
                    symbols,
                    target_params,
                    target_this,
                    target_return,
                    &source_params,
                    &source_this,
                    &source_return,
                    types,
                    options,
                )
                .is_assignable()
            {
                return Assignability::Assignable;
            }
        }

        Assignability::NotAssignable
    }

    /// Check object field assignability (structural subtyping).
    fn is_object_fields_assignable(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_fields: &[TypeField],
        source_fields: &[TypeField],
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Assignability {
        let undefined_ty_id = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Undefined,
        });

        // for each target field, find matching source field
        for target_field in target_fields {
            let source_field = source_fields
                .iter()
                .find(|field| field.key.matches(&target_field.key));

            match source_field {
                Some(source_field) => {
                    if !target_field.is_optional && source_field.is_optional {
                        return Assignability::NotAssignable;
                    }

                    let mut is_assignable = self
                        .is_type_assignable_inner(
                            module,
                            profile,
                            symbols,
                            target_field.ty,
                            source_field.ty,
                            types,
                            options,
                        )
                        .is_assignable();

                    if !options.exact_optional_property_types && target_field.is_optional {
                        is_assignable |= self
                            .is_type_assignable_inner(
                                module,
                                profile,
                                symbols,
                                undefined_ty_id,
                                source_field.ty,
                                types,
                                options,
                            )
                            .is_assignable();
                    }

                    if !options.exact_optional_property_types && source_field.is_optional {
                        let undefined_assignable = self
                            .is_type_assignable_inner(
                                module,
                                profile,
                                symbols,
                                target_field.ty,
                                undefined_ty_id,
                                types,
                                options,
                            )
                            .is_assignable();
                        is_assignable &= undefined_assignable;
                    }

                    if !is_assignable {
                        return Assignability::NotAssignable;
                    }
                }
                None => {
                    // field missing: only okay if target field is optional
                    if !target_field.is_optional {
                        return Assignability::NotAssignable;
                    }
                }
            }
        }

        Assignability::Assignable
    }

    /// Follow cached alias instances for assignability comparisons.
    fn unwrap_assignability_alias_type(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> LocalTypeId {
        let mut visited_symbols: Vec<GlobalSymbolId> = Vec::new();
        let mut current_id = type_id;

        loop {
            let Type::Reference { symbol, .. } = types.get_type(current_id) else {
                break;
            };

            if symbol.ty() != SymbolType::TypeAlias {
                break;
            }

            if visited_symbols.contains(symbol) {
                break;
            }
            visited_symbols.push(*symbol);

            let Some(instance_ty_id) = types.get_instance_type_id(*symbol) else {
                break;
            };
            current_id = instance_ty_id;
        }

        current_id
    }

    /// Check assignability of signature sets (target signatures must be matched).
    fn is_signature_set_assignable(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_signatures: &[LocalTypeId],
        source_signatures: &[LocalTypeId],
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        if target_signatures.is_empty() {
            return true;
        }
        for target_signature in target_signatures {
            let mut matched = false;
            for source_signature in source_signatures {
                if self
                    .is_type_assignable(
                        module,
                        profile,
                        symbols,
                        *target_signature,
                        *source_signature,
                        types,
                        options,
                    )
                    .is_assignable()
                {
                    matched = true;
                    break;
                }
            }
            if !matched {
                return false;
            }
        }
        true
    }

    /// Check assignability of call signatures against a single function source.
    fn is_call_signatures_assignable_from_function(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_signatures: &[LocalTypeId],
        source_params: &[LocalTypeId],
        source_this: &Option<LocalTypeId>,
        source_return: &Option<LocalTypeId>,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        if target_signatures.is_empty() {
            return true;
        }

        for target_signature in target_signatures {
            let signature = types.get_type(*target_signature).clone();
            let Type::Function {
                dynamic_parameters: target_params,
                this_parameter: target_this,
                return_type: target_return,
                ..
            } = signature
            else {
                return false;
            };

            if !self
                .is_function_type_assignable(
                    module,
                    profile,
                    symbols,
                    &target_params,
                    &target_this,
                    &target_return,
                    source_params,
                    source_this,
                    source_return,
                    types,
                    options,
                )
                .is_assignable()
            {
                return false;
            }
        }

        true
    }

    /// Check assignability of index signatures.
    fn is_index_signatures_assignable(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_signatures: &[TypeIndexSignature],
        source_signatures: &[TypeIndexSignature],
        source_fields: &[TypeField],
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        if target_signatures.is_empty() {
            return true;
        }
        for target_signature in target_signatures {
            let mut matched = false;
            let mut has_matching_key_kind = false;
            for source_signature in source_signatures {
                let target_kind = index_key_kind_for_type(target_signature.key_type, types);
                let source_kind = index_key_kind_for_type(source_signature.key_type, types);
                if !index_key_kinds_compatible(target_kind, source_kind) {
                    continue;
                }
                has_matching_key_kind = true;
                if self.is_index_signature_assignable(
                    module,
                    profile,
                    symbols,
                    target_signature,
                    source_signature,
                    types,
                    options,
                ) {
                    matched = true;
                    break;
                }
            }
            if matched {
                continue;
            }
            if has_matching_key_kind {
                return false;
            }
            if !self.are_fields_assignable_to_index_signature(
                module,
                profile,
                symbols,
                target_signature,
                source_fields,
                types,
                options,
            ) {
                return false;
            }
        }
        true
    }

    /// Check assignability for a single index signature.
    fn is_index_signature_assignable(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_signature: &TypeIndexSignature,
        source_signature: &TypeIndexSignature,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        let target_kind = index_key_kind_for_type(target_signature.key_type, types);
        let source_kind = index_key_kind_for_type(source_signature.key_type, types);
        if !index_key_kinds_compatible(target_kind, source_kind) {
            return false;
        }

        if !self
            .is_type_assignable(
                module,
                profile,
                symbols,
                target_signature.value_type,
                source_signature.value_type,
                types,
                options,
            )
            .is_assignable()
        {
            return false;
        }

        true
    }

    /// Check if any source field violates a target index signature.
    fn are_fields_assignable_to_index_signature(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_signature: &TypeIndexSignature,
        source_fields: &[TypeField],
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        let key_kind = index_key_kind_for_type(target_signature.key_type, types);
        for field in source_fields {
            if !field_key_matches_index_kind(&field.key, key_kind) {
                continue;
            }
            if !self.is_field_type_assignable_to_index_signature(
                module,
                profile,
                symbols,
                target_signature.value_type,
                field,
                types,
                options,
            ) {
                return false;
            }
        }
        true
    }

    /// Check if a field type is compatible with an index signature value type.
    fn is_field_type_assignable_to_index_signature(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        value_type: LocalTypeId,
        field: &TypeField,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        let undefined_ty_id = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Undefined,
        });

        let mut is_assignable = self
            .is_type_assignable_inner(
                module, profile, symbols, value_type, field.ty, types, options,
            )
            .is_assignable();

        if !options.exact_optional_property_types && field.is_optional {
            let undefined_assignable = self
                .is_type_assignable_inner(
                    module,
                    profile,
                    symbols,
                    value_type,
                    undefined_ty_id,
                    types,
                    options,
                )
                .is_assignable();
            is_assignable &= undefined_assignable;
        }

        is_assignable
    }

    /// Check function type assignability (contravariant params, covariant return).
    fn is_function_type_assignable(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_params: &[LocalTypeId],
        target_this: &Option<LocalTypeId>,
        target_return: &Option<LocalTypeId>,
        source_params: &[LocalTypeId],
        source_this: &Option<LocalTypeId>,
        source_return: &Option<LocalTypeId>,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Assignability {
        // this parameter: contravariant when strict, bivariant otherwise
        if let (Some(target_this), Some(source_this)) = (target_this, source_this) {
            let strict_assignable = self
                .is_type_assignable(
                    module,
                    profile,
                    symbols,
                    *source_this,
                    *target_this,
                    types,
                    options,
                )
                .is_assignable();
            let loose_assignable = self
                .is_type_assignable(
                    module,
                    profile,
                    symbols,
                    *target_this,
                    *source_this,
                    types,
                    options,
                )
                .is_assignable();
            if options.strict_function_types {
                if !strict_assignable {
                    return Assignability::NotAssignable;
                }
            } else if !strict_assignable && !loose_assignable {
                return Assignability::NotAssignable;
            }
        }

        // source must not require more parameters than target provides
        if source_params.len() > target_params.len() {
            return Assignability::NotAssignable;
        }

        // parameters: contravariant when strict, bivariant otherwise
        for (target_param, source_param) in target_params.iter().zip(source_params.iter()) {
            let strict_assignable = self
                .is_type_assignable(
                    module,
                    profile,
                    symbols,
                    *source_param,
                    *target_param,
                    types,
                    options,
                )
                .is_assignable();
            let loose_assignable = self
                .is_type_assignable(
                    module,
                    profile,
                    symbols,
                    *target_param,
                    *source_param,
                    types,
                    options,
                )
                .is_assignable();
            if options.strict_function_types {
                if !strict_assignable {
                    return Assignability::NotAssignable;
                }
            } else if !strict_assignable && !loose_assignable {
                return Assignability::NotAssignable;
            }
        }

        // return type: covariant (target return must be assignable from source return)
        match (target_return, source_return) {
            (Some(target_ret), Some(source_ret)) => self.is_type_assignable(
                module,
                profile,
                symbols,
                *target_ret,
                *source_ret,
                types,
                options,
            ),
            (None, _) => Assignability::Assignable,
            (Some(_), None) => Assignability::NotAssignable,
        }
    }

    /// Check if source_symbol is a subtype of target_symbol via lineage (follows inheritance chain).
    /// (This also checks visible extensions that add `implements` clauses to the source type.)
    pub fn is_type_lineage_assignable(
        &self,
        source_symbol: GlobalSymbolId,
        target_symbol: GlobalSymbolId,
        types: &TypeTable,
    ) -> bool {
        // step 1: check the type's own lineage
        if let Some(lineage) = types.get_lineage_for_symbol(source_symbol) {
            // check direct extends
            if let Some(extends) = lineage.extends {
                if extends == target_symbol {
                    return true;
                }
                if self.is_type_lineage_assignable(extends, target_symbol, types) {
                    return true;
                }
            }

            // check direct implements
            for &implements in &lineage.implements {
                if implements == target_symbol {
                    return true;
                }
                if self.is_type_lineage_assignable(implements, target_symbol, types) {
                    return true;
                }
            }

            // check embedded types (composition can also contribute to assignability)
            for &embedded in &lineage.embedded {
                if embedded == target_symbol {
                    return true;
                }
                if self.is_type_lineage_assignable(embedded, target_symbol, types) {
                    return true;
                }
            }
        }

        // step 2: check visible extensions that add implements clauses
        if let Some(extension_ids) = types.get_extensions_for_target(source_symbol) {
            let module = self.program.modules.get(types.module_id);
            let module = module.read();

            for extension_id in extension_ids {
                let extension = types.get_extension(*extension_id);

                // check visibility (reuses the same function as member lookup)
                if !self.is_extension_visible(&module, extension) {
                    continue;
                }

                // check extension's lineage (implements clauses)
                if let Some(lineage_id) = extension.lineage {
                    let lineage = types.get_lineage(lineage_id);

                    // extensions typically only add implements, but check all for completeness
                    for &implements in &lineage.implements {
                        if implements == target_symbol {
                            return true;
                        }
                        if self.is_type_lineage_assignable(implements, target_symbol, types) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use destack_dir::{
        Asynchrony, FloatType, FunctionCardinality, IntType, PrimitiveType, ScalarLiteral,
        StaticKey, Type, TypeElement, TypeField, TypeIndexSignature, TypeLiteral,
    };
    use destack_source::{FileContent, Span};

    use crate::{Assignability, TestProgram};

    /// Number is assignable to number.
    #[test]
    fn test_analyze_assignability_same_primitive() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "let x: number = 42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, number_ty, number_ty, &mut types, &options
            ),
            Assignability::Assignable
        );
    }

    /// String is not assignable to number.
    #[test]
    fn test_analyze_assignability_different_primitives() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let string_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, number_ty, string_ty, &mut types, &options
            ),
            Assignability::NotAssignable
        );
    }

    /// Literal 42 is assignable to number.
    #[test]
    fn test_analyze_assignability_literal_to_primitive() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let literal_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(42)),
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, number_ty, literal_ty, &mut types, &options
            ),
            Assignability::Assignable
        );
    }

    /// Anything is assignable to any.
    #[test]
    fn test_analyze_assignability_any() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);

        let any_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Any,
        });
        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, any_ty, number_ty, &mut types, &options
            ),
            Assignability::Assignable
        );
    }

    /// Never is assignable to anything (bottom type).
    #[test]
    fn test_analyze_assignability_never_source() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);

        let never_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Never,
        });
        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, number_ty, never_ty, &mut types, &options
            ),
            Assignability::Assignable
        );
    }

    /// Nothing is assignable to never (except never itself).
    #[test]
    fn test_analyze_assignability_never_target() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);

        let never_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Never,
        });
        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, never_ty, number_ty, &mut types, &options
            ),
            Assignability::NotAssignable
        );
    }

    /// [number, string] is assignable to [number, string].
    #[test]
    fn test_analyze_assignability_tuple() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let string_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });
        let tuple_ty = types.insert_type(Type::Tuple {
            elements: vec![TypeElement::new(number_ty), TypeElement::new(string_ty)],
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, tuple_ty, tuple_ty, &mut types, &options
            ),
            Assignability::Assignable
        );
    }

    /// [number, string] is not assignable to [number].
    #[test]
    fn test_analyze_assignability_tuple_different_length() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let string_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });
        let tuple_short = types.insert_type(Type::Tuple {
            elements: vec![TypeElement::new(number_ty)],
        });
        let tuple_long = types.insert_type(Type::Tuple {
            elements: vec![TypeElement::new(number_ty), TypeElement::new(string_ty)],
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module,
                profile,
                &symbols,
                tuple_short,
                tuple_long,
                &mut types,
                &options
            ),
            Assignability::NotAssignable
        );
    }

    /// number[] is assignable to number[].
    #[test]
    fn test_analyze_assignability_array() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let array_ty = types.insert_type(Type::Array {
            element: Some(number_ty),
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, array_ty, array_ty, &mut types, &options
            ),
            Assignability::Assignable
        );
    }

    /// [number, number] is assignable to number[].
    #[test]
    fn test_analyze_assignability_tuple_to_array() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let tuple_ty = types.insert_type(Type::Tuple {
            elements: vec![TypeElement::new(number_ty), TypeElement::new(number_ty)],
        });
        let array_ty = types.insert_type(Type::Array {
            element: Some(number_ty),
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, array_ty, tuple_ty, &mut types, &options
            ),
            Assignability::Assignable
        );
    }

    /// { a: number, b: string } is assignable to { a: number }.
    #[test]
    fn test_analyze_assignability_object_structural() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);
        let strings = test.program.strings.clone();

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let string_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });

        let key_a = destack_dir::StaticKey::Name(strings.intern("a"));
        let key_b = destack_dir::StaticKey::Name(strings.intern("b"));

        let obj_small = types.insert_type(Type::Object {
            fields: vec![TypeField {
                key: key_a,
                ty: number_ty,
                is_optional: false,
                is_readonly: false,
            }],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        });
        let obj_large = types.insert_type(Type::Object {
            fields: vec![
                TypeField {
                    key: key_a,
                    ty: number_ty,
                    is_optional: false,
                    is_readonly: false,
                },
                TypeField {
                    key: key_b,
                    ty: string_ty,
                    is_optional: false,
                    is_readonly: false,
                },
            ],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        });

        // larger object assignable to smaller (has all required fields)
        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, obj_small, obj_large, &mut types, &options
            ),
            Assignability::Assignable
        );

        // smaller object not assignable to larger (missing field b)
        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, obj_large, obj_small, &mut types, &options
            ),
            Assignability::NotAssignable
        );
    }

    /// Optional fields are not assignable to required fields.
    #[test]
    fn test_analyze_assignability_object_optional_field() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);
        let strings = test.program.strings.clone();

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let required_field = TypeField {
            key: StaticKey::Name(strings.intern("a")),
            ty: number_ty,
            is_optional: false,
            is_readonly: false,
        };
        let optional_field = TypeField {
            key: StaticKey::Name(strings.intern("a")),
            ty: number_ty,
            is_optional: true,
            is_readonly: false,
        };

        let required_obj = types.insert_type(Type::Object {
            fields: vec![required_field],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        });
        let optional_obj = types.insert_type(Type::Object {
            fields: vec![optional_field],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module,
                profile,
                &symbols,
                required_obj,
                optional_obj,
                &mut types,
                &options
            ),
            Assignability::NotAssignable
        );
        assert_eq!(
            test.compiler.is_type_assignable(
                &module,
                profile,
                &symbols,
                optional_obj,
                required_obj,
                &mut types,
                &options
            ),
            Assignability::Assignable
        );
    }

    /// Object call signatures must be assignable.
    #[test]
    fn test_analyze_assignability_object_call_signatures() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let string_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });
        let signature_ty = types.insert_type(Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            static_parameters: Vec::new(),
            this_parameter: None,
            dynamic_parameters: vec![number_ty],
            return_type: Some(string_ty),
        });

        let target_obj = types.insert_type(Type::Object {
            fields: Vec::new(),
            call_signatures: vec![signature_ty],
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        });
        let source_obj = types.insert_type(Type::Object {
            fields: Vec::new(),
            call_signatures: vec![signature_ty],
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        });
        let missing_call = types.insert_type(Type::Object {
            fields: Vec::new(),
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, target_obj, source_obj, &mut types, &options
            ),
            Assignability::Assignable
        );
        assert_eq!(
            test.compiler.is_type_assignable(
                &module,
                profile,
                &symbols,
                target_obj,
                missing_call,
                &mut types,
                &options
            ),
            Assignability::NotAssignable
        );
    }

    /// Functions with fewer parameters are assignable to targets with more parameters.
    #[test]
    fn test_analyze_assignability_function_param_count() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let string_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });

        let target_fn = types.insert_type(Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            static_parameters: Vec::new(),
            this_parameter: None,
            dynamic_parameters: vec![number_ty, string_ty],
            return_type: Some(number_ty),
        });
        let source_fn_fewer = types.insert_type(Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            static_parameters: Vec::new(),
            this_parameter: None,
            dynamic_parameters: vec![number_ty],
            return_type: Some(number_ty),
        });
        let source_fn_more = types.insert_type(Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            static_parameters: Vec::new(),
            this_parameter: None,
            dynamic_parameters: vec![number_ty, string_ty, number_ty],
            return_type: Some(number_ty),
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module,
                profile,
                &symbols,
                target_fn,
                source_fn_fewer,
                &mut types,
                &options
            ),
            Assignability::Assignable
        );
        assert_eq!(
            test.compiler.is_type_assignable(
                &module,
                profile,
                &symbols,
                target_fn,
                source_fn_more,
                &mut types,
                &options
            ),
            Assignability::NotAssignable
        );
    }

    /// Functions with this parameters are contravariant in this.
    #[test]
    fn test_analyze_assignability_function_this_parameter() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);
        let strings = test.program.strings.clone();

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });

        let key_a = StaticKey::Name(strings.intern("a"));
        let key_b = StaticKey::Name(strings.intern("b"));

        let this_small = types.insert_type(Type::Object {
            fields: vec![TypeField {
                key: key_a,
                ty: number_ty,
                is_optional: false,
                is_readonly: false,
            }],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        });
        let this_large = types.insert_type(Type::Object {
            fields: vec![
                TypeField {
                    key: key_a,
                    ty: number_ty,
                    is_optional: false,
                    is_readonly: false,
                },
                TypeField {
                    key: key_b,
                    ty: number_ty,
                    is_optional: false,
                    is_readonly: false,
                },
            ],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        });

        let target_fn = types.insert_type(Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            static_parameters: Vec::new(),
            this_parameter: Some(this_small),
            dynamic_parameters: Vec::new(),
            return_type: None,
        });
        let source_fn_wider_this = types.insert_type(Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            static_parameters: Vec::new(),
            this_parameter: Some(this_large),
            dynamic_parameters: Vec::new(),
            return_type: None,
        });
        let source_fn_narrow_this = types.insert_type(Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            static_parameters: Vec::new(),
            this_parameter: Some(this_small),
            dynamic_parameters: Vec::new(),
            return_type: None,
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module,
                profile,
                &symbols,
                target_fn,
                source_fn_wider_this,
                &mut types,
                &options
            ),
            Assignability::NotAssignable
        );
        assert_eq!(
            test.compiler.is_type_assignable(
                &module,
                profile,
                &symbols,
                source_fn_wider_this,
                source_fn_narrow_this,
                &mut types,
                &options
            ),
            Assignability::Assignable
        );
    }

    /// Object index signatures must be assignable.
    #[test]
    fn test_analyze_assignability_object_index_signatures() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);
        let strings = test.program.strings.clone();

        let string_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });
        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let index_signature = TypeIndexSignature {
            name: strings.intern("k"),
            key_type: string_ty,
            value_type: number_ty,
            is_readonly: false,
        };
        let matching_field = TypeField {
            key: StaticKey::Name(strings.intern("a")),
            ty: number_ty,
            is_optional: false,
            is_readonly: false,
        };
        let mismatched_field = TypeField {
            key: StaticKey::Name(strings.intern("a")),
            ty: string_ty,
            is_optional: false,
            is_readonly: false,
        };

        let target_obj = types.insert_type(Type::Object {
            fields: Vec::new(),
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: vec![index_signature.clone()],
        });
        let source_obj = types.insert_type(Type::Object {
            fields: Vec::new(),
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: vec![index_signature.clone()],
        });
        let compatible_fields = types.insert_type(Type::Object {
            fields: vec![matching_field],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        });
        let incompatible_fields = types.insert_type(Type::Object {
            fields: vec![mismatched_field],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        });
        let missing_index = types.insert_type(Type::Object {
            fields: Vec::new(),
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, target_obj, source_obj, &mut types, &options
            ),
            Assignability::Assignable
        );
        assert_eq!(
            test.compiler.is_type_assignable(
                &module,
                profile,
                &symbols,
                target_obj,
                missing_index,
                &mut types,
                &options
            ),
            Assignability::Assignable
        );
        assert_eq!(
            test.compiler.is_type_assignable(
                &module,
                profile,
                &symbols,
                target_obj,
                compatible_fields,
                &mut types,
                &options
            ),
            Assignability::Assignable
        );
        assert_eq!(
            test.compiler.is_type_assignable(
                &module,
                profile,
                &symbols,
                target_obj,
                incompatible_fields,
                &mut types,
                &options
            ),
            Assignability::NotAssignable
        );
    }

    /// String index signatures satisfy number index signatures.
    #[test]
    fn test_analyze_assignability_object_index_signatures_string_source() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);
        let strings = test.program.strings.clone();

        let string_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });
        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let number_index = TypeIndexSignature {
            name: strings.intern("k"),
            key_type: number_ty,
            value_type: number_ty,
            is_readonly: false,
        };
        let string_index = TypeIndexSignature {
            name: strings.intern("k"),
            key_type: string_ty,
            value_type: number_ty,
            is_readonly: false,
        };

        let target_number = types.insert_type(Type::Object {
            fields: Vec::new(),
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: vec![number_index.clone()],
        });
        let source_string = types.insert_type(Type::Object {
            fields: Vec::new(),
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: vec![string_index.clone()],
        });
        let target_string = types.insert_type(Type::Object {
            fields: Vec::new(),
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: vec![string_index],
        });
        let source_number = types.insert_type(Type::Object {
            fields: Vec::new(),
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: vec![number_index],
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module,
                profile,
                &symbols,
                target_number,
                source_string,
                &mut types,
                &options
            ),
            Assignability::Assignable
        );
        assert_eq!(
            test.compiler.is_type_assignable(
                &module,
                profile,
                &symbols,
                target_string,
                source_number,
                &mut types,
                &options
            ),
            Assignability::Assignable
        );
    }

    /// Numeric and string literal keys are compatible for fields.
    #[test]
    fn test_analyze_assignability_object_numeric_key_field_match() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);
        let strings = test.program.strings.clone();

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let key = strings.intern("1");
        let target_field = TypeField {
            key: StaticKey::Name(key),
            ty: number_ty,
            is_optional: false,
            is_readonly: false,
        };
        let source_field = TypeField {
            key: StaticKey::Number(key),
            ty: number_ty,
            is_optional: false,
            is_readonly: false,
        };

        let target_obj = types.insert_type(Type::Object {
            fields: vec![target_field],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        });
        let source_obj = types.insert_type(Type::Object {
            fields: vec![source_field],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, target_obj, source_obj, &mut types, &options
            ),
            Assignability::Assignable
        );
    }

    /// Optional fields ignore undefined when matching index signatures.
    #[test]
    fn test_analyze_assignability_object_index_signature_optional_field() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);
        let strings = test.program.strings.clone();

        let string_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });
        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let undefined_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Undefined,
        });
        let target_index = TypeIndexSignature {
            name: strings.intern("k"),
            key_type: string_ty,
            value_type: number_ty,
            is_readonly: false,
        };
        let optional_number_field = TypeField {
            key: StaticKey::Name(strings.intern("a")),
            ty: number_ty,
            is_optional: true,
            is_readonly: false,
        };
        let optional_undefined_field = TypeField {
            key: StaticKey::Name(strings.intern("a")),
            ty: undefined_ty,
            is_optional: true,
            is_readonly: false,
        };

        let target_obj = types.insert_type(Type::Object {
            fields: Vec::new(),
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: vec![target_index],
        });
        let source_optional_number = types.insert_type(Type::Object {
            fields: vec![optional_number_field],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        });
        let source_optional_undefined = types.insert_type(Type::Object {
            fields: vec![optional_undefined_field],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module,
                profile,
                &symbols,
                target_obj,
                source_optional_number,
                &mut types,
                &options
            ),
            Assignability::Assignable
        );
        assert_eq!(
            test.compiler.is_type_assignable(
                &module,
                profile,
                &symbols,
                target_obj,
                source_optional_undefined,
                &mut types,
                &options
            ),
            Assignability::NotAssignable
        );
    }

    /// Number is assignable to number | string.
    #[test]
    fn test_analyze_assignability_union_target() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let string_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });
        let union_ty = types.insert_type(Type::Union {
            elements: vec![number_ty, string_ty],
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, union_ty, number_ty, &mut types, &options
            ),
            Assignability::Assignable
        );
    }

    /// Number literal should be assignable to number type.
    #[test]
    fn test_type_check_let_compatible_types() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "let x: number = 42;");
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// String literal should not be assignable to number type.
    #[test]
    fn test_type_check_let_incompatible_types() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", r#"let x: number = "hello";"#);
        test.analyze_module(module_id);
        test.compile();
        test.check_diagnostics(&["EA004"]);
    }

    /// Excess properties on object literals should error.
    #[test]
    fn test_type_check_excess_property_object_literal() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "let x: { a: number } = { a: 1, b: 2 };");
        test.analyze_module(module_id);
        test.compile();
        test.check_diagnostics(&["EA036"]);
    }

    /// Excess property diagnostics should anchor to the object literal span.
    #[test]
    fn test_type_check_excess_property_anchor() {
        let test = TestProgram::memory_sequential();
        let source = "let x: { a: number } = { a: 1, b: 2 };";
        let module_id = test.add_module("test.ds", source);
        test.analyze_module(module_id);
        test.compile();

        let diagnostics = test.program.diagnostics.collect();
        let diagnostic_vec = diagnostics.iter();
        let diagnostic = diagnostic_vec
            .iter()
            .find(|diag| diag.code == "EA036")
            .unwrap_or_else(|| panic!("expected diagnostic EA036"));

        let file = test.file(module_id);
        let content = match &file.content {
            FileContent::Text { content } => content,
            _ => panic!("expected text file content"),
        };
        let literal = "{ a: 1, b: 2 }";
        let start = content
            .find(literal)
            .unwrap_or_else(|| panic!("missing object literal in source"));
        let end = start + literal.len();
        let expected_span = Span::new(file.id, start as u32, end as u32);

        assert_eq!(diagnostic.file_id, file.id);
        assert!(
            diagnostic.primary_span.span.intersects(expected_span),
            "diagnostic span should intersect object literal span"
        );
    }

    /// Excess property diagnostics should be reported for each extra field.
    #[test]
    fn test_type_check_excess_property_multiple() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "let x: { a: number } = { a: 1, b: 2, c: 3 };");
        test.analyze_module(module_id);
        test.compile();

        test.check_diagnostic_count("EA036", 2);
    }

    /// Excess property checks should respect union candidates.
    #[test]
    fn test_type_check_excess_property_union_candidate() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            "let x: { a: number } | { a: number, b: number } = { a: 1, b: 2 };",
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Index signatures allow extra object literal properties.
    #[test]
    fn test_type_check_excess_property_index_signature() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            "let x: { [key: string]: number } = { a: 1, b: 2 };",
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Boolean literal should not be assignable to string type.
    #[test]
    fn test_type_check_let_boolean_to_string_error() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "let x: string = true;");
        test.analyze_module(module_id);
        test.compile();
        test.check_diagnostics(&["EA004"]);
    }

    /// Anything should be assignable to any.
    #[test]
    fn test_type_check_let_any_accepts_all() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "let x: any = 42;");
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Function call with compatible argument types.
    #[test]
    fn test_type_check_function_call_compatible_args() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function add(x: number, y: number): number {
    return x + y;
}
let result = add(1, 2);
"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Function call with incompatible argument types.
    #[test]
    fn test_type_check_function_call_incompatible_args() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function greet(name: string): string {
    return name;
}
let result = greet(42);
"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_diagnostics(&["EA004"]);
    }

    /// Function with declared return type should have that type.
    #[test]
    fn test_type_check_function_return_type() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function getNumber(): number {
    return 42;
}
let x: number = getNumber();
"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Integer literal should be assignable to int type.
    #[test]
    fn test_type_check_let_int_compatible() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "let x: int = 4;");
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Integer literal should be assignable to int32 type.
    #[test]
    fn test_type_check_let_int32_compatible() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "let x: int32 = 42;");
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Float literal should be assignable to float type.
    #[test]
    fn test_type_check_let_float_compatible() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "let x: float = 3.14;");
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Integer literal should be assignable to float type (implicit conversion).
    #[test]
    fn test_type_check_let_int_to_float_compatible() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "let x: float = 42;");
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Value within int8 range should be valid.
    #[test]
    fn test_type_check_int8_range_valid() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "let x: int8 = 127;");
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Value outside int8 range should fail.
    #[test]
    fn test_type_check_int8_range_overflow() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "let x: int8 = 128;");
        test.analyze_module(module_id);
        test.compile();
        test.check_diagnostics(&["EA004"]);
    }

    /// Value within uint8 range should be valid.
    #[test]
    fn test_type_check_uint8_range_valid() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "let x: uint8 = 255;");
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Value outside uint8 range should fail.
    #[test]
    fn test_type_check_uint8_range_overflow() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "let x: uint8 = 256;");
        test.analyze_module(module_id);
        test.compile();
        test.check_diagnostics(&["EA004"]);
    }

    /// Negative value should not be assignable to unsigned type.
    /// Now works because we have constant folding for unary negation.
    #[test]
    fn test_type_check_uint_negative_fails() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "let x: uint = -1;");
        test.analyze_module(module_id);
        test.compile();
        test.check_diagnostics(&["EA004"]);
    }

    /// Integer literal 42 is assignable to int32.
    #[test]
    fn test_analyze_assignability_literal_to_int() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);

        let int_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32)),
        });
        let literal_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(42)),
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, int_ty, literal_ty, &mut types, &options
            ),
            Assignability::Assignable
        );
    }

    /// Float literal 3.14 is assignable to float64.
    #[test]
    fn test_analyze_assignability_literal_to_float() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);

        let float_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Float(FloatType::Float64)),
        });
        let literal_ty = types.insert_type(Type::TypeLiteral {
            #[allow(clippy::approx_constant)]
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Float(3.14f64)),
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, float_ty, literal_ty, &mut types, &options
            ),
            Assignability::Assignable
        );
    }

    /// Integer literal 1000 is not assignable to int8 (range: minus 128 to 127).
    #[test]
    fn test_analyze_assignability_int_out_of_range() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);

        let int8_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int8)),
        });
        let literal_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1000)),
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, int8_ty, literal_ty, &mut types, &options
            ),
            Assignability::NotAssignable
        );
    }

    /// int8 widens to int16, but not vice versa.
    #[test]
    fn test_analyze_numeric_widening_int() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);

        let int8_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int8)),
        });
        let int16_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int16)),
        });

        // widening allowed
        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, int16_ty, int8_ty, &mut types, &options
            ),
            Assignability::Assignable
        );
        // narrowing not allowed
        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, int8_ty, int16_ty, &mut types, &options
            ),
            Assignability::NotAssignable
        );
    }

    /// Signed integers cannot widen to unsigned (may lose negative values).
    #[test]
    fn test_analyze_numeric_widening_signed_to_unsigned_not_allowed() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let profile = test.default_profile_id(module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = test.compiler.analyze_context_options_for_module(module.id);

        let int8_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int8)),
        });
        let uint8_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Uint8)),
        });

        assert_eq!(
            test.compiler.is_type_assignable(
                &module, profile, &symbols, uint8_ty, int8_ty, &mut types, &options
            ),
            Assignability::NotAssignable
        );
    }
    /// Verify lineage chain for multilevel inheritance.
    #[test]
    fn test_analyze_lineage_chain_multilevel() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
class Animal { name: string }
class Dog extends Animal { breed: string }
class Labrador extends Dog { color: string }
"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();

        let animal_id = test.resolve_to_symbol("test.ds", "Animal").unwrap();
        let dog_id = test.resolve_to_symbol("test.ds", "Dog").unwrap();
        let labrador_id = test.resolve_to_symbol("test.ds", "Labrador").unwrap();

        let module = test.module("test.ds");
        let module = module.read();
        let types = module.dir(test.default_profile_id(module_id)).types.read();

        // labrador extends dog
        let labrador_lineage = types
            .get_lineage_for_symbol(labrador_id)
            .expect("Labrador should have lineage");
        assert_eq!(labrador_lineage.extends, Some(dog_id));

        // dog extends animal
        let dog_lineage = types
            .get_lineage_for_symbol(dog_id)
            .expect("Dog should have lineage");
        assert_eq!(dog_lineage.extends, Some(animal_id));

        // animal has no extends (or empty lineage)
        let animal_lineage = types.get_lineage_for_symbol(animal_id);
        assert!(
            animal_lineage.is_none() || animal_lineage.unwrap().extends.is_none(),
            "Animal should not extend anything"
        );
    }

    /// Verify implements creates lineage entries.
    #[test]
    fn test_analyze_lineage_created_for_implements() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
interface Printable { print(): void }
interface Saveable { save(): void }
class Document implements Printable, Saveable {
    print(): void {}
    save(): void {}
}
"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();

        let printable_id = test.resolve_to_symbol("test.ds", "Printable").unwrap();
        let saveable_id = test.resolve_to_symbol("test.ds", "Saveable").unwrap();
        let document_id = test.resolve_to_symbol("test.ds", "Document").unwrap();

        let module = test.module("test.ds");
        let module = module.read();
        let types = module.dir(test.default_profile_id(module_id)).types.read();
        let doc_lineage = types
            .get_lineage_for_symbol(document_id)
            .expect("Document should have lineage");
        assert!(
            doc_lineage.extends.is_none(),
            "Document should not extend anything"
        );
        assert_eq!(
            doc_lineage.implements.len(),
            2,
            "Document should implement two interfaces"
        );
        assert!(
            doc_lineage.implements.contains(&printable_id),
            "Document should implement Printable"
        );
        assert!(
            doc_lineage.implements.contains(&saveable_id),
            "Document should implement Saveable"
        );
    }
}
