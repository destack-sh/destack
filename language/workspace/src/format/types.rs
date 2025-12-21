use destack_base::StringPool;
use destack_dir as dir;

use crate::ModuleRegistry;

/// Format a global type id as a human-readable string.
pub fn format_global_type(
    ty_id: dir::GlobalTypeId,
    modules: &ModuleRegistry,
    strings: &StringPool,
) -> String {
    let module = modules.get(ty_id.module_id);
    let module = module.read();
    let Some(dir) = &module.dir else {
        return "<unknown>".to_string();
    };
    let types = dir.types.read();
    let ty = types.get_type(ty_id.local_id);
    format_type(ty, &types, modules, strings)
}

/// Format a type by its id.
pub fn format_local_type(
    ty_id: dir::LocalTypeId,
    types: &dir::TypeTable,
    modules: &ModuleRegistry,
    strings: &StringPool,
) -> String {
    let ty = types.get_type(ty_id);
    format_type(ty, types, modules, strings)
}

/// Format a type as a human-readable string.
pub fn format_type(
    ty: &dir::Type,
    types: &dir::TypeTable,
    modules: &ModuleRegistry,
    strings: &StringPool,
) -> String {
    match ty {
        dir::Type::TypeLiteral { value } => format_type_literal(value, strings),
        dir::Type::Value { value } => {
            format!(
                "type {}",
                format_local_type(*value, types, modules, strings)
            )
        }
        dir::Type::Reference {
            symbol,
            static_arguments,
        } => format_type_reference(*symbol, static_arguments.as_deref(), modules, strings),
        dir::Type::Unevaluated(_) => "<unevaluated>".to_string(),
        dir::Type::InferVar { id } => format!("<infer {}>", id.0),
        dir::Type::Unary { operator, right } => {
            format_type_unary(*operator, *right, types, modules, strings)
        }
        dir::Type::Mutable { mutability, right } => {
            let right_str = format_local_type(*right, types, modules, strings);
            match mutability {
                dir::Mutability::Immutable => right_str,
                dir::Mutability::Mutable => format!("var {right_str}"),
            }
        }
        dir::Type::ValueOf {
            mutability,
            variance,
            right,
        } => {
            let right_str = format_local_type(*right, types, modules, strings);
            let mut result = String::from("^");
            if let Some(m) = mutability {
                result.push_str(match m {
                    dir::Mutability::Mutable => "var ",
                    dir::Mutability::Immutable => "",
                });
            }
            if let Some(v) = variance {
                result.push_str(&format!("{v:?} ").to_lowercase());
            }
            result.push_str(&right_str);
            result
        }
        dir::Type::ReferenceOf {
            mutability,
            variance,
            right,
        } => {
            let right_str = format_local_type(*right, types, modules, strings);
            let mut result = String::from("&");
            if let Some(m) = mutability {
                result.push_str(match m {
                    dir::Mutability::Mutable => "var ",
                    dir::Mutability::Immutable => "",
                });
            }
            if let Some(v) = variance {
                result.push_str(&format!("{v:?} ").to_lowercase());
            }
            result.push_str(&right_str);
            result
        }
        dir::Type::Binary {
            left,
            operator,
            right,
        } => format_type_binary(*left, *operator, *right, types, modules, strings),
        dir::Type::ArraySized { element, count: _ } => {
            let elem_str = format_local_type(*element, types, modules, strings);
            format!("{elem_str}[]")
        }
        dir::Type::Array { element } => {
            if let Some(elem) = element {
                let elem_str = format_local_type(*elem, types, modules, strings);
                format!("{elem_str}[]")
            } else {
                "[]".to_string()
            }
        }
        dir::Type::Tuple { elements } => {
            let elements: Vec<_> = elements
                .iter()
                .map(|e| format_local_type(*e, types, modules, strings))
                .collect();
            format!("({})", elements.join(", "))
        }
        dir::Type::Object { fields } => {
            if fields.is_empty() {
                "{}".to_string()
            } else {
                let field_strs: Vec<_> = fields
                    .iter()
                    .map(|f| {
                        let key = format_static_key(&f.key, strings);
                        let ty = format_local_type(f.ty, types, modules, strings);
                        let opt = if f.is_optional { "?" } else { "" };
                        let readonly = if f.is_readonly { "readonly " } else { "" };
                        format!("{readonly}{key}{opt}: {ty}")
                    })
                    .collect();
                format!("{{ {} }}", field_strs.join(", "))
            }
        }
        dir::Type::Function {
            asynchrony,
            cardinality: _,
            static_parameters,
            dynamic_parameters,
            return_type,
        } => {
            let async_str = if *asynchrony == dir::Asynchrony::Async {
                "async "
            } else {
                ""
            };
            let static_params_str = if static_parameters.is_empty() {
                String::new()
            } else {
                let params: Vec<_> = static_parameters
                    .iter()
                    .map(|p| format_local_type(*p, types, modules, strings))
                    .collect();
                format!("<{}>", params.join(", "))
            };
            let dynamic_params: Vec<_> = dynamic_parameters
                .iter()
                .map(|p| format_local_type(*p, types, modules, strings))
                .collect();
            let ret = if let Some(ret_ty) = return_type {
                format!(": {}", format_local_type(*ret_ty, types, modules, strings))
            } else {
                String::new()
            };
            format!(
                "{async_str}{static_params_str}({}){ret}",
                dynamic_params.join(", ")
            )
        }
        dir::Type::Union { elements } => {
            let elements: Vec<_> = elements
                .iter()
                .map(|e| format_local_type(*e, types, modules, strings))
                .collect();
            elements.join(" | ")
        }
        dir::Type::Intersection { elements } => {
            let elements: Vec<_> = elements
                .iter()
                .map(|e| format_local_type(*e, types, modules, strings))
                .collect();
            elements.join(" & ")
        }
        dir::Type::Error => "<error>".to_string(),
    }
}

/// Format a TypeLiteral.
pub fn format_type_literal(lit: &dir::TypeLiteral, strings: &StringPool) -> String {
    match lit {
        dir::TypeLiteral::Never => "never".to_string(),
        dir::TypeLiteral::Any => "any".to_string(),
        dir::TypeLiteral::Infer => "_".to_string(),
        dir::TypeLiteral::Undefined => "undefined".to_string(),
        dir::TypeLiteral::Unknown => "unknown".to_string(),
        dir::TypeLiteral::Void => "void".to_string(),
        dir::TypeLiteral::Null => "null".to_string(),
        dir::TypeLiteral::Primitive(p) => format_primitive_type(p),
        dir::TypeLiteral::Composite(c) => format_composite_type(c),
        dir::TypeLiteral::ScalarLiteral(s) => format_scalar_literal(s, strings),
    }
}

/// Format a PrimitiveType.
pub fn format_primitive_type(prim: &dir::PrimitiveType) -> String {
    match prim {
        dir::PrimitiveType::Boolean => "boolean".to_string(),
        dir::PrimitiveType::Character => "char".to_string(),
        dir::PrimitiveType::String => "string".to_string(),
        dir::PrimitiveType::Bigint => "bigint".to_string(),
        dir::PrimitiveType::Number => "number".to_string(),
        dir::PrimitiveType::Int(int_type) => int_type.as_str(),
        dir::PrimitiveType::Float(float_type) => float_type.as_str(),
        dir::PrimitiveType::Symbol => "symbol".to_string(),
        dir::PrimitiveType::UniqueSymbol => "unique symbol".to_string(),
    }
}

/// Format a DeclarationType.
pub fn format_composite_type(comp: &dir::DeclarationType) -> String {
    match comp {
        dir::DeclarationType::Type => "type".to_string(),
        dir::DeclarationType::Namespace => "namespace".to_string(),
        dir::DeclarationType::Struct => "struct".to_string(),
        dir::DeclarationType::Class => "class".to_string(),
        dir::DeclarationType::Enum => "enum".to_string(),
        dir::DeclarationType::Union => "union".to_string(),
        dir::DeclarationType::Interface => "interface".to_string(),
        dir::DeclarationType::Extension => "extension".to_string(),
        dir::DeclarationType::Function => "function".to_string(),
    }
}

/// Format a ScalarLiteral.
pub fn format_scalar_literal(scalar: &dir::ScalarLiteral, strings: &StringPool) -> String {
    match scalar {
        dir::ScalarLiteral::Boolean(b) => b.to_string(),
        dir::ScalarLiteral::Integer(i) => i.to_string(),
        dir::ScalarLiteral::Bigint(i) => format!("{i}n"),
        dir::ScalarLiteral::Float(f) => {
            // ensure float has decimal point for clarity
            let s = f.to_string();
            if s.contains('.') || s.contains('e') || s.contains('E') {
                s
            } else {
                format!("{s}.0")
            }
        }
        dir::ScalarLiteral::Character(c) => format!("'{c}'"),
        dir::ScalarLiteral::String(string_id) => {
            let s = &*strings.get(*string_id);
            format!("\"{s}\"")
        }
        dir::ScalarLiteral::RegexString { content, flags } => {
            let content_str = strings.get(*content).to_string();
            if let Some(flags_id) = flags {
                let flags_str = strings.get(*flags_id).to_string();
                format!("/{content_str}/{flags_str}")
            } else {
                format!("/{content_str}/")
            }
        }
    }
}

/// Format a type reference (symbol with optional static arguments).
pub fn format_type_reference(
    symbol: dir::GlobalSymbolId,
    static_arguments: Option<&[dir::StaticArgument]>,
    modules: &ModuleRegistry,
    strings: &StringPool,
) -> String {
    let name = format_symbol_name(symbol, modules, strings);
    if let Some(arguments) = static_arguments
        && !arguments.is_empty()
    {
        let argument_strs: Vec<_> = arguments
            .iter()
            .map(|argument| format_static_argument(argument, strings))
            .collect();
        format!("{name}<{}>", argument_strs.join(", "))
    } else {
        name
    }
}

/// Get the name of a symbol from any module.
pub fn format_symbol_name(
    symbol_id: dir::GlobalSymbolId,
    modules: &ModuleRegistry,
    strings: &StringPool,
) -> String {
    let module = modules.get(symbol_id.module_id);
    let module = module.read();
    let Some(dir) = &module.dir else {
        return "<unknown>".to_string();
    };
    let symbols = dir.symbols.read();
    let symbol = symbols.get_symbol(symbol_id.into_local());
    if let Some(name_id) = symbol.name() {
        strings.get(name_id).to_string()
    } else {
        "<anonymous>".to_string()
    }
}

/// Format a StaticKey.
pub fn format_static_key(key: &dir::StaticKey, strings: &StringPool) -> String {
    match key {
        dir::StaticKey::Name(name_id) => strings.get(*name_id).to_string(),
        dir::StaticKey::UniqueSymbol(_) => "<unique symbol>".to_string(),
        dir::StaticKey::GlobalSymbol(name_id) => {
            let name = &*strings.get(*name_id);
            format!("[Symbol.{name}]")
        }
    }
}

/// Format a StaticArgument.
pub fn format_static_argument(argument: &dir::StaticArgument, strings: &StringPool) -> String {
    match argument {
        dir::StaticArgument::Unevaluated { .. } => "<unevaluated>".to_string(),
        dir::StaticArgument::Evaluated { name, value } => {
            let value_str = format_static_expression(value, strings);
            if let Some(name_id) = name {
                let name_str = &*strings.get(*name_id);
                format!("{name_str}: {value_str}")
            } else {
                value_str
            }
        }
    }
}

/// Format a StaticExpression.
pub fn format_static_expression(
    expression: &dir::StaticExpression,
    strings: &StringPool,
) -> String {
    match expression {
        dir::StaticExpression::Unevaluated { .. } => "<unevaluated>".to_string(),
        dir::StaticExpression::ScalarLiteral { value } => format_scalar_literal(value, strings),
        dir::StaticExpression::TypeLiteral { value } => format_type_literal(value, strings),
        dir::StaticExpression::Declaration { .. } => "<declaration>".to_string(),
        dir::StaticExpression::Type { .. } => "<type>".to_string(),
        dir::StaticExpression::RangeExpression {
            start,
            end,
            is_inclusive,
        } => {
            let start_str = format_static_expression(start, strings);
            let end_str = format_static_expression(end, strings);
            let op = if *is_inclusive { "..=" } else { ".." };
            format!("{start_str}{op}{end_str}")
        }
        dir::StaticExpression::ArrayExpression { elements } => {
            let elements: Vec<_> = elements
                .iter()
                .map(|e| format_static_expression(e, strings))
                .collect();
            format!("[{}]", elements.join(", "))
        }
        dir::StaticExpression::TupleExpression { elements } => {
            let elements: Vec<_> = elements
                .iter()
                .map(|e| format_static_expression(e, strings))
                .collect();
            format!("({})", elements.join(", "))
        }
        dir::StaticExpression::ObjectExpression { .. } => "{...}".to_string(),
    }
}

/// Format a type unary operator expression.
fn format_type_unary(
    operator: dir::TypeUnaryOperator,
    right: dir::LocalTypeId,
    types: &dir::TypeTable,
    modules: &ModuleRegistry,
    strings: &StringPool,
) -> String {
    let right_str = format_local_type(right, types, modules, strings);
    match operator {
        dir::TypeUnaryOperator::Not => format!("!{right_str}"),
        dir::TypeUnaryOperator::Maybe => format!("{right_str}?"),
        dir::TypeUnaryOperator::Must => format!("{right_str}!"),
        dir::TypeUnaryOperator::Newtype => format!("newtype {right_str}"),
        dir::TypeUnaryOperator::Type => format!("type {right_str}"),
        dir::TypeUnaryOperator::Readonly => format!("readonly {right_str}"),
        dir::TypeUnaryOperator::Typeof => format!("typeof {right_str}"),
        dir::TypeUnaryOperator::Keyof => format!("keyof {right_str}"),
        dir::TypeUnaryOperator::Infer => format!("infer {right_str}"),
        dir::TypeUnaryOperator::AsConst => format!("{right_str} as const"),
        dir::TypeUnaryOperator::Asserts => format!("asserts {right_str}"),
    }
}

/// Format a type binary operator expression.
fn format_type_binary(
    left: dir::LocalTypeId,
    operator: dir::TypeBinaryOperator,
    right: dir::LocalTypeId,
    types: &dir::TypeTable,
    modules: &ModuleRegistry,
    strings: &StringPool,
) -> String {
    let left_str = format_local_type(left, types, modules, strings);
    let right_str = format_local_type(right, types, modules, strings);
    let op_str = match operator {
        dir::TypeBinaryOperator::Cast => "as",
        dir::TypeBinaryOperator::In => "in",
        dir::TypeBinaryOperator::Is => "is",
        dir::TypeBinaryOperator::InstanceOf => "instanceof",
        dir::TypeBinaryOperator::Satisfies => "satisfies",
        dir::TypeBinaryOperator::Extends => "extends",
        dir::TypeBinaryOperator::Implements => "implements",
    };
    format!("{left_str} {op_str} {right_str}")
}
