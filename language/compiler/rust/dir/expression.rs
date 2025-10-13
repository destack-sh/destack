use dyst_ast as ast;
use dyst_dir::{
    Expression, FunctionStyle, Mutability, NodeId, Runtime, ScopedMutability, Visibility,
};
use dyst_source::SourceId;

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower visibility into a DIR visibility.
    #[inline]
    pub fn lower_visibility(&self, visibility: ast::Visibility) -> Visibility {
        match visibility {
            ast::Visibility::Public => Visibility::Public,
            ast::Visibility::Private => Visibility::Private,
        }
    }

    /// Lower runtime into a DIR runtime.
    #[inline]
    pub fn lower_runtime(&self, runtime: ast::Runtime) -> Runtime {
        match runtime {
            ast::Runtime::Dynamic => Runtime::Dynamic,
            ast::Runtime::Static => Runtime::Static,
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

    /// Lower function style into a DIR function style.
    #[inline]
    pub fn lower_function_style(&self, style: ast::FunctionStyle) -> FunctionStyle {
        match style {
            ast::FunctionStyle::Function => FunctionStyle::Function,
            ast::FunctionStyle::Lambda => FunctionStyle::Lambda,
        }
    }

    /// Lower scoped mutability into a DIR scoped mutability.
    #[inline]
    pub fn lower_scoped_mutability(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        scoped_mutability: &ast::ScopedMutability,
    ) -> ScopedMutability {
        match scoped_mutability {
            ast::ScopedMutability::Unscoped { mutability } => ScopedMutability::Unscoped {
                mutability: self.lower_mutability(*mutability),
            },
            ast::ScopedMutability::Scoped { mutability, scopes } => ScopedMutability::Scoped {
                mutability: self.lower_mutability(*mutability),
                scopes: scopes
                    .iter()
                    .map(|scope| self.lower_path(source_id, ast, scope))
                    .collect(),
            },
        }
    }

    /// Lower an expression to a DIR expression.
    pub fn lower_expression(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        expression_id: ast::NodeId<ast::Expression>,
    ) -> NodeId<Expression> {
        let expression = ast.get(expression_id);
        let expression = match expression {
            ast::Expression::Block(block_id) => {
                let block_id = self.lower_block(source_id, ast, *block_id);
                Expression::Block { block: block_id }
            }
            ast::Expression::Definition(definition_id) => {
                let definition_id = self.lower_definition(source_id, ast, *definition_id);
                Expression::Definition {
                    definition: definition_id,
                }
            }

            ast::Expression::With { clauses, body } => {
                let clauses = clauses
                    .iter()
                    .map(|clause| self.lower_with_clause(source_id, ast, *clause))
                    .collect();
                let body = body.map(|body| self.lower_block(source_id, ast, body));
                Expression::With { clauses, body }
            }
            ast::Expression::Use {
                visibility,
                clauses,
                body,
            } => {
                let visibility = visibility.map(|visibility| self.lower_visibility(visibility));
                let body = body.map(|body| self.lower_block(source_id, ast, body));
                let items = clauses
                    .iter()
                    .flat_map(|clause| self.lower_use_clause(source_id, ast, *clause))
                    .collect();
                Expression::Use {
                    visibility,
                    items,
                    body,
                }
            }
            ast::Expression::Let {
                mutability,
                visibility,
                pattern,
                ty,
                value,
            } => {
                let mutability = self.lower_scoped_mutability(source_id, ast, mutability);
                let visibility = visibility.map(|visibility| self.lower_visibility(visibility));
                let pattern = self.lower_pattern(source_id, ast, *pattern);
                let ty = ty.map(|ty| self.lower_expression_to_type(source_id, ast, ty));
                let value = value.map(|value| self.lower_expression(source_id, ast, value));
                Expression::Let {
                    mutability,
                    visibility,
                    pattern,
                    ty,
                    value,
                }
            }
            ast::Expression::Type {
                name,
                visibility,
                value,
            } => {
                let name = name.map(|name| self.intern_string(source_id, name));
                let visibility = visibility.map(|visibility| self.lower_visibility(visibility));
                let value = self.lower_expression(source_id, ast, *value);
                Expression::Type {
                    name,
                    visibility,
                    value,
                }
            }

            ast::Expression::Unary { operator, right } => {
                let right = self.lower_expression(source_id, ast, *right);
                let operator = self.lower_unary_operator(*operator);
                Expression::Unary { operator, right }
            }
            ast::Expression::Reference { mutability, right } => {
                let right = self.lower_expression(source_id, ast, *right);
                let mutability = self.lower_scoped_mutability(source_id, ast, mutability);
                Expression::Reference { mutability, right }
            }
            ast::Expression::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.lower_expression(source_id, ast, *left);
                let right = self.lower_expression(source_id, ast, *right);
                let operator = self.lower_binary_operator(*operator);
                Expression::Binary {
                    left,
                    operator,
                    right,
                }
            }
            ast::Expression::Assign {
                left,
                operator,
                right,
            } => {
                let left = self.lower_expression(source_id, ast, *left);
                let right = self.lower_expression(source_id, ast, *right);
                let operator = self.lower_assign_operator(*operator);
                if let Some(operator) = operator {
                    Expression::AssignBinary {
                        left,
                        operator,
                        right,
                    }
                } else {
                    Expression::AssignDirect { left, right }
                }
            }

            ast::Expression::Member { receiver, path } => {
                let left = self.lower_expression(source_id, ast, *receiver);
                let path = self.lower_path(source_id, ast, path);
                Expression::Member { left, path }
            }
            ast::Expression::Call {
                runtime,
                receiver,
                dynamic_arguments,
            } => {
                let runtime = runtime.map(|runtime| self.lower_runtime(runtime));
                let receiver = self.lower_expression(source_id, ast, *receiver);
                let dynamic_arguments = dynamic_arguments
                    .iter()
                    .map(|argument| self.lower_argument(source_id, ast, *argument))
                    .collect();
                Expression::Call {
                    runtime,
                    left: receiver,
                    dynamic_arguments,
                }
            }
            ast::Expression::Index { receiver, index } => {
                let receiver = self.lower_expression(source_id, ast, *receiver);
                let index = index.map(|index| self.lower_expression(source_id, ast, index));
                Expression::Index {
                    left: receiver,
                    right: index,
                }
            }
            ast::Expression::Maybe(expr) => {
                let expr = self.lower_expression(source_id, ast, *expr);
                Expression::Maybe { left: expr }
            }
            ast::Expression::Must(expr) => {
                let expr = self.lower_expression(source_id, ast, *expr);
                Expression::Must { left: expr }
            }

            ast::Expression::Path {
                path,
                static_arguments: _,
            } => {
                let path = self.lower_path(source_id, ast, path);
                Expression::Path { path }
            }
            ast::Expression::ScalarLiteral(value) => {
                let value = self.lower_scalar_literal(source_id, ast, value);
                Expression::ScalarLiteral { value }
            }
            ast::Expression::TypeLiteral(value) => {
                let value = self.lower_type_literal(value);
                Expression::TypeLiteral { value }
            }
            ast::Expression::StructLiteral { ty, fields } => {
                let ty = ty.map(|ty| self.lower_expression_to_type(source_id, ast, ty));
                let fields = fields
                    .iter()
                    .map(|field| self.lower_argument(source_id, ast, *field))
                    .collect();
                Expression::StructLiteral { ty, fields }
            }
            ast::Expression::TupleLiteral { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| self.lower_argument(source_id, ast, *element))
                    .collect();
                Expression::TupleLiteral { elements }
            }
            ast::Expression::ArrayLiteral { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| self.lower_expression(source_id, ast, *element))
                    .collect();
                Expression::ArrayLiteral { elements }
            }

            ast::Expression::If {
                runtime,
                style: _,
                condition,
                then_expression,
                else_expression,
            } => {
                let runtime = runtime.map(|runtime| self.lower_runtime(runtime));
                let condition = self.lower_expression(source_id, ast, *condition);
                let then_expression = self.lower_expression(source_id, ast, *then_expression);
                let else_expression = else_expression
                    .map(|else_expression| self.lower_expression(source_id, ast, else_expression));
                Expression::If {
                    runtime,
                    condition,
                    then_expression,
                    else_expression,
                }
            }
            ast::Expression::Break { label, value } => {
                let destination = label.map(|label| self.lower_label(source_id, ast, label));
                let value = value.map(|value| self.lower_expression(source_id, ast, value));
                Expression::Break { destination, value }
            }
            ast::Expression::Continue { label } => {
                let destination = label.map(|label| self.lower_label(source_id, ast, label));
                Expression::Continue { destination }
            }
            ast::Expression::Return { value } => {
                let value = value.map(|value| self.lower_expression(source_id, ast, value));
                Expression::Return { value }
            }

            ast::Expression::Error => Expression::Error,

            _ => todo!("Compiler::lower_expression {:?}", expression),
        };
        self.tree.insert(expression, source_id, expression_id)
    }
}
