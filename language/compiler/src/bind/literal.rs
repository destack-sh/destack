use crate::Compiler;
use destack_ast as ast;
use destack_dir::{
    DeclarationType, FloatType, IntType, LocalNodeIdAny, LocalScopeId, LocalScopeMark, Module,
    NodeTree, PrimitiveType, ScalarLiteral, SymbolTable, TemplateLiteral, TypeLiteral, TypeTable,
};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a scalar literal to a DIR scalar literal.
    pub(super) fn bind_scalar_literal(
        &self,
        module: &Module,
        scalar_literal: &ast::ScalarLiteral,
    ) -> ScalarLiteral {
        match scalar_literal {
            ast::ScalarLiteral::Boolean(boolean) => ScalarLiteral::Boolean(*boolean),
            ast::ScalarLiteral::Byte(byte) => ScalarLiteral::Byte(*byte),
            ast::ScalarLiteral::Integer(integer) => ScalarLiteral::Integer(*integer),
            ast::ScalarLiteral::Bigint(bigint) => ScalarLiteral::Bigint(*bigint),
            ast::ScalarLiteral::Float(float) => ScalarLiteral::Float(*float),
            ast::ScalarLiteral::Character(character) => ScalarLiteral::Character(*character),
            ast::ScalarLiteral::String(string) => {
                let string = self
                    .program
                    .strings
                    .intern_from(&module.ast_strings, *string);
                ScalarLiteral::String(string)
            }
            ast::ScalarLiteral::RegexString { content, flags } => {
                let content = self
                    .program
                    .strings
                    .intern_from(&module.ast_strings, *content);
                let flags =
                    flags.map(|flag| self.program.strings.intern_from(&module.ast_strings, flag));
                ScalarLiteral::RegexString { content, flags }
            }
            ast::ScalarLiteral::ByteString(byte_string) => {
                ScalarLiteral::ByteString(byte_string.clone())
            }
        }
    }

    /// Bind a template literal to a DIR template literal.
    pub(super) fn bind_template_literal(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        template_literal: &ast::TemplateLiteral,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> TemplateLiteral {
        match template_literal {
            ast::TemplateLiteral::String { string } => {
                let string = self
                    .program
                    .strings
                    .intern_from(&module.ast_strings, *string);
                TemplateLiteral::String { string }
            }
            ast::TemplateLiteral::InterpolatedString { strings, arguments } => {
                let strings = strings
                    .iter()
                    .map(|string| {
                        self.program
                            .strings
                            .intern_from(&module.ast_strings, *string)
                    })
                    .collect();
                let arguments = arguments
                    .iter()
                    .map(|argument| {
                        self.bind_argument(
                            module, scope, *argument, parent_id, tree, symbols, types,
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
                width: self.options.resolve.default_int_width,
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
                width: self.options.resolve.default_float_width,
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
