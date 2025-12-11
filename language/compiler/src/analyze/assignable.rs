use destack_dir::{
    GlobalSymbolId, IntType, LocalTypeId, PrimitiveType, ScalarLiteral, Type, TypeLiteral,
    TypeTable,
};

use crate::Compiler;

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

impl Compiler {
    /// Check if `source` type is assignable to `target` type.
    /// Returns true if a value of type `source` can be assigned to a location of type `target`.
    pub fn check_is_type_assignable(
        &self,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
        types: &TypeTable,
    ) -> Assignability {
        // same type id: trivially assignable
        if target_id == source_id {
            return Assignability::Assignable;
        }

        let target = types.get_type(target_id);
        let source = types.get_type(source_id);

        self.is_type_assignable(target, source, types)
    }

    /// Inner assignability check on Type values.
    fn is_type_assignable(&self, target: &Type, source: &Type, types: &TypeTable) -> Assignability {
        // handle special target types first
        match target {
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
        match source {
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
                self.is_type_literal_assignable(target_lit, source_lit)
            }

            // arrays: covariant in element type
            (
                Type::Array {
                    element: Some(target_elem),
                },
                Type::Array {
                    element: Some(source_elem),
                },
            ) => self.check_is_type_assignable(*target_elem, *source_elem, types),

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
                        .check_is_type_assignable(*target_elem, *source_elem, types)
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
                        .check_is_type_assignable(*target_elem, *source_elem, types)
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
                },
                Type::Object {
                    fields: source_fields,
                },
            ) => self.is_object_type_assignable(target_fields, source_fields, types),

            // functions: contravariant params, covariant return
            (
                Type::Function {
                    dynamic_parameters: target_params,
                    return_type: target_return,
                    ..
                },
                Type::Function {
                    dynamic_parameters: source_params,
                    return_type: source_return,
                    ..
                },
            ) => self.is_function_type_assignable(
                target_params,
                target_return,
                source_params,
                source_return,
                types,
            ),

            // union target: source must be assignable to at least one element
            (
                Type::Union {
                    elements: target_elems,
                },
                _,
            ) => {
                for target_elem in target_elems {
                    let target_elem_ty = types.get_type(*target_elem);
                    if self
                        .is_type_assignable(target_elem_ty, source, types)
                        .is_assignable()
                    {
                        return Assignability::Assignable;
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
                    let source_elem_ty = types.get_type(*source_elem);
                    if !self
                        .is_type_assignable(target, source_elem_ty, types)
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
                    let target_elem_ty = types.get_type(*target_elem);
                    if !self
                        .is_type_assignable(target_elem_ty, source, types)
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
                    let source_elem_ty = types.get_type(*source_elem);
                    if self
                        .is_type_assignable(target, source_elem_ty, types)
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
                    || self.is_type_lineage_assignable(*source_symbol, *target_symbol, types)
                {
                    return Assignability::Assignable;
                }

                // structural check: only for interfaces
                if target_symbol.ty().is_interface()
                    && let (Some(target_instance_ty), Some(source_instance_ty)) = (
                        types.get_instance_type(*target_symbol),
                        types.get_instance_type(*source_symbol),
                    )
                    && let (
                        Type::Object {
                            fields: target_fields,
                        },
                        Type::Object {
                            fields: source_fields,
                        },
                    ) = (target_instance_ty, source_instance_ty)
                {
                    return self.is_object_type_assignable(target_fields, source_fields, types);
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
            IntType::IntP | IntType::UintP => {
                // pointer-sized integers: use target platform's pointer size
                // for now, assume 64-bit
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

            // float widening: float32 -> float64
            (PrimitiveType::Float(target_float), PrimitiveType::Float(source_float)) => {
                target_float.width() >= source_float.width()
            }

            // int to float: always allowed (may lose precision for large ints)
            (PrimitiveType::Float(_), PrimitiveType::Int(_)) => true,

            // signed int widening: int8 -> int16 -> int32 -> int64 -> int128
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
                    // pointer-sized ints: only allow same signedness
                    _ => false,
                }
            }

            _ => false,
        }
    }

    /// Check object type assignability (structural subtyping).
    fn is_object_type_assignable(
        &self,
        target_fields: &[destack_dir::TypeField],
        source_fields: &[destack_dir::TypeField],
        types: &TypeTable,
    ) -> Assignability {
        // for each target field, find matching source field
        for target_field in target_fields {
            let source_field = source_fields.iter().find(|f| f.key == target_field.key);

            match source_field {
                Some(source_field) => {
                    // field exists: check type assignability
                    if !self
                        .check_is_type_assignable(target_field.ty, source_field.ty, types)
                        .is_assignable()
                    {
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

    /// Check function type assignability (contravariant params, covariant return).
    fn is_function_type_assignable(
        &self,
        target_params: &[LocalTypeId],
        target_return: &Option<LocalTypeId>,
        source_params: &[LocalTypeId],
        source_return: &Option<LocalTypeId>,
        types: &TypeTable,
    ) -> Assignability {
        // parameter count must match (for now, no optional params handling)
        if target_params.len() != source_params.len() {
            return Assignability::NotAssignable;
        }

        // parameters: contravariant (source param must be assignable to target param)
        for (target_param, source_param) in target_params.iter().zip(source_params.iter()) {
            if !self
                .check_is_type_assignable(*source_param, *target_param, types)
                .is_assignable()
            {
                return Assignability::NotAssignable;
            }
        }

        // return type: covariant (target return must be assignable from source return)
        match (target_return, source_return) {
            (Some(target_ret), Some(source_ret)) => {
                self.check_is_type_assignable(*target_ret, *source_ret, types)
            }
            (None, _) => Assignability::Assignable,
            (Some(_), None) => Assignability::NotAssignable,
        }
    }

    /// Check if source_symbol is a subtype of target_symbol via lineage (follows inheritance chain).
    /// This also checks visible extensions that add `implements` clauses to the source type.
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
        ExtensionKind, FloatType, IntType, PrimitiveType, ScalarLiteral, Type, TypeField,
        TypeLiteral,
    };

    use crate::{Assignability, TestProgram};

    /// Number is assignable to number.
    #[test]
    fn test_analyze_assignability_same_primitive() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "let x: number = 42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });

        assert_eq!(
            test.compiler
                .check_is_type_assignable(number_ty, number_ty, &types),
            Assignability::Assignable
        );
    }

    /// String is not assignable to number.
    #[test]
    fn test_analyze_assignability_different_primitives() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let string_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });

        assert_eq!(
            test.compiler
                .check_is_type_assignable(number_ty, string_ty, &types),
            Assignability::NotAssignable
        );
    }

    /// Literal 42 is assignable to number.
    #[test]
    fn test_analyze_assignability_literal_to_primitive() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let literal_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(42)),
        });

        assert_eq!(
            test.compiler
                .check_is_type_assignable(number_ty, literal_ty, &types),
            Assignability::Assignable
        );
    }

    /// Anything is assignable to any.
    #[test]
    fn test_analyze_assignability_any() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

        let any_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Any,
        });
        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });

        assert_eq!(
            test.compiler
                .check_is_type_assignable(any_ty, number_ty, &types),
            Assignability::Assignable
        );
    }

    /// Never is assignable to anything (bottom type).
    #[test]
    fn test_analyze_assignability_never_source() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

        let never_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Never,
        });
        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });

        assert_eq!(
            test.compiler
                .check_is_type_assignable(number_ty, never_ty, &types),
            Assignability::Assignable
        );
    }

    /// Nothing is assignable to never (except never itself).
    #[test]
    fn test_analyze_assignability_never_target() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

        let never_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Never,
        });
        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });

        assert_eq!(
            test.compiler
                .check_is_type_assignable(never_ty, number_ty, &types),
            Assignability::NotAssignable
        );
    }

    /// [number, string] is assignable to [number, string].
    #[test]
    fn test_analyze_assignability_tuple() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let string_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });
        let tuple_ty = types.insert_type(Type::Tuple {
            elements: vec![number_ty, string_ty],
        });

        assert_eq!(
            test.compiler
                .check_is_type_assignable(tuple_ty, tuple_ty, &types),
            Assignability::Assignable
        );
    }

    /// [number, string] is not assignable to [number].
    #[test]
    fn test_analyze_assignability_tuple_different_length() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let string_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });
        let tuple_short = types.insert_type(Type::Tuple {
            elements: vec![number_ty],
        });
        let tuple_long = types.insert_type(Type::Tuple {
            elements: vec![number_ty, string_ty],
        });

        assert_eq!(
            test.compiler
                .check_is_type_assignable(tuple_short, tuple_long, &types),
            Assignability::NotAssignable
        );
    }

    /// number[] is assignable to number[].
    #[test]
    fn test_analyze_assignability_array() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let array_ty = types.insert_type(Type::Array {
            element: Some(number_ty),
        });

        assert_eq!(
            test.compiler
                .check_is_type_assignable(array_ty, array_ty, &types),
            Assignability::Assignable
        );
    }

    /// [number, number] is assignable to number[].
    #[test]
    fn test_analyze_assignability_tuple_to_array() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let tuple_ty = types.insert_type(Type::Tuple {
            elements: vec![number_ty, number_ty],
        });
        let array_ty = types.insert_type(Type::Array {
            element: Some(number_ty),
        });

        assert_eq!(
            test.compiler
                .check_is_type_assignable(array_ty, tuple_ty, &types),
            Assignability::Assignable
        );
    }

    /// { a: number, b: string } is assignable to { a: number }.
    #[test]
    fn test_analyze_assignability_object_structural() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();
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
        });

        // larger object assignable to smaller (has all required fields)
        assert_eq!(
            test.compiler
                .check_is_type_assignable(obj_small, obj_large, &types),
            Assignability::Assignable
        );

        // smaller object not assignable to larger (missing field b)
        assert_eq!(
            test.compiler
                .check_is_type_assignable(obj_large, obj_small, &types),
            Assignability::NotAssignable
        );
    }

    /// Number is assignable to number | string.
    #[test]
    fn test_analyze_assignability_union_target() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

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
            test.compiler
                .check_is_type_assignable(union_ty, number_ty, &types),
            Assignability::Assignable
        );
    }

    /// Number literal should be assignable to number type.
    #[test]
    fn test_type_check_let_compatible_types() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "let x: number = 42;");
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// String literal should not be assignable to number type.
    #[test]
    fn test_type_check_let_incompatible_types() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", r#"let x: number = "hello";"#);
        test.analyze_module(module_id);
        test.compile();
        test.check_diagnostics(&["EA004"]);
    }

    /// Boolean literal should not be assignable to string type.
    #[test]
    fn test_type_check_let_boolean_to_string_error() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "let x: string = true;");
        test.analyze_module(module_id);
        test.compile();
        test.check_diagnostics(&["EA004"]);
    }

    /// Anything should be assignable to any.
    #[test]
    fn test_type_check_let_any_accepts_all() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "let x: any = 42;");
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Function call with compatible argument types.
    #[test]
    fn test_type_check_function_call_compatible_args() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module(
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
        let module_id = test.register_module(
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
        let module_id = test.register_module(
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
        let module_id = test.register_module("test.ds", "let x: int = 4;");
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Integer literal should be assignable to int32 type.
    #[test]
    fn test_type_check_let_int32_compatible() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "let x: int32 = 42;");
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Float literal should be assignable to float type.
    #[test]
    fn test_type_check_let_float_compatible() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "let x: float = 3.14;");
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Integer literal should be assignable to float type (implicit conversion).
    #[test]
    fn test_type_check_let_int_to_float_compatible() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "let x: float = 42;");
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Value within int8 range should be valid.
    #[test]
    fn test_type_check_int8_range_valid() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "let x: int8 = 127;");
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Value outside int8 range should fail.
    #[test]
    fn test_type_check_int8_range_overflow() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "let x: int8 = 128;");
        test.analyze_module(module_id);
        test.compile();
        test.check_diagnostics(&["EA004"]);
    }

    /// Value within uint8 range should be valid.
    #[test]
    fn test_type_check_uint8_range_valid() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "let x: uint8 = 255;");
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Value outside uint8 range should fail.
    #[test]
    fn test_type_check_uint8_range_overflow() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "let x: uint8 = 256;");
        test.analyze_module(module_id);
        test.compile();
        test.check_diagnostics(&["EA004"]);
    }

    /// Negative value should not be assignable to unsigned type.
    /// Now works because we have constant folding for unary negation.
    #[test]
    fn test_type_check_uint_negative_fails() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "let x: uint = -1;");
        test.analyze_module(module_id);
        test.compile();
        test.check_diagnostics(&["EA004"]);
    }

    /// Integer literal 42 is assignable to int32.
    #[test]
    fn test_analyze_assignability_literal_to_int() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

        let int_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32)),
        });
        let literal_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(42)),
        });

        assert_eq!(
            test.compiler
                .check_is_type_assignable(int_ty, literal_ty, &types),
            Assignability::Assignable
        );
    }

    /// Float literal 3.14 is assignable to float64.
    #[test]
    fn test_analyze_assignability_literal_to_float() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

        let float_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Float(FloatType::Float64)),
        });
        let literal_ty = types.insert_type(Type::TypeLiteral {
            #[allow(clippy::approx_constant)]
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Float(3.14f64)),
        });

        assert_eq!(
            test.compiler
                .check_is_type_assignable(float_ty, literal_ty, &types),
            Assignability::Assignable
        );
    }

    /// Integer literal 1000 is not assignable to int8 (range: -128 to 127).
    #[test]
    fn test_analyze_assignability_int_out_of_range() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

        let int8_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int8)),
        });
        let literal_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1000)),
        });

        assert_eq!(
            test.compiler
                .check_is_type_assignable(int8_ty, literal_ty, &types),
            Assignability::NotAssignable
        );
    }

    /// Resolve member access on object literal to field type.
    #[test]
    fn test_analyze_member_access_object_field() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module(
            "test.ds",
            r#"
let obj = { x: 42, y: "hello" };
let a = obj.x;
let b = obj.y;
"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Resolve member access across multiple fields.
    #[test]
    fn test_analyze_member_access_multiple_fields() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module(
            "test.ds",
            r#"
let obj = { x: 42, y: "hello", z: true };
let a = obj.x;
let b = obj.y;
let c = obj.z;
"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Resolve chained member access on nested objects.
    #[test]
    fn test_analyze_member_access_chained() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module(
            "test.ds",
            r#"
let obj = { inner: { value: 42 } };
let a = obj.inner.value;
"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// int8 widens to int16, but not vice versa.
    #[test]
    fn test_analyze_numeric_widening_int() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

        let int8_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int8)),
        });
        let int16_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int16)),
        });

        // widening allowed
        assert_eq!(
            test.compiler
                .check_is_type_assignable(int16_ty, int8_ty, &types),
            Assignability::Assignable
        );
        // narrowing not allowed
        assert_eq!(
            test.compiler
                .check_is_type_assignable(int8_ty, int16_ty, &types),
            Assignability::NotAssignable
        );
    }

    /// Signed integers cannot widen to unsigned (may lose negative values).
    #[test]
    fn test_analyze_numeric_widening_signed_to_unsigned_not_allowed() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

        let int8_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int8)),
        });
        let uint8_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Uint8)),
        });

        assert_eq!(
            test.compiler
                .check_is_type_assignable(uint8_ty, int8_ty, &types),
            Assignability::NotAssignable
        );
    }

    /// Verify lineage is created for classes with extends.
    #[test]
    fn test_analyze_lineage_created_for_class_extends() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module(
            "test.ds",
            r#"
class Animal { name: string }
class Dog extends Animal { breed: string }
"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();

        let animal_id = test.resolve_to_symbol("test.ds", "Animal").unwrap();
        let dog_id = test.resolve_to_symbol("test.ds", "Dog").unwrap();

        let module = test.module("test.ds");
        let module = module.read();
        let types = module.dir.types.read();
        let lineage = types
            .get_lineage_for_symbol(dog_id)
            .expect("Dog should have lineage");
        assert_eq!(lineage.extends, Some(animal_id), "Dog should extend Animal");
    }

    /// Verify lineage chain for multi-level inheritance.
    #[test]
    fn test_analyze_lineage_chain_multilevel() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module(
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
        let types = module.dir.types.read();

        // Labrador extends Dog
        let labrador_lineage = types
            .get_lineage_for_symbol(labrador_id)
            .expect("Labrador should have lineage");
        assert_eq!(labrador_lineage.extends, Some(dog_id));

        // Dog extends Animal
        let dog_lineage = types
            .get_lineage_for_symbol(dog_id)
            .expect("Dog should have lineage");
        assert_eq!(dog_lineage.extends, Some(animal_id));

        // Animal has no extends (or empty lineage)
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
        let module_id = test.register_module(
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
        let types = module.dir.types.read();
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

    /// Verify extensions are registered in TypeTable with correct methods.
    #[test]
    fn test_analyze_extension() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module(
            "test.ds",
            r#"
struct Point { x: number, y: number }

extension Point {
    magnitude(): number { return 0 }
}
"#,
        );
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let point_id = test.resolve_to_symbol("test.ds", "Point").unwrap();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let types = module.dir.types.read();

        // check extension for Point
        let extension_ids = types
            .get_extensions_for_target(point_id)
            .expect("Point should have extensions");
        assert_eq!(extension_ids.len(), 1, "Point should have one extension");
        let extension = types.get_extension(extension_ids[0]);
        assert_eq!(extension.kind, ExtensionKind::Inherent);
        assert_eq!(extension.target, point_id);
    }
}
