use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;
use crate::import::DEFAULT_FLOAT_WIDTH;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Unbind a DIR scalar literal to an AST scalar literal.
    pub(super) fn unbind_scalar_literal(
        &self,
        literal: &dir::ScalarLiteral,
        _ast_strings: &mut StringPool,
        _context: &mut UnbindContext,
    ) -> ast::ScalarLiteral {
        match literal {
            dir::ScalarLiteral::Null => ast::ScalarLiteral::Null,
            dir::ScalarLiteral::Boolean(boolean) => ast::ScalarLiteral::Boolean(*boolean),
            dir::ScalarLiteral::Integer(integer) => ast::ScalarLiteral::Integer(*integer),
            dir::ScalarLiteral::Bigint(bigint) => ast::ScalarLiteral::Bigint(*bigint),
            dir::ScalarLiteral::Float(float) => ast::ScalarLiteral::Float(*float),
            dir::ScalarLiteral::Character(character) => ast::ScalarLiteral::Character(*character),
            dir::ScalarLiteral::String(string) => {
                let string = *string;
                ast::ScalarLiteral::String(string)
            }
            dir::ScalarLiteral::RegexString { content, flags } => {
                let content = *content;
                let flags = flags.map(|flag| flag);
                ast::ScalarLiteral::RegexString { content, flags }
            }
        }
    }

    /// Unbind a DIR type literal to an AST type literal.
    pub(super) fn unbind_type_literal(
        &self,
        literal: &dir::TypeLiteral,
        context: &mut UnbindContext,
    ) -> ast::TypeLiteral {
        match literal {
            dir::TypeLiteral::Any => ast::TypeLiteral::Any,
            dir::TypeLiteral::Never => ast::TypeLiteral::Never,
            dir::TypeLiteral::Infer => ast::TypeLiteral::Infer,
            dir::TypeLiteral::Undefined => ast::TypeLiteral::Undefined,
            dir::TypeLiteral::Unknown => ast::TypeLiteral::Unknown,
            dir::TypeLiteral::Object => ast::TypeLiteral::Object,
            dir::TypeLiteral::Void => ast::TypeLiteral::Void,
            dir::TypeLiteral::Null => ast::TypeLiteral::Null,
            dir::TypeLiteral::Primitive(primitive) => match primitive {
                dir::PrimitiveType::Boolean => ast::TypeLiteral::Boolean,
                dir::PrimitiveType::Character => ast::TypeLiteral::Character,
                dir::PrimitiveType::String => ast::TypeLiteral::String,
                dir::PrimitiveType::Bigint => ast::TypeLiteral::Bigint,
                dir::PrimitiveType::Number => ast::TypeLiteral::Number,
                dir::PrimitiveType::Symbol => ast::TypeLiteral::Symbol,
                dir::PrimitiveType::UniqueSymbol => ast::TypeLiteral::UniqueSymbol,
                dir::PrimitiveType::Int(int_type) => {
                    ast::TypeLiteral::Int(self.unbind_int_type(int_type, context))
                }
                dir::PrimitiveType::Float(float_type) => {
                    ast::TypeLiteral::Float(self.unbind_float_type(float_type, context))
                }
            },
            dir::TypeLiteral::Intrinsic(intrinsic) => {
                ast::TypeLiteral::Intrinsic(self.unbind_type_intrinsic(intrinsic, context))
            }
            dir::TypeLiteral::ScalarLiteral(scalar) => match scalar {
                dir::ScalarLiteral::Null => ast::TypeLiteral::Null,
                dir::ScalarLiteral::Boolean(_) => ast::TypeLiteral::Boolean,
                dir::ScalarLiteral::Integer(_) => ast::TypeLiteral::Int(ast::IntType::Arbitrary {
                    width: Some(32),
                    is_signed: true,
                }),
                dir::ScalarLiteral::Bigint(_) => ast::TypeLiteral::Bigint,
                dir::ScalarLiteral::Float(_) => {
                    ast::TypeLiteral::Float(ast::FloatType { width: Some(64) })
                }
                dir::ScalarLiteral::Character(_) => ast::TypeLiteral::Character,
                dir::ScalarLiteral::String(_) => ast::TypeLiteral::String,
                dir::ScalarLiteral::RegexString { .. } => ast::TypeLiteral::String,
            },
        }
    }

    /// Unbind a DIR type intrinsic to an AST type intrinsic.
    fn unbind_type_intrinsic(
        &self,
        intrinsic: &dir::IntrinsicType,
        _context: &mut UnbindContext,
    ) -> ast::IntrinsicType {
        match intrinsic {
            dir::IntrinsicType::Uppercase => ast::IntrinsicType::Uppercase,
            dir::IntrinsicType::Lowercase => ast::IntrinsicType::Lowercase,
            dir::IntrinsicType::Capitalize => ast::IntrinsicType::Capitalize,
            dir::IntrinsicType::Uncapitalize => ast::IntrinsicType::Uncapitalize,
            dir::IntrinsicType::NoInfer => ast::IntrinsicType::NoInfer,
            dir::IntrinsicType::BuiltinIteratorReturn => ast::IntrinsicType::BuiltinIteratorReturn,
        }
    }

    /// Unbind a DIR int type to an AST int type.
    fn unbind_int_type(
        &self,
        int_type: &dir::IntType,
        _context: &mut UnbindContext,
    ) -> ast::IntType {
        match int_type {
            dir::IntType::Isize => ast::IntType::Pointer { is_signed: true },
            dir::IntType::Usize => ast::IntType::Pointer { is_signed: false },
            dir::IntType::Int8 => ast::IntType::Arbitrary {
                width: Some(8),
                is_signed: true,
            },
            dir::IntType::Int16 => ast::IntType::Arbitrary {
                width: Some(16),
                is_signed: true,
            },
            dir::IntType::Int32 => ast::IntType::Arbitrary {
                width: Some(32),
                is_signed: true,
            },
            dir::IntType::Int64 => ast::IntType::Arbitrary {
                width: Some(64),
                is_signed: true,
            },
            dir::IntType::Int128 => ast::IntType::Arbitrary {
                width: Some(128),
                is_signed: true,
            },
            dir::IntType::Int256 => ast::IntType::Arbitrary {
                width: Some(256),
                is_signed: true,
            },
            dir::IntType::Uint8 => ast::IntType::Arbitrary {
                width: Some(8),
                is_signed: false,
            },
            dir::IntType::Uint16 => ast::IntType::Arbitrary {
                width: Some(16),
                is_signed: false,
            },
            dir::IntType::Uint32 => ast::IntType::Arbitrary {
                width: Some(32),
                is_signed: false,
            },
            dir::IntType::Uint64 => ast::IntType::Arbitrary {
                width: Some(64),
                is_signed: false,
            },
            dir::IntType::Uint128 => ast::IntType::Arbitrary {
                width: Some(128),
                is_signed: false,
            },
            dir::IntType::Uint256 => ast::IntType::Arbitrary {
                width: Some(256),
                is_signed: false,
            },
            dir::IntType::Arbitrary { width, is_signed } => ast::IntType::Arbitrary {
                width: Some(*width),
                is_signed: *is_signed,
            },
        }
    }

    /// Unbind a DIR float type to an AST float type.
    fn unbind_float_type(
        &self,
        float_type: &dir::FloatType,
        _context: &mut UnbindContext,
    ) -> ast::FloatType {
        match float_type {
            dir::FloatType::Float32 => ast::FloatType { width: Some(32) },
            dir::FloatType::Float64 => ast::FloatType { width: Some(64) },
            dir::FloatType::Arbitrary { width } => {
                if *width == DEFAULT_FLOAT_WIDTH {
                    ast::FloatType { width: None }
                } else {
                    ast::FloatType {
                        width: Some(*width),
                    }
                }
            }
        }
    }

    /// Unbind a DIR template literal to an AST template literal.
    pub(super) fn unbind_template_literal(
        &self,
        module: &Module,
        literal: &dir::TemplateLiteral,
        tree: &dir::Tree,
        symbols: &dir::BindingTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::TemplateLiteral {
        match literal {
            dir::TemplateLiteral::String { string } => {
                let string = *string;
                ast::TemplateLiteral::String { string }
            }
            dir::TemplateLiteral::InterpolatedString { strings, arguments } => {
                let strings = strings.iter().map(|string| *string).collect();
                let arguments = arguments
                    .iter()
                    .map(|argument| {
                        self.unbind_argument(
                            module,
                            *argument,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                ast::TemplateLiteral::InterpolatedString { strings, arguments }
            }
        }
    }
}
