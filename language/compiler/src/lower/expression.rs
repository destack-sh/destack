use dyst_ast as ast;
use dyst_dir::{
    Asynchrony, DependencySource, Expression, FunctionAbstraction, FunctionCardinality,
    FunctionKind, FunctionMode, IfKind, Module, NodeId, Path, PathBase, Runtime, Visibility,
};

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower visibility into a DIR visibility.
    #[inline]
    pub fn lower_visibility(&self, visibility: ast::Visibility) -> Visibility {
        match visibility {
            ast::Visibility::Public => Visibility::Public,
            ast::Visibility::Protected => Visibility::Protected,
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

    /// Lower if kind into a DIR if kind.
    #[inline]
    pub fn lower_if_kind(&self, kind: ast::IfKind) -> IfKind {
        match kind {
            ast::IfKind::If => IfKind::If,
            ast::IfKind::Ternary => IfKind::Ternary,
        }
    }

    /// Lower function kind into a DIR function kind.
    #[inline]
    pub fn lower_function_kind(&self, kind: ast::FunctionKind) -> FunctionKind {
        match kind {
            ast::FunctionKind::Function => FunctionKind::Function,
            ast::FunctionKind::Lambda => FunctionKind::Lambda,
        }
    }

    /// Lower asynchrony into a DIR asynchrony.
    #[inline]
    pub fn lower_asynchrony(&self, asynchrony: ast::Asynchrony) -> Asynchrony {
        match asynchrony {
            ast::Asynchrony::Sync => Asynchrony::Sync,
            ast::Asynchrony::Async => Asynchrony::Async,
        }
    }

    /// Lower function cardinality into a DIR function cardinality.
    #[inline]
    pub fn lower_function_cardinality(
        &self,
        cardinality: ast::FunctionCardinality,
    ) -> FunctionCardinality {
        match cardinality {
            ast::FunctionCardinality::Scalar => FunctionCardinality::Scalar,
            ast::FunctionCardinality::Generator => FunctionCardinality::Generator,
        }
    }

    /// Lower function mode into a DIR function mode.
    #[inline]
    pub fn lower_function_mode(&self, mode: ast::FunctionMode) -> FunctionMode {
        match mode {
            ast::FunctionMode::Getter => FunctionMode::Getter,
            ast::FunctionMode::Setter => FunctionMode::Setter,
            ast::FunctionMode::Constructor => FunctionMode::Constructor,
            ast::FunctionMode::New => FunctionMode::New,
            ast::FunctionMode::Call => FunctionMode::Call,
        }
    }

    /// Lower function abstraction into a DIR function abstraction.
    #[inline]
    pub fn lower_function_abstraction(
        &self,
        abstraction: ast::FunctionAbstraction,
    ) -> FunctionAbstraction {
        match abstraction {
            ast::FunctionAbstraction::Abstract => FunctionAbstraction::Abstract,
            ast::FunctionAbstraction::AbstractOverride => FunctionAbstraction::AbstractOverride,
            ast::FunctionAbstraction::ConcreteOverride => FunctionAbstraction::ConcreteOverride,
            ast::FunctionAbstraction::Concrete => FunctionAbstraction::Concrete,
        }
    }

    /// Lower an expression to a DIR expression.
    pub fn lower_expression(
        &mut self,
        module: &Module,
        expression_id: ast::NodeId<ast::Expression>,
    ) -> NodeId<Expression> {
        let expression = module.get(expression_id);
        let expression = match expression {
            ast::Expression::Block(block_id) => {
                let block_id = self.lower_block(module, *block_id);
                Expression::Block { block: block_id }
            }
            ast::Expression::Definition(definition_id) => {
                let definition_id = self.lower_definition(module, *definition_id);
                Expression::Definition {
                    definition: definition_id,
                }
            }

            ast::Expression::With { clauses, body } => {
                let clauses = clauses
                    .iter()
                    .map(|clause| self.lower_with_clause(module, *clause))
                    .collect();
                let body = body.map(|body| self.lower_block(module, body));
                Expression::With { clauses, body }
            }
            ast::Expression::Import {
                kind,
                target,
                alias,
                items,
                arguments,
            } => {
                let items = self.lower_dependency_items(
                    module,
                    expression_id,
                    *kind,
                    DependencySource::Import,
                    Some(*target),
                    alias.as_ref().copied(),
                    items.as_ref().map(|items| items.as_slice()),
                );
                let arguments = arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.lower_argument(module, *argument))
                        .collect()
                });
                let kind = self.lower_dependency_kind(*kind);
                Expression::Import {
                    kind,
                    items,
                    arguments,
                }
            }
            ast::Expression::Export {
                mode,
                kind,
                target,
                alias,
                items,
                value,
            } => {
                let items = self.lower_dependency_items(
                    module,
                    expression_id,
                    *kind,
                    DependencySource::Import,
                    target.as_ref().copied(),
                    alias.as_ref().copied(),
                    items.as_ref().map(|items| items.as_slice()),
                );
                let value = value.map(|value| self.lower_expression(module, value));
                let kind = self.lower_dependency_kind(*kind);
                Expression::Export {
                    mode: self.lower_export_type(*mode),
                    kind,
                    items: if items.is_empty() { None } else { Some(items) },
                    value,
                }
            }
            ast::Expression::Let {
                meta: _,
                mutability,
                pattern,
                ty,
                value,
            } => {
                let mutability = self.lower_scoped_mutability(module, mutability);
                let pattern = self.lower_pattern(module, *pattern);
                let ty = ty.map(|ty| self.lower_expression_to_type(module, ty));
                let value = value.map(|value| self.lower_expression(module, value));
                Expression::Let {
                    mutability,
                    pattern,
                    ty,
                    value,
                }
            }
            ast::Expression::LetType {
                meta,
                mutability,
                static_parameters,
                value,
            } => {
                let name = self.session.strings.intern_from(
                    &module.strings,
                    meta.name.expect("LetType must have a name").string(),
                );
                let mutability = mutability
                    .as_ref()
                    .map(|mutability| self.lower_mutability(*mutability));
                let static_parameters = static_parameters.as_ref().map(|params| {
                    params
                        .iter()
                        .map(|param| self.lower_parameter(module, *param))
                        .collect()
                });
                let value = self.lower_expression(module, *value);
                Expression::LetType {
                    mutability,
                    name,
                    static_parameters,
                    value,
                }
            }

            ast::Expression::Unary { operator, right } => {
                let right = self.lower_expression(module, *right);
                let operator = self.lower_unary_operator(*operator);
                Expression::Unary { operator, right }
            }

            ast::Expression::TypeUnary { operator, right } => {
                let right = self.lower_expression(module, *right);
                let operator = self.lower_type_unary_operator(*operator);
                Expression::TypeUnary { operator, right }
            }

            ast::Expression::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let mutability = mutability
                    .as_ref()
                    .map(|mutability| self.lower_scoped_mutability(module, mutability));
                let variance = variance.map(|variance| self.lower_variance_bound(variance));
                let right = self.lower_expression(module, *right);
                Expression::ValueOf {
                    mutability,
                    variance,
                    right,
                }
            }
            ast::Expression::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let mutability = mutability
                    .as_ref()
                    .map(|mutability| self.lower_scoped_mutability(module, mutability));
                let variance = variance.map(|variance| self.lower_variance_bound(variance));
                let right = self.lower_expression(module, *right);
                Expression::ReferenceOf {
                    mutability,
                    variance,
                    right,
                }
            }
            ast::Expression::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.lower_expression(module, *left);
                let right = self.lower_expression(module, *right);
                let operator = self.lower_binary_operator(*operator);
                Expression::Binary {
                    left,
                    operator,
                    right,
                }
            }
            ast::Expression::TypeBinary {
                left,
                operator,
                right,
            } => {
                let left = self.lower_expression(module, *left);
                let right = self.lower_expression(module, *right);
                let operator = self.lower_type_binary_operator(*operator);
                Expression::TypeBinary {
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
                let left = self.lower_expression(module, *left);
                let right = self.lower_expression(module, *right);
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

            ast::Expression::Member {
                left: receiver,
                path,
                static_arguments,
            } => {
                let left = self.lower_expression(module, *receiver);
                let path = self.lower_path(module, path);
                let static_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.lower_argument(module, *argument))
                        .collect()
                });
                Expression::Member {
                    left,
                    path,
                    static_arguments,
                }
            }
            ast::Expression::Call {
                position: _,
                left: receiver,
                dynamic_arguments,
            } => {
                let receiver = self.lower_expression(module, *receiver);
                let dynamic_arguments = dynamic_arguments
                    .iter()
                    .map(|argument| self.lower_argument(module, *argument))
                    .collect();
                Expression::Call {
                    left: receiver,
                    dynamic_arguments,
                }
            }
            ast::Expression::Index {
                position: _,
                left: receiver,
                index,
            } => {
                let receiver = self.lower_expression(module, *receiver);
                let index = index.map(|index| self.lower_expression(module, index));
                Expression::Index {
                    left: receiver,
                    right: index,
                }
            }
            ast::Expression::Maybe { position: _, left } => {
                let left = self.lower_expression(module, *left);
                Expression::Maybe { left }
            }
            ast::Expression::Must { position: _, left } => {
                let left = self.lower_expression(module, *left);
                Expression::Must { left }
            }

            ast::Expression::Path {
                path,
                static_arguments,
            } => {
                let path = self.lower_path(module, path);
                let static_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.lower_argument(module, *argument))
                        .collect()
                });
                Expression::Path {
                    path,
                    static_arguments,
                }
            }
            ast::Expression::ScalarLiteral(value) => {
                let value = self.lower_scalar_literal(module, value);
                Expression::ScalarLiteral { value }
            }
            ast::Expression::TypeLiteral(value) => {
                if *value == ast::TypeLiteral::Self_ {
                    Expression::Path {
                        path: Path::UnevaluatedBase {
                            base: PathBase::SelfType,
                        },
                        static_arguments: None,
                    }
                } else {
                    let value = self.lower_type_literal(value);
                    Expression::TypeLiteral { value }
                }
            }
            ast::Expression::StructLiteral { ty, fields } => {
                let ty = ty.map(|ty| self.lower_expression_to_type(module, ty));
                let fields = fields
                    .iter()
                    .map(|field| self.lower_argument(module, *field))
                    .collect();
                Expression::StructLiteral { ty, fields }
            }
            ast::Expression::TupleLiteral { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| self.lower_argument(module, *element))
                    .collect();
                Expression::TupleLiteral { ty: None, elements }
            }
            ast::Expression::ArrayLiteral { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| self.lower_argument(module, *element))
                    .collect();
                Expression::ArrayLiteral { elements }
            }

            ast::Expression::If {
                kind,
                condition,
                then_expression,
                else_expression,
            } => {
                let kind = self.lower_if_kind(*kind);
                let condition = self.lower_expression(module, *condition);
                let then_expression = self.lower_expression(module, *then_expression);
                let else_expression = else_expression
                    .map(|else_expression| self.lower_expression(module, else_expression));
                Expression::If {
                    kind,
                    condition,
                    then_expression,
                    else_expression,
                }
            }
            ast::Expression::Break { label, value } => {
                let target = label.map(|label| self.lower_label(module, label));
                let value = value.map(|value| self.lower_expression(module, value));
                Expression::Break { target, value }
            }
            ast::Expression::Continue { label } => {
                let target = label.map(|label| self.lower_label(module, label));
                Expression::Continue { target }
            }
            ast::Expression::Return { value } => {
                let value = value.map(|value| self.lower_expression(module, value));
                Expression::Return { value }
            }

            ast::Expression::Error => Expression::Error,

            _ => todo!("Compiler::lower_expression {:?}", expression),
        };
        self.session
            .tree
            .insert_from_ast(expression, module.id, expression_id)
    }
}
