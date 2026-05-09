use destack_artifact::Ast;
use destack_ast as ast;
use destack_dir::{
    BindingTable, DeclaredModule, FloatType, IntegerType, IntrinsicType, LocalNodeIdAny,
    LocalScopeId, LocalScopeMark, PrimitiveType, ScalarLiteral, SymbolSpace, TemplateLiteral, Tree,
    TypeLiteral, TypeTable,
};
use destack_workspace::Module;

use crate::Compiler;
#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a scalar literal to a DIR scalar literal.
    pub(super) fn bind_scalar_literal(
        &self,
        _module: &Module,
        _ast: &Ast,
        scalar_literal: &ast::ScalarLiteral,
    ) -> ScalarLiteral {
        match scalar_literal {
            ast::ScalarLiteral::Null => ScalarLiteral::Null,
            ast::ScalarLiteral::Boolean(boolean) => ScalarLiteral::Boolean(*boolean),
            ast::ScalarLiteral::Integer(integer) => ScalarLiteral::Integer(*integer),
            ast::ScalarLiteral::Bigint(bigint) => ScalarLiteral::Bigint(*bigint),
            ast::ScalarLiteral::Float(float) => ScalarLiteral::Float(*float),
            ast::ScalarLiteral::Character(character) => ScalarLiteral::Character(*character),
            ast::ScalarLiteral::String(string) => {
                let string = *string;
                ScalarLiteral::String(string)
            }
            ast::ScalarLiteral::RegexString { content, flags } => {
                let content = *content;
                let flags = flags.map(|flag| flag);
                ScalarLiteral::RegexString { content, flags }
            }
        }
    }

    /// Bind a template literal to a DIR template literal.
    pub(super) fn bind_template_literal(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_scope: LocalScopeId,
        declared_modules: &mut Vec<DeclaredModule>,
        scope: (LocalScopeId, LocalScopeMark),
        template_literal: &ast::TemplateLiteral,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
        symbols: &mut BindingTable,
        types: &mut TypeTable,
    ) -> TemplateLiteral {
        match template_literal {
            ast::TemplateLiteral::String { string } => {
                let string = *string;
                TemplateLiteral::String { string }
            }
            ast::TemplateLiteral::InterpolatedString { strings, arguments } => {
                let strings = strings.iter().map(|string| *string).collect();
                let arguments = arguments
                    .iter()
                    .map(|argument| {
                        self.bind_argument(
                            module,
                            ast,
                            namespace_scope,
                            global_scope,
                            declared_modules,
                            scope,
                            *argument,
                            parent_id,
                            tree,
                            symbols,
                            types,
                            SymbolSpace::Value,
                        )
                    })
                    .collect();
                TemplateLiteral::InterpolatedString { strings, arguments }
            }
        }
    }

    /// Bind an integer type to a DIR integer type.
    pub(super) fn bind_integer_type(&self, int_type: &ast::IntegerType) -> IntegerType {
        match int_type {
            ast::IntegerType::Integer { is_signed: true } => IntegerType::Fixed {
                width: 64,
                is_signed: true,
            },
            ast::IntegerType::Integer { is_signed: false } => IntegerType::Fixed {
                width: 64,
                is_signed: false,
            },
            ast::IntegerType::Fixed { width, is_signed } => IntegerType::Fixed {
                width: *width,
                is_signed: *is_signed,
            },
            ast::IntegerType::Pointer { is_signed } => IntegerType::Pointer {
                is_signed: *is_signed,
            },
        }
    }

    /// Bind a float type to a DIR float type.
    pub(super) fn bind_float_type(&self, float_type: &ast::FloatType) -> FloatType {
        match float_type {
            ast::FloatType::Float => FloatType::Float64,
            ast::FloatType::Float32 => FloatType::Float32,
            ast::FloatType::Float64 => FloatType::Float64,
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
            ast::TypeLiteral::Object => TypeLiteral::Object,
            ast::TypeLiteral::Void => TypeLiteral::Void,
            ast::TypeLiteral::Null => TypeLiteral::Null,
            ast::TypeLiteral::Boolean => TypeLiteral::Primitive(PrimitiveType::Boolean),
            ast::TypeLiteral::Character => TypeLiteral::Primitive(PrimitiveType::Character),
            ast::TypeLiteral::String => TypeLiteral::Primitive(PrimitiveType::String),
            ast::TypeLiteral::Bigint => TypeLiteral::Primitive(PrimitiveType::Bigint),
            ast::TypeLiteral::Number => {
                TypeLiteral::Primitive(PrimitiveType::Float(FloatType::Float64))
            }
            ast::TypeLiteral::Integer(int_type) => {
                TypeLiteral::Primitive(PrimitiveType::Integer(self.bind_integer_type(int_type)))
            }
            ast::TypeLiteral::Float(float_type) => {
                TypeLiteral::Primitive(PrimitiveType::Float(self.bind_float_type(float_type)))
            }
            ast::TypeLiteral::Symbol => TypeLiteral::Primitive(PrimitiveType::Symbol),
            ast::TypeLiteral::UniqueSymbol => TypeLiteral::Primitive(PrimitiveType::UniqueSymbol),
            ast::TypeLiteral::Intrinsic(intrinsic) => {
                TypeLiteral::Intrinsic(self.bind_type_intrinsic(intrinsic))
            }
        }
    }

    /// Bind a type intrinsic to a DIR type intrinsic.
    pub(super) fn bind_type_intrinsic(&self, intrinsic: &ast::IntrinsicType) -> IntrinsicType {
        match intrinsic {
            ast::IntrinsicType::Uppercase => IntrinsicType::Uppercase,
            ast::IntrinsicType::Lowercase => IntrinsicType::Lowercase,
            ast::IntrinsicType::Capitalize => IntrinsicType::Capitalize,
            ast::IntrinsicType::Uncapitalize => IntrinsicType::Uncapitalize,
            ast::IntrinsicType::NoInfer => IntrinsicType::NoInfer,
            ast::IntrinsicType::BuiltinIteratorReturn => IntrinsicType::BuiltinIteratorReturn,
        }
    }
}
