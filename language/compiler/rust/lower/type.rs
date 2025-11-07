use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{Module, NodeId, Type, TypeLiteral};

impl<'a> Compiler<'a> {
    /// Lower a an expression into a type (without evaluating it at all).
    pub fn lower_expression_to_type(
        &mut self,
        module: &Module,
        expression_id: ast::NodeId<ast::Expression>,
    ) -> NodeId<Type> {
        let expression = self.lower_expression(module, expression_id);
        let type_id = self.tree.insert_from_ast(
            Type::UnevaluatedExpression(expression),
            module.id,
            expression_id,
        );
        self.tree
            .alias_from_ast(module.id, expression_id.id, type_id);
        type_id
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
    pub fn lower_string_to_type(&mut self, string_id: ast::StringId) -> Option<TypeLiteral> {
        let string = self.get_string(string_id);

        // NOTE: we map the string to an AST type literal first
        let ast_literal = match string {
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
            "int" => Some(ast::TypeLiteral::Int(ast::IntType {
                width: None,
                is_signed: true,
            })),
            int_str if let Some(width) = self.is_type_with_width("int", int_str) => {
                Some(ast::TypeLiteral::Int(ast::IntType {
                    width: Some(width),
                    is_signed: true,
                }))
            }
            int_str if let Some(width) = self.is_type_with_width("i", int_str) => {
                Some(ast::TypeLiteral::Int(ast::IntType {
                    width: Some(width),
                    is_signed: true,
                }))
            }
            // uint (followed by number or nothing)
            "uint" => Some(ast::TypeLiteral::Int(ast::IntType {
                width: None,
                is_signed: false,
            })),
            uint_str if let Some(width) = self.is_type_with_width("uint", uint_str) => {
                Some(ast::TypeLiteral::Int(ast::IntType {
                    width: Some(width),
                    is_signed: false,
                }))
            }
            uint_str if let Some(width) = self.is_type_with_width("u", uint_str) => {
                Some(ast::TypeLiteral::Int(ast::IntType {
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
