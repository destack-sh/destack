use crate::Compiler;
use destack_artifact::Ast;
use destack_ast as ast;
use destack_dir::{
    FloatType, IntType, IntrinsicType, LocalNodeIdAny, LocalScopeId, LocalScopeMark, ModuleBinding,
    PrimitiveType, ScalarLiteral, SymbolSpaceOrder, SymbolTable, TemplateLiteral, Tree,
    TypeLiteral, TypeTable,
};
use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a scalar literal to a DIR scalar literal.
    pub(super) fn bind_scalar_literal(
        &self,
        _module: &Module,
        ast: &Ast,
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
                let string = self.repository.strings.intern_from(&ast.strings, *string);
                ScalarLiteral::String(string)
            }
            ast::ScalarLiteral::RegexString { content, flags } => {
                let content = self.repository.strings.intern_from(&ast.strings, *content);
                let flags =
                    flags.map(|flag| self.repository.strings.intern_from(&ast.strings, flag));
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
        global_augmentation_scope: LocalScopeId,
        module_bindings: &mut Vec<ModuleBinding>,
        scope: (LocalScopeId, LocalScopeMark),
        template_literal: &ast::TemplateLiteral,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> TemplateLiteral {
        match template_literal {
            ast::TemplateLiteral::String { string } => {
                let string = self.repository.strings.intern_from(&ast.strings, *string);
                TemplateLiteral::String { string }
            }
            ast::TemplateLiteral::InterpolatedString { strings, arguments } => {
                let strings = strings
                    .iter()
                    .map(|string| self.repository.strings.intern_from(&ast.strings, *string))
                    .collect();
                let arguments = arguments
                    .iter()
                    .map(|argument| {
                        self.bind_argument(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            scope,
                            *argument,
                            parent_id,
                            tree,
                            symbols,
                            types,
                            SymbolSpaceOrder::ValueThenType,
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
            ast::IntType::Pointer { is_signed: true } => IntType::Isize,
            ast::IntType::Pointer { is_signed: false } => IntType::Usize,
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
        let float_type = match float_type {
            ast::FloatType { width: Some(32) } => FloatType::Float32,
            ast::FloatType { width: Some(64) } => FloatType::Float64,
            ast::FloatType { width: None } => FloatType::Arbitrary {
                width: self.options.default_float_width,
            },
            ast::FloatType { width: Some(width) } => FloatType::Arbitrary { width: *width },
        };
        float_type.simplify()
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
            ast::TypeLiteral::Number => TypeLiteral::Primitive(PrimitiveType::Number),
            ast::TypeLiteral::Int(int_type) => {
                TypeLiteral::Primitive(PrimitiveType::Int(self.bind_int_type(int_type)))
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
