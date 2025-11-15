use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{
    Generics, Heritage, Module, Mutability, NodeId, ReferenceType, Type, TypeKind, TypeLiteral,
    VarianceBound,
};
use dyst_source::StringId;

impl<'a> Compiler<'a> {
    /// Lower a an expression into a type (without evaluating it at all).
    pub fn lower_expression_to_type(
        &mut self,
        module: &Module,
        expression_id: ast::NodeId<ast::Expression>,
    ) -> NodeId<Type> {
        let expression = self.lower_expression(module, expression_id);
        let type_id = self.session.tree.insert_from_ast(
            Type::UnevaluatedExpression(expression),
            module.id,
            expression_id,
        );
        self.session
            .tree
            .alias_from_ast(module.id, expression_id.id, type_id);
        type_id
    }

    /// Lower reference type into a DIR reference type.
    #[inline]
    pub fn lower_reference_type(&self, reference_type: ast::ReferenceType) -> ReferenceType {
        match reference_type {
            ast::ReferenceType::Value => ReferenceType::Value,
            ast::ReferenceType::Reference => ReferenceType::Reference,
        }
    }

    /// Lower mutability into a DIR mutability.
    #[inline]
    pub fn lower_mutability(&self, mutability: ast::Mutability) -> Mutability {
        match mutability {
            ast::Mutability::Immutable => Mutability::Immutable,
            ast::Mutability::Mutable => Mutability::Mutable,
        }
    }

    /// Lower a TypeKind to a DIR type kind.
    pub fn lower_type_kind(&mut self, kind: ast::TypeKind) -> TypeKind {
        match kind {
            ast::TypeKind::Structural => TypeKind::Structural,
            ast::TypeKind::Nominal => TypeKind::Nominal,
        }
    }

    /// Lower a VarianceBound to a DIR variance bound.
    pub fn lower_variance_bound(&mut self, bound: ast::VarianceBound) -> VarianceBound {
        match bound {
            ast::VarianceBound::Implements => VarianceBound::Implements,
            ast::VarianceBound::Extends => VarianceBound::Extends,
            ast::VarianceBound::Super => VarianceBound::Super,
        }
    }

    /// Lower AST definition generics into DIR definition generics.
    pub fn lower_generics(&mut self, module: &Module, generics: &ast::Generics) -> Generics {
        let static_parameters = generics
            .static_parameters
            .as_ref()
            .map(|static_parameters| {
                static_parameters
                    .iter()
                    .map(|static_parameter| self.lower_parameter(module, *static_parameter))
                    .collect()
            });
        let with_clauses = generics.with_clauses.as_ref().map(|with_clauses| {
            with_clauses
                .iter()
                .map(|with_clause| self.lower_with_clause(module, *with_clause))
                .collect()
        });
        let where_clauses = generics.where_clauses.as_ref().map(|where_clauses| {
            where_clauses
                .iter()
                .map(|where_clause| self.lower_where_clause(module, *where_clause))
                .collect()
        });
        Generics {
            static_parameters,
            with_clauses,
            where_clauses,
        }
    }

    /// Lower AST heritage into DIR heritage.
    pub fn lower_heritage(&mut self, module: &Module, heritage: &ast::Heritage) -> Heritage {
        let extends_types = heritage.extends_types.as_ref().map(|extends_types| {
            extends_types
                .iter()
                .map(|extends_type| self.lower_expression_to_type(module, *extends_type))
                .collect()
        });
        let implements_types = heritage.implements_types.as_ref().map(|implements_types| {
            implements_types
                .iter()
                .map(|implements_type| self.lower_expression_to_type(module, *implements_type))
                .collect()
        });
        Heritage {
            extends_types,
            implements_types,
            embedded_types: None,
        }
    }

    /// Whether the token string encodes a type literal with an explicit width.
    fn is_type_with_width(&self, prefix: &'static str, target: &str) -> Option<u16> {
        if let Some(target) = target.strip_prefix(prefix) {
            target.parse::<u16>().ok()
        } else {
            None
        }
    }

    /// Lower an expression string into a DIR type literal.
    pub fn lower_string_to_type(&mut self, string_id: StringId) -> Option<TypeLiteral> {
        let string = self.session.strings.get(string_id);

        // NOTE: we map the string to an AST type literal first
        let ast_literal = match string.as_ref() {
            // undefined
            "undefined" => Some(ast::TypeLiteral::Undefined),
            // unknown
            "unknown" => Some(ast::TypeLiteral::Unknown),
            // void
            "void" => Some(ast::TypeLiteral::Void),
            // null
            "null" => Some(ast::TypeLiteral::Null),
            // any
            "any" => Some(ast::TypeLiteral::Any),
            // never
            "never" => Some(ast::TypeLiteral::Never),
            // boolean
            "boolean" | "bool" => Some(ast::TypeLiteral::Boolean),
            // character
            "character" | "char" => Some(ast::TypeLiteral::Character),
            // string
            "string" | "str" => Some(ast::TypeLiteral::String),
            // number
            "number" => Some(ast::TypeLiteral::Number),
            // Self
            "Self" => Some(ast::TypeLiteral::Self_),
            // int (followed by number or nothing)
            "int" => Some(ast::TypeLiteral::Int(ast::IntType::Arbitrary {
                width: None,
                is_signed: true,
            })),
            "intp" => Some(ast::TypeLiteral::Int(ast::IntType::Pointer {
                is_signed: true,
            })),
            int_str if let Some(width) = self.is_type_with_width("int", int_str) => {
                Some(ast::TypeLiteral::Int(ast::IntType::Arbitrary {
                    width: Some(width),
                    is_signed: true,
                }))
            }
            int_str if let Some(width) = self.is_type_with_width("i", int_str) => {
                Some(ast::TypeLiteral::Int(ast::IntType::Arbitrary {
                    width: Some(width),
                    is_signed: true,
                }))
            }
            // uint (followed by number or nothing)
            "uint" => Some(ast::TypeLiteral::Int(ast::IntType::Arbitrary {
                width: None,
                is_signed: false,
            })),
            "uintp" => Some(ast::TypeLiteral::Int(ast::IntType::Pointer {
                is_signed: false,
            })),
            uint_str if let Some(width) = self.is_type_with_width("uint", uint_str) => {
                Some(ast::TypeLiteral::Int(ast::IntType::Arbitrary {
                    width: Some(width),
                    is_signed: false,
                }))
            }
            uint_str if let Some(width) = self.is_type_with_width("u", uint_str) => {
                Some(ast::TypeLiteral::Int(ast::IntType::Arbitrary {
                    width: Some(width),
                    is_signed: false,
                }))
            }
            // float (followed by number or nothing)
            "float" => Some(ast::TypeLiteral::Float(ast::FloatType { width: None })),
            float_str if let Some(width) = self.is_type_with_width("float", float_str) => {
                Some(ast::TypeLiteral::Float(ast::FloatType {
                    width: Some(width),
                }))
            }
            float_str if let Some(width) = self.is_type_with_width("f", float_str) => {
                Some(ast::TypeLiteral::Float(ast::FloatType {
                    width: Some(width),
                }))
            }
            // composite type
            _ => None,
        };

        // then map to DIR type literal (re-using the existing mapping)
        ast_literal.map(|ast_literal| self.lower_type_literal(&ast_literal))
    }
}
