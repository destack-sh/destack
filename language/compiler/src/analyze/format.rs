use crate::Compiler;
use destack_dir::{
    Asynchrony, DeclarationType, GlobalSymbolId, GlobalTypeId, LocalTypeId, Mutability,
    PrimitiveType, ScalarLiteral, StaticArgument, StaticExpression, StaticKey, Type,
    TypeBinaryOperator, TypeLiteral, TypeTable, TypeUnaryOperator,
};

impl Compiler {
    /// Format a global type id as a human-readable string.
    pub fn format_global_type(&self, ty_id: GlobalTypeId) -> String {
        let module = self.program.modules.get(ty_id.module_id);
        let module = module.read();
        let types = module.dir.types.read();
        let ty = types.get_type(ty_id.local_id);
        self.format_type(ty, &types)
    }

    /// Format a type by its id.
    pub fn format_local_type(&self, ty_id: LocalTypeId, types: &TypeTable) -> String {
        let ty = types.get_type(ty_id);
        self.format_type(ty, types)
    }

    /// Format a type as a human-readable string.
    pub fn format_type(&self, ty: &Type, types: &TypeTable) -> String {
        match ty {
            Type::TypeLiteral { value } => self.format_type_literal(value),
            Type::Value { value } => format!("type {}", self.format_local_type(*value, types)),
            Type::Reference {
                symbol,
                static_arguments,
            } => self.format_type_reference(*symbol, static_arguments.as_deref()),
            Type::Unevaluated(_) => "<unevaluated>".to_string(),
            Type::Unary { operator, right } => self.format_type_unary(*operator, *right, types),
            Type::Mutable { mutability, right } => {
                let right_str = self.format_local_type(*right, types);
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
                let right_str = self.format_local_type(*right, types);
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
                let right_str = self.format_local_type(*right, types);
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
            } => self.format_type_binary(*left, *operator, *right, types),
            Type::ArraySized { element, count: _ } => {
                let elem_str = self.format_local_type(*element, types);
                format!("{elem_str}[]")
            }
            Type::Array { element } => {
                if let Some(elem) = element {
                    let elem_str = self.format_local_type(*elem, types);
                    format!("{elem_str}[]")
                } else {
                    "[]".to_string()
                }
            }
            Type::Tuple { elements } => {
                let elems: Vec<_> = elements
                    .iter()
                    .map(|e| self.format_local_type(*e, types))
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
                            let key = self.format_static_key(&f.key);
                            let ty = self.format_local_type(f.ty, types);
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
                let static_parameers = if static_parameters.is_empty() {
                    String::new()
                } else {
                    let parameers: Vec<_> = static_parameters
                        .iter()
                        .map(|p| self.format_local_type(*p, types))
                        .collect();
                    format!("<{}>", parameers.join(", "))
                };
                let dynamic_parameers: Vec<_> = dynamic_parameters
                    .iter()
                    .map(|p| self.format_local_type(*p, types))
                    .collect();
                let ret = if let Some(ret_ty) = return_type {
                    format!(" -> {}", self.format_local_type(*ret_ty, types))
                } else {
                    String::new()
                };
                format!(
                    "{async_str}{static_parameers}({}){ret}",
                    dynamic_parameers.join(", ")
                )
            }
            Type::Union { elements } => {
                let elems: Vec<_> = elements
                    .iter()
                    .map(|e| self.format_local_type(*e, types))
                    .collect();
                elems.join(" | ")
            }
            Type::Intersection { elements } => {
                let elems: Vec<_> = elements
                    .iter()
                    .map(|e| self.format_local_type(*e, types))
                    .collect();
                elems.join(" & ")
            }
            Type::Error => "<error>".to_string(),
        }
    }

    /// Format a TypeLiteral.
    fn format_type_literal(&self, lit: &TypeLiteral) -> String {
        match lit {
            TypeLiteral::Never => "never".to_string(),
            TypeLiteral::Any => "any".to_string(),
            TypeLiteral::Infer => "_".to_string(),
            TypeLiteral::Undefined => "undefined".to_string(),
            TypeLiteral::Unknown => "unknown".to_string(),
            TypeLiteral::Void => "void".to_string(),
            TypeLiteral::Null => "null".to_string(),
            TypeLiteral::Primitive(p) => self.format_primitive_type(p),
            TypeLiteral::Composite(c) => self.format_composite_type(c),
            TypeLiteral::ScalarLiteral(s) => self.format_scalar_literal(s),
        }
    }

    /// Format a PrimitiveType.
    fn format_primitive_type(&self, prim: &PrimitiveType) -> String {
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
    fn format_composite_type(&self, comp: &DeclarationType) -> String {
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
    fn format_scalar_literal(&self, scalar: &ScalarLiteral) -> String {
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
                let s = &*self.program.strings.get(*string_id);
                format!("\"{s}\"")
            }
            ScalarLiteral::RegexString { content, flags } => {
                let content_str = &*self.program.strings.get(*content);
                if let Some(flags_id) = flags {
                    let flags_str = &*self.program.strings.get(*flags_id);
                    format!("/{content_str}/{flags_str}")
                } else {
                    format!("/{content_str}/")
                }
            }
        }
    }

    /// Format a type reference (symbol with optional static arguments).
    fn format_type_reference(
        &self,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
    ) -> String {
        let name = self.get_symbol_name(symbol);
        if let Some(args) = static_arguments
            && !args.is_empty()
        {
            let arg_strs: Vec<_> = args
                .iter()
                .map(|arg| self.format_static_argument(arg))
                .collect();
            format!("{name}<{}>", arg_strs.join(", "))
        } else {
            name
        }
    }

    /// Get the name of a symbol from any module.
    fn get_symbol_name(&self, symbol_id: GlobalSymbolId) -> String {
        let module = self.program.modules.get(symbol_id.module_id);
        let module = module.read();
        let symbols = module.dir.symbols.read();
        let symbol = symbols.get_symbol(symbol_id.into_local());
        if let Some(name_id) = symbol.name() {
            self.program.strings.get(name_id).to_string()
        } else {
            "<anonymous>".to_string()
        }
    }

    /// Format a StaticKey.
    pub(super) fn format_static_key(&self, key: &StaticKey) -> String {
        match key {
            StaticKey::Name(name_id) => self.program.strings.get(*name_id).to_string(),
            StaticKey::UniqueSymbol(_) => "<unique symbol>".to_string(),
            StaticKey::GlobalSymbol(name_id) => {
                let name = &*self.program.strings.get(*name_id);
                format!("[Symbol.{name}]")
            }
        }
    }

    /// Format a StaticArgument.
    fn format_static_argument(&self, arg: &StaticArgument) -> String {
        match arg {
            StaticArgument::Unevaluated { .. } => "<unevaluated>".to_string(),
            StaticArgument::Evaluated { name, value } => {
                let value_str = self.format_static_expression(value);
                if let Some(name_id) = name {
                    let name_str = &*self.program.strings.get(*name_id);
                    format!("{name_str}: {value_str}")
                } else {
                    value_str
                }
            }
        }
    }

    /// Format a StaticExpression.
    fn format_static_expression(&self, expr: &StaticExpression) -> String {
        match expr {
            StaticExpression::Unevaluated { .. } => "<unevaluated>".to_string(),
            StaticExpression::ScalarLiteral { value } => self.format_scalar_literal(value),
            StaticExpression::TypeLiteral { value } => self.format_type_literal(value),
            StaticExpression::Declaration { .. } => "<declaration>".to_string(),
            StaticExpression::Type { .. } => "<type>".to_string(),
            StaticExpression::RangeExpression {
                start,
                end,
                is_inclusive,
            } => {
                let start_str = self.format_static_expression(start);
                let end_str = self.format_static_expression(end);
                let op = if *is_inclusive { "..=" } else { ".." };
                format!("{start_str}{op}{end_str}")
            }
            StaticExpression::ArrayExpression { elements } => {
                let elems: Vec<_> = elements
                    .iter()
                    .map(|e| self.format_static_expression(e))
                    .collect();
                format!("[{}]", elems.join(", "))
            }
            StaticExpression::TupleExpression { elements } => {
                let elems: Vec<_> = elements
                    .iter()
                    .map(|e| self.format_static_expression(e))
                    .collect();
                format!("({})", elems.join(", "))
            }
            StaticExpression::ObjectExpression { .. } => "{...}".to_string(),
        }
    }

    /// Format a type unary operator expression.
    fn format_type_unary(
        &self,
        operator: TypeUnaryOperator,
        right: LocalTypeId,
        types: &TypeTable,
    ) -> String {
        let right_str = self.format_local_type(right, types);
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
        &self,
        left: LocalTypeId,
        operator: TypeBinaryOperator,
        right: LocalTypeId,
        types: &TypeTable,
    ) -> String {
        let left_str = self.format_local_type(left, types);
        let right_str = self.format_local_type(right, types);
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
}

#[cfg(test)]
mod tests {
    use destack_dir::{
        Asynchrony, FloatType, FunctionCardinality, IntType, PrimitiveType, ScalarLiteral, Type,
        TypeLiteral,
    };

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

        for (prim, expected) in types {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Primitive(prim),
            };
            let formatted = test.compiler.format_type(&ty, &module_types);
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

        for (lit, expected) in types {
            let ty = Type::TypeLiteral { value: lit };
            let formatted = test.compiler.format_type(&ty, &module_types);
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
        assert_eq!(test.compiler.format_type(&ty, &module_types), "42");

        // negative integer
        let ty = Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(-123)),
        };
        assert_eq!(test.compiler.format_type(&ty, &module_types), "-123");

        // float
        let ty = Type::TypeLiteral {
            #[allow(clippy::approx_constant)]
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Float(3.14)),
        };
        assert_eq!(test.compiler.format_type(&ty, &module_types), "3.14");

        // boolean true
        let ty = Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(true)),
        };
        assert_eq!(test.compiler.format_type(&ty, &module_types), "true");

        // boolean false
        let ty = Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(false)),
        };
        assert_eq!(test.compiler.format_type(&ty, &module_types), "false");

        // bigint
        let ty = Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Bigint(999)),
        };
        assert_eq!(test.compiler.format_type(&ty, &module_types), "999n");

        // character
        let ty = Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Character('x')),
        };
        assert_eq!(test.compiler.format_type(&ty, &module_types), "'x'");
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
        let empty_arr = Type::Array { element: None };
        assert_eq!(test.compiler.format_type(&empty_arr, &types), "[]");

        // number[]
        let num_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let num_arr = Type::Array {
            element: Some(num_ty),
        };
        assert_eq!(test.compiler.format_type(&num_arr, &types), "number[]");

        // string[]
        let str_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });
        let str_arr = Type::Array {
            element: Some(str_ty),
        };
        assert_eq!(test.compiler.format_type(&str_arr, &types), "string[]");
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
        let empty_tup = Type::Tuple { elements: vec![] };
        assert_eq!(test.compiler.format_type(&empty_tup, &types), "()");

        // (number)
        let num_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let single_tup = Type::Tuple {
            elements: vec![num_ty],
        };
        assert_eq!(test.compiler.format_type(&single_tup, &types), "(number)");

        // (number, string)
        let str_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });
        let pair_tup = Type::Tuple {
            elements: vec![num_ty, str_ty],
        };
        assert_eq!(
            test.compiler.format_type(&pair_tup, &types),
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

        let num_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let str_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });
        let bool_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Boolean),
        });

        // number | string
        let union_ty = Type::Union {
            elements: vec![num_ty, str_ty],
        };
        assert_eq!(
            test.compiler.format_type(&union_ty, &types),
            "number | string"
        );

        // number & string & boolean
        let inter_ty = Type::Intersection {
            elements: vec![num_ty, str_ty, bool_ty],
        };
        assert_eq!(
            test.compiler.format_type(&inter_ty, &types),
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
        let fn_ty = Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            static_parameters: vec![],
            dynamic_parameters: vec![],
            return_type: Some(void_ty),
        };
        assert_eq!(test.compiler.format_type(&fn_ty, &types), "() -> void");

        // (number) -> string
        let num_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
        let str_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        });
        let fn_ty2 = Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            static_parameters: vec![],
            dynamic_parameters: vec![num_ty],
            return_type: Some(str_ty),
        };
        assert_eq!(
            test.compiler.format_type(&fn_ty2, &types),
            "(number) -> string"
        );

        // async (number, string) -> boolean
        let bool_ty = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Boolean),
        });
        let async_fn = Type::Function {
            asynchrony: Asynchrony::Async,
            cardinality: FunctionCardinality::Scalar,
            static_parameters: vec![],
            dynamic_parameters: vec![num_ty, str_ty],
            return_type: Some(bool_ty),
        };
        assert_eq!(
            test.compiler.format_type(&async_fn, &types),
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
        assert_eq!(test.compiler.format_type(&error_ty, &types), "<error>");
    }
}
