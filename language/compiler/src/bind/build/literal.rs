use crate::Compiler;
use destack_ast as ast;
use destack_dir::{
    DeclarationType, FloatType, IntType, LocalNodeIdAny, LocalScopeId, LocalScopeMark, NodeTree,
    PrimitiveType, ScalarLiteral, SymbolTable, TemplateLiteral, TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ModuleAst};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a scalar literal to a DIR scalar literal.
    pub(super) fn bind_scalar_literal(
        &self,
        _module: &Module,
        ast: &ModuleAst,
        scalar_literal: &ast::ScalarLiteral,
    ) -> ScalarLiteral {
        match scalar_literal {
            ast::ScalarLiteral::Boolean(boolean) => ScalarLiteral::Boolean(*boolean),
            ast::ScalarLiteral::Integer(integer) => ScalarLiteral::Integer(*integer),
            ast::ScalarLiteral::Bigint(bigint) => ScalarLiteral::Bigint(*bigint),
            ast::ScalarLiteral::Float(float) => ScalarLiteral::Float(*float),
            ast::ScalarLiteral::Character(character) => ScalarLiteral::Character(*character),
            ast::ScalarLiteral::String(string) => {
                let string = self.program.strings.intern_from(&ast.strings, *string);
                ScalarLiteral::String(string)
            }
            ast::ScalarLiteral::RegexString { content, flags } => {
                let content = self.program.strings.intern_from(&ast.strings, *content);
                let flags = flags.map(|flag| self.program.strings.intern_from(&ast.strings, flag));
                ScalarLiteral::RegexString { content, flags }
            }
        }
    }

    /// Bind a template literal to a DIR template literal.
    pub(super) fn bind_template_literal(
        &self,
        module: &Module,
        ast: &ModuleAst,
        scope: (LocalScopeId, LocalScopeMark),
        template_literal: &ast::TemplateLiteral,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> TemplateLiteral {
        match template_literal {
            ast::TemplateLiteral::String { string } => {
                let string = self.program.strings.intern_from(&ast.strings, *string);
                TemplateLiteral::String { string }
            }
            ast::TemplateLiteral::InterpolatedString { strings, arguments } => {
                let strings = strings
                    .iter()
                    .map(|string| self.program.strings.intern_from(&ast.strings, *string))
                    .collect();
                let arguments = arguments
                    .iter()
                    .map(|argument| {
                        self.bind_argument(
                            module, ast, scope, *argument, parent_id, tree, symbols, types,
                        )
                    })
                    .collect();
                TemplateLiteral::InterpolatedString { strings, arguments }
            }
        }
    }

    /// Bind an int type to a DIR int type.
    pub(super) fn bind_int_type(&self, int_type: &ast::IntType) -> IntType {
        match int_type {
            // pointer
            ast::IntType::Pointer { is_signed: true } => IntType::IntP,
            ast::IntType::Pointer { is_signed: false } => IntType::UintP,
            // fixed builtin
            ast::IntType::Arbitrary {
                width: Some(8),
                is_signed: true,
            } => IntType::Int8,
            ast::IntType::Arbitrary {
                width: Some(16),
                is_signed: true,
            } => IntType::Int16,
            ast::IntType::Arbitrary {
                width: Some(32),
                is_signed: true,
            } => IntType::Int32,
            ast::IntType::Arbitrary {
                width: Some(64),
                is_signed: true,
            } => IntType::Int64,
            ast::IntType::Arbitrary {
                width: Some(128),
                is_signed: true,
            } => IntType::Int128,
            ast::IntType::Arbitrary {
                width: Some(256),
                is_signed: true,
            } => IntType::Int256,
            ast::IntType::Arbitrary {
                width: Some(8),
                is_signed: false,
            } => IntType::Uint8,
            ast::IntType::Arbitrary {
                width: Some(16),
                is_signed: false,
            } => IntType::Uint16,
            ast::IntType::Arbitrary {
                width: Some(32),
                is_signed: false,
            } => IntType::Uint32,
            ast::IntType::Arbitrary {
                width: Some(64),
                is_signed: false,
            } => IntType::Uint64,
            ast::IntType::Arbitrary {
                width: Some(128),
                is_signed: false,
            } => IntType::Uint128,
            ast::IntType::Arbitrary {
                width: Some(256),
                is_signed: false,
            } => IntType::Uint256,
            // fixed variable
            ast::IntType::Arbitrary {
                width: None,
                is_signed,
            } => IntType::Arbitrary {
                width: self.options.default_int_width,
                is_signed: *is_signed,
            },
            ast::IntType::Arbitrary {
                width: Some(width),
                is_signed,
            } => IntType::Arbitrary {
                width: *width,
                is_signed: *is_signed,
            },
        }
    }

    /// Bind a float type to a DIR float type.
    pub(super) fn bind_float_type(&self, float_type: &ast::FloatType) -> FloatType {
        match float_type {
            ast::FloatType { width: Some(32) } => FloatType::Float32,
            ast::FloatType { width: Some(64) } => FloatType::Float64,
            ast::FloatType { width: None } => FloatType::Arbitrary {
                width: self.options.default_float_width,
            },
            ast::FloatType { width: Some(width) } => FloatType::Arbitrary { width: *width },
        }
    }

    /// Bind a composite type to a DIR composite type.
    pub(super) fn bind_declaration_type(
        &self,
        composite_type: &ast::DeclarationType,
    ) -> DeclarationType {
        match composite_type {
            ast::DeclarationType::Type => DeclarationType::Type,
            ast::DeclarationType::Namespace => DeclarationType::Namespace,
            ast::DeclarationType::Struct => DeclarationType::Struct,
            ast::DeclarationType::Class => DeclarationType::Class,
            ast::DeclarationType::Enum => DeclarationType::Enum,
            ast::DeclarationType::Union => DeclarationType::Union,
            ast::DeclarationType::Interface => DeclarationType::Interface,
            ast::DeclarationType::Extension => DeclarationType::Extension,
            ast::DeclarationType::Function => DeclarationType::Function,
        }
    }

    /// Bind a type literal to a DIR type literal.
    pub(super) fn bind_type_literal(&self, type_literal: &ast::TypeLiteral) -> TypeLiteral {
        match type_literal {
            ast::TypeLiteral::Any => TypeLiteral::Any,
            ast::TypeLiteral::Never => TypeLiteral::Never,
            ast::TypeLiteral::Infer => TypeLiteral::Infer,
            ast::TypeLiteral::Undefined => TypeLiteral::Undefined,
            ast::TypeLiteral::Unknown => TypeLiteral::Unknown,
            ast::TypeLiteral::Void => TypeLiteral::Void,
            ast::TypeLiteral::Null => TypeLiteral::Null,
            ast::TypeLiteral::Boolean => TypeLiteral::Primitive(PrimitiveType::Boolean),
            ast::TypeLiteral::Character => TypeLiteral::Primitive(PrimitiveType::Character),
            ast::TypeLiteral::String => TypeLiteral::Primitive(PrimitiveType::String),
            ast::TypeLiteral::Bigint => TypeLiteral::Primitive(PrimitiveType::Bigint),
            ast::TypeLiteral::Number => TypeLiteral::Primitive(PrimitiveType::Number),
            ast::TypeLiteral::Int(int_type) => {
                TypeLiteral::Primitive(PrimitiveType::Int(self.bind_int_type(int_type)))
            }
            ast::TypeLiteral::Float(float_type) => {
                TypeLiteral::Primitive(PrimitiveType::Float(self.bind_float_type(float_type)))
            }
            ast::TypeLiteral::Composite(composite_type) => {
                TypeLiteral::Composite(self.bind_declaration_type(composite_type))
            }
            ast::TypeLiteral::Symbol => TypeLiteral::Primitive(PrimitiveType::Symbol),
            ast::TypeLiteral::UniqueSymbol => TypeLiteral::Primitive(PrimitiveType::UniqueSymbol),
        }
    }
}

/// Evaluate a numeric literal string to its canonical JS string representation.
///
/// Handles decimal, hex (0x), octal (0o, legacy 0), binary (0b), scientific notation,
/// and bigint (n suffix). Returns the canonical string that JS would use as an object key.
pub(super) fn evaluate_numeric_literal(source: &str) -> String {
    let source = source.trim();

    // handle bigint suffix
    let (source, is_bigint) = if source.ends_with('n') || source.ends_with('N') {
        (&source[..source.len() - 1], true)
    } else {
        (source, false)
    };

    // empty after stripping
    if source.is_empty() {
        return "0".to_string();
    }

    // parse based on prefix
    let value: f64 = if source.len() > 2 {
        match &source[..2] {
            // hexadecimal
            "0x" | "0X" => i64::from_str_radix(&source[2..], 16)
                .map(|v| v as f64)
                .unwrap_or(f64::NAN),
            // ES6 octal
            "0o" | "0O" => i64::from_str_radix(&source[2..], 8)
                .map(|v| v as f64)
                .unwrap_or(f64::NAN),
            // binary
            "0b" | "0B" => i64::from_str_radix(&source[2..], 2)
                .map(|v| v as f64)
                .unwrap_or(f64::NAN),
            _ => parse_decimal_or_legacy_octal(source),
        }
    } else {
        parse_decimal_or_legacy_octal(source)
    };

    // convert to canonical string representation
    if is_bigint {
        // bigint: just the integer digits
        if value.is_finite() {
            format!("{}", value as i64)
        } else {
            "0".to_string()
        }
    } else {
        number_to_string(value)
    }
}

/// Parse a decimal number or legacy octal (starts with 0).
fn parse_decimal_or_legacy_octal(source: &str) -> f64 {
    // legacy octal: starts with 0 and contains only 0-7
    if source.starts_with('0')
        && source.len() > 1
        && source[1..].chars().all(|c| c.is_ascii_digit())
        && source.chars().skip(1).all(|c| c >= '0' && c <= '7')
    {
        return i64::from_str_radix(&source[1..], 8)
            .map(|v| v as f64)
            .unwrap_or_else(|_| source.parse().unwrap_or(f64::NAN));
    }

    // regular decimal (including scientific notation)
    source.parse().unwrap_or(f64::NAN)
}

/// Convert an f64 to its canonical JS string representation.
fn number_to_string(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value.is_infinite() {
        if value.is_sign_positive() {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        }
    } else if value == 0.0 {
        "0".to_string()
    } else if value.fract() == 0.0 && value.abs() < 1e15 {
        // integer-valued, not too large: no decimal point
        format!("{}", value as i64)
    } else {
        // use JS-like representation
        let s = format!("{value}");
        // JS uses lowercase 'e' for scientific notation
        s.replace('E', "e")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_decimal() {
        assert_eq!(evaluate_numeric_literal("123"), "123");
        assert_eq!(evaluate_numeric_literal("0"), "0");
        assert_eq!(evaluate_numeric_literal("42"), "42");
    }

    #[test]
    fn test_evaluate_float() {
        assert_eq!(evaluate_numeric_literal("1.5"), "1.5");
        assert_eq!(evaluate_numeric_literal("3.14159"), "3.14159");
    }

    #[test]
    fn test_evaluate_scientific() {
        assert_eq!(evaluate_numeric_literal("1e2"), "100");
        assert_eq!(evaluate_numeric_literal("1e10"), "10000000000");
        assert_eq!(evaluate_numeric_literal("2e308"), "Infinity");
    }

    #[test]
    fn test_evaluate_hex() {
        assert_eq!(evaluate_numeric_literal("0x1a"), "26");
        assert_eq!(evaluate_numeric_literal("0XFF"), "255");
        assert_eq!(evaluate_numeric_literal("0x10"), "16");
    }

    #[test]
    fn test_evaluate_octal() {
        assert_eq!(evaluate_numeric_literal("0o17"), "15");
        assert_eq!(evaluate_numeric_literal("0O21"), "17");
    }

    #[test]
    fn test_evaluate_legacy_octal() {
        assert_eq!(evaluate_numeric_literal("017"), "15");
        assert_eq!(evaluate_numeric_literal("021"), "17");
    }

    #[test]
    fn test_evaluate_binary() {
        assert_eq!(evaluate_numeric_literal("0b101"), "5");
        assert_eq!(evaluate_numeric_literal("0B1111"), "15");
    }

    #[test]
    fn test_evaluate_bigint() {
        assert_eq!(evaluate_numeric_literal("123n"), "123");
        assert_eq!(evaluate_numeric_literal("0x10n"), "16");
    }
}
