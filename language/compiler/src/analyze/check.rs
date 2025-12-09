use destack_dir::{LocalTypeId, PrimitiveType, Type, TypeLiteral, TypeTable};

use crate::Compiler;

/// Result of a type assignability check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assignability {
    /// Types are assignable
    Assignable,
    /// Types are not assignable
    NotAssignable,
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

            // references: same symbol required (nominal typing)
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
                if target_symbol == source_symbol {
                    Assignability::Assignable
                } else {
                    Assignability::NotAssignable
                }
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

            // primitives: must be the same
            (TypeLiteral::Primitive(target_prim), TypeLiteral::Primitive(source_prim)) => {
                if target_prim == source_prim {
                    Assignability::Assignable
                } else {
                    // NOTE #Incomplete: numeric widening (i32 -> i64, ..?)?
                    Assignability::NotAssignable
                }
            }

            // scalar literal to primitive: check if literal is of that primitive type
            (TypeLiteral::Primitive(prim), TypeLiteral::ScalarLiteral(lit)) => {
                if self.scalar_literal_matches_primitive(lit, prim) {
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
    fn scalar_literal_matches_primitive(
        &self,
        lit: &destack_dir::ScalarLiteral,
        prim: &PrimitiveType,
    ) -> bool {
        use destack_dir::ScalarLiteral;
        match (lit, prim) {
            (ScalarLiteral::Boolean(_), PrimitiveType::Boolean) => true,
            (ScalarLiteral::String(_), PrimitiveType::String) => true,
            (ScalarLiteral::Integer(_), PrimitiveType::Number) => true,
            (ScalarLiteral::Float(_), PrimitiveType::Number) => true,
            // NOTE #Incomplete: specific numeric types (i32, f64, etc)
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
}

#[cfg(test)]
mod tests {
    use destack_dir::{PrimitiveType, ScalarLiteral, Type, TypeField, TypeLiteral};

    use crate::{Assignability, TestProgram};

    #[test]
    fn test_analyze_assignability_same_primitive() {
        // number is assignable to number
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

    #[test]
    fn test_analyze_assignability_different_primitives() {
        // string is not assignable to number
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

    #[test]
    fn test_analyze_assignability_literal_to_primitive() {
        // literal 42 is assignable to number
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

    #[test]
    fn test_analyze_assignability_any() {
        // anything is assignable to any
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

    #[test]
    fn test_analyze_assignability_never_source() {
        // never is assignable to anything (bottom type)
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

    #[test]
    fn test_analyze_assignability_never_target() {
        // nothing is assignable to never (except never itself)
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

    #[test]
    fn test_analyze_assignability_tuple() {
        // [number, string] is assignable to [number, string]
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

    #[test]
    fn test_analyze_assignability_tuple_different_length() {
        // [number, string] is not assignable to [number]
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

    #[test]
    fn test_analyze_assignability_array() {
        // number[] is assignable to number[]
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

    #[test]
    fn test_analyze_assignability_tuple_to_array() {
        // [number, number] is assignable to number[]
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

    #[test]
    fn test_analyze_assignability_object_structural() {
        // { a: number, b: string } is assignable to { a: number }
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

    #[test]
    fn test_analyze_assignability_union_target() {
        // number is assignable to number | string
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

    #[test]
    fn test_type_check_let_compatible_types() {
        // number literal should be assignable to number type
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "let x: number = 42;");
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    #[test]
    fn test_type_check_let_incompatible_types() {
        // string literal should not be assignable to number type
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", r#"let x: number = "hello";"#);
        test.analyze_module(module_id);
        test.compile();
        test.check_diagnostics(&["EA004"]);
    }

    #[test]
    fn test_type_check_let_boolean_to_string_error() {
        // boolean literal should not be assignable to string type
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "let x: string = true;");
        test.analyze_module(module_id);
        test.compile();
        test.check_diagnostics(&["EA004"]);
    }

    #[test]
    fn test_type_check_let_any_accepts_all() {
        // anything should be assignable to any
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "let x: any = 42;");
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    #[test]
    fn test_type_check_function_call_compatible_args() {
        // function call with compatible argument types
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

    #[test]
    fn test_type_check_function_call_incompatible_args() {
        // function call with incompatible argument types
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

    #[test]
    fn test_type_check_function_return_type() {
        // function with declared return type should have that type
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
}
