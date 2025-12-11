use destack_dir::{
    Asynchrony, DeclarationType, GlobalSymbolId, GlobalTypeId, LocalTypeId, Mutability,
    PrimitiveType, ScalarLiteral, StaticArgument, StaticExpression, StaticKey, Type,
    TypeBinaryOperator, TypeLiteral, TypeTable, TypeUnaryOperator,
};
use destack_workspace::Program;

/// Format a global type id as a human-readable string.
pub fn format_global_type(ty_id: GlobalTypeId, program: &Program) -> String {
    let module = program.modules.get(ty_id.module_id);
    let module = module.read();
    let types = module.dir.types.read();
    let ty = types.get_type(ty_id.local_id);
    format_type(ty, &types, program)
}

/// Format a type by its id.
pub fn format_local_type(ty_id: LocalTypeId, types: &TypeTable, program: &Program) -> String {
    let ty = types.get_type(ty_id);
    format_type(ty, types, program)
}

/// Format a type as a human-readable string.
pub fn format_type(ty: &Type, types: &TypeTable, program: &Program) -> String {
    match ty {
        Type::TypeLiteral { value } => format_type_literal(value, program),
        Type::Value { value } => {
            format!("type {}", format_local_type(*value, types, program))
        }
        Type::Reference {
            symbol,
            static_arguments,
        } => format_type_reference(*symbol, static_arguments.as_deref(), program),
        Type::Unevaluated(_) => "<unevaluated>".to_string(),
        Type::Unary { operator, right } => format_type_unary(*operator, *right, types, program),
        Type::Mutable { mutability, right } => {
            let right_str = format_local_type(*right, types, program);
            match mutability {
                Mutability::Immutable => right_str,
                Mutability::Mutable => format!("var {right_str}"),
            }
        }
        Type::ValueOf {
            mutability,
            variance,
            right,
        } => {
            let right_str = format_local_type(*right, types, program);
            let mut result = String::from("^");
            if let Some(m) = mutability {
                result.push_str(match m {
                    Mutability::Mutable => "var ",
                    Mutability::Immutable => "",
                });
            }
            if let Some(v) = variance {
                result.push_str(&format!("{v:?} ").to_lowercase());
            }
            result.push_str(&right_str);
            result
        }
        Type::ReferenceOf {
            mutability,
            variance,
            right,
        } => {
            let right_str = format_local_type(*right, types, program);
            let mut result = String::from("&");
            if let Some(m) = mutability {
                result.push_str(match m {
                    Mutability::Mutable => "var ",
                    Mutability::Immutable => "",
                });
            }
            if let Some(v) = variance {
                result.push_str(&format!("{v:?} ").to_lowercase());
            }
            result.push_str(&right_str);
            result
        }
        Type::Binary {
            left,
            operator,
            right,
        } => format_type_binary(*left, *operator, *right, types, program),
        Type::ArraySized { element, count: _ } => {
            let elem_str = format_local_type(*element, types, program);
            format!("{elem_str}[]")
        }
        Type::Array { element } => {
            if let Some(elem) = element {
                let elem_str = format_local_type(*elem, types, program);
                format!("{elem_str}[]")
            } else {
                "[]".to_string()
            }
        }
        Type::Tuple { elements } => {
            let elems: Vec<_> = elements
                .iter()
                .map(|e| format_local_type(*e, types, program))
                .collect();
            format!("({})", elems.join(", "))
        }
        Type::Object { fields } => {
            if fields.is_empty() {
                "{}".to_string()
            } else {
                let field_strs: Vec<_> = fields
                    .iter()
                    .map(|f| {
                        let key = format_static_key(&f.key, program);
                        let ty = format_local_type(f.ty, types, program);
                        let opt = if f.is_optional { "?" } else { "" };
                        let readonly = if f.is_readonly { "readonly " } else { "" };
                        format!("{readonly}{key}{opt}: {ty}")
                    })
                    .collect();
                format!("{{ {} }}", field_strs.join(", "))
            }
        }
        Type::Function {
            asynchrony,
            cardinality: _,
            static_parameters,
            dynamic_parameters,
            return_type,
        } => {
            let async_str = if *asynchrony == Asynchrony::Async {
                "async "
            } else {
                ""
            };
            let static_params_str = if static_parameters.is_empty() {
                String::new()
            } else {
                let params: Vec<_> = static_parameters
                    .iter()
                    .map(|p| format_local_type(*p, types, program))
                    .collect();
                format!("<{}>", params.join(", "))
            };
            let dynamic_params: Vec<_> = dynamic_parameters
                .iter()
                .map(|p| format_local_type(*p, types, program))
                .collect();
            let ret = if let Some(ret_ty) = return_type {
                format!(" -> {}", format_local_type(*ret_ty, types, program))
            } else {
                String::new()
            };
            format!(
                "{async_str}{static_params_str}({}){ret}",
                dynamic_params.join(", ")
            )
        }
        Type::Union { elements } => {
            let elems: Vec<_> = elements
                .iter()
                .map(|e| format_local_type(*e, types, program))
                .collect();
            elems.join(" | ")
        }
        Type::Intersection { elements } => {
            let elems: Vec<_> = elements
                .iter()
                .map(|e| format_local_type(*e, types, program))
                .collect();
            elems.join(" & ")
        }
        Type::Error => "<error>".to_string(),
    }
}

/// Format a TypeLiteral.
pub fn format_type_literal(lit: &TypeLiteral, program: &Program) -> String {
    match lit {
        TypeLiteral::Never => "never".to_string(),
        TypeLiteral::Any => "any".to_string(),
        TypeLiteral::Infer => "_".to_string(),
        TypeLiteral::Undefined => "undefined".to_string(),
        TypeLiteral::Unknown => "unknown".to_string(),
        TypeLiteral::Void => "void".to_string(),
        TypeLiteral::Null => "null".to_string(),
        TypeLiteral::Primitive(p) => format_primitive_type(p),
        TypeLiteral::Composite(c) => format_composite_type(c),
        TypeLiteral::ScalarLiteral(s) => format_scalar_literal(s, program),
    }
}

/// Format a PrimitiveType.
pub fn format_primitive_type(prim: &PrimitiveType) -> String {
    match prim {
        PrimitiveType::Boolean => "boolean".to_string(),
        PrimitiveType::Character => "char".to_string(),
        PrimitiveType::String => "string".to_string(),
        PrimitiveType::Bigint => "bigint".to_string(),
        PrimitiveType::Number => "number".to_string(),
        PrimitiveType::Int(int_type) => int_type.as_str(),
        PrimitiveType::Float(float_type) => float_type.as_str(),
        PrimitiveType::Symbol => "symbol".to_string(),
        PrimitiveType::UniqueSymbol => "unique symbol".to_string(),
    }
}

/// Format a DeclarationType.
pub fn format_composite_type(comp: &DeclarationType) -> String {
    match comp {
        DeclarationType::Type => "type".to_string(),
        DeclarationType::Namespace => "namespace".to_string(),
        DeclarationType::Struct => "struct".to_string(),
        DeclarationType::Class => "class".to_string(),
        DeclarationType::Enum => "enum".to_string(),
        DeclarationType::Union => "union".to_string(),
        DeclarationType::Interface => "interface".to_string(),
        DeclarationType::Extension => "extension".to_string(),
        DeclarationType::Function => "function".to_string(),
    }
}

/// Format a ScalarLiteral.
pub fn format_scalar_literal(scalar: &ScalarLiteral, program: &Program) -> String {
    match scalar {
        ScalarLiteral::Boolean(b) => b.to_string(),
        ScalarLiteral::Integer(i) => i.to_string(),
        ScalarLiteral::Bigint(i) => format!("{i}n"),
        ScalarLiteral::Float(f) => {
            // ensure float has decimal point for clarity
            let s = f.to_string();
            if s.contains('.') || s.contains('e') || s.contains('E') {
                s
            } else {
                format!("{s}.0")
            }
        }
        ScalarLiteral::Character(c) => format!("'{c}'"),
        ScalarLiteral::String(string_id) => {
            let s = &*program.strings.get(*string_id);
            format!("\"{s}\"")
        }
        ScalarLiteral::RegexString { content, flags } => {
            let content_str = &*program.strings.get(*content);
            if let Some(flags_id) = flags {
                let flags_str = &*program.strings.get(*flags_id);
                format!("/{content_str}/{flags_str}")
            } else {
                format!("/{content_str}/")
            }
        }
    }
}

/// Format a type reference (symbol with optional static arguments).
pub fn format_type_reference(
    symbol: GlobalSymbolId,
    static_arguments: Option<&[StaticArgument]>,
    program: &Program,
) -> String {
    let name = format_symbol_name(symbol, program);
    if let Some(arguments) = static_arguments
        && !arguments.is_empty()
    {
        let argument_strs: Vec<_> = arguments
            .iter()
            .map(|argument| format_static_argument(argument, program))
            .collect();
        format!("{name}<{}>", argument_strs.join(", "))
    } else {
        name
    }
}

/// Get the name of a symbol from any module.
pub fn format_symbol_name(symbol_id: GlobalSymbolId, program: &Program) -> String {
    let module = program.modules.get(symbol_id.module_id);
    let module = module.read();
    let symbols = module.dir.symbols.read();
    let symbol = symbols.get_symbol(symbol_id.into_local());
    if let Some(name_id) = symbol.name() {
        program.strings.get(name_id).to_string()
    } else {
        "<anonymous>".to_string()
    }
}

/// Format a StaticKey.
pub fn format_static_key(key: &StaticKey, program: &Program) -> String {
    match key {
        StaticKey::Name(name_id) => program.strings.get(*name_id).to_string(),
        StaticKey::UniqueSymbol(_) => "<unique symbol>".to_string(),
        StaticKey::GlobalSymbol(name_id) => {
            let name = &*program.strings.get(*name_id);
            format!("[Symbol.{name}]")
        }
    }
}

/// Format a StaticArgument.
pub fn format_static_argument(argument: &StaticArgument, program: &Program) -> String {
    match argument {
        StaticArgument::Unevaluated { .. } => "<unevaluated>".to_string(),
        StaticArgument::Evaluated { name, value } => {
            let value_str = format_static_expression(value, program);
            if let Some(name_id) = name {
                let name_str = &*program.strings.get(*name_id);
                format!("{name_str}: {value_str}")
            } else {
                value_str
            }
        }
    }
}

/// Format a StaticExpression.
pub fn format_static_expression(expression: &StaticExpression, program: &Program) -> String {
    match expression {
        StaticExpression::Unevaluated { .. } => "<unevaluated>".to_string(),
        StaticExpression::ScalarLiteral { value } => format_scalar_literal(value, program),
        StaticExpression::TypeLiteral { value } => format_type_literal(value, program),
        StaticExpression::Declaration { .. } => "<declaration>".to_string(),
        StaticExpression::Type { .. } => "<type>".to_string(),
        StaticExpression::RangeExpression {
            start,
            end,
            is_inclusive,
        } => {
            let start_str = format_static_expression(start, program);
            let end_str = format_static_expression(end, program);
            let op = if *is_inclusive { "..=" } else { ".." };
            format!("{start_str}{op}{end_str}")
        }
        StaticExpression::ArrayExpression { elements } => {
            let elems: Vec<_> = elements
                .iter()
                .map(|e| format_static_expression(e, program))
                .collect();
            format!("[{}]", elems.join(", "))
        }
        StaticExpression::TupleExpression { elements } => {
            let elems: Vec<_> = elements
                .iter()
                .map(|e| format_static_expression(e, program))
                .collect();
            format!("({})", elems.join(", "))
        }
        StaticExpression::ObjectExpression { .. } => "{...}".to_string(),
    }
}

/// Format a type unary operator expression.
fn format_type_unary(
    operator: TypeUnaryOperator,
    right: LocalTypeId,
    types: &TypeTable,
    program: &Program,
) -> String {
    let right_str = format_local_type(right, types, program);
    match operator {
        TypeUnaryOperator::Not => format!("!{right_str}"),
        TypeUnaryOperator::Maybe => format!("{right_str}?"),
        TypeUnaryOperator::Must => format!("{right_str}!"),
        TypeUnaryOperator::Newtype => format!("newtype {right_str}"),
        TypeUnaryOperator::Type => format!("type {right_str}"),
        TypeUnaryOperator::Readonly => format!("readonly {right_str}"),
        TypeUnaryOperator::Typeof => format!("typeof {right_str}"),
        TypeUnaryOperator::Keyof => format!("keyof {right_str}"),
        TypeUnaryOperator::Infer => format!("infer {right_str}"),
        TypeUnaryOperator::AsConst => format!("{right_str} as const"),
        TypeUnaryOperator::Asserts => format!("asserts {right_str}"),
    }
}

/// Format a type binary operator expression.
fn format_type_binary(
    left: LocalTypeId,
    operator: TypeBinaryOperator,
    right: LocalTypeId,
    types: &TypeTable,
    program: &Program,
) -> String {
    let left_str = format_local_type(left, types, program);
    let right_str = format_local_type(right, types, program);
    let op_str = match operator {
        TypeBinaryOperator::Cast => "as",
        TypeBinaryOperator::In => "in",
        TypeBinaryOperator::Is => "is",
        TypeBinaryOperator::InstanceOf => "instanceof",
        TypeBinaryOperator::Satisfies => "satisfies",
        TypeBinaryOperator::Extends => "extends",
        TypeBinaryOperator::Implements => "implements",
    };
    format!("{left_str} {op_str} {right_str}")
}

#[cfg(test)]
mod tests {
    use destack_dir::{
        Asynchrony, FloatType, FunctionCardinality, IntType, PrimitiveType, ScalarLiteral, Type,
        TypeLiteral,
    };

    use super::format_type;
    use crate::TestProgram;

    /// Format primitive types as human-readable strings.
    #[test]
    fn test_format_type_primitives() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "declare let a: number");
        test.analyze_module(module_id);

        let types = [
            (PrimitiveType::Boolean, "boolean"),
            (PrimitiveType::Character, "char"),
            (PrimitiveType::String, "string"),
            (PrimitiveType::Number, "number"),
            (PrimitiveType::Bigint, "bigint"),
            (PrimitiveType::Int(IntType::Int32), "int32"),
            (PrimitiveType::Int(IntType::Uint64), "uint64"),
            (PrimitiveType::Float(FloatType::Float64), "float64"),
            (PrimitiveType::Symbol, "symbol"),
            (PrimitiveType::UniqueSymbol, "unique symbol"),
        ];

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let module_types = module.dir.types.read();

        for (primitive, expected) in types {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Primitive(primitive),
            };
            let formatted = format_type(&ty, &module_types, &test.program);
            assert_eq!(formatted, expected);
        }
    }

    /// Format type literals (never, any, void, etc.) as human-readable strings.
    #[test]
    fn test_format_type_literals() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "declare let a: any");
        test.analyze_module(module_id);

        let types = [
            (TypeLiteral::Never, "never"),
            (TypeLiteral::Any, "any"),
            (TypeLiteral::Infer, "_"),
            (TypeLiteral::Undefined, "undefined"),
            (TypeLiteral::Unknown, "unknown"),
            (TypeLiteral::Void, "void"),
            (TypeLiteral::Null, "null"),
        ];

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let module_types = module.dir.types.read();

        for (literal, expected) in types {
            let ty = Type::TypeLiteral { value: literal };
            let formatted = format_type(&ty, &module_types, &test.program);
            assert_eq!(formatted, expected);
        }
    }

    /// Format scalar literal types as human-readable strings.
    #[test]
    fn test_format_scalar_literals() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "declare let a: 42");
        test.analyze_module(module_id);

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let module_types = module.dir.types.read();

        // integer
        let ty = Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(42)),
        };
        assert_eq!(format_type(&ty, &module_types, &test.program), "42");

        // negative integer
        let ty = Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(-123)),
        };
        assert_eq!(format_type(&ty, &module_types, &test.program), "-123");

        // float
        let ty = Type::TypeLiteral {
            #[allow(clippy::approx_constant)]
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Float(3.14)),
        };
        assert_eq!(format_type(&ty, &module_types, &test.program), "3.14");

        // boolean true
        let ty = Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(true)),
        };
        assert_eq!(format_type(&ty, &module_types, &test.program), "true");

        // boolean false
        let ty = Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(false)),
        };
        assert_eq!(format_type(&ty, &module_types, &test.program), "false");

        // bigint
        let ty = Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Bigint(999)),
        };
        assert_eq!(format_type(&ty, &module_types, &test.program), "999n");

        // character
        let ty = Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Character('x')),
        };
        assert_eq!(format_type(&ty, &module_types, &test.program), "'x'");
    }

    /// Format array types as human-readable strings.
    #[test]
    fn test_format_array_types() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "declare let a: number[]");
        test.analyze_module(module_id);

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

        // empty array type
        let empty_array = Type::Array { element: None };
        assert_eq!(format_type(&empty_array, &types, &test.program), "[]");

        // number[]
        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let number_array = Type::Array {
            element: Some(number_ty),
        };
        assert_eq!(
            format_type(&number_array, &types, &test.program),
            "number[]"
        );

        // string[]
        let string_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });
        let string_array = Type::Array {
            element: Some(string_ty),
        };
        assert_eq!(
            format_type(&string_array, &types, &test.program),
            "string[]"
        );
    }

    /// Format tuple types as human-readable strings.
    #[test]
    fn test_format_tuple_types() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "declare let a: (number, string)");
        test.analyze_module(module_id);

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

        // empty tuple
        let empty_tuple = Type::Tuple { elements: vec![] };
        assert_eq!(format_type(&empty_tuple, &types, &test.program), "()");

        // (number)
        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let single_tuple = Type::Tuple {
            elements: vec![number_ty],
        };
        assert_eq!(
            format_type(&single_tuple, &types, &test.program),
            "(number)"
        );

        // (number, string)
        let string_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });
        let pair_tuple = Type::Tuple {
            elements: vec![number_ty, string_ty],
        };
        assert_eq!(
            format_type(&pair_tuple, &types, &test.program),
            "(number, string)"
        );
    }

    /// Format union and intersection types as human-readable strings.
    #[test]
    fn test_format_union_intersection_types() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "declare let a: number | string");
        test.analyze_module(module_id);

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let string_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });
        let boolean_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Boolean),
        });

        // number | string
        let union_ty = Type::Union {
            elements: vec![number_ty, string_ty],
        };
        assert_eq!(
            format_type(&union_ty, &types, &test.program),
            "number | string"
        );

        // number & string & boolean
        let intersection_ty = Type::Intersection {
            elements: vec![number_ty, string_ty, boolean_ty],
        };
        assert_eq!(
            format_type(&intersection_ty, &types, &test.program),
            "number & string & boolean"
        );
    }

    /// Format function types as human-readable strings.
    #[test]
    fn test_format_function_types() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "declare let f: () -> void");
        test.analyze_module(module_id);

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let mut types = module.dir.types.write();

        // () -> void
        let void_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Void,
        });
        let function_ty = Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            static_parameters: vec![],
            dynamic_parameters: vec![],
            return_type: Some(void_ty),
        };
        assert_eq!(
            format_type(&function_ty, &types, &test.program),
            "() -> void"
        );

        // (number) -> string
        let number_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let string_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });
        let function_ty2 = Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            static_parameters: vec![],
            dynamic_parameters: vec![number_ty],
            return_type: Some(string_ty),
        };
        assert_eq!(
            format_type(&function_ty2, &types, &test.program),
            "(number) -> string"
        );

        // async (number, string) -> boolean
        let boolean_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Boolean),
        });
        let async_function = Type::Function {
            asynchrony: Asynchrony::Async,
            cardinality: FunctionCardinality::Scalar,
            static_parameters: vec![],
            dynamic_parameters: vec![number_ty, string_ty],
            return_type: Some(boolean_ty),
        };
        assert_eq!(
            format_type(&async_function, &types, &test.program),
            "async (number, string) -> boolean"
        );
    }

    /// Format error and unevaluated types as human-readable strings.
    #[test]
    fn test_format_error_and_unevaluated() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module("test.ds", "declare let a: any");
        test.analyze_module(module_id);

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let types = module.dir.types.read();

        let error_ty = Type::Error;
        assert_eq!(format_type(&error_ty, &types, &test.program), "<error>");
    }
}
